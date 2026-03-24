use anyhow::Result;
use futures::TryStreamExt;
use http_body_util::BodyExt;
use hyper::{
    HeaderMap, StatusCode,
    header::{ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, HeaderValue, SERVER as HK_SERVER, VARY},
};
use percent_encoding::percent_decode_str;
use std::{io, path::Path};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{
    http::{HTTPRequest, HTTPResponse, HV_SERVER, response_404},
    workers::StaticFilesConfig,
};

#[derive(Clone, Copy)]
pub(crate) enum Encoding {
    Plain,
    Gzip,
    Brotli,
    Zstd,
}

impl Encoding {
    fn as_header_value(self) -> Option<&'static str> {
        match self {
            Encoding::Plain => None,
            Encoding::Gzip => Some("gzip"),
            Encoding::Brotli => Some("br"),
            Encoding::Zstd => Some("zstd"),
        }
    }
}

const SUPPORTED_ENCODINGS: &[(&str, &str, Encoding)] = &[
    ("zstd", ".zst", Encoding::Zstd),
    ("br", ".br", Encoding::Brotli),
    ("gzip", ".gz", Encoding::Gzip),
];

const SIDECAR_CACHE_MAX_SIZE: usize = 10_000;

/// Check if a sidecar file exists, using the cache to avoid repeated filesystem checks.
/// Results are cached lazily on first access. When cache exceeds max size, ~half of
/// entries are evicted randomly to prevent full cache flush attacks.
#[inline]
fn sidecar_exists(config: &StaticFilesConfig, path: &str) -> bool {
    {
        let cache = config.sidecar_cache.read().unwrap();
        if let Some(&exists) = cache.get(path) {
            return exists;
        }
    }

    let exists = Path::new(path).exists();
    {
        let mut cache = config.sidecar_cache.write().unwrap();
        if cache.len() >= SIDECAR_CACHE_MAX_SIZE {
            use std::hash::{Hash, Hasher};
            cache.retain(|k, _| {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                k.hash(&mut h);
                h.finish().is_multiple_of(2)
            });
        }
        cache.insert(path.to_string(), exists);
    }
    exists
}

/// Parse Accept-Encoding header and return encodings sorted by client priority (q-value).
/// Returns a Vec of (`encoding_name`, `q_value`) sorted by `q_value` descending.
/// Stable sort preserves client order for equal q-values.
/// Q-values are clamped to 0.0-1.0 per RFC 7231.
fn parse_accept_encoding(header_value: &str) -> Vec<(&str, f32)> {
    let mut encodings: Vec<(&str, f32)> = header_value
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }

            let mut iter = part.splitn(2, ';');
            let encoding = iter.next()?.trim();

            let q_value = iter
                .next()
                .and_then(|params| {
                    params.split(';').find_map(|p| {
                        let p = p.trim();
                        if let Some(q_str) = p.strip_prefix("q=") {
                            q_str.trim().parse::<f32>().ok()
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);

            Some((encoding, q_value))
        })
        .collect();

    encodings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    encodings
}

/// Resolve the static file path and encoding based on Accept-Encoding header.
/// Returns the path to serve and the encoding to use.
/// Respects client q-value priorities, with client order as tiebreaker for equal q-values.
/// Uses lazy caching to avoid repeated filesystem checks.
fn resolve_precompressed(config: &StaticFilesConfig, base_path: &str, req: &HTTPRequest) -> (String, Encoding) {
    let accept_encoding = req
        .headers()
        .get(ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if accept_encoding.is_empty() {
        return (base_path.to_string(), Encoding::Plain);
    }

    let client_prefs = parse_accept_encoding(accept_encoding);

    let rejected: Vec<&str> = client_prefs
        .iter()
        .filter(|(_, q)| *q == 0.0)
        .map(|(name, _)| *name)
        .collect();

    for (encoding_name, q_value) in &client_prefs {
        if *q_value == 0.0 {
            continue;
        }

        if *encoding_name == "*" {
            for (supported_name, ext, encoding) in SUPPORTED_ENCODINGS {
                if rejected.iter().any(|r| r.eq_ignore_ascii_case(supported_name)) {
                    continue;
                }
                let compressed_path = format!("{base_path}{ext}");
                if sidecar_exists(config, &compressed_path) {
                    return (compressed_path, *encoding);
                }
            }
            continue;
        }

        for (supported_name, ext, encoding) in SUPPORTED_ENCODINGS {
            if encoding_name.eq_ignore_ascii_case(supported_name) {
                let compressed_path = format!("{base_path}{ext}");
                if sidecar_exists(config, &compressed_path) {
                    return (compressed_path, *encoding);
                }
                break;
            }
        }
    }

    (base_path.to_string(), Encoding::Plain)
}

#[inline(always)]
pub(crate) fn match_static_file(config: &StaticFilesConfig, req: &HTTPRequest) -> Option<Result<(String, Encoding)>> {
    let decoded_uri_path = percent_decode_str(req.uri().path()).decode_utf8_lossy();
    for (prefix, mount_point) in &config.mounts {
        if let Some(file_path) = decoded_uri_path.strip_prefix(prefix) {
            #[cfg(not(windows))]
            let fpath = format!("{mount_point}{file_path}");
            #[cfg(windows)]
            let fpath = format!("{mount_point}{}", file_path.replace("/", "\\"));
            // PERF: sync syscall below is in hot path
            match Path::new(&fpath).canonicalize() {
                Ok(mut full_path) => {
                    if full_path.is_dir() {
                        match &config.dir_to_file {
                            Some(rewrite_file) => {
                                full_path = full_path.join(rewrite_file);
                            }
                            None => return Some(Err(anyhow::anyhow!("dir"))),
                        }
                    }
                    #[cfg(windows)]
                    let full_path_str = &full_path.display().to_string()[4..];
                    #[cfg(not(windows))]
                    let full_path_str = full_path.to_str()?;
                    if full_path_str.starts_with(mount_point) {
                        let base_path = full_path_str.to_owned();
                        let (path, encoding) = if config.precompressed {
                            resolve_precompressed(config, &base_path, req)
                        } else {
                            (base_path, Encoding::Plain)
                        };
                        return Some(Ok((path, encoding)));
                    }
                    return Some(Err(anyhow::anyhow!("outside mount path")));
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    return Some(Err(err.into()));
                }
                _ => {}
            }
        }
    }
    None
}

pub(crate) async fn serve_static_file(config: &StaticFilesConfig, path: String, encoding: Encoding) -> HTTPResponse {
    match File::open(&path).await {
        Ok(file) => {
            let mime_path = match encoding {
                Encoding::Brotli => path.strip_suffix(".br").unwrap_or(&path),
                Encoding::Gzip => path.strip_suffix(".gz").unwrap_or(&path),
                Encoding::Zstd => path.strip_suffix(".zst").unwrap_or(&path),
                Encoding::Plain => &path,
            };
            let mime = mime_guess::from_path(mime_path).first();
            let stream = ReaderStream::with_capacity(file, 131_072);
            let stream_body = http_body_util::StreamBody::new(stream.map_ok(hyper::body::Frame::data));
            let mut headers = HeaderMap::new();
            let mut res = hyper::Response::new(BodyExt::map_err(stream_body, std::convert::Into::into).boxed());

            headers.insert(HK_SERVER, HV_SERVER);
            if let Some(expires) = &config.expires {
                headers.insert(
                    CACHE_CONTROL,
                    HeaderValue::from_str(&format!("max-age={expires}")).unwrap(),
                );
            }
            if let Some(mime) = mime
                && let Ok(hv) = HeaderValue::from_str(mime.essence_str())
            {
                headers.insert(CONTENT_TYPE, hv);
            }
            if let Some(encoding_value) = encoding.as_header_value() {
                headers.insert(CONTENT_ENCODING, HeaderValue::from_static(encoding_value));
                headers.append(VARY, HeaderValue::from_name(ACCEPT_ENCODING));
            }

            *res.status_mut() = StatusCode::from_u16(200).unwrap();
            *res.headers_mut() = headers;
            res
        }
        Err(_) => {
            log::info!("Request static file {path} not found");
            response_404()
        }
    }
}

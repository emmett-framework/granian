use anyhow::Result;
use futures::TryStreamExt;
use http_body_util::BodyExt;
use hyper::{
    HeaderMap, StatusCode,
    header::{ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, HeaderValue, SERVER as HK_SERVER, VARY},
};
use std::{io, path::Path};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{
    http::{HTTPRequest, HTTPResponse, HV_SERVER, response_404},
    utils::header_contains_value,
    workers::StaticFilesConfig,
};

#[inline(always)]
pub(crate) fn match_static_file(uri_path: &str, config: &StaticFilesConfig) -> Option<Result<String>> {
    if let Some(file_path) = uri_path.strip_prefix(&config.prefix) {
        let mount_point = &config.mount;
        #[cfg(not(windows))]
        let fpath = format!("{mount_point}{file_path}");
        #[cfg(windows)]
        let fpath = format!("{mount_point}{}", file_path.replace("/", "\\"));
        match Path::new(&fpath).canonicalize() {
            Ok(full_path) => {
                #[cfg(windows)]
                let full_path = &full_path.display().to_string()[4..];
                if full_path.starts_with(mount_point) {
                    #[cfg(not(windows))]
                    return full_path.to_str().map(ToOwned::to_owned).map(Ok);
                    #[cfg(windows)]
                    return Some(Ok(full_path.to_owned()));
                }
                return Some(Err(anyhow::anyhow!("outside mount path")));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Some(Err(err.into()));
            }
            _ => {}
        }
    }
    None
}

pub(crate) async fn serve_static_file(req: HTTPRequest, path: String, config: &StaticFilesConfig) -> HTTPResponse {
    let build_response = |file, content_encoding: Option<&str>| {
        let mime = mime_guess::from_path(&path).first();
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
        if let Some(mime) = mime {
            if let Ok(hv) = HeaderValue::from_str(mime.essence_str()) {
                headers.insert(CONTENT_TYPE, hv);
            }
        }
        if let Some(content_encoding) = content_encoding {
            headers.insert(CONTENT_ENCODING, HeaderValue::from_str(content_encoding).unwrap());
            headers.append(VARY, HeaderValue::from_name(ACCEPT_ENCODING));
        }

        *res.status_mut() = StatusCode::from_u16(200).unwrap();
        *res.headers_mut() = headers;
        res
    };

    if header_contains_value(req.headers(), ACCEPT_ENCODING, "br") {
        if let Ok(file) = File::open(format!("{path}.br")).await {
            return build_response(file, Some("br"));
        }
    }
    if header_contains_value(req.headers(), ACCEPT_ENCODING, "gzip") {
        if let Ok(file) = File::open(format!("{path}.gz")).await {
            return build_response(file, Some("gzip"));
        }
    }

    match File::open(&path).await {
        Ok(file) => build_response(file, None),
        Err(_) => {
            log::info!("Request static file {path} not found");
            response_404()
        }
    }
}

use anyhow::Result;
use futures::TryStreamExt;
use http_body_util::BodyExt;
use hyper::{
    HeaderMap, StatusCode,
    header::{HeaderValue, SERVER as HK_SERVER},
};
use percent_encoding::percent_decode_str;
use std::{io, path::Path};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::http::{HTTPResponse, HV_SERVER, response_404};

#[inline(always)]
pub(crate) fn match_static_file(uri_path: &str, prefix: &str, mount_point: &str) -> Option<Result<String>> {
    let decoded_uri_path = percent_decode_str(uri_path).decode_utf8_lossy();
    if let Some(file_path) = decoded_uri_path.strip_prefix(prefix) {
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

pub(crate) async fn serve_static_file(path: String, expires: Option<String>) -> HTTPResponse {
    match File::open(&path).await {
        Ok(file) => {
            let mime = mime_guess::from_path(path).first();
            let stream = ReaderStream::with_capacity(file, 131_072);
            let stream_body = http_body_util::StreamBody::new(stream.map_ok(hyper::body::Frame::data));
            let mut headers = HeaderMap::new();
            let mut res = hyper::Response::new(BodyExt::map_err(stream_body, std::convert::Into::into).boxed());

            headers.insert(HK_SERVER, HV_SERVER);
            if let Some(expires) = expires {
                headers.insert(
                    "cache-control",
                    HeaderValue::from_str(&format!("max-age={expires}")).unwrap(),
                );
            }
            if let Some(mime) = mime
                && let Ok(hv) = HeaderValue::from_str(mime.essence_str())
            {
                headers.insert("content-type", hv);
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

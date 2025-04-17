use futures::TryStreamExt;
use http_body_util::BodyExt;
use hyper::{
    header::{HeaderValue, SERVER as HK_SERVER},
    HeaderMap, StatusCode,
};
use std::path::Path;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::http::{response_404, HTTPResponse, HV_SERVER};

#[inline(always)]
pub(crate) fn match_static_file(uri_path: &str, prefix: &str, mount_point: &str) -> Option<String> {
    if let Some(file_path) = uri_path.strip_prefix(prefix) {
        let fpath = format!("{mount_point}{file_path}");
        if let Ok(full_path) = Path::new(&fpath).canonicalize() {
            if full_path.starts_with(mount_point) {
                return full_path.to_str().map(ToOwned::to_owned);
            }
        }
    }
    None
}

pub(crate) async fn serve_static_file(path: String, expires: String) -> HTTPResponse {
    match File::open(&path).await {
        Ok(file) => {
            let mime = mime_guess::from_path(path).first();
            let stream = ReaderStream::with_capacity(file, 131_072);
            let stream_body = http_body_util::StreamBody::new(stream.map_ok(hyper::body::Frame::data));
            let mut headers = HeaderMap::new();
            let mut res = hyper::Response::new(BodyExt::map_err(stream_body, std::convert::Into::into).boxed());

            headers.insert(HK_SERVER, HV_SERVER);
            headers.insert(
                "cache-control",
                HeaderValue::from_str(&format!("max-age={expires}")).unwrap(),
            );
            if let Some(mime) = mime {
                if let Ok(hv) = HeaderValue::from_str(mime.essence_str()) {
                    headers.insert("content-type", hv);
                }
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

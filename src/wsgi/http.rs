use hyper::{
    header::{HeaderName, HeaderValue, SERVER as HK_SERVER},
    Response,
};
use std::net::SocketAddr;

use super::callbacks::call_http;
use crate::{
    callbacks::CallbackWrapper,
    http::{response_500, HTTPRequest, HTTPResponse, HTTPResponseBody, HV_SERVER},
    runtime::RuntimeRef,
    utils::log_application_callable_exception,
};

#[inline(always)]
fn build_response(status: i32, pyheaders: Vec<(String, String)>, body: HTTPResponseBody) -> HTTPResponse {
    let mut res = Response::new(body);
    *res.status_mut() = hyper::StatusCode::from_u16(status as u16).unwrap();
    let headers = res.headers_mut();
    headers.insert(HK_SERVER, HV_SERVER);
    for (key, val) in pyheaders {
        headers.append(
            HeaderName::from_bytes(key.as_bytes()).unwrap(),
            HeaderValue::from_str(&val).unwrap(),
        );
    }
    res
}

#[inline]
pub(crate) async fn handle(
    rt: RuntimeRef,
    callback: CallbackWrapper,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    req: HTTPRequest,
    scheme: &str,
) -> HTTPResponse {
    let (parts, body) = req.into_parts();
    if let Ok(res) = call_http(rt, callback, server_addr, client_addr, scheme, parts, body).await {
        match res {
            Ok((status, headers, body)) => {
                return build_response(status, headers, body);
            }
            Err(ref err) => {
                log_application_callable_exception(err);
            }
        }
    } else {
        log::error!("WSGI protocol failure");
    }
    response_500()
}

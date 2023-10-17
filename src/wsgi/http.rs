use hyper::{
    header::{HeaderName, HeaderValue, SERVER as HK_SERVER},
    Body, Request, Response,
};
use std::net::SocketAddr;

use super::{callbacks::call_http, types::WSGIScope as Scope};
use crate::{
    callbacks::CallbackWrapper,
    http::{response_500, HV_SERVER},
    runtime::RuntimeRef,
};

#[inline(always)]
fn build_response(status: i32, pyheaders: Vec<(String, String)>, body: Body) -> Response<Body> {
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

pub(crate) async fn handle(
    _rt: RuntimeRef,
    callback: CallbackWrapper,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    req: Request<Body>,
    scheme: &str,
) -> Response<Body> {
    let scope = Scope::new(scheme, server_addr, client_addr, req).await;
    if let Ok(res) = call_http(callback, scope).await {
        if let Ok((status, headers, body)) = res {
            return build_response(status, headers, body);
        }
        log::warn!("Application callable raised an exception");
    } else {
        log::error!("WSGI protocol failure");
    }
    response_500()
}

use hyper::{
    header::{HeaderName, HeaderValue, SERVER as HK_SERVER},
    Body, Request, Response,
};
use std::net::SocketAddr;

use super::{
    callbacks::{call_rtb_http, call_rtt_http},
    types::WSGIScope as Scope,
};
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

pub(crate) async fn handle_rtt(
    _rt: RuntimeRef,
    callback: CallbackWrapper,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    req: Request<Body>,
    scheme: &str,
) -> Response<Body> {
    if let Ok(res) = call_rtt_http(callback, Scope::new(scheme, server_addr, client_addr, req).await).await {
        if let Ok((status, headers, body)) = res {
            return build_response(status, headers, body);
        }
        log::warn!("Application callable raised an exception");
    } else {
        log::error!("WSGI protocol failure");
    }
    response_500()
}

pub(crate) async fn handle_rtb(
    _rt: RuntimeRef,
    callback: CallbackWrapper,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    req: Request<Body>,
    scheme: &str,
) -> Response<Body> {
    match call_rtb_http(callback, Scope::new(scheme, server_addr, client_addr, req).await) {
        Ok((status, headers, body)) => build_response(status, headers, body),
        _ => {
            log::warn!("Application callable raised an exception");
            response_500()
        }
    }
}

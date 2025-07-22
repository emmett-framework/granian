use hyper::Response;
use std::sync::Arc;
use tokio::sync::Notify;

use super::callbacks::call_http;
use crate::{
    callbacks::ArcCBScheduler,
    http::{HTTPProto, HTTPRequest, HTTPResponse, HTTPResponseBody, response_500},
    net::SockAddr,
    runtime::RuntimeRef,
};

#[inline(always)]
fn build_response(status: u16, pyheaders: hyper::HeaderMap, body: HTTPResponseBody) -> HTTPResponse {
    let mut res = Response::new(body);
    *res.status_mut() = hyper::StatusCode::from_u16(status).unwrap();
    *res.headers_mut() = pyheaders;
    res
}

#[inline]
pub(crate) async fn handle(
    rt: RuntimeRef,
    _disconnect_guard: Arc<Notify>,
    callback: ArcCBScheduler,
    server_addr: SockAddr,
    client_addr: SockAddr,
    req: HTTPRequest,
    scheme: HTTPProto,
) -> HTTPResponse {
    let (parts, body) = req.into_parts();
    if let Ok((status, headers, body)) = call_http(rt, callback, server_addr, client_addr, scheme, parts, body).await {
        return build_response(status, headers, body);
    }

    log::error!("WSGI protocol failure");
    response_500()
}

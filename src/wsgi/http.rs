use hyper::{
    Body,
    Request,
    Response,
    header::{SERVER as HK_SERVER, HeaderName, HeaderValue}
};
use std::net::SocketAddr;

use crate::{
    callbacks::CallbackWrapper,
    http::{HV_SERVER, response_500},
    runtime::RuntimeRef,
};
use super::{
    callbacks::call_http,
    types::WSGIScope as Scope
};


pub(crate) async fn handle_request(
    _rt: RuntimeRef,
    callback: CallbackWrapper,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    req: Request<Body>,
    scheme: &str
) -> Response<Body> {
    match call_http(
        callback,
        Scope::new(scheme, server_addr, client_addr, req).await
    ).await {
        Ok((status, pyheaders, body)) => {
            let mut res = Response::new(Body::from(body));
            *res.status_mut() = hyper::StatusCode::from_u16(status as u16).unwrap();
            let headers = res.headers_mut();
            headers.insert(HK_SERVER, HV_SERVER);
            for (key, val) in pyheaders {
                headers.insert(
                    HeaderName::from_bytes(key.as_bytes()).unwrap(),
                    HeaderValue::from_str(&val).unwrap()
                );
            }
            res
        },
        _ => response_500()
    }
}

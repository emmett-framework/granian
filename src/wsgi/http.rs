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
    callbacks::{call_rtb_http, call_rtt_http},
    types::WSGIScope as Scope
};


macro_rules! handle_request {
    ($func_name:ident, $handler:expr) => {
        pub(crate) async fn $func_name(
            _rt: RuntimeRef,
            callback: CallbackWrapper,
            server_addr: SocketAddr,
            client_addr: SocketAddr,
            req: Request<Body>,
            scheme: &str
        ) -> Response<Body> {
            match $handler(
                callback,
                Scope::new(scheme, server_addr, client_addr, req).await
            ).await {
                Ok((status, pyheaders, body)) => {
                    let mut res = Response::new(Body::from(body));
                    *res.status_mut() = hyper::StatusCode::from_u16(status as u16).unwrap();
                    let headers = res.headers_mut();
                    headers.insert(HK_SERVER, HV_SERVER);
                    for (key, val) in pyheaders {
                        headers.append(
                            HeaderName::from_bytes(key.as_bytes()).unwrap(),
                            HeaderValue::from_str(&val).unwrap()
                        );
                    }
                    res
                },
                _ => response_500()
            }
        }
    };
}

handle_request!(handle_rtt, call_rtt_http);
handle_request!(handle_rtb, call_rtb_http);

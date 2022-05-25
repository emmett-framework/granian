use hyper::{Body, Request, Response};
use std::net::SocketAddr;

use super::super::callbacks::CallbackWrapper;
use super::super::http::response_500;
use super::super::io::Receiver;
use super::super::runtime::ThreadIsolation;
use super::callbacks::call as callback_caller;
use super::types::Scope;

// pub(crate) async fn handle_request(
//     cb_wrapper: CallbackWrapper,
//     client_addr: SocketAddr,
//     req: Request<Body>,
//     sender: Sender
// ) -> PyResult<()> {
//     let scope = Scope::new(
//         "http",
//         req.version(),
//         req.uri().clone(),
//         req.method().as_ref(),
//         client_addr,
//         req.headers()
//     );
//     let receiver = Receiver::new(req);

//     callback_caller(cb_wrapper, receiver, sender, scope).await?;
//     Ok(())
// }

pub(crate) async fn handle_request(
    thread_mode: ThreadIsolation,
    cb_wrapper: CallbackWrapper,
    client_addr: SocketAddr,
    req: Request<Body>
) -> Response<Body> {
    let scope = Scope::new(
        "http",
        req.version(),
        req.uri().clone(),
        req.method().as_ref(),
        client_addr,
        req.headers()
    );
    let receiver = Receiver::new(thread_mode, req);

    match callback_caller(cb_wrapper, receiver, scope).await {
        Ok(rx) => {
            match rx.await {
                Ok(res) => res,
                _ => response_500()
            }
        },
        _ => response_500()
    }
}

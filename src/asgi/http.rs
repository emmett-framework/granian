use hyper::{Body, Request, Response};
use std::net::SocketAddr;

use super::super::callbacks::CallbackWrapper;
use super::callbacks::call as callback_caller;
use super::io::{Receiver};
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
    cb_wrapper: CallbackWrapper,
    client_addr: SocketAddr,
    req: Request<Body>,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let scope = Scope::new(
        "http",
        req.version(),
        req.uri().clone(),
        req.method().as_ref(),
        client_addr,
        req.headers()
    );
    let receiver = Receiver::new(req);

    let rx = callback_caller(cb_wrapper, receiver, scope).await?;
    Ok(rx.await?)
}

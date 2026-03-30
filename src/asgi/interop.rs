use std::sync::Arc;
use tokio::sync::{Notify, SetOnce, oneshot};

use super::{
    io::{ASGIHTTPProtocol as HTTPProtocol, ASGIWebsocketProtocol as WebsocketProtocol, WebsocketDetachedTransport},
    utils::{build_scope_http, build_scope_ws},
};
use crate::{
    http::{HTTPProto, HTTPResponse},
    net::SockAddr,
    py::interop::ArcApp,
    runtime::{Runtime, RuntimeRef},
    utils::GuardedReceiver,
    ws::{HyperWebsocket, UpgradeData},
};

#[inline]
pub(crate) fn call_http(
    app: ArcApp,
    rt: RuntimeRef,
    server_addr: SockAddr,
    client_addr: SockAddr,
    scheme: HTTPProto,
    req: hyper::http::request::Parts,
    body: hyper::body::Incoming,
) -> GuardedReceiver<HTTPResponse> {
    let (tx, rx) = oneshot::channel();
    let disconnect_guard = Arc::new(SetOnce::new());
    let protocol = HTTPProtocol::new(rt.clone(), disconnect_guard.clone(), body, tx);

    rt.spawn_blocking(move |py| {
        if let Ok(scope) = build_scope_http(py, req, server_addr, client_addr, scheme) {
            app.get().handle_request(py, (protocol, scope));
        }
    });

    GuardedReceiver::new(rx, disconnect_guard)
}

#[inline]
pub(crate) fn call_ws(
    app: ArcApp,
    rt: RuntimeRef,
    disconnect_guard: Arc<Notify>,
    server_addr: SockAddr,
    client_addr: SockAddr,
    scheme: HTTPProto,
    ws: HyperWebsocket,
    req: hyper::http::request::Parts,
    upgrade: UpgradeData,
) -> oneshot::Receiver<WebsocketDetachedTransport> {
    let (tx, rx) = oneshot::channel();
    let protocol = WebsocketProtocol::new(rt.clone(), tx, ws, upgrade, disconnect_guard);

    rt.spawn_blocking(move |py| {
        if let Ok(scope) = build_scope_ws(py, req, server_addr, client_addr, scheme) {
            app.get().handle_websocket(py, (protocol, scope));
        }
    });

    rx
}

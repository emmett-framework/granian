use std::sync::Arc;
use tokio::sync::{Notify, SetOnce, oneshot};

use super::io::{HTTPProtocol, WebsocketDetachedTransport, WebsocketProtocol};
use crate::{
    py::interop::ArcApp,
    rsgi::types::{PyResponse, RSGIHTTPScope as HTTPScope, RSGIWebsocketScope as WebsocketScope},
    runtime::{Runtime, RuntimeRef},
    utils::GuardedReceiver,
    ws::{HyperWebsocket, UpgradeData},
};

#[inline]
pub(crate) fn call_http(
    app: ArcApp,
    rt: RuntimeRef,
    body: hyper::body::Incoming,
    scope: HTTPScope,
) -> GuardedReceiver<PyResponse> {
    let (tx, rx) = oneshot::channel();
    let disconnect_guard = Arc::new(SetOnce::new());
    let protocol = HTTPProtocol::new(rt.clone(), disconnect_guard.clone(), tx, body);

    rt.spawn_blocking(move |py| {
        app.get().handle_request(py, (protocol, scope));
    });

    GuardedReceiver::new(rx, disconnect_guard)
}

#[inline]
pub(crate) fn call_ws(
    app: ArcApp,
    rt: RuntimeRef,
    disconnect_guard: Arc<Notify>,
    ws: HyperWebsocket,
    upgrade: UpgradeData,
    scope: WebsocketScope,
) -> oneshot::Receiver<WebsocketDetachedTransport> {
    let (tx, rx) = oneshot::channel();
    let protocol = WebsocketProtocol::new(rt.clone(), tx, ws, upgrade, disconnect_guard);

    rt.spawn_blocking(move |py| {
        app.get().handle_websocket(py, (protocol, scope));
    });

    rx
}

use std::sync::{Arc, RwLock};

use futures::future::BoxFuture;

#[cfg(feature = "python-module")]
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

#[cfg(feature = "python-module")]
use futures::{SinkExt, StreamExt};
#[cfg(feature = "python-module")]
use http_body_util::BodyExt;
#[cfg(feature = "python-module")]
use tokio::sync::mpsc;
#[cfg(feature = "python-module")]
use tokio_tungstenite::tungstenite::Message;

use crate::http::{HTTPRequest, HTTPResponse};
use crate::net::SockAddr;
#[cfg(feature = "python-module")]
use crate::ws::{WSStream, is_upgrade_request, upgrade_intent};

/// Contextual information handed over to native handlers.
#[allow(dead_code)]
pub struct NativeContext {
    /// Handle of the runtime serving the request.
    pub rt: tokio::runtime::Handle,
    /// Address the server is listening on.
    pub addr_local: SockAddr,
    /// Address of the client.
    pub addr_remote: SockAddr,
    /// Whether the connection is TLS protected.
    pub secure: bool,
}

/// A native handler is an async callable serving a request without involving Python.
pub type NativeHandler = Arc<dyn Fn(HTTPRequest, NativeContext) -> BoxFuture<'static, HTTPResponse> + Send + Sync>;

/// The kind of a native route.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NativeRouteKind {
    /// A regular HTTP request.
    Http,
    /// A websockets connection: the handler is responsible for the handshake.
    Websocket,
}

struct NativeRoute {
    prefix: String,
    handler: NativeHandler,
    kind: NativeRouteKind,
}

/// Registry of the routes served directly by the Rust side.
///
/// Routes are matched by path prefix, on segment boundaries, with the most
/// specific (longest) prefix winning.
#[derive(Default)]
pub(crate) struct NativeRouter {
    routes: Vec<NativeRoute>,
}

impl NativeRouter {
    fn add(&mut self, prefix: &str, handler: NativeHandler, kind: NativeRouteKind) {
        let prefix = prefix.trim_end_matches('/').to_string();
        self.routes.push(NativeRoute { prefix, handler, kind });
        self.routes.sort_by_key(|route| std::cmp::Reverse(route.prefix.len()));
    }

    fn has_ws_routes(&self) -> bool {
        self.routes.iter().any(|route| route.kind == NativeRouteKind::Websocket)
    }

    pub(crate) fn match_path(&self, path: &str) -> Option<&NativeHandler> {
        self.routes.iter().find_map(|route| {
            path.strip_prefix(route.prefix.as_str())
                .filter(|rest| rest.is_empty() || rest.starts_with('/'))
                .map(|_| &route.handler)
        })
    }
}

static ROUTER: RwLock<NativeRouter> = RwLock::new(NativeRouter { routes: Vec::new() });

/// Register a route to be served directly by the Rust side.
///
/// The handler is called for every request whose path matches the given prefix
/// (on segment boundaries), bypassing the Python application entirely.
///
/// Registration is meant to happen at module initialisation time, before the
/// server starts serving requests.
///
/// # Panics
///
/// Panics if the routes registry lock is poisoned.
pub fn register_native_route<F>(path: &str, kind: NativeRouteKind, handler: F)
where
    F: Fn(HTTPRequest, NativeContext) -> BoxFuture<'static, HTTPResponse> + Send + Sync + 'static,
{
    ROUTER.write().unwrap().add(path, Arc::new(handler), kind);
}

/// The native routes registry.
pub(crate) fn router() -> std::sync::RwLockReadGuard<'static, NativeRouter> {
    ROUTER.read().unwrap()
}

/// Whether any of the registered native routes requires connection upgrades support.
pub(crate) fn has_ws_routes() -> bool {
    ROUTER.read().unwrap().has_ws_routes()
}

#[cfg(feature = "python-module")]
fn response_400() -> HTTPResponse {
    let mut builder = hyper::Response::builder().status(400);
    let headers = builder.headers_mut().unwrap();
    headers.insert(hyper::header::SERVER, crate::http::HV_SERVER);
    builder
        .body(
            http_body_util::Full::new("Bad request".into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}

#[cfg(feature = "python-module")]
fn response_501() -> HTTPResponse {
    let mut builder = hyper::Response::builder().status(501);
    let headers = builder.headers_mut().unwrap();
    headers.insert(hyper::header::SERVER, crate::http::HV_SERVER);
    builder
        .body(
            http_body_util::Full::new("Not implemented".into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}

/// Register the reference native routes.
///
/// These are only registered by the Granian module itself, they are meant to
/// serve as a reference implementation (and to be covered by the test suite).
/// Crates embedding Granian as a library are expected to register their own
/// routes instead.
#[cfg(feature = "python-module")]
pub(crate) fn register_reference_routes() {
    register_native_route("/native_view", NativeRouteKind::Http, http_view_handler());
    register_native_route("/native_ws", NativeRouteKind::Websocket, ws_consumer_handler());
}

/// A native HTTP "view": it produces the response without involving Python.
#[cfg(feature = "python-module")]
fn http_view_handler() -> impl Fn(HTTPRequest, NativeContext) -> BoxFuture<'static, HTTPResponse> + Send + Sync + 'static
{
    |req, ctx| {
        Box::pin(async move {
            let path = req.uri().path();
            let query = req.uri().query().unwrap_or_default();

            let body = format!(
                "{{\"path\":\"{path}\",\"query\":\"{query}\",\"remote\":\"{}\",\"secure\":{}}}",
                ctx.addr_remote.to_string(),
                ctx.secure,
            );

            let mut builder = hyper::Response::builder().status(200);
            builder = builder.header(hyper::header::CONTENT_TYPE, "application/json");
            builder
                .body(http_body_util::Full::new(body.into()).map_err(|e| match e {}).boxed())
                .unwrap()
        })
    }
}

#[cfg(feature = "python-module")]
type ClientRegistry = Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<Message>>>>;

/// A native websockets "consumer": it handles the connection without involving
/// Python, broadcasting every message to the clients of the same group.
#[cfg(feature = "python-module")]
fn ws_consumer_handler()
-> impl Fn(HTTPRequest, NativeContext) -> BoxFuture<'static, HTTPResponse> + Send + Sync + 'static {
    let clients: ClientRegistry = Arc::new(Mutex::new(HashMap::new()));
    let counter = Arc::new(AtomicU64::new(0));

    move |req, ctx| {
        let clients = clients.clone();
        let counter = counter.clone();

        Box::pin(async move {
            let mut req = req;
            if !is_upgrade_request(&req) {
                // HTTP/2 has no "Connection: upgrade" header, hence websockets
                // over HTTP/2 (RFC 8441) are not supported.
                if req.version() == hyper::Version::HTTP_2 {
                    return response_501();
                }
                return response_400();
            }

            match upgrade_intent(&mut req, None) {
                Ok((builder, ws)) => {
                    let client_id = counter.fetch_add(1, Ordering::Relaxed);
                    ctx.rt.spawn(async move {
                        match ws.await {
                            Ok(stream) => {
                                ws_session(client_id, stream, clients).await;
                            }
                            Err(err) => {
                                log::warn!("Native websockets upgrade failed with {err:?}");
                            }
                        }
                    });

                    builder.body(crate::http::empty_body()).unwrap()
                }
                Err(err) => {
                    log::info!("Native websockets handshake failed with {err:?}");
                    response_400()
                }
            }
        })
    }
}

#[cfg(feature = "python-module")]
async fn ws_session(client_id: u64, stream: WSStream, clients: ClientRegistry) {
    let (mut sink, mut source) = stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    clients.lock().unwrap().insert(client_id, tx);

    loop {
        tokio::select! {
            msg = source.next() => {
                match msg {
                    Some(Ok(Message::Text(data))) => broadcast(&clients, Message::Text(data)),
                    Some(Ok(Message::Binary(data))) => broadcast(&clients, Message::Binary(data)),
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        log::debug!("Native websockets client error: {err:?}");
                        break;
                    }
                }
            }
            msg = rx.recv() => {
                match msg {
                    Some(msg) => {
                        if sink.send(msg).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    clients.lock().unwrap().remove(&client_id);
}

#[cfg(feature = "python-module")]
fn broadcast(clients: &ClientRegistry, msg: Message) {
    let clients = clients.lock().unwrap();
    for tx in clients.values() {
        _ = tx.send(msg.clone());
    }
}

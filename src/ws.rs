use futures::sink::SinkExt;
use http_body_util::BodyExt;
use hyper::{
    Request, Response, StatusCode,
    header::{CONNECTION, UPGRADE},
    http::response::Builder,
};
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::{Mutex as AsyncMutex, Notify, mpsc};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Error as TungsteniteError, Message,
        error::ProtocolError,
        handshake::derive_accept_key,
        protocol::{
            Role, WebSocketConfig,
            frame::{CloseFrame, coding::CloseCode},
        },
    },
};

use super::http::HTTPResponse;
use super::utils::header_contains_value;
use crate::runtime::{Runtime, RuntimeRef};

pub(crate) type WSStream = WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;
pub(crate) type WSRxStream = futures::stream::SplitStream<WSStream>;
pub(crate) type WSTxStream = futures::stream::SplitSink<WSStream, Message>;

static WS_PING_COUNTER: AtomicU32 = AtomicU32::new(0);

#[inline]
fn next_ping_id() -> u32 {
    loop {
        let id = WS_PING_COUNTER.fetch_add(1, Ordering::Relaxed);
        if id != 0 {
            return id;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WsKeepaliveConfig {
    interval: Option<Duration>,
    timeout: Option<Duration>,
}

impl WsKeepaliveConfig {
    pub const fn disabled() -> Self {
        Self {
            interval: None,
            timeout: None,
        }
    }

    pub fn new(interval: Option<f64>, timeout: Option<f64>) -> Self {
        let sanitize = |value: Option<f64>| value.filter(|v| v.is_finite() && *v > 0.0).map(Duration::from_secs_f64);
        Self {
            interval: sanitize(interval),
            timeout: sanitize(timeout),
        }
    }

    pub fn enabled(&self) -> bool {
        self.interval.is_some()
    }
}

pub(crate) struct WsKeepalive {
    // `0` means no ping is currently in flight; ping ids start at `1`.
    pending: AtomicU32,
    pong: Notify,
}

impl WsKeepalive {
    fn new() -> Self {
        Self {
            pending: AtomicU32::new(0),
            pong: Notify::new(),
        }
    }

    #[inline]
    fn track(&self, id: u32) {
        self.pending.store(id, Ordering::Release);
    }

    #[inline]
    fn clear(&self) {
        self.pending.store(0, Ordering::Release);
    }

    #[inline]
    fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Acquire) != 0
    }

    pub fn on_pong(&self, payload: &[u8]) {
        let Ok(bytes) = <[u8; 4]>::try_from(payload) else {
            return;
        };
        let id = u32::from_be_bytes(bytes);
        if id != 0
            && self
                .pending
                .compare_exchange(id, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            self.pong.notify_one();
        }
    }
}

pub(crate) fn spawn_keepalive(
    rt: &RuntimeRef,
    config: WsKeepaliveConfig,
    tx: Arc<AsyncMutex<Option<WSTxStream>>>,
    closed: Arc<AtomicBool>,
    disconnect_guard: Arc<Notify>,
) -> Option<Arc<WsKeepalive>> {
    if !config.enabled() {
        return None;
    }
    let keepalive = Arc::new(WsKeepalive::new());
    let ka = keepalive.clone();
    rt.spawn(async move {
        run_keepalive(config, tx, closed, disconnect_guard, ka).await;
    });
    Some(keepalive)
}

async fn run_keepalive(
    config: WsKeepaliveConfig,
    tx: Arc<AsyncMutex<Option<WSTxStream>>>,
    closed: Arc<AtomicBool>,
    disconnect_guard: Arc<Notify>,
    keepalive: Arc<WsKeepalive>,
) {
    let Some(interval) = config.interval else {
        return;
    };

    loop {
        tokio::select! {
            biased;
            () = tokio::time::sleep(interval) => {},
            () = disconnect_guard.notified() => return,
        }
        if closed.load(Ordering::Acquire) {
            return;
        }

        let id = next_ping_id();
        keepalive.track(id);
        let payload = id.to_be_bytes();

        {
            let mut guard = tx.lock().await;
            match guard.as_mut() {
                Some(stream) => {
                    if stream.send(Message::Ping(payload.to_vec().into())).await.is_err() {
                        return;
                    }
                }
                None => return,
            }
        }

        let Some(timeout) = config.timeout else {
            continue;
        };

        let sleep = tokio::time::sleep(timeout);
        tokio::pin!(sleep);
        loop {
            tokio::select! {
                biased;
                () = keepalive.pong.notified() => {
                    if !keepalive.is_pending() {
                        break;
                    }
                },
                () = &mut sleep => {
                    keepalive.clear();
                    if closed.load(Ordering::Acquire) {
                        return;
                    }
                    log::info!("WebSocket keepalive ping timeout");
                    closed.store(true, Ordering::Release);
                    {
                        let mut guard = tx.lock().await;
                        if let Some(stream) = guard.as_mut() {
                            let _ = stream
                                .send(Message::Close(Some(CloseFrame {
                                    code: CloseCode::Error,
                                    reason: "keepalive ping timeout".into(),
                                })))
                                .await;
                            let _ = stream.close().await;
                        }
                    }
                    disconnect_guard.notify_one();
                    return;
                },
                () = disconnect_guard.notified() => return,
            }
        }
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct HyperWebsocket {
        #[pin]
        inner: hyper::upgrade::OnUpgrade,
        config: Option<WebSocketConfig>,
    }
}

impl Future for HyperWebsocket {
    type Output = Result<WSStream, TungsteniteError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let upgraded = match this.inner.poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(x) => x,
        };

        let upgraded = upgraded.map_err(|_| TungsteniteError::Protocol(ProtocolError::HandshakeIncomplete))?;

        let io = hyper_util::rt::TokioIo::new(upgraded);
        let stream = WebSocketStream::from_raw_socket(io, Role::Server, this.config.take());
        tokio::pin!(stream);

        match stream.as_mut().poll(cx) {
            Poll::Pending => unreachable!(),
            Poll::Ready(x) => Poll::Ready(Ok(x)),
        }
    }
}

pub(crate) struct UpgradeData {
    response: Option<(Builder, mpsc::Sender<HTTPResponse>)>,
}

impl UpgradeData {
    pub fn new(response_builder: Builder, response_tx: mpsc::Sender<HTTPResponse>) -> Self {
        Self {
            response: Some((response_builder, response_tx)),
        }
    }

    pub async fn send(
        &mut self,
        status: Option<u16>,
        headers: Option<hyper::HeaderMap>,
        body: Option<hyper::body::Bytes>,
    ) -> anyhow::Result<()> {
        if let Some((mut builder, tx)) = self.response.take() {
            if let Some(status) = status {
                builder = builder.status(status);
            }
            if let Some(mut headers) = headers {
                let rheaders = builder.headers_mut().unwrap();
                for (key, val) in headers.drain() {
                    rheaders.append(key.unwrap(), val);
                }
            }
            let res = match body {
                Some(bytes) => builder
                    .body(http_body_util::Full::new(bytes).map_err(|e| match e {}).boxed())
                    .unwrap(),
                _ => builder
                    .body(http_body_util::Empty::new().map_err(|e| match e {}).boxed())
                    .unwrap(),
            };
            return Ok(tx.send(res).await?);
        }
        Err(anyhow::Error::msg("Already consumed"))
    }
}

#[inline]
pub(crate) fn is_upgrade_request<B>(request: &Request<B>) -> bool {
    header_contains_value(request.headers(), CONNECTION, "Upgrade")
        && header_contains_value(request.headers(), UPGRADE, "websocket")
}

pub(crate) fn upgrade_intent<B>(
    request: &mut Request<B>,
    config: Option<WebSocketConfig>,
) -> Result<(Builder, HyperWebsocket), ProtocolError> {
    let key = request
        .headers()
        .get("Sec-WebSocket-Key")
        .ok_or(ProtocolError::MissingSecWebSocketKey)?;

    if request
        .headers()
        .get("Sec-WebSocket-Version")
        .map(hyper::http::HeaderValue::as_bytes)
        != Some(b"13")
    {
        return Err(ProtocolError::MissingSecWebSocketVersionHeader);
    }

    let response_builder = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(CONNECTION, "upgrade")
        .header(UPGRADE, "websocket")
        .header("Sec-WebSocket-Accept", &derive_accept_key(key.as_bytes()));

    let stream = HyperWebsocket {
        inner: hyper::upgrade::on(request),
        config,
    };

    Ok((response_builder, stream))
}

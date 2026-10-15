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
    sync::{Arc, atomic},
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    sync::{Mutex as AsyncMutex, Notify, mpsc},
    time,
};
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
use super::runtime::{Runtime, RuntimeRef};
use super::utils::header_contains_value;

pub(crate) type WSStream = WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;
pub(crate) type WSRxStream = futures::stream::SplitStream<WSStream>;
pub(crate) type WSTxStream = futures::stream::SplitSink<WSStream, Message>;
pub(crate) type WSPing = (Option<f64>, Option<f64>);

/// Builds a `'static` websocket target by copying the ping config out of `&self`.
macro_rules! ws_target {
    ($self:expr, $handler:path) => {{
        let websocket_ping = $self.config.websocket_ping;
        move |rt, dg, cb, sa, ca, req, sch| $handler(rt, dg, cb, sa, ca, req, sch, websocket_ping)
    }};
}

pub(crate) use ws_target;

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

async fn ws_send(sink: &AsyncMutex<Option<WSTxStream>>, msg: Message) -> bool {
    match &mut *sink.lock().await {
        Some(tx) => tx.send(msg).await.is_ok(),
        None => false,
    }
}

/// Pings an idle peer every interval and closes it with 1011 if nothing is received within the timeout.
pub(crate) fn spawn_keepalive(
    rt: &RuntimeRef,
    cfg: WSPing,
    sink: Arc<AsyncMutex<Option<WSTxStream>>>,
    closed: Arc<atomic::AtomicBool>,
    alive: Arc<atomic::AtomicBool>,
    disconnect_guard: Arc<Notify>,
) {
    let to_duration = |secs: Option<f64>| {
        secs.and_then(|v| Duration::try_from_secs_f64(v).ok())
            .filter(|d| !d.is_zero())
    };
    let Some(interval) = to_duration(cfg.0) else {
        return;
    };
    let timeout = to_duration(cfg.1);

    rt.spawn(async move {
        let mut ticker = time::interval_at(time::Instant::now() + interval, interval);
        ticker.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            if closed.load(atomic::Ordering::Acquire) {
                return;
            }
            if alive.swap(false, atomic::Ordering::Relaxed) {
                continue;
            }
            if !ws_send(&sink, Message::Ping(hyper::body::Bytes::new())).await {
                return;
            }
            if let Some(timeout) = timeout {
                time::sleep(timeout).await;
                if closed.load(atomic::Ordering::Acquire) {
                    return;
                }
                if !alive.swap(false, atomic::Ordering::Relaxed) {
                    log::info!("Websocket keepalive ping timeout, closing connection");
                    let frame = CloseFrame {
                        code: CloseCode::Error,
                        reason: "keepalive ping timeout".into(),
                    };
                    ws_send(&sink, Message::Close(Some(frame))).await;
                    disconnect_guard.notify_one();
                    return;
                }
            }
        }
    });
}

use futures::{sink::SinkExt, stream::StreamExt};
use hyper::{
    Body,
    Request,
    Response,
    StatusCode,
    header::{CONNECTION, UPGRADE},
    http::response::Builder
};
use tungstenite::{
    error::ProtocolError,
    handshake::derive_accept_key,
    protocol::{Role, WebSocketConfig}
};
use pin_project::pin_project;
use std::{future::Future, pin::Pin, sync::Arc, task::{Context, Poll}};
use tokio_tungstenite::WebSocketStream;
use tokio::sync::{Mutex, mpsc};
use tungstenite::Message;

use super::utils::header_contains_value;


#[pin_project]
#[derive(Debug)]
pub struct HyperWebsocket {
    #[pin]
    inner: hyper::upgrade::OnUpgrade,
    config: Option<WebSocketConfig>,
}

impl Future for HyperWebsocket {
    type Output = Result<WebSocketStream<hyper::upgrade::Upgraded>, tungstenite::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let upgraded = match this.inner.poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(x) => x,
        };

        let upgraded = upgraded.map_err(|_|
            tungstenite::Error::Protocol(ProtocolError::HandshakeIncomplete)
        )?;

        let stream = WebSocketStream::from_raw_socket(
            upgraded,
            Role::Server,
            this.config.take(),
        );
        tokio::pin!(stream);

        match stream.as_mut().poll(cx) {
            Poll::Pending => unreachable!(),
            Poll::Ready(x) => Poll::Ready(Ok(x)),
        }
    }
}

pub(crate) struct WebsocketTransport {
    socket: Mutex<HyperWebsocket>,
    stream: Option<Arc<Mutex<WebSocketStream<hyper::upgrade::Upgraded>>>>,
    pub accepted: bool,
    pub closed: bool
}

impl WebsocketTransport {
    pub fn new(socket: HyperWebsocket) -> Self {
        Self {
            socket: Mutex::new(socket),
            stream: None,
            accepted: false,
            closed: false
        }
    }

    pub fn set_closed(&mut self) {
        self.closed = true;
    }

    pub async fn accept(&mut self) -> Result<(), tungstenite::Error> {
        self.stream = Some(
            Arc::new(Mutex::new((&mut *(self.socket.lock().await)).await?))
        );
        self.accepted = true;
        Ok(())
    }

    pub async fn receive(&self) -> Result<Message, tungstenite::Error> {
        match &self.stream {
            Some(wrapped) => {
                let wsw = wrapped.clone();
                let mut sock = wsw.lock().await;
                match (&mut *sock).next().await {
                    Some(message) => message,
                    _ => Err(tungstenite::Error::ConnectionClosed)
                }
            },
            _ => Err(tungstenite::Error::ConnectionClosed)
        }
    }

    pub async fn send(&self, data: Message) -> Result<(), tungstenite::Error> {
        match &self.stream {
            Some(wrapped) => {
                let wsw = wrapped.clone();
                let mut sock = wsw.lock().await;
                (&mut sock).send(data).await
            },
            _ => Err(tungstenite::Error::ConnectionClosed)
        }
    }
}

pub(crate) struct UpgradeData {
    response_builder: Option<Builder>,
    response_tx: Option<mpsc::Sender<Response<Body>>>,
    pub consumed: bool
}

impl UpgradeData {
    pub fn new(
        response_builder: Builder,
        response_tx: mpsc::Sender<Response<Body>>)
    -> Self {
        Self {
            response_builder: Some(response_builder),
            response_tx: Some(response_tx),
            consumed: false
        }
    }

    pub async fn send(&mut self) -> Result<(), mpsc::error::SendError<Response<Body>>> {
        let res = self.response_builder.take().unwrap().body(Body::from("")).unwrap();
        match self.response_tx.take().unwrap().send(res).await {
            Ok(_) => {
                self.consumed = true;
                Ok(())
            },
            err => err
        }
    }
}

#[inline]
pub(crate) fn is_upgrade_request<B>(request: &Request<B>) -> bool {
    header_contains_value(
        request.headers(), CONNECTION, "Upgrade"
    ) && header_contains_value(
        request.headers(), UPGRADE, "websocket"
    )
}

pub(crate) fn upgrade_intent<B>(
    mut request: impl std::borrow::BorrowMut<Request<B>>,
    config: Option<WebSocketConfig>,
) -> Result<(Builder, HyperWebsocket), ProtocolError> {
    let request = request.borrow_mut();

    let key = request.headers()
        .get("Sec-WebSocket-Key")
        .ok_or(ProtocolError::MissingSecWebSocketKey)?;

    if request.headers().get("Sec-WebSocket-Version").map(
        |v| v.as_bytes()
    ) != Some(b"13") {
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

use http_body_util::BodyExt;
use hyper::{
    header::{HeaderName, HeaderValue, CONNECTION, UPGRADE},
    http::response::Builder,
    Request, Response, StatusCode,
};
use pin_project::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    tungstenite::{
        error::ProtocolError,
        handshake::derive_accept_key,
        protocol::{Role, WebSocketConfig},
        Error as TungsteniteError, Message,
    },
    WebSocketStream,
};

use super::http::HTTPResponse;
use super::utils::header_contains_value;

pub(crate) type WSStream = WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;
pub(crate) type WSRxStream = futures::stream::SplitStream<WSStream>;
pub(crate) type WSTxStream = futures::stream::SplitSink<WSStream, Message>;

#[pin_project]
#[derive(Debug)]
pub struct HyperWebsocket {
    #[pin]
    inner: hyper::upgrade::OnUpgrade,
    config: Option<WebSocketConfig>,
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

    pub async fn send(&mut self, headers: Option<Vec<(String, String)>>) -> anyhow::Result<()> {
        if let Some((mut builder, tx)) = self.response.take() {
            if let Some(headers) = headers {
                let rheaders = builder.headers_mut().unwrap();
                for (key, val) in &headers {
                    rheaders.append(
                        HeaderName::from_bytes(key.as_bytes()).unwrap(),
                        HeaderValue::from_str(val).unwrap(),
                    );
                }
            }
            let res = builder
                .body(http_body_util::Empty::new().map_err(|e| match e {}).boxed())
                .unwrap();
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

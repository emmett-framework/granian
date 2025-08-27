use hyper::{HeaderMap, body};
use tokio_tungstenite::tungstenite::{Message, protocol::CloseFrame};

pub(crate) enum ASGIMessageType {
    HTTPResponseStart((u16, HeaderMap, bool)),
    HTTPResponseBody((Box<[u8]>, bool)),
    HTTPResponseTrailers((HeaderMap, bool)),
    HTTPResponseFile(String),
    HTTPDisconnect,
    HTTPRequestBody((body::Bytes, bool)),
    WSAccept(Option<String>),
    WSConnect,
    WSClose(Option<CloseFrame>),
    WSMessage(Message),
}

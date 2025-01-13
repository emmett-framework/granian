use hyper::{body, HeaderMap};
use tokio_tungstenite::tungstenite::Message;

pub(crate) enum ASGIMessageType {
    HTTPResponseStart((u16, HeaderMap)),
    HTTPResponseBody((Box<[u8]>, bool)),
    HTTPResponseFile(String),
    HTTPDisconnect,
    HTTPRequestBody((body::Bytes, bool)),
    WSAccept(Option<String>),
    WSConnect,
    WSClose,
    WSMessage(Message),
}

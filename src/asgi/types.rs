use hyper::HeaderMap;
use tokio_tungstenite::tungstenite::Message;

pub(crate) enum ASGIMessageType {
    HTTPStart((i16, HeaderMap)),
    HTTPBody((Box<[u8]>, bool)),
    HTTPFile(String),
    WSAccept(Option<String>),
    WSClose,
    WSMessage(Message),
}

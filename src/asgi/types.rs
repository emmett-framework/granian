pub(crate) enum ASGIMessageType {
    HTTPStart,
    HTTPBody,
    HTTPFile,
    WSAccept,
    WSClose,
    WSMessage,
}

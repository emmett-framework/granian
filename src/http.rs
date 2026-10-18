use http_body_util::BodyExt;
use hyper::{
    Response, StatusCode,
    body::Bytes,
    header::{HeaderValue, SERVER as HK_SERVER},
};

pub(crate) type HTTPRequest = hyper::Request<hyper::body::Incoming>;
pub(crate) type HTTPResponseBody = http_body_util::combinators::BoxBody<Bytes, anyhow::Error>;
pub(crate) type HTTPResponse = hyper::Response<HTTPResponseBody>;

pub(crate) const HV_SERVER: HeaderValue = HeaderValue::from_static("granian");

#[derive(Clone)]
pub(crate) enum HTTPProto {
    Plain,
    Tls,
}

impl HTTPProto {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Plain => "http",
            Self::Tls => "https",
        }
    }
}

pub(crate) fn response_404() -> HTTPResponse {
    Response::builder()
        .status(404)
        .body(
            http_body_util::Full::new("Not found".into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}

pub(crate) fn response_500() -> HTTPResponse {
    Response::builder()
        .status(500)
        .body(
            http_body_util::Full::new("Internal server error".into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}

// Adds the default `server` header, unless the application already set one.
// Websocket handshake responses are left untouched.
#[inline(always)]
pub(crate) fn with_server_header(mut res: HTTPResponse, enabled: bool) -> HTTPResponse {
    if enabled && res.status() != StatusCode::SWITCHING_PROTOCOLS {
        res.headers_mut().entry(HK_SERVER).or_insert(HV_SERVER);
    }
    res
}

#[inline(always)]
pub(crate) fn empty_body() -> HTTPResponseBody {
    http_body_util::Empty::<Bytes>::new().map_err(|e| match e {}).boxed()
}

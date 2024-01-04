use http_body_util::BodyExt;
use hyper::{
    body::Bytes,
    header::{HeaderValue, SERVER as HK_SERVER},
    Response,
};

pub(crate) type HTTPRequest = hyper::Request<hyper::body::Incoming>;
pub(crate) type HTTPResponseBody = http_body_util::combinators::BoxBody<Bytes, anyhow::Error>;
pub(crate) type HTTPResponse = hyper::Response<HTTPResponseBody>;

pub(crate) const HV_SERVER: HeaderValue = HeaderValue::from_static("granian");

pub(crate) fn response_404() -> HTTPResponse {
    let mut builder = Response::builder().status(404);
    let headers = builder.headers_mut().unwrap();
    headers.insert(HK_SERVER, HV_SERVER);
    builder
        .body(
            http_body_util::Full::new("Not found".into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}

pub(crate) fn response_500() -> HTTPResponse {
    let mut builder = Response::builder().status(500);
    let headers = builder.headers_mut().unwrap();
    headers.insert(HK_SERVER, HV_SERVER);
    builder
        .body(
            http_body_util::Full::new("Internal server error".into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}

#[inline(always)]
pub(crate) fn empty_body() -> HTTPResponseBody {
    http_body_util::Empty::<Bytes>::new().map_err(|e| match e {}).boxed()
}

use hyper::{Body, Response, header::{HeaderValue, SERVER as HK_SERVER}};

pub(crate) const HV_SERVER: HeaderValue = HeaderValue::from_static("granian");

pub(crate) fn response_500() -> Response<Body> {
    let mut builder = Response::builder().status(500);
    let headers = builder.headers_mut().unwrap();
    headers.insert(HK_SERVER, HV_SERVER);
    builder.body(Body::from("Internal server error")).unwrap()
}

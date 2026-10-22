//! A native HTTP "view": it produces the response without involving Python.
//!
//! This is the Rust counterpart of a Django view: it receives the request and
//! returns the response, with no access to the Django machinery (middleware,
//! sessions, authentication, templates...).

use futures::future::BoxFuture;
use http_body_util::BodyExt;

use granian::{HTTPRequest, HTTPResponse, NativeContext};

/// Serve a JSON payload describing the request.
pub fn info_view(
    req: HTTPRequest,
    ctx: NativeContext,
) -> BoxFuture<'static, HTTPResponse> {
    Box::pin(async move {
        let path = req.uri().path();
        let query = req.uri().query().unwrap_or_default();

        let body = format!(
            "{{\"path\":\"{path}\",\"query\":\"{query}\",\"remote\":\"{}\",\"secure\":{},\"source\":\"native-views\"}}",
            ctx.addr_remote.to_string(),
            ctx.secure,
        );

        let mut builder = hyper::Response::builder().status(200);
        builder = builder.header(hyper::header::CONTENT_TYPE, "application/json");
        builder
            .body(
                http_body_util::Full::new(body.into())
                    .map_err(|e| match e {})
                    .boxed(),
            )
            .unwrap()
    })
}

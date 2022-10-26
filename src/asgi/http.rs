use hyper::{
    Body,
    Request,
    Response,
    StatusCode,
    header::SERVER as HK_SERVER,
    http::response::Builder as ResponseBuilder
};
use std::net::SocketAddr;
use tokio::sync::mpsc;

use crate::{
    callbacks::CallbackWrapper,
    http::{HV_SERVER, response_500},
    runtime::RuntimeRef,
    ws::{UpgradeData, is_upgrade_request as is_ws_upgrade, upgrade_intent as ws_upgrade}
};
use super::{
    callbacks::{call_http, call_ws},
    types::ASGIScope as Scope
};


macro_rules! default_scope {
    ($server_addr:expr, $client_addr:expr, $req:expr, $scheme:expr) => {
        Scope::new(
            $req.version(),
            $scheme,
            $req.uri().clone(),
            $req.method().as_ref(),
            $server_addr,
            $client_addr,
            $req.headers()
        )
    };
}

macro_rules! handle_http_response {
    ($rt:expr, $callback:expr, $req:expr, $scope:expr) => {
        match call_http($callback, $rt, $req, $scope).await {
            Ok(res) => res,
            _ => response_500()
        }
    }
}

pub(crate) async fn handle_request(
    rt: RuntimeRef,
    callback: CallbackWrapper,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    req: Request<Body>,
    scheme: &str
) -> Response<Body> {
    let scope = default_scope!(server_addr, client_addr, &req, scheme);
    handle_http_response!(rt, callback, req, scope)
}

pub(crate) async fn handle_request_with_ws(
    rt: RuntimeRef,
    callback: CallbackWrapper,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    req: Request<Body>,
    scheme: &str
) -> Response<Body> {
    let mut scope = default_scope!(server_addr, client_addr, &req, scheme);

    if is_ws_upgrade(&req) {
        scope.set_websocket();

        return match ws_upgrade(req, None) {
            Ok((res, ws)) => {
                let rth = rt.clone();
                let (restx, mut resrx) = mpsc::channel(1);

                rt.inner.spawn(async move {
                    let tx_ref = restx.clone();

                    match call_ws(
                        callback,
                        rth,
                        ws,
                        UpgradeData::new(res, restx),
                        scope
                    ).await {
                        Ok(consumed) => {
                            if !consumed {
                                let _ = tx_ref.send(
                                    ResponseBuilder::new()
                                        .status(StatusCode::FORBIDDEN)
                                        .header(HK_SERVER, HV_SERVER)
                                        .body(Body::from(""))
                                        .unwrap()
                                ).await;
                            };
                        },
                        _ => {
                            let _ = tx_ref.send(response_500()).await;
                        }
                    }
                });

                match resrx.recv().await {
                    Some(res) => {
                        resrx.close();
                        res
                    },
                    _ => response_500()
                }
            },
            Err(err) => {
                return ResponseBuilder::new()
                    .status(StatusCode::BAD_REQUEST)
                    .header(HK_SERVER, HV_SERVER)
                    .body(Body::from(format!("{}", err)))
                    .unwrap()
            }
        };
    }

    handle_http_response!(rt, callback, req, scope)
}

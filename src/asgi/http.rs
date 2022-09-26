use hyper::{
    Body,
    Request,
    Response,
    StatusCode,
    header::SERVER as HK_SERVER,
    http::response::Builder as ResponseBuilder
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{sync::{Mutex, mpsc, oneshot}};

use crate::{
    callbacks::CallbackWrapper,
    http::{HV_SERVER, response_500},
    runtime::RuntimeRef,
    ws::{UpgradeData, is_upgrade_request as is_ws_upgrade, upgrade_intent as ws_upgrade}
};
use super::{
    callbacks::call as callback_caller,
    io::{ASGIHTTPProtocol as HTTPProtocol, ASGIWebsocketProtocol as WebsocketProtocol},
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
    ($rt:expr, $callback:expr, $req:expr, $scope:expr, $tx:expr, $rx:expr) => {
        match callback_caller(
            $callback, HTTPProtocol::new($rt, $req, $tx), $scope
        ).await {
            Ok(_) => {
                match $rx.await {
                    Ok(res) => res,
                    _ => response_500()
                }
            },
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
    let (tx, rx) = oneshot::channel();

    handle_http_response!(rt, callback, req, scope, tx, rx)
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
                    let upgrade = Arc::new(Mutex::new(UpgradeData::new(res, restx)));
                    let upgrade_ref = upgrade.clone();
                    let protocol = WebsocketProtocol::new(rth, ws, upgrade);

                    match callback_caller(callback, protocol, scope).await {
                        Ok(_) => {
                            let upgrade_res = upgrade_ref.lock().await;
                            if !(*upgrade_res).consumed {
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
                            let upgrade_res = upgrade_ref.lock().await;
                            if !(*upgrade_res).consumed {
                                let _ = tx_ref.send(response_500()).await;
                            };
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

    let (tx, rx) = oneshot::channel();

    handle_http_response!(rt, callback, req, scope, tx, rx)
}

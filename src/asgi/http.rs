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
    ws::{
        HyperWebsocket,
        UpgradeData,
        is_upgrade_request as is_ws_upgrade,
        upgrade_intent as ws_upgrade
    }
};
use super::{
    callbacks::call as callback_caller,
    io::{ASGIHTTPProtocol as HTTPProtocol, ASGIWebsocketProtocol as WebsocketProtocol},
    types::ASGIScope as Scope
};


pub(crate) async fn handle_request(
    rt: RuntimeRef,
    callback: CallbackWrapper,
    client_addr: SocketAddr,
    req: Request<Body>
) -> Response<Body> {
    let mut scope = Scope::new(
        "http",
        req.version(),
        req.uri().clone(),
        req.method().as_ref(),
        client_addr,
        req.headers()
    );

    if is_ws_upgrade(&req) {
        scope.set_proto("ws");

        return match ws_upgrade(req, None) {
            Ok((res, ws)) => {
                let rth = rt.clone();
                let (restx, mut resrx) = mpsc::channel(1);

                rt.inner.spawn(async move {
                   handle_websocket(rth, res, restx, ws, callback, scope).await
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

    match callback_caller(callback, HTTPProtocol::new(rt, req, tx), scope).await {
        Ok(_) => {
            match rx.await {
                Ok(res) => res,
                _ => response_500()
            }
        },
        _ => response_500()
    }
}

async fn handle_websocket(
    rt: RuntimeRef,
    response: ResponseBuilder,
    tx: mpsc::Sender<Response<Body>>,
    websocket: HyperWebsocket,
    callback: CallbackWrapper,
    scope: Scope
) {
    let tx_ref = tx.clone();
    let upgrade = Arc::new(Mutex::new(UpgradeData::new(response, tx)));
    let upgrade_ref = upgrade.clone();
    let protocol = WebsocketProtocol::new(rt, websocket, upgrade);

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
}

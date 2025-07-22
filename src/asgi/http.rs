use http_body_util::BodyExt;
use hyper::{StatusCode, header::SERVER as HK_SERVER, http::response::Builder as ResponseBuilder};
use std::sync::Arc;
use tokio::sync::{Notify, mpsc};

use super::callbacks::{call_http, call_ws};
use crate::{
    callbacks::ArcCBScheduler,
    http::{HTTPProto, HTTPRequest, HTTPResponse, HV_SERVER, empty_body, response_500},
    net::SockAddr,
    runtime::{Runtime, RuntimeRef},
    ws::{UpgradeData, is_upgrade_request as is_ws_upgrade, upgrade_intent as ws_upgrade},
};

macro_rules! handle_http_response {
    ($handler:expr, $rt:expr, $disconnect_guard:expr, $callback:expr, $server_addr:expr, $client_addr:expr, $scheme:expr, $req:expr, $body:expr) => {
        match $handler(
            $callback,
            $rt,
            $disconnect_guard,
            $server_addr,
            $client_addr,
            $req,
            $scheme,
            $body,
        )
        .await
        {
            Ok(res) => res,
            _ => {
                log::error!("ASGI protocol failure");
                response_500()
            }
        }
    };
}

macro_rules! handle_request {
    ($func_name:ident, $handler:expr) => {
        #[inline]
        pub(crate) async fn $func_name(
            rt: RuntimeRef,
            disconnect_guard: Arc<Notify>,
            callback: ArcCBScheduler,
            server_addr: SockAddr,
            client_addr: SockAddr,
            req: HTTPRequest,
            scheme: HTTPProto,
        ) -> HTTPResponse {
            let (parts, body) = req.into_parts();
            handle_http_response!(
                $handler,
                rt,
                disconnect_guard,
                callback,
                server_addr,
                client_addr,
                parts,
                scheme,
                body
            )
        }
    };
}

macro_rules! handle_request_with_ws {
    ($func_name:ident, $handler_req:expr, $handler_ws:expr) => {
        #[inline]
        pub(crate) async fn $func_name(
            rt: RuntimeRef,
            disconnect_guard: Arc<Notify>,
            callback: ArcCBScheduler,
            server_addr: SockAddr,
            client_addr: SockAddr,
            mut req: HTTPRequest,
            scheme: HTTPProto,
        ) -> HTTPResponse {
            if is_ws_upgrade(&req) {
                return match ws_upgrade(&mut req, None) {
                    Ok((res, ws)) => {
                        let (restx, mut resrx) = mpsc::channel(1);
                        let (parts, _) = req.into_parts();
                        let rth = rt.clone();

                        rt.spawn(async move {
                            let tx_ref = restx.clone();

                            match $handler_ws(
                                callback,
                                rth,
                                server_addr,
                                client_addr,
                                scheme,
                                ws,
                                parts,
                                UpgradeData::new(res, restx),
                            )
                            .await
                            {
                                Ok(mut detached) => {
                                    match detached.consumed {
                                        false => {
                                            let _ = tx_ref
                                                .send(
                                                    ResponseBuilder::new()
                                                        .status(StatusCode::FORBIDDEN)
                                                        .header(HK_SERVER, HV_SERVER)
                                                        .body(empty_body())
                                                        .unwrap(),
                                                )
                                                .await;
                                        }
                                        true => {
                                            detached.close().await;
                                        }
                                    };
                                }
                                _ => {
                                    log::error!("ASGI protocol failure");
                                    let _ = tx_ref.send(response_500()).await;
                                }
                            }
                        });

                        match resrx.recv().await {
                            Some(res) => {
                                resrx.close();
                                res
                            }
                            _ => response_500(),
                        }
                    }
                    Err(err) => {
                        log::info!("Websocket handshake failed with {:?}", err);
                        return ResponseBuilder::new()
                            .status(StatusCode::BAD_REQUEST)
                            .header(HK_SERVER, HV_SERVER)
                            .body(
                                http_body_util::Full::new(format!("{}", err).into())
                                    .map_err(|e| match e {})
                                    .boxed(),
                            )
                            .unwrap();
                    }
                };
            }

            let (parts, body) = req.into_parts();
            handle_http_response!(
                $handler_req,
                rt,
                disconnect_guard,
                callback,
                server_addr,
                client_addr,
                parts,
                scheme,
                body
            )
        }
    };
}

handle_request!(handle, call_http);
handle_request_with_ws!(handle_ws, call_http, call_ws);

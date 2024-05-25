use http_body_util::BodyExt;
use hyper::{header::SERVER as HK_SERVER, http::response::Builder as ResponseBuilder, StatusCode};
use std::net::SocketAddr;
use tokio::sync::mpsc;

use super::callbacks::{call_http, call_http_pyw, call_ws, call_ws_pyw};
use crate::{
    callbacks::CallbackWrapper,
    http::{empty_body, response_500, HTTPRequest, HTTPResponse, HV_SERVER},
    runtime::RuntimeRef,
    ws::{is_upgrade_request as is_ws_upgrade, upgrade_intent as ws_upgrade, UpgradeData},
};

const SCHEME_HTTPS: &str = "https";
const SCHEME_WS: &str = "ws";
const SCHEME_WSS: &str = "wss";

macro_rules! handle_http_response {
    ($handler:expr, $rt:expr, $callback:expr, $server_addr:expr, $client_addr:expr, $scheme:expr, $req:expr, $body:expr) => {
        match $handler($callback, $rt, $server_addr, $client_addr, $req, $scheme, $body).await {
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
            callback: CallbackWrapper,
            server_addr: SocketAddr,
            client_addr: SocketAddr,
            req: HTTPRequest,
            scheme: &str,
        ) -> HTTPResponse {
            let (parts, body) = req.into_parts();
            handle_http_response!(
                $handler,
                rt,
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
            callback: CallbackWrapper,
            server_addr: SocketAddr,
            client_addr: SocketAddr,
            mut req: HTTPRequest,
            scheme: &str,
        ) -> HTTPResponse {
            if is_ws_upgrade(&req) {
                return match ws_upgrade(&mut req, None) {
                    Ok((res, ws)) => {
                        let (restx, mut resrx) = mpsc::channel(1);
                        let (parts, _) = req.into_parts();
                        let scheme: std::sync::Arc<str> = match scheme {
                            SCHEME_HTTPS => SCHEME_WSS,
                            _ => SCHEME_WS,
                        }
                        .into();

                        tokio::task::spawn(async move {
                            let tx_ref = restx.clone();

                            match $handler_ws(
                                callback,
                                rt,
                                server_addr,
                                client_addr,
                                &scheme,
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
// handle_request!(handle_rtb, call_rtb_http);
handle_request!(handle_pyw, call_http_pyw);
// handle_request!(handle_rtb_pyw, call_rtb_http_pyw);
handle_request_with_ws!(handle_ws, call_http, call_ws);
// handle_request_with_ws!(handle_rtb_ws, call_rtb_http, call_rtb_ws);
handle_request_with_ws!(handle_ws_pyw, call_http_pyw, call_ws_pyw);
// handle_request_with_ws!(handle_rtb_ws_pyw, call_rtb_http_pyw, call_rtb_ws_pyw);

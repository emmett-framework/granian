use http_body_util::BodyExt;
use hyper::{header::SERVER as HK_SERVER, http::response::Builder as ResponseBuilder, StatusCode};
use std::net::SocketAddr;
use tokio::sync::mpsc;

use super::{
    callbacks::{call_http, call_http_pyw, call_ws, call_ws_pyw},
    types::{PyResponse, RSGIHTTPScope as HTTPScope, RSGIWebsocketScope as WebsocketScope},
};
use crate::{
    callbacks::CallbackWrapper,
    http::{empty_body, response_500, HTTPRequest, HTTPResponse, HV_SERVER},
    runtime::RuntimeRef,
    ws::{is_upgrade_request as is_ws_upgrade, upgrade_intent as ws_upgrade, UpgradeData},
};

macro_rules! build_scope {
    ($cls:ty, $server_addr:expr, $client_addr:expr, $req:expr, $scheme:expr) => {
        <$cls>::new(
            $req.version,
            $scheme,
            $req.uri,
            $req.method,
            $server_addr,
            $client_addr,
            $req.headers,
        )
    };
}

macro_rules! handle_http_response {
    ($handler:expr, $rt:expr, $callback:expr, $body:expr, $scope:expr) => {
        match $handler($callback, $rt, $body, $scope).await {
            Ok(PyResponse::Body(pyres)) => pyres.to_response(),
            Ok(PyResponse::File(pyres)) => pyres.to_response().await,
            _ => {
                log::error!("RSGI protocol failure");
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
            let scope = build_scope!(HTTPScope, server_addr, client_addr, parts, scheme);
            handle_http_response!($handler, rt, callback, body, scope)
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
                match ws_upgrade(&mut req, None) {
                    Ok((res, ws)) => {
                        let (parts, _) = req.into_parts();
                        let scope = build_scope!(WebsocketScope, server_addr, client_addr, parts, scheme);
                        let (restx, mut resrx) = mpsc::channel(1);

                        tokio::task::spawn(async move {
                            let tx_ref = restx.clone();

                            match $handler_ws(callback, rt, ws, UpgradeData::new(res, restx), scope).await {
                                Ok((status, consumed, handle)) => match (consumed, handle) {
                                    (false, _) => {
                                        let _ = tx_ref
                                            .send(
                                                ResponseBuilder::new()
                                                    .status(status as u16)
                                                    .header(HK_SERVER, HV_SERVER)
                                                    .body(empty_body())
                                                    .unwrap(),
                                            )
                                            .await;
                                    }
                                    (true, Some(handle)) => {
                                        let _ = handle.await;
                                    }
                                    _ => {}
                                },
                                _ => {
                                    log::error!("RSGI protocol failure");
                                    let _ = tx_ref.send(response_500()).await;
                                }
                            }
                        });

                        return match resrx.recv().await {
                            Some(res) => {
                                resrx.close();
                                res
                            }
                            _ => response_500(),
                        };
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
                }
            }

            let (parts, body) = req.into_parts();
            let scope = build_scope!(HTTPScope, server_addr, client_addr, parts, scheme);
            handle_http_response!($handler_req, rt, callback, body, scope)
        }
    };
}

handle_request!(handle, call_http);
handle_request!(handle_pyw, call_http_pyw);
handle_request_with_ws!(handle_ws, call_http, call_ws);
handle_request_with_ws!(handle_ws_pyw, call_http_pyw, call_ws_pyw);

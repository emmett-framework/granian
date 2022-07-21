use hyper::{
    Body,
    Request,
    Response,
    StatusCode,
    header::{HeaderName, HeaderValue, SERVER as HK_SERVER},
    http::response::Builder as ResponseBuilder
};
use std::{collections::HashMap, net::SocketAddr};
use tokio::{fs::File, sync::mpsc};
use tokio_util::codec::{BytesCodec, FramedRead};

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
    callbacks::{call_response, call_protocol},
    io::{HTTPProtocol, WebsocketProtocol},
    types::{ResponseType, Scope}
};


const RESPONSE_BYTES: u32 = ResponseType::Bytes as u32;
const RESPONSE_FILEPATH: u32 = ResponseType::FilePath as u32;
const RESPONSE_STR: u32 = ResponseType::String as u32;

pub(crate) const EMPTY_BODY: &[u8] = b"";

pub trait HTTPResponseData {}

pub struct HTTPResponse<R: HTTPResponseData> {
    status: i32,
    headers: HashMap<String, String>,
    response_data: R
}

impl<T: HTTPResponseData> HTTPResponse<T> {
    pub fn response(&self) -> ResponseBuilder {
        let mut builder = Response::builder().status(self.status as u16);
        let headers = builder.headers_mut().unwrap();
        headers.insert(HK_SERVER, HV_SERVER);
        for (key, value) in self.headers.iter() {
            headers.insert(
                HeaderName::from_bytes(&key.clone().into_bytes()).unwrap(),
                HeaderValue::from_str(&value.clone().as_str()).unwrap()
            );
        };
        builder
    }

    // pub fn apply(&self, builder: ResponseBuilder) -> ResponseBuilder {
    //     let mut mbuilder = builder.status(self.status as u16);
    //     let headers = mbuilder.headers_mut().unwrap();
    //     for (key, value) in self.headers.iter() {
    //         headers.insert(
    //             HeaderName::from_bytes(&key.clone().into_bytes()).unwrap(),
    //             HeaderValue::from_str(&value.clone().as_str()).unwrap()
    //         );
    //     };
    //     mbuilder
    // }
}

pub struct HTTPEmptyResponse {}

impl HTTPResponseData for HTTPEmptyResponse {}

impl HTTPResponse<HTTPEmptyResponse> {
    pub fn new(status: i32, headers: HashMap<String, String>) -> Self {
        Self {
            status: status,
            headers: headers,
            response_data: HTTPEmptyResponse{}
        }
    }

    pub fn get_body(&mut self) -> Body {
        Body::from(EMPTY_BODY)
    }
}

// pub struct HTTPBodyResponse {
//     body: Vec<u8>
// }

// impl HTTPBodyResponse {
//     fn new() -> Self {
//         Self { body: EMPTY_BODY.to_owned() }
//     }
// }

// impl HTTPResponseData for HTTPBodyResponse {}

// impl HTTPResponse<HTTPBodyResponse> {
//     pub fn new(status: i32, headers: HashMap<String, String>) -> Self {
//         Self {
//             status: status,
//             headers: headers,
//             response_data: HTTPBodyResponse::new()
//         }
//     }

//     pub fn get_body(&mut self) -> Body {
//         // let stream = futures_util::stream::iter(self.response_data.body);
//         // Body::wrap_stream(stream)
//         // Body::from(std::mem::take(&mut self.response_data.body))
//         Body::from(self.response_data.body.to_owned())
//     }
// }

pub(crate) struct HTTPFileResponse {
    file_path: String
}

impl HTTPFileResponse {
    fn new(file_path: String) -> Self {
        Self { file_path: file_path }
    }
}

impl HTTPResponseData for HTTPFileResponse {}

impl HTTPResponse<HTTPFileResponse> {
    pub fn new(status: i32, headers: HashMap<String, String>, file_path: String) -> Self {
        Self {
            status: status,
            headers: headers,
            response_data: HTTPFileResponse::new(file_path)
        }
    }

    pub async fn get_body(&self) -> Body {
        // if let Ok(file) = File::open(&self.file_path.as_str()).await {
        //     let stream = FramedRead::new(file, BytesCodec::new());
        //     return Ok(Body::wrap_stream(stream));
        // }
        // Ok(Body::empty())
        let file = File::open(&self.response_data.file_path.as_str()).await.unwrap();
        let stream = FramedRead::new(file, BytesCodec::new());
        Body::wrap_stream(stream)
    }
}

pub(crate) async fn handle_request(
    rt: RuntimeRef,
    callback: CallbackWrapper,
    client_addr: SocketAddr,
    req: Request<Body>,
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

        match ws_upgrade(req, None) {
            Ok((res, ws)) => {
                let rth = rt.clone();
                let (restx, mut resrx) = mpsc::channel(1);

                rt.inner.spawn(async move {
                    handle_websocket(rth, res, restx, ws, callback, scope).await
                });

                return match resrx.recv().await {
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
        }
    }

    match call_response(callback, HTTPProtocol::new(rt, req), scope).await {
        Ok(pyres) => {
            let res = match pyres.mode {
                RESPONSE_BYTES => {
                    HTTPResponse::<HTTPEmptyResponse>::new(
                        pyres.status,
                        pyres.headers
                    ).response().body(pyres.bytes_data.unwrap().into())
                },
                RESPONSE_STR => {
                    HTTPResponse::<HTTPEmptyResponse>::new(
                        pyres.status,
                        pyres.headers
                    ).response().body(pyres.str_data.unwrap().into())
                },
                RESPONSE_FILEPATH => {
                    let http_obj = HTTPResponse::<HTTPFileResponse>::new(
                        pyres.status,
                        pyres.headers,
                        pyres.file_path.unwrap().to_owned()
                    );
                    http_obj.response().body(http_obj.get_body().await)
                },
                _ => {
                    let mut http_obj = HTTPResponse::<HTTPEmptyResponse>::new(
                        pyres.status,
                        pyres.headers
                    );
                    http_obj.response().body(http_obj.get_body())
                }
            };
            match res {
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

    match call_protocol(
        callback,
        WebsocketProtocol::new(rt, websocket, UpgradeData::new(response, tx)),
        scope
    ).await {
        Ok((status, consumed)) => {
            if !consumed {
                let _ = tx_ref.send(
                    ResponseBuilder::new()
                        .status(
                            StatusCode::from_u16(status as u16).unwrap_or(
                                StatusCode::FORBIDDEN
                            )
                        )
                        .header(HK_SERVER, HV_SERVER)
                        .body(Body::from(""))
                        .unwrap()
                ).await;
            }
        },
        _ => {
            let _ = tx_ref.send(response_500()).await;
        }
    }
}

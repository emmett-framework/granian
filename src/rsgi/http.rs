use hyper::{
    Body,
    Request,
    Response,
    header::{HeaderName, HeaderValue, SERVER},
    http::response::{Builder as ResponseBuilder}
};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use super::super::callbacks::CallbackWrapper;
use super::callbacks::call as callback_caller;
use super::io::Receiver;
use super::types::{ResponseType, Scope};

const RESPONSE_BYTES: u32 = ResponseType::Bytes as u32;
const RESPONSE_FILEPATH: u32 = ResponseType::FilePath as u32;
const RESPONSE_STR: u32 = ResponseType::String as u32;

const EMPTY_BODY: &[u8] = b"";
const HDR_SERVER: HeaderValue = HeaderValue::from_static("granian");

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
        headers.insert(SERVER, HDR_SERVER);
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
    callback: CallbackWrapper,
    client_addr: SocketAddr,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let scope = Scope::new(
        "http",
        req.version(),
        req.uri().clone(),
        req.method().as_ref(),
        client_addr,
        req.headers()
    );
    let receiver = Receiver::new(req);
    let pyres = callback_caller(callback, receiver, scope).await.unwrap();

    Ok(match pyres.mode {
        RESPONSE_BYTES => {
            HTTPResponse::<HTTPEmptyResponse>::new(
                pyres.status,
                pyres.headers
            ).response().body(pyres.bytes_data.unwrap().into()).unwrap()
        },
        RESPONSE_STR => {
            HTTPResponse::<HTTPEmptyResponse>::new(
                pyres.status,
                pyres.headers
            ).response().body(pyres.str_data.unwrap().into()).unwrap()
        },
        RESPONSE_FILEPATH => {
            let http_obj = HTTPResponse::<HTTPFileResponse>::new(
                pyres.status,
                pyres.headers,
                pyres.file_path.unwrap().to_owned()
            );
            http_obj.response().body(http_obj.get_body().await).unwrap()
        },
        _ => {
            let mut http_obj = HTTPResponse::<HTTPEmptyResponse>::new(
                pyres.status,
                pyres.headers
            );
            http_obj.response().body(http_obj.get_body()).unwrap()
        }
    })
}

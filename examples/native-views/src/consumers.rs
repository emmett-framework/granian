//! A native websockets "consumer": it handles the connection without involving
//! Python.
//!
//! This is the Rust counterpart of a Django (Channels) consumer: the handshake
//! is performed by Granian, the messages are handled here.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use futures::future::BoxFuture;
use futures::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use granian::{HTTPRequest, HTTPResponse, NativeContext};

type Clients = Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<Message>>>>;

/// Build the handler for a websockets endpoint broadcasting every message to
/// the clients connected to the same worker.
///
/// The clients group is bound to the handler, hence it is shared by all the
/// connections it serves.
pub fn broadcast_consumer(
) -> impl Fn(HTTPRequest, NativeContext) -> BoxFuture<'static, HTTPResponse> + Send + Sync + 'static {
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));
    let counter = Arc::new(AtomicU64::new(0));

    move |req, ctx| {
        let clients = clients.clone();
        let counter = counter.clone();

        Box::pin(async move {
            let mut req = req;
            if !granian::ws::is_upgrade_request(&req) {
                return bad_request();
            }

            match granian::ws::upgrade_intent(&mut req, None) {
                Ok((builder, ws)) => {
                    let client_id = counter.fetch_add(1, Ordering::Relaxed);
                    ctx.rt.spawn(async move {
                        match ws.await {
                            Ok(stream) => session(client_id, stream, clients).await,
                            Err(err) => {
                                log::warn!("Native websockets upgrade failed with {err:?}");
                            }
                        }
                    });

                    builder.body(granian::http::empty_body()).unwrap()
                }
                Err(err) => {
                    log::info!("Native websockets handshake failed with {err:?}");
                    bad_request()
                }
            }
        })
    }
}

fn bad_request() -> HTTPResponse {
    let builder = hyper::Response::builder().status(400);
    builder
        .body(
            http_body_util::Full::new("Bad request".into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}

async fn session(client_id: u64, stream: granian::ws::WSStream, clients: Clients) {
    let (mut sink, mut source) = stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    clients.lock().unwrap().insert(client_id, tx);

    loop {
        tokio::select! {
            msg = source.next() => {
                match msg {
                    Some(Ok(Message::Text(data))) => broadcast(&clients, Message::Text(data)),
                    Some(Ok(Message::Binary(data))) => broadcast(&clients, Message::Binary(data)),
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        log::debug!("Native websockets client error: {err:?}");
                        break;
                    }
                }
            }
            msg = rx.recv() => {
                match msg {
                    Some(msg) => {
                        if sink.send(msg).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    clients.lock().unwrap().remove(&client_id);
}

fn broadcast(clients: &Clients, msg: Message) {
    let clients = clients.lock().unwrap();
    for tx in clients.values() {
        _ = tx.send(msg.clone());
    }
}

use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use futures::channel::mpsc;
use gloo_net::websocket::{futures::WebSocket, Message};

use wasm_bindgen_futures::spawn_local;
use yew::Properties;

pub struct WebsocketService {
    pub tx: mpsc::UnboundedSender<String>,
}

impl WebsocketService {
    pub fn new() -> Self {
        let ws = WebSocket::open("ws://127.0.0.1:3000/ws").unwrap();

        let (mut write, mut read) = ws.split();

        let (in_tx, mut in_rx) = mpsc::unbounded::<String>();

        spawn_local(async move {
            while let Some(s) = in_rx.next().await {
                log::debug!("got event from channel! {}", s);
                write.send(Message::Text(s)).await.unwrap();
            }
        });

        spawn_local(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(data)) => {
                        log::debug!("from websocket: {}", data);
                    }
                    Ok(Message::Bytes(b)) => {
                        let decoded = std::str::from_utf8(&b);
                        if let Ok(val) = decoded {
                            log::debug!("from websocket: {}", val);
                        }
                    }
                    Err(e) => {
                        log::error!("ws: {:?}", e)
                    }
                }
            }
            log::debug!("WebSocket Closed");
        });

        Self { tx: in_tx }
    }

    pub fn send(&self, msg: String) {
        let _ = self.tx.unbounded_send(msg);
    }
}
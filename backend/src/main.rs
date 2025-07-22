//! Example chat application.
//!
//! Run with
//!
//! ```not_rust
//! cargo run -p example-chat
//! ```

use axum::{extract::{
    ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
    State,
}, response::{Html, IntoResponse}, routing::get, Json, Router};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use futures_util::{sink::SinkExt, stream::StreamExt};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tower_http::cors::CorsLayer;
use tower_http::cors::Any;
use tokio::sync::broadcast;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_subscriber::fmt::layer;

use axum::{
    extract::Multipart,
    routing::post
};
use serde::Serialize;
use std::net::SocketAddr;


// Our shared state
struct AppState {
    // We require unique usernames. This tracks which usernames have been taken.
    user_set: Mutex<HashSet<String>>,

    // Channel used to send messages to all connected clients.
   // tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=trace", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Set up application state for use with with_state().
    let user_set = Mutex::new(HashSet::new());
//    let (tx, _rx) = broadcast::channel(100);

    let app_state = Arc::new(AppState { user_set });

    let serve_dir = ServeDir::new("web/verifier").not_found_service(ServeFile::new("web/verifier/index.html"));
    //let yew_serve_dir = ServeDir::new("web/yew").not_found_service(ServeFile::new("web/yew/index.html"));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);


    let app = Router::new()
        .layer(cors)
        .route("/", get(index))
 //       .route("/wasm", get(wasm_index))
        .nest_service("/verifier", serve_dir.clone())
 //       .nest_service("/yew", yew_serve_dir.clone())
        .route("/ws", get(websocket_handler))
        .route("/upload", post(upload_file))
        .with_state(app_state);
    

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

// This function deals with a single websocket connection, i.e., a single
// connected client / user, for which we will spawn two independent tasks (for
// receiving / sending chat messages).
async fn websocket(mut stream: WebSocket, state: Arc<AppState>) {
    println!("WebSocket connection established");

    while let Some(Ok(msg)) = stream.next().await {
        if let Message::Text(txt) = msg {
            println!("Received: {}", txt);
            let _ = stream.send(Message::Text(txt)).await;
        }
    }

    println!("WebSocket connection closed");
}


#[derive(Serialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

async fn upload_file(mut multipart: Multipart) -> Json<EmbeddingResponse> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("").to_string();
        let data = field.bytes().await.unwrap();

        let content = String::from_utf8_lossy(&data);
        let embedding = vectorize_text(&content);

        return Json(EmbeddingResponse { embedding });
    }

    Json(EmbeddingResponse { embedding: vec![] })
}

fn vectorize_text(text: &str) -> Vec<f32> {
    let avg_ascii = text.bytes().map(|b| b as f32).sum::<f32>() / text.len() as f32;
    vec![text.len() as f32, avg_ascii]
}

// Include utf-8 file at **compile** time.
async fn index() -> Html<&'static str> {
    Html(std::include_str!("../index.html"))
}
/*
async fn wasm_index() -> Html<&'static str> {
    Html(std::include_str!("../web/wasm/wasm.html"))
}

 */
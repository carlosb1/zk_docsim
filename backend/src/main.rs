//! Example chat application.
//!
//! Run with
//!
//! ```not_rust
//! cargo run -p example-chat
//! ```

extern crate core;

mod services;

use axum::{extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    State,
}, response::{Html, IntoResponse}, routing::get, Form, Json, Router};
use tower_http::services::{ServeDir, ServeFile};

use futures_util::stream::StreamExt;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use arroy::distances::Euclidean;
use tower_http::cors::CorsLayer;
use tower_http::cors::Any;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use axum::{
    extract::Multipart,
    routing::post
};

use axum::debug_handler;
use axum::http::StatusCode;
use fastembed::EmbeddingModel::ModernBertEmbedLarge;
use serde::{Deserialize, Serialize};
use tracing::log::error;
use services::embed::{DocumentEntry, ModelEmbed};
use crate::services::simple_db_nn::{DBConfig, SimpleDBNN};

#[derive(Serialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}


#[derive(Serialize, Deserialize)]
pub struct UploadFileForm {
    name: Option<String>,
    content: Option<String>,
}



// Our shared state
struct AppState {
    memory_db: Mutex<SimpleDBNN<ModelEmbed, Euclidean>>
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
    let memory_db = Mutex::new(SimpleDBNN::from_config(DBConfig::default()).unwrap());
    let app_state = Arc::new(AppState {memory_db});

    let serve_dir = ServeDir::new("web/verifier").not_found_service(ServeFile::new("web/verifier/index.html"));
    //let yew_serve_dir = ServeDir::new("web/yew").not_found_service(ServeFile::new("web/yew/index.html"));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);


    let app = Router::new()
        .layer(cors)
        .route("/", get(index))
        .nest_service("/verifier", serve_dir.clone())
        .route("/ws", get(websocket_handler))
        .route("/upload", post(upload_file))
        .route("/search", post(search))
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



async fn upload_file(
    State(state): State<Arc<AppState>>,
    Form(form): Form<UploadFileForm>) -> Result<Json<EmbeddingResponse>, (StatusCode, Json<EmbeddingResponse>)>   {
    let name = form.name.unwrap_or(String::new());
    let content = form.content.unwrap_or(String::new());

    if content.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(EmbeddingResponse {
                embedding: vec![],
            }),
        ));
    }
    let query = format!("{:}\n{:}", name, content);
    let embedding = state.memory_db.lock().unwrap().put(query.as_str()).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(EmbeddingResponse {
                embedding: vec![], // or include error message in another field
            }),
        )
    })?;
    Ok(Json(EmbeddingResponse { embedding }))
}

#[derive(Deserialize)]
struct SearchRequest {
    content: String,
    top_k: usize,
}

#[derive(Serialize)]
struct SearchResult {
    id: u32,
    content: String,
    score: f32,
    embedding: Vec<f32>,
    receipt: Vec<u8>,
}

impl SearchResult {
    pub fn new(id: u32, score: f32,  content: String, embedding: Vec<f32>, receipt: Vec<u8>) -> Self {
        Self { id, content, score, embedding, receipt }
    }
}

async fn search(State(state): State<Arc<AppState>>, Json(req): Json<SearchRequest>)->
                                                                                   Result<
                                                                                       Json<Vec<SearchResult>>,
                                                                                       (StatusCode, Json<Vec<SearchResult>>)
                                                                                   > {

    let original_embed =  ModelEmbed::new().calculate_one_embed(DocumentEntry{content: req.content.clone()})
    .map_err(|err| {
        error!("Err={:?}", err.to_string());
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Vec::new()),
        )
    })?;
        ;
    let results = state.memory_db.lock().unwrap().get(req.content.as_str(), req.top_k).map_err(|err| {
        error!("Err={:?}", err.to_string());
        (

            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Vec::new()),
        )
    })?;
    let search_results = results.iter().map(|(id, score,entry )|
        {

            let cloned = (*entry).clone();
            let content = cloned.content;
            let embedding = cloned.embedding;
            let receipt =  host::execute_and_serialize_receipt(original_embed.clone(), embedding.clone()).unwrap();
            SearchResult::new(*id, *score, content, embedding,receipt)
        }
    ).collect::<Vec<SearchResult>>();
    Ok(Json(search_results))
}


async fn index() -> Html<&'static str> {
    Html(std::include_str!("../index.html"))
}


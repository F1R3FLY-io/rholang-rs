#![cfg(not(target_arch = "wasm32"))]

use std::{net::SocketAddr, sync::Arc};

use axum::{extract::State, http::StatusCode, routing::{post, get}, Json, Router};
use rholang_shell::providers::{InterpreterProvider, RholangParserInterpreterProvider, InterpretationResult};
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, services::ServeDir};

#[derive(Clone)]
struct AppState {
    interp: Arc<RholangParserInterpreterProvider>,
}

#[derive(Debug, Deserialize)]
struct RunRequest {
    code: String,
}

#[derive(Debug, Serialize)]
struct RunResponse {
    ok: bool,
    output: String,
    error: Option<String>,
}

#[tokio::main]
async fn main() {
    // Initialize real interpreter provider
    let interp = Arc::new(RholangParserInterpreterProvider::new().expect("Failed to init interpreter"));

    let state = AppState { interp };

    // Serve static files under /www and /pkg, and expose /api/run
    let app = Router::new()
        .route("/api/run", post(run))
        .route("/health", get(health))
        .nest_service("/www", ServeDir::new("rholang-wasm/www"))
        .nest_service("/pkg", ServeDir::new("rholang-wasm/pkg"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = ([0, 0, 0, 0], 8080).into();
    println!("rholang-wasm server listening on http://{addr}");
    eprintln!("Open: http://127.0.0.1:8080/www/index.html");

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("server");
}

async fn health() -> &'static str { "ok" }

async fn run(State(state): State<AppState>, Json(req): Json<RunRequest>) -> Result<Json<RunResponse>, (StatusCode, String)> {
    let code = req.code.trim().to_string();
    if code.is_empty() {
        return Ok(Json(RunResponse { ok: true, output: "<empty input>".to_string(), error: None }));
    }

    // Call the real interpreter
    match state.interp.interpret(&code).await {
        InterpretationResult::Success(out) => Ok(Json(RunResponse { ok: true, output: out, error: None })),
        InterpretationResult::Error(e) => Ok(Json(RunResponse { ok: false, output: String::new(), error: Some(e.to_string()) })),
    }
}

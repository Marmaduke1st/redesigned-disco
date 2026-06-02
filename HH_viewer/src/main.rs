use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;

use hh_viewer::{default_hands_root, read_hand_text, render_hand_page, render_index, AppState};

#[tokio::main]
async fn main() {
    let hands_root = env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(default_hands_root);
    let port = env::args()
        .nth(2)
        .and_then(|value| value.parse::<u16>().ok())
        .or_else(|| env::var("PORT").ok().and_then(|value| value.parse::<u16>().ok()))
        .unwrap_or(3001);
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let state = AppState { hands_root };

    let app = Router::new()
        .route("/", get(index))
        .route("/hand/:hand_id", get(view_hand))
        .route("/download/:hand_id", get(download_hand))
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse().expect("valid bind address");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind viewer port");
    axum::serve(listener, app).await.expect("serve viewer app");
}

async fn index(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_index(&state.hands_root, query.get("q").map(String::as_str))
        .map(Html)
        .map_err(internal_error)
}

async fn view_hand(
    State(state): State<AppState>,
    Path(hand_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let hand_text = read_hand_text(&state.hands_root, &hand_id)
        .map_err(|_| (StatusCode::NOT_FOUND, format!("hand not found: {hand_id}")))?;
    Ok(Html(render_hand_page(&hand_id, &hand_text)))
}

async fn download_hand(
    State(state): State<AppState>,
    Path(hand_id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let hand_text = read_hand_text(&state.hands_root, &hand_id)
        .map_err(|_| (StatusCode::NOT_FOUND, format!("hand not found: {hand_id}")))?;

    let mut response = hand_text.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{hand_id}.txt\""))
            .map_err(internal_error)?,
    );
    Ok(response)
}

fn internal_error<E: std::fmt::Display>(error: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

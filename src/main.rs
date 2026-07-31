mod db;
mod lb;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db::{Db, NowPlaying};
use crate::lb::SubmitRequest;

struct AppState {
    db: Db,
    token: String,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn json_response(status: StatusCode, body: Value) -> Response {
    (status, Json(body)).into_response()
}

fn err(status: StatusCode, message: &str) -> Response {
    json_response(status, json!({ "error": message }))
}

fn ok() -> Response {
    json_response(StatusCode::OK, json!({ "status": "ok" }))
}

fn authorized(state: &AppState, headers: &HeaderMap) -> Option<Response> {
    if state.token.is_empty() {
        return Some(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "server not configured: NOWPLAYING_TOKEN is not set",
        ));
    }
    let expected = format!("Token {}", state.token);
    match headers.get("authorization").and_then(|v| v.to_str().ok()) {
        Some(value) if value == expected => None,
        _ => Some(err(
            StatusCode::UNAUTHORIZED,
            "invalid authorization token",
        )),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bind = std::env::var("NOWPLAYING_BIND").unwrap_or_else(|_| "[::]:8080".to_string());
    let db_path = std::env::var("NOWPLAYING_DB").unwrap_or_else(|_| "nowplaying.db".to_string());
    let token = std::env::var("NOWPLAYING_TOKEN").unwrap_or_default();

    let db = Db::open(&db_path).context("open database")?;
    let state = Arc::new(AppState { db, token });

    let app = Router::new()
        .route("/", get(health))
        .route("/1/submit-listens", axum::routing::post(submit_listens))
        .route("/api/v1/nowplaying", get(nowplaying))
        .route("/api/v1/listens", get(listens))
        .with_state(state);

    let addr: SocketAddr = bind
        .parse()
        .with_context(|| format!("invalid NOWPLAYING_BIND: {bind}"))?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("cannot bind {addr}"))?;

    println!("nowplaying listening on {addr} (db: {db_path})");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    json_response(StatusCode::OK, json!({ "status": "ok" }))
}

async fn submit_listens(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Option<Json<SubmitRequest>>,
) -> Response {
    if let Some(response) = authorized(&state, &headers) {
        return response;
    }
    let Some(Json(request)) = body else {
        return err(StatusCode::BAD_REQUEST, "invalid JSON body");
    };

    match request.listen_type.as_str() {
        "playing_now" => {
            let Some(listen) = request.payload.first() else {
                return err(StatusCode::BAD_REQUEST, "playing_now payload is empty");
            };
            state.db.set_now_playing(listen, unix_now());
            ok()
        }
        "single" | "import" => {
            if request.payload.is_empty() {
                return err(StatusCode::BAD_REQUEST, "payload is empty");
            }
            state.db.insert_listens(&request.payload, unix_now());
            ok()
        }
        other => err(
            StatusCode::BAD_REQUEST,
            &format!("unknown listen_type: {other}"),
        ),
    }
}

fn position_of(np: &NowPlaying, now: i64) -> i64 {
    let frozen = np.position_frozen.unwrap_or(0).max(0);
    let position = if np.state == "playing" {
        (now - np.started_at).max(0)
    } else {
        frozen
    };
    if let Some(duration) = np.duration.filter(|d| *d > 0) {
        position.min(duration)
    } else {
        position
    }
}

async fn nowplaying(State(state): State<Arc<AppState>>) -> Response {
    let Some(np) = state.db.latest_now_playing() else {
        return err(StatusCode::NO_CONTENT, "nothing playing");
    };
    json_response(
        StatusCode::OK,
        json!({
            "origin": np.origin,
            "artist": np.artist,
            "title": np.title,
            "album": np.album,
            "length": np.duration,
            "position": position_of(&np, unix_now()),
            "state": np.state,
            "started_at": np.started_at,
            "updated_at": np.updated_at,
        }),
    )
}

#[derive(Debug, Deserialize, Default)]
struct LimitQuery {
    limit: Option<u32>,
}

async fn listens(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LimitQuery>,
) -> Response {
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let rows = state.db.list_listens(limit);
    let listens: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "listened_at": r.listened_at,
                "artist": r.artist,
                "title": r.title,
                "album": r.album,
                "duration": r.duration,
                "origin": r.origin,
            })
        })
        .collect();
    json_response(
        StatusCode::OK,
        json!({ "count": listens.len(), "listens": listens }),
    )
}

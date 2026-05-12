//! HTTP REST server for clipboarder.
//!
//! Endpoints (all under /v1, bearer auth required):
//!   GET    /v1/health                  → 200 ok
//!   GET    /v1/whoami                  → {namespace, label}
//!   GET    /v1/items?q&kind&limit      → list/search
//!   GET    /v1/items/:id               → show
//!   POST   /v1/items                   → add  (body: {content, kind?, meta?, source_app?})
//!   DELETE /v1/items/:id               → delete
//!   POST   /v1/items/:id/pin           → pin
//!   DELETE /v1/items/:id/pin           → unpin
//!   POST   /v1/clear                   → clear non-pinned in this namespace
//!   GET    /v1/stats                   → {total, pinned, by_kind, ...}
//!   GET    /v1/watch                   → SSE stream of new items
//!
//! Namespace is implicit — it's derived from the bearer token.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{sse::Event, IntoResponse, Sse},
    routing::{delete, get, post},
    Json, Router,
};
use futures::stream::Stream;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::broadcast;
use tokio::time::interval;

use crate::classify;
use crate::server_config::{ServerConfig, TokenEntry};
use crate::storage::{ClipItem, NewItem, Storage};

#[derive(Clone)]
struct AppState {
    config: Arc<ServerConfig>,
    storage: Arc<Mutex<Storage>>,
    /// Broadcast of (namespace, item-id) for newly-inserted rows. Watch
    /// subscribers receive only items in their own namespace.
    new_items: broadcast::Sender<(String, i64)>,
}

#[derive(Debug, Clone)]
struct AuthedNs(String);

pub async fn run(config_path: PathBuf, bind_override: Option<String>) -> Result<()> {
    let config = ServerConfig::load(&config_path)
        .with_context(|| format!("load config: {}", config_path.display()))?;

    let bind = bind_override.clone().unwrap_or_else(|| config.bind.clone());
    let addr: SocketAddr = bind.parse().with_context(|| format!("parse bind: {bind}"))?;

    let data_dir = config
        .data_dir
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join("Library/Application Support/com.clipboarder.app")
        });
    std::fs::create_dir_all(&data_dir).ok();
    let db_path = data_dir.join("clipboarder.sqlite");
    let storage = Storage::open(&db_path).context("open clipboarder.sqlite")?;

    let (tx, _) = broadcast::channel(256);
    let state = AppState {
        config: Arc::new(config),
        storage: Arc::new(Mutex::new(storage)),
        new_items: tx,
    };

    let app = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/whoami", get(whoami))
        .route("/v1/items", get(items_list).post(items_create))
        .route("/v1/items/:id", get(items_show).delete(items_delete))
        .route("/v1/items/:id/pin", post(items_pin).delete(items_unpin))
        .route("/v1/clear", post(clear_history))
        .route("/v1/stats", get(stats))
        .route("/v1/watch", get(watch_sse))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(8 * 1024 * 1024))
        .with_state(state);

    eprintln!("[clipboarder server] listening on {addr}");
    eprintln!("[clipboarder server] config: {}", config_path.display());
    eprintln!("[clipboarder server] data dir: {}", data_dir.display());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// ── auth middleware (function-style; runs inline at the top of each handler) ─

fn auth(state: &AppState, headers: &HeaderMap) -> Result<AuthedNs, StatusCode> {
    let Some(value) = headers.get(header::AUTHORIZATION) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let s = value.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;
    let token = s.strip_prefix("Bearer ").ok_or(StatusCode::UNAUTHORIZED)?.trim();
    let entry = state.config.lookup_token(token).ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(AuthedNs(entry.namespace.clone()))
}

fn lookup_label<'a>(state: &'a AppState, token: &str) -> Option<&'a TokenEntry> {
    state.config.lookup_token(token)
}

// ── handlers ────────────────────────────────────────────────────────────────

async fn health() -> &'static str { "ok" }

#[derive(Serialize)]
struct WhoamiBody {
    namespace: String,
    label: Option<String>,
}

async fn whoami(State(s): State<AppState>, headers: HeaderMap) -> Result<Json<WhoamiBody>, StatusCode> {
    let AuthedNs(ns) = auth(&s, &headers)?;
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
        .trim()
        .to_string();
    let entry = lookup_label(&s, &token);
    Ok(Json(WhoamiBody {
        namespace: ns,
        label: entry.and_then(|t| t.label.clone()),
    }))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 { 50 }

async fn items_list(
    State(s): State<AppState>,
    headers: HeaderMap,
    Query(qry): Query<ListQuery>,
) -> Result<Json<Vec<ClipItem>>, StatusCode> {
    let AuthedNs(ns) = auth(&s, &headers)?;
    let kind = qry.kind.unwrap_or_else(|| "all".into());
    let q = qry.q.unwrap_or_default();
    let items = s
        .storage
        .lock()
        .search(&q, &kind, qry.limit.max(1).min(10_000), &ns)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(items))
}

async fn items_show(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<ClipItem>, StatusCode> {
    let AuthedNs(ns) = auth(&s, &headers)?;
    let it = s
        .storage
        .lock()
        .get(id, &ns)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(it))
}

#[derive(Deserialize)]
struct CreateBody {
    content: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    meta: Option<String>,
    #[serde(default)]
    source_app: Option<String>,
}

#[derive(Serialize)]
struct CreateReply {
    id: i64,
    inserted: bool,
    kind: String,
}

async fn items_create(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateBody>,
) -> Result<Json<CreateReply>, StatusCode> {
    let AuthedNs(ns) = auth(&s, &headers)?;
    if body.content.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let cls = classify::classify_text(&body.content);
    let kind = body
        .kind
        .as_deref()
        .map(classify::Kind::from_str)
        .unwrap_or(cls.kind);
    let meta_owned: String;
    let meta_ref: Option<&str> = if let Some(m) = body.meta.as_deref() {
        Some(m)
    } else if let Some(m) = cls.meta.as_deref() {
        meta_owned = m.to_string();
        Some(meta_owned.as_str())
    } else {
        None
    };

    let mut h = Sha256::new();
    h.update(body.content.as_bytes());
    let hash: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();

    let (id, inserted) = s
        .storage
        .lock()
        .upsert(
            &NewItem {
                kind,
                content: &body.content,
                preview: &cls.preview,
                meta: meta_ref,
                source_app: body.source_app.as_deref(),
                source_app_id: None,
                image_path: None,
                content_hash: &hash,
                size: body.content.len() as i64,
            },
            &ns,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if inserted {
        let _ = s.new_items.send((ns.clone(), id));
    }

    Ok(Json(CreateReply {
        id,
        inserted,
        kind: kind.as_str().to_string(),
    }))
}

async fn items_delete(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let AuthedNs(ns) = auth(&s, &headers)?;
    // Only delete if it actually exists in the caller's namespace.
    let exists = s
        .storage
        .lock()
        .get(id, &ns)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();
    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }
    let img = s
        .storage
        .lock()
        .delete(id, &ns)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(p) = img {
        let _ = std::fs::remove_file(p);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct PinReply { id: i64, pinned: bool }

async fn items_pin(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<PinReply>, StatusCode> {
    set_pin(s, headers, id, true).await
}

async fn items_unpin(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<PinReply>, StatusCode> {
    set_pin(s, headers, id, false).await
}

async fn set_pin(
    s: AppState,
    headers: HeaderMap,
    id: i64,
    want: bool,
) -> Result<Json<PinReply>, StatusCode> {
    let AuthedNs(ns) = auth(&s, &headers)?;
    let cur = s
        .storage
        .lock()
        .get(id, &ns)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if cur.pinned != want {
        s.storage
            .lock()
            .toggle_pin(id, &ns)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(Json(PinReply { id, pinned: want }))
}

async fn clear_history(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let AuthedNs(ns) = auth(&s, &headers)?;
    let imgs = s
        .storage
        .lock()
        .clear(&ns)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for p in imgs {
        let _ = std::fs::remove_file(p);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct StatsBody {
    total: i64,
    pinned: i64,
    by_kind: std::collections::BTreeMap<String, i64>,
    namespace: String,
}

async fn stats(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<StatsBody>, StatusCode> {
    let AuthedNs(ns) = auth(&s, &headers)?;
    let all = s
        .storage
        .lock()
        .search("", "all", 1_000_000, &ns)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let total = all.len() as i64;
    let pinned = all.iter().filter(|i| i.pinned).count() as i64;
    let mut by_kind = std::collections::BTreeMap::new();
    for it in &all {
        *by_kind.entry(it.kind.clone()).or_insert(0) += 1;
    }
    Ok(Json(StatsBody { total, pinned, by_kind, namespace: ns }))
}

// ── SSE watch ───────────────────────────────────────────────────────────────

async fn watch_sse(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, std::io::Error>>>, StatusCode> {
    let AuthedNs(ns) = auth(&s, &headers)?;
    let mut rx = s.new_items.subscribe();
    let storage = s.storage.clone();

    let stream = async_stream::stream! {
        // Initial keepalive so clients confirm the stream is open.
        yield Ok(Event::default().event("ready").data(""));
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok((row_ns, id)) if row_ns == ns => {
                            let item = storage.lock().get(id, &ns).ok().flatten();
                            if let Some(it) = item {
                                let json = serde_json::to_string(&it).unwrap_or_default();
                                yield Ok(Event::default().event("item").data(json));
                            }
                        }
                        Ok(_) => { /* different namespace, skip */ }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(15)) => {
                    // keepalive ping every 15s so reverse proxies don't kill us
                    yield Ok(Event::default().event("ping").data(""));
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    ))
}

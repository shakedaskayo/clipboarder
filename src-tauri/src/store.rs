//! Abstract item store — the CLI talks to this, not directly to SQLite.
//!
//! Two implementations:
//! - `LocalStore`  : wraps `Storage` (the in-process SQLite handle)
//! - `RemoteStore` : speaks to a `clipboarder serve` HTTP backend with a
//!                   bearer token, fully namespace-scoped on the server side
//!
//! The CLI picks one at runtime via `open_store()`:
//!
//!     CLIPBOARDER_SERVER + CLIPBOARDER_TOKEN  → RemoteStore
//!     otherwise                                → LocalStore
//!
//! …with `~/.config/clipboarder/client.toml` as the fallback for either.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::classify;
use crate::server_config::resolve_client;
use crate::storage::{ClipItem, NewItem, Storage, DEFAULT_NAMESPACE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackend {
    Local,
    Remote,
}

pub trait ItemStore: Send + Sync {
    fn backend(&self) -> StoreBackend;
    fn describe(&self) -> String;
    fn namespace(&self) -> &str;

    fn search(&self, query: &str, kind: &str, limit: i64) -> Result<Vec<ClipItem>>;
    fn get(&self, id: i64) -> Result<Option<ClipItem>>;
    fn upsert(&self, req: &UpsertRequest) -> Result<UpsertReply>;
    fn set_pin(&self, id: i64, want_pinned: bool) -> Result<()>;
    fn delete(&self, id: i64) -> Result<()>;
    fn clear(&self) -> Result<()>;
    fn stats(&self) -> Result<StatsView>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRequest {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertReply {
    pub id: i64,
    pub inserted: bool,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsView {
    pub total: i64,
    pub pinned: i64,
    pub by_kind: std::collections::BTreeMap<String, i64>,
    pub namespace: String,
}

// ── factory ────────────────────────────────────────────────────────────────

pub fn open_store(data_dir: PathBuf) -> Result<Box<dyn ItemStore>> {
    let (server, token, ns) = resolve_client();
    if let (Some(server), Some(token)) = (server, token) {
        let ns = ns.unwrap_or_else(|| DEFAULT_NAMESPACE.into());
        let remote = RemoteStore::new(server, token, ns)
            .context("create remote store")?;
        return Ok(Box::new(remote));
    }
    let ns = ns.unwrap_or_else(|| DEFAULT_NAMESPACE.into());
    let local = LocalStore::open(&ns, data_dir).context("open local store")?;
    Ok(Box::new(local))
}

// ── LocalStore ─────────────────────────────────────────────────────────────

pub struct LocalStore {
    storage: Mutex<Storage>,
    namespace: String,
    data_dir: PathBuf,
}

impl LocalStore {
    pub fn open(namespace: &str, data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir).ok();
        let db = data_dir.join("clipboarder.sqlite");
        let storage = Storage::open(&db).context("open clipboarder database")?;
        Ok(Self {
            storage: Mutex::new(storage),
            namespace: namespace.to_string(),
            data_dir,
        })
    }

    /// Direct mutable access for things that bypass the trait (e.g. retention
    /// pruning called by the GUI's startup path). The CLI doesn't use this.
    #[allow(dead_code)]
    pub fn raw_storage(&self) -> &Mutex<Storage> { &self.storage }
}

impl ItemStore for LocalStore {
    fn backend(&self) -> StoreBackend { StoreBackend::Local }

    fn describe(&self) -> String {
        format!("local ({}, namespace `{}`)", self.data_dir.display(), self.namespace)
    }

    fn namespace(&self) -> &str { &self.namespace }

    fn search(&self, query: &str, kind: &str, limit: i64) -> Result<Vec<ClipItem>> {
        self.storage.lock().search(query, kind, limit, &self.namespace)
    }

    fn get(&self, id: i64) -> Result<Option<ClipItem>> {
        self.storage.lock().get(id, &self.namespace)
    }

    fn upsert(&self, req: &UpsertRequest) -> Result<UpsertReply> {
        if req.content.is_empty() {
            anyhow::bail!("upsert: empty content");
        }
        let cls = classify::classify_text(&req.content);
        let kind = req
            .kind
            .as_deref()
            .map(classify::Kind::from_str)
            .unwrap_or(cls.kind);
        let meta = req.meta.clone().or(cls.meta);

        let mut h = Sha256::new();
        h.update(req.content.as_bytes());
        let hash: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();

        let mut db = self.storage.lock();
        let (id, inserted) = db.upsert(
            &NewItem {
                kind,
                content: &req.content,
                preview: &cls.preview,
                meta: meta.as_deref(),
                source_app: req.source_app.as_deref(),
                source_app_id: None,
                image_path: None,
                content_hash: &hash,
                size: req.content.len() as i64,
            },
            &self.namespace,
        )?;

        Ok(UpsertReply {
            id,
            inserted,
            kind: kind.as_str().to_string(),
        })
    }

    fn set_pin(&self, id: i64, want_pinned: bool) -> Result<()> {
        let mut db = self.storage.lock();
        let cur = db.get(id, &self.namespace)?;
        let Some(cur) = cur else {
            anyhow::bail!("item {id} not found");
        };
        if cur.pinned != want_pinned {
            db.toggle_pin(id, &self.namespace)?;
        }
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        let mut db = self.storage.lock();
        let img = db.delete(id, &self.namespace)?;
        drop(db);
        if let Some(p) = img {
            let _ = std::fs::remove_file(p);
        }
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        let mut db = self.storage.lock();
        let imgs = db.clear(&self.namespace)?;
        drop(db);
        for p in imgs {
            let _ = std::fs::remove_file(p);
        }
        Ok(())
    }

    fn stats(&self) -> Result<StatsView> {
        let items = self
            .storage
            .lock()
            .search("", "all", 1_000_000, &self.namespace)?;
        let mut by_kind = std::collections::BTreeMap::new();
        for it in &items {
            *by_kind.entry(it.kind.clone()).or_insert(0) += 1;
        }
        Ok(StatsView {
            total: items.len() as i64,
            pinned: items.iter().filter(|i| i.pinned).count() as i64,
            by_kind,
            namespace: self.namespace.clone(),
        })
    }
}

// ── RemoteStore ────────────────────────────────────────────────────────────

pub struct RemoteStore {
    client: reqwest::blocking::Client,
    base: String,
    token: String,
    /// Cached from /v1/whoami. The server is the source of truth for which
    /// namespace this token belongs to; we display it in doctor / stats.
    namespace: String,
}

impl RemoteStore {
    pub fn new(base: String, token: String, requested_namespace: String) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("reqwest client")?;
        let base = base.trim_end_matches('/').to_string();
        // Verify the server is reachable + token is valid + read the actual
        // namespace from /v1/whoami. The server's mapping wins over any
        // user-supplied CLIPBOARDER_NAMESPACE override; we only use the
        // hint if /v1/whoami isn't available.
        let resp = client
            .get(format!("{base}/v1/whoami"))
            .bearer_auth(&token)
            .send();
        let namespace = match resp {
            Ok(r) if r.status().is_success() => {
                let body: serde_json::Value = r.json().unwrap_or_default();
                body.get("namespace")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or(requested_namespace)
            }
            Ok(r) => {
                anyhow::bail!(
                    "server rejected token: HTTP {} from {base}/v1/whoami",
                    r.status()
                );
            }
            Err(e) => {
                anyhow::bail!("cannot reach {base}: {e}");
            }
        };
        Ok(Self { client, base, token, namespace })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn req(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> reqwest::blocking::RequestBuilder {
        self.client
            .request(method, self.url(path))
            .bearer_auth(&self.token)
    }
}

impl ItemStore for RemoteStore {
    fn backend(&self) -> StoreBackend { StoreBackend::Remote }

    fn describe(&self) -> String {
        format!("remote ({}, namespace `{}`)", self.base, self.namespace)
    }

    fn namespace(&self) -> &str { &self.namespace }

    fn search(&self, query: &str, kind: &str, limit: i64) -> Result<Vec<ClipItem>> {
        let mut req = self
            .req(reqwest::Method::GET, "/v1/items")
            .query(&[("limit", &limit.to_string())]);
        if !query.is_empty() {
            req = req.query(&[("q", query)]);
        }
        if kind != "all" {
            req = req.query(&[("kind", kind)]);
        }
        let resp = req.send().context("GET /v1/items")?;
        if !resp.status().is_success() {
            anyhow::bail!("remote search: HTTP {}", resp.status());
        }
        Ok(resp.json().context("decode items")?)
    }

    fn get(&self, id: i64) -> Result<Option<ClipItem>> {
        let resp = self
            .req(reqwest::Method::GET, &format!("/v1/items/{id}"))
            .send()
            .context("GET /v1/items/:id")?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            anyhow::bail!("remote get: HTTP {}", resp.status());
        }
        Ok(Some(resp.json().context("decode item")?))
    }

    fn upsert(&self, req: &UpsertRequest) -> Result<UpsertReply> {
        let resp = self
            .req(reqwest::Method::POST, "/v1/items")
            .json(req)
            .send()
            .context("POST /v1/items")?;
        if !resp.status().is_success() {
            anyhow::bail!("remote upsert: HTTP {}", resp.status());
        }
        Ok(resp.json().context("decode upsert reply")?)
    }

    fn set_pin(&self, id: i64, want_pinned: bool) -> Result<()> {
        let method = if want_pinned {
            reqwest::Method::POST
        } else {
            reqwest::Method::DELETE
        };
        let resp = self
            .req(method, &format!("/v1/items/{id}/pin"))
            .send()
            .context("/v1/items/:id/pin")?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("item {id} not found");
        }
        if !resp.status().is_success() {
            anyhow::bail!("remote pin: HTTP {}", resp.status());
        }
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        let resp = self
            .req(reqwest::Method::DELETE, &format!("/v1/items/{id}"))
            .send()
            .context("DELETE /v1/items/:id")?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("item {id} not found");
        }
        if !resp.status().is_success() {
            anyhow::bail!("remote delete: HTTP {}", resp.status());
        }
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        let resp = self
            .req(reqwest::Method::POST, "/v1/clear")
            .send()
            .context("POST /v1/clear")?;
        if !resp.status().is_success() {
            anyhow::bail!("remote clear: HTTP {}", resp.status());
        }
        Ok(())
    }

    fn stats(&self) -> Result<StatsView> {
        let resp = self
            .req(reqwest::Method::GET, "/v1/stats")
            .send()
            .context("GET /v1/stats")?;
        if !resp.status().is_success() {
            anyhow::bail!("remote stats: HTTP {}", resp.status());
        }
        Ok(resp.json().context("decode stats")?)
    }
}

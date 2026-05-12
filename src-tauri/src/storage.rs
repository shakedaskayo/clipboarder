//! SQLite storage with FTS5 full-text search.
//!
//! Schema:
//!   items(id, kind, content, preview, meta, source_app, image_path, pinned,
//!         size, content_hash, created_at, last_used_at)
//!   items_fts(content, preview, meta) -- FTS5 mirror, kept in sync via triggers.

use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::classify::Kind;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS items (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    kind          TEXT NOT NULL,
    content       TEXT NOT NULL,
    preview       TEXT NOT NULL,
    meta          TEXT,
    source_app    TEXT,
    source_app_id TEXT,
    image_path    TEXT,
    pinned        INTEGER NOT NULL DEFAULT 0,
    size          INTEGER NOT NULL DEFAULT 0,
    content_hash  TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    last_used_at  INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_items_hash ON items(content_hash);
CREATE INDEX IF NOT EXISTS idx_items_last_used ON items(last_used_at DESC);
CREATE INDEX IF NOT EXISTS idx_items_kind ON items(kind, last_used_at DESC);

CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
    content,
    preview,
    meta,
    content='items',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS items_ai AFTER INSERT ON items BEGIN
    INSERT INTO items_fts(rowid, content, preview, meta)
    VALUES (new.id, new.content, new.preview, COALESCE(new.meta, ''));
END;

CREATE TRIGGER IF NOT EXISTS items_ad AFTER DELETE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, content, preview, meta)
    VALUES ('delete', old.id, old.content, old.preview, COALESCE(old.meta, ''));
END;

CREATE TRIGGER IF NOT EXISTS items_au AFTER UPDATE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, content, preview, meta)
    VALUES ('delete', old.id, old.content, old.preview, COALESCE(old.meta, ''));
    INSERT INTO items_fts(rowid, content, preview, meta)
    VALUES (new.id, new.content, new.preview, COALESCE(new.meta, ''));
END;
"#;

pub struct Storage {
    conn: Connection,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClipItem {
    pub id: i64,
    pub kind: String,
    pub content: String,
    pub preview: String,
    pub source_app: Option<String>,
    pub source_app_id: Option<String>,
    pub meta: Option<String>,
    pub image_path: Option<String>,
    pub pinned: bool,
    pub size: i64,
    pub created_at: i64,
    pub last_used_at: i64,
}

#[derive(Debug, Clone)]
pub struct NewItem<'a> {
    pub kind: Kind,
    pub content: &'a str,
    pub preview: &'a str,
    pub meta: Option<&'a str>,
    pub source_app: Option<&'a str>,
    pub source_app_id: Option<&'a str>,
    pub image_path: Option<&'a str>,
    pub content_hash: &'a str,
    pub size: i64,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(SCHEMA)?;

        // Migrations for existing DBs created before new columns existed.
        let _ = conn.execute("ALTER TABLE items ADD COLUMN source_app_id TEXT", []);

        Ok(Self { conn })
    }

    /// Insert or refresh an item (dedup by content_hash). Returns the row id and
    /// whether the row was newly inserted (false = existing item bumped).
    pub fn upsert(&mut self, item: &NewItem) -> Result<(i64, bool)> {
        let now = chrono::Utc::now().timestamp_millis();
        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM items WHERE content_hash = ?1",
                params![item.content_hash],
                |r| r.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            self.conn.execute(
                "UPDATE items SET last_used_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
            Ok((id, false))
        } else {
            self.conn.execute(
                "INSERT INTO items (kind, content, preview, meta, source_app, source_app_id, image_path, pinned, size, content_hash, created_at, last_used_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10, ?10)",
                params![
                    item.kind.as_str(),
                    item.content,
                    item.preview,
                    item.meta,
                    item.source_app,
                    item.source_app_id,
                    item.image_path,
                    item.size,
                    item.content_hash,
                    now,
                ],
            )?;
            Ok((self.conn.last_insert_rowid(), true))
        }
    }

    pub fn search(&self, query: &str, kind: &str, limit: i64) -> Result<Vec<ClipItem>> {
        let q = query.trim();
        let kind_filter = match kind {
            "all" | "" => None,
            "pinned" => None,
            k => Some(k.to_string()),
        };
        let only_pinned = kind == "pinned";

        let mut rows = Vec::new();
        if q.is_empty() {
            // No query: just recent items, optionally filtered by kind.
            let mut sql = String::from(
                "SELECT id, kind, content, preview, meta, source_app, source_app_id, image_path, pinned, size, created_at, last_used_at
                 FROM items WHERE 1=1",
            );
            if kind_filter.is_some() { sql.push_str(" AND kind = ?1"); }
            if only_pinned { sql.push_str(" AND pinned = 1"); }
            sql.push_str(" ORDER BY pinned DESC, last_used_at DESC LIMIT ?2");
            let mut stmt = self.conn.prepare(&sql)?;
            let kind_param = kind_filter.clone().unwrap_or_default();
            let mut q = stmt.query(params![kind_param, limit])?;
            while let Some(r) = q.next()? {
                rows.push(row_to_item(r)?);
            }
        } else {
            // FTS5 query — escape special chars by quoting each term.
            let match_expr = build_match_expr(q);
            let mut sql = String::from(
                "SELECT i.id, i.kind, i.content, i.preview, i.meta, i.source_app, i.source_app_id, i.image_path, i.pinned, i.size, i.created_at, i.last_used_at
                 FROM items_fts f
                 JOIN items i ON i.id = f.rowid
                 WHERE items_fts MATCH ?1",
            );
            if kind_filter.is_some() { sql.push_str(" AND i.kind = ?2"); }
            if only_pinned { sql.push_str(" AND i.pinned = 1"); }
            sql.push_str(" ORDER BY i.pinned DESC, bm25(items_fts) ASC, i.last_used_at DESC LIMIT ?3");
            let mut stmt = self.conn.prepare(&sql)?;
            let kind_param = kind_filter.clone().unwrap_or_default();
            let mut q = stmt.query(params![match_expr, kind_param, limit])?;
            while let Some(r) = q.next()? {
                rows.push(row_to_item(r)?);
            }
        }
        Ok(rows)
    }

    pub fn get(&self, id: i64) -> Result<Option<ClipItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, content, preview, meta, source_app, source_app_id, image_path, pinned, size, created_at, last_used_at
             FROM items WHERE id = ?1",
        )?;
        let item = stmt
            .query_row(params![id], |r| Ok(row_to_item(r).unwrap()))
            .optional()?;
        Ok(item)
    }

    pub fn bump(&mut self, id: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        self.conn.execute("UPDATE items SET last_used_at = ?1 WHERE id = ?2", params![now, id])?;
        Ok(())
    }

    pub fn toggle_pin(&mut self, id: i64) -> Result<bool> {
        let cur: i64 = self
            .conn
            .query_row("SELECT pinned FROM items WHERE id = ?1", params![id], |r| r.get(0))?;
        let next = if cur == 0 { 1 } else { 0 };
        self.conn
            .execute("UPDATE items SET pinned = ?1 WHERE id = ?2", params![next, id])?;
        Ok(next == 1)
    }

    pub fn delete(&mut self, id: i64) -> Result<Option<String>> {
        let img: Option<String> = self
            .conn
            .query_row("SELECT image_path FROM items WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?
            .flatten();
        self.conn.execute("DELETE FROM items WHERE id = ?1", params![id])?;
        Ok(img)
    }

    pub fn clear(&mut self) -> Result<Vec<String>> {
        let mut imgs = Vec::new();
        let mut stmt = self.conn.prepare("SELECT image_path FROM items WHERE image_path IS NOT NULL AND pinned = 0")?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            if let Ok(Some(p)) = r.get::<_, Option<String>>(0) { imgs.push(p); }
        }
        self.conn.execute("DELETE FROM items WHERE pinned = 0", [])?;
        Ok(imgs)
    }

    /// Enforce a max-item budget by deleting the oldest non-pinned rows.
    /// Returns image paths whose rows were deleted (caller should unlink them).
    pub fn enforce_limit(&mut self, max_items: u32) -> Result<Vec<String>> {
        if max_items == 0 { return Ok(Vec::new()); }
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM items WHERE pinned = 0", [], |r| r.get(0))?;
        let max = max_items as i64;
        if count <= max { return Ok(Vec::new()); }
        let excess = count - max;
        let mut imgs = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT id, image_path FROM items
             WHERE pinned = 0
             ORDER BY last_used_at ASC
             LIMIT ?1",
        )?;
        let mut rows = stmt.query(params![excess])?;
        let mut ids: Vec<i64> = Vec::new();
        while let Some(r) = rows.next()? {
            ids.push(r.get::<_, i64>(0)?);
            if let Ok(Some(p)) = r.get::<_, Option<String>>(1) { imgs.push(p); }
        }
        drop(rows);
        drop(stmt);
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        if !ids.is_empty() {
            let sql = format!("DELETE FROM items WHERE id IN ({})", placeholders);
            let mut stmt = self.conn.prepare(&sql)?;
            stmt.execute(rusqlite::params_from_iter(ids.iter()))?;
        }
        Ok(imgs)
    }

    /// Delete non-pinned items older than `days` (by last_used_at).
    pub fn prune_older_than(&mut self, days: u32) -> Result<Vec<String>> {
        if days == 0 { return Ok(Vec::new()); }
        let cutoff = chrono::Utc::now().timestamp_millis() - (days as i64) * 86_400_000;
        let mut imgs = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT image_path FROM items WHERE pinned = 0 AND last_used_at < ?1 AND image_path IS NOT NULL",
        )?;
        let mut rows = stmt.query(params![cutoff])?;
        while let Some(r) = rows.next()? {
            if let Ok(Some(p)) = r.get::<_, Option<String>>(0) { imgs.push(p); }
        }
        drop(rows);
        drop(stmt);
        self.conn.execute(
            "DELETE FROM items WHERE pinned = 0 AND last_used_at < ?1",
            params![cutoff],
        )?;
        Ok(imgs)
    }
}

fn row_to_item(r: &rusqlite::Row) -> rusqlite::Result<ClipItem> {
    Ok(ClipItem {
        id: r.get(0)?,
        kind: r.get(1)?,
        content: r.get(2)?,
        preview: r.get(3)?,
        meta: r.get(4)?,
        source_app: r.get(5)?,
        source_app_id: r.get(6)?,
        image_path: r.get(7)?,
        pinned: { let v: i64 = r.get(8)?; v != 0 },
        size: r.get(9)?,
        created_at: r.get(10)?,
        last_used_at: r.get(11)?,
    })
}

/// Build a safe FTS5 MATCH expression by quoting each whitespace-separated
/// token and appending '*' for prefix matching.
fn build_match_expr(q: &str) -> String {
    let mut parts = Vec::new();
    for tok in q.split_whitespace() {
        let cleaned: String = tok
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .collect();
        if cleaned.is_empty() { continue; }
        parts.push(format!("\"{}\"*", cleaned));
    }
    if parts.is_empty() {
        // Fallback to a no-op match: return original token quoted.
        return format!("\"{}\"", q.replace('"', "\"\""));
    }
    parts.join(" ")
}

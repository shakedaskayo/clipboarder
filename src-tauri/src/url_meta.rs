//! Fetch + cache OpenGraph metadata for an arbitrary URL.
//!
//! The cache lives at `<app_data>/url_meta/<sha1(url)>.json`. We only fetch
//! when the user actually previews a URL row; results are then reused forever.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UrlMetadata {
    pub url: String,
    pub final_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub site_name: Option<String>,
    pub icon: Option<String>,
    pub fetched_at: i64,
    /// On failure we still cache an entry so we don't hammer broken URLs.
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct UrlMetaCache {
    pub dir: PathBuf,
    inflight: Mutex<std::collections::HashMap<String, ()>>,
}

impl UrlMetaCache {
    pub fn new(dir: PathBuf) -> Self {
        std::fs::create_dir_all(&dir).ok();
        Self {
            dir,
            inflight: Mutex::new(Default::default()),
        }
    }

    pub fn cached(&self, url: &str) -> Option<UrlMetadata> {
        let path = self.path_for(url);
        if !path.exists() { return None; }
        let raw = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub async fn fetch(&self, url: &str) -> Result<UrlMetadata> {
        if let Some(c) = self.cached(url) { return Ok(c); }

        // Cheap re-entrancy guard so concurrent previews don't double-fetch.
        {
            let mut inflight = self.inflight.lock();
            if inflight.contains_key(url) { return Ok(UrlMetadata { url: url.into(), ..Default::default() }); }
            inflight.insert(url.into(), ());
        }
        let result = fetch_metadata(url).await;
        self.inflight.lock().remove(url);

        let meta = result.unwrap_or_else(|err| UrlMetadata {
            url: url.into(),
            error: Some(format!("{err:#}")),
            fetched_at: chrono::Utc::now().timestamp_millis(),
            ..Default::default()
        });
        self.store(&meta)?;
        Ok(meta)
    }

    fn store(&self, meta: &UrlMetadata) -> Result<()> {
        let path = self.path_for(&meta.url);
        let json = serde_json::to_string_pretty(meta)?;
        std::fs::write(&path, json).context("write url_meta cache")?;
        Ok(())
    }

    fn path_for(&self, url: &str) -> PathBuf {
        let mut h = Sha256::new();
        h.update(url.as_bytes());
        let hash = h.finalize();
        let hex: String = hash.iter().take(12).map(|b| format!("{b:02x}")).collect();
        self.dir.join(format!("{hex}.json"))
    }
}

async fn fetch_metadata(url: &str) -> Result<UrlMetadata> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::limited(8))
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_0) AppleWebKit/605.1.15 \
             (KHTML, like Gecko) Version/17.0 Safari/605.1.15 clipboarder/0.1",
        )
        .build()
        .context("reqwest client")?;

    let resp = client.get(url).send().await.context("request")?;
    let final_url = resp.url().to_string();
    let bytes = resp.bytes().await.context("read body")?;
    // Only parse the first 256 KB to keep things fast on huge pages.
    let slice = if bytes.len() > 256 * 1024 { &bytes[..256 * 1024] } else { &bytes[..] };
    let html = String::from_utf8_lossy(slice);
    let parsed = parse_meta(&html);

    let mut meta = UrlMetadata {
        url: url.into(),
        final_url: Some(final_url.clone()),
        title: parsed.title,
        description: parsed.description,
        image: parsed.image.map(|i| resolve(&final_url, &i)),
        site_name: parsed.site_name,
        icon: parsed.icon.map(|i| resolve(&final_url, &i)),
        fetched_at: chrono::Utc::now().timestamp_millis(),
        error: None,
    };

    // Fallback favicon at /favicon.ico when no link rel=icon was found.
    if meta.icon.is_none() {
        if let Ok(parsed_url) = url::Url::parse(&final_url) {
            if let Some(host) = parsed_url.host_str() {
                let scheme = parsed_url.scheme();
                meta.icon = Some(format!("{scheme}://{host}/favicon.ico"));
            }
        }
    }

    Ok(meta)
}

#[derive(Default)]
struct ParsedMeta {
    title: Option<String>,
    description: Option<String>,
    image: Option<String>,
    site_name: Option<String>,
    icon: Option<String>,
}

fn parse_meta(html: &str) -> ParsedMeta {
    ParsedMeta {
        title: meta_property(html, &["og:title", "twitter:title"])
            .or_else(|| TITLE_RE.captures(html).map(|c| decode(c[1].trim()))),
        description: meta_property(html, &["og:description", "twitter:description"])
            .or_else(|| meta_name(html, "description")),
        image: meta_property(html, &["og:image", "og:image:secure_url", "twitter:image"]),
        site_name: meta_property(html, &["og:site_name"]),
        icon: link_icon(html),
    }
}

fn meta_property(html: &str, names: &[&str]) -> Option<String> {
    for cap in META_RE.captures_iter(html) {
        let attrs = &cap[1];
        let prop = attr(attrs, "property").or_else(|| attr(attrs, "name"));
        let Some(prop) = prop else { continue; };
        if names.iter().any(|n| prop.eq_ignore_ascii_case(n)) {
            if let Some(content) = attr(attrs, "content") {
                let v = content.trim();
                if !v.is_empty() {
                    return Some(decode(v));
                }
            }
        }
    }
    None
}

fn meta_name(html: &str, name: &str) -> Option<String> {
    for cap in META_RE.captures_iter(html) {
        let attrs = &cap[1];
        if let Some(n) = attr(attrs, "name") {
            if n.eq_ignore_ascii_case(name) {
                if let Some(content) = attr(attrs, "content") {
                    let v = content.trim();
                    if !v.is_empty() {
                        return Some(decode(v));
                    }
                }
            }
        }
    }
    None
}

fn link_icon(html: &str) -> Option<String> {
    let mut best: Option<(u32, String)> = None;
    for cap in LINK_RE.captures_iter(html) {
        let attrs = &cap[1];
        let Some(rel) = attr(attrs, "rel") else { continue; };
        let rel_l = rel.to_ascii_lowercase();
        if !(rel_l.contains("icon")) { continue; }
        let Some(href) = attr(attrs, "href") else { continue; };
        let size_score = if rel_l.contains("apple-touch-icon") { 200 }
            else if rel_l.contains("shortcut") { 100 }
            else { 50 };
        let score: u32 = attr(attrs, "sizes")
            .and_then(|s| s.split('x').next().map(|n| n.parse::<u32>().unwrap_or(0)))
            .unwrap_or(0) + size_score;
        match &best {
            Some((s, _)) if *s >= score => {}
            _ => best = Some((score, href.to_string())),
        }
    }
    best.map(|(_, h)| h)
}

fn attr(attrs: &str, name: &str) -> Option<String> {
    // Match: name="..." or name='...' (case-insensitive on the name).
    let re = Regex::new(&format!(r#"(?is){}\s*=\s*("([^"]*)"|'([^']*)')"#, regex::escape(name))).ok()?;
    let cap = re.captures(attrs)?;
    let value = cap.get(2).or_else(|| cap.get(3))?.as_str();
    Some(value.to_string())
}

fn decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

fn resolve(base: &str, href: &str) -> String {
    if let Ok(parsed) = url::Url::parse(href) {
        return parsed.to_string();
    }
    if let Ok(b) = url::Url::parse(base) {
        if let Ok(joined) = b.join(href) {
            return joined.to_string();
        }
    }
    href.to_string()
}

static META_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<meta\s+([^>]*?)/?>").unwrap());
static LINK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<link\s+([^>]*?)/?>").unwrap());
static TITLE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap());

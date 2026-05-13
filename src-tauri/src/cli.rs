//! Command-line interface for clipboarder.
//!
//! The same binary serves both the GUI (when invoked with no args) and the CLI
//! (when invoked with one of the subcommands defined below). Both modes share
//! the same SQLite store at `~/Library/Application Support/com.clipboarder.app/`.
//!
//! Design goals:
//! - Reads (`list`, `search`, `show`, `stats`) work even when the GUI isn't
//!   running, against a read-only SQLite handle.
//! - Writes (`add`, `pin`, `unpin`, `delete`, `clear`, `copy`) update the
//!   shared DB. WAL mode means the GUI can read concurrently.
//! - `--json` everywhere produces stable, machine-readable output for agents.
//! - Exit codes: 0 on success, 1 on item-not-found, 2 on argument errors,
//!   3 on storage errors.

use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::secrets;
use crate::storage::ClipItem;
use crate::store::{open_store, ItemStore, StoreBackend, UpsertRequest};

/// Command vocabulary the dual-mode dispatch in main.rs recognizes. Listed
/// here so the CLI and the dispatch logic stay in sync.
pub const SUBCOMMANDS: &[&str] = &[
    "list", "ls", "search", "find", "show", "cat", "get",
    "add", "ingest", "pin", "unpin", "star", "unstar",
    "delete", "rm", "clear", "copy", "stats", "watch",
    "cp", "pipe", "p", "paste", "last", "pop",
    "doctor", "test-paste",
    "serve", "admin",
    "help", "-h", "--help", "-V", "--version",
];

/// Returns true when argv looks like a CLI invocation.
pub fn looks_like_cli(args: &[String]) -> bool {
    if args.len() < 2 { return false; }
    SUBCOMMANDS.contains(&args[1].as_str())
}

#[derive(Parser)]
#[command(
    name = "clipboarder",
    version,
    about = "A fast, beautiful clipboard manager for macOS",
    long_about = "clipboarder — CLI for the clipboard manager.\n\
                  Search, ingest, pin, paste-back, and stream your local clipboard \
                  history. Designed for shell pipelines and AI agents.",
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Flags shared by every read command. They're optional and zero-impact when
/// not set, but they're the difference between dumping the entire clipboard
/// into an agent's context and giving it just the relevant slice.
#[derive(Debug, Clone, clap::Args)]
pub struct AgentOpts {
    /// Only include items younger than this duration. Accepts e.g. `30s`,
    /// `5m`, `2h`, `3d`, `1w`.
    #[arg(long, value_parser = parse_duration_arg)]
    pub since: Option<i64>,

    /// Truncate each item's content to N bytes (at a UTF-8 char boundary).
    /// Helpful when you're going to put the result into a token budget.
    #[arg(long, value_name = "N")]
    pub max_bytes: Option<usize>,

    /// Drop items that look like API keys, OAuth tokens, JWTs, or other
    /// credentials. Items are kept but the content is replaced with a
    /// `[redacted: <kind>]` placeholder so the agent knows something was
    /// hidden.
    #[arg(long)]
    pub no_secrets: bool,

    /// Replace each item's content with a small window around the first
    /// matching token of the query. Implies `--max-bytes` behavior. Only
    /// meaningful with `cb search` or `cb p --grep`.
    #[arg(long, value_name = "BYTES")]
    pub snippet: Option<usize>,

    /// Emit a minimal JSON shape (`{id, kind, content, meta}`) instead of
    /// the full row. Reduces tokens ~40% on average.
    #[arg(long)]
    pub compact: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// List most recent items.
    #[command(alias = "ls")]
    List {
        /// How many items to return (default 20).
        #[arg(short, long, default_value_t = 20)]
        limit: i64,
        /// Restrict to one kind: text|url|email|code|color|image|file|pdf|music|video|repo|pinned|all
        #[arg(short, long)]
        kind: Option<String>,
        /// Emit JSON instead of a human table.
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        agent: AgentOpts,
    },

    /// Full-text search the clipboard history.
    #[command(alias = "find")]
    Search {
        /// The query. Multiple words = AND. Prefix-matched.
        query: String,
        #[arg(short, long, default_value_t = 20)]
        limit: i64,
        #[arg(short, long)]
        kind: Option<String>,
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        agent: AgentOpts,
    },

    /// Print one item's full content.
    #[command(alias = "cat", alias = "get")]
    Show {
        /// Numeric item id from `list` / `search`.
        id: i64,
        /// Emit the full row as JSON (default: only the content body).
        #[arg(long)]
        json: bool,
    },

    /// Add a new item from stdin or a positional argument.
    #[command(alias = "ingest")]
    Add {
        /// Item text. Omit to read from stdin.
        text: Option<String>,
        /// Override the auto-classification (e.g. --kind code).
        #[arg(long)]
        kind: Option<String>,
        /// Also write to the macOS clipboard (the GUI watcher will pick it up).
        #[arg(long)]
        copy: bool,
        /// Tag where it came from (shown in the row meta).
        #[arg(long)]
        source: Option<String>,
        /// Print the new row id (default: silent on success).
        #[arg(long)]
        json: bool,
    },

    /// Pin an item (survives clear + always floats to the top).
    #[command(alias = "star")]
    Pin { id: i64 },

    /// Unpin an item.
    #[command(alias = "unstar")]
    Unpin { id: i64 },

    /// Delete an item.
    #[command(alias = "rm")]
    Delete { id: i64 },

    /// Clear all non-pinned items.
    Clear {
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Copy an item to the macOS clipboard. Doesn't paste — that requires the GUI.
    Copy { id: i64 },

    /// Print database statistics.
    Stats {
        #[arg(long)]
        json: bool,
    },

    /// Stream newly-captured items as JSON Lines on stdout (one row per line).
    /// Polls the DB every 500 ms.
    Watch {
        /// Only emit items of this kind.
        #[arg(short, long)]
        kind: Option<String>,
    },

    /// pbcopy++ — stdin → clipboarder history + the macOS pasteboard.
    ///
    /// Like `pbcopy`, but the content is also persisted to clipboarder's
    /// searchable history.
    #[command(alias = "pipe")]
    Cp {
        /// Override the auto-classification.
        #[arg(long)]
        kind: Option<String>,
        /// Tag where it came from (shown in the row meta).
        #[arg(long, default_value = "cli")]
        source: String,
        /// Don't touch the macOS pasteboard — only persist to history.
        #[arg(long)]
        no_clipboard: bool,
        /// Emit `{"id":N,"inserted":bool,"kind":"..."}` on success.
        #[arg(long)]
        json: bool,
    },

    /// pbpaste++ — print the Nth most-recent item's content to stdout.
    ///
    /// Supports `--kind`/`--grep` filters and `--copy` to also put the
    /// item on the macOS pasteboard. Perfect for shell pipelines.
    #[command(alias = "paste", alias = "last")]
    P {
        /// 1-indexed position (1 = most recent). Defaults to 1.
        #[arg(default_value_t = 1)]
        n: usize,
        /// Restrict to one kind.
        #[arg(short, long)]
        kind: Option<String>,
        /// Only consider items matching this FTS query (prefix-matched).
        #[arg(short, long)]
        grep: Option<String>,
        /// After printing, also write the content to the macOS pasteboard.
        #[arg(long)]
        copy: bool,
        /// Print all matches (one per line) instead of just the Nth.
        #[arg(long)]
        all: bool,
        /// Output JSON row instead of just the content body.
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        agent: AgentOpts,
    },

    /// Print the most recent item *and* delete it from history.
    Pop {
        #[arg(short, long)]
        kind: Option<String>,
    },

    /// Diagnose paste-back / install issues. Prints permission and process state.
    Doctor,

    /// Write a marker string to the macOS clipboard and synthesize ⌘V.
    /// Useful to confirm paste-back works in isolation — run this in your
    /// terminal; if you see the marker appear at the prompt, paste-back is
    /// working. If only the clipboard updates but no marker appears, the
    /// CGEventPost path is being silently denied.
    #[command(name = "test-paste")]
    TestPaste {
        /// What to write to the clipboard before synthesizing ⌘V.
        #[arg(short, long, default_value = "clipboarder paste-back ✓")]
        marker: String,
        /// Sleep duration in milliseconds between hide and ⌘V (default: 250 — generous).
        #[arg(short, long, default_value_t = 250)]
        delay_ms: u64,
    },

    /// Run the HTTP server (a shared backend for multiple clients).
    Serve {
        /// Address to bind. Defaults to 127.0.0.1:7474. Use 0.0.0.0:7474 for
        /// LAN access. Always front with a reverse proxy + TLS in production.
        #[arg(long)]
        bind: Option<String>,
        /// Path to the server config TOML (token → namespace mapping).
        /// Default: `~/Library/Application Support/com.clipboarder.app/server.toml`.
        #[arg(long)]
        config: Option<std::path::PathBuf>,
    },

    /// Server administration: manage tokens and namespaces.
    Admin {
        #[command(subcommand)]
        action: AdminCommand,
    },
}

/// Admin subcommands. Edit the server's tokens + namespaces from the CLI.
#[derive(Subcommand)]
pub enum AdminCommand {
    /// Token management.
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },
}

#[derive(Subcommand)]
pub enum TokenAction {
    /// Create a new bearer token bound to a namespace. Prints the new token
    /// on stdout — copy it to your client immediately, it's not stored
    /// anywhere else in human-readable form.
    Create {
        /// Namespace this token will access. Created on first use.
        #[arg(long)]
        namespace: String,
        /// Optional human-readable label for the GUI / `whoami`.
        #[arg(long)]
        label: Option<String>,
        /// Path to the server config TOML. See `clipboarder serve --help`.
        #[arg(long)]
        config: Option<std::path::PathBuf>,
    },

    /// List existing tokens (token prefix only — full value isn't echoed).
    List {
        #[arg(long)]
        config: Option<std::path::PathBuf>,
        #[arg(long)]
        json: bool,
    },

    /// Revoke a token by full value (or by prefix if unique).
    Revoke {
        token: String,
        #[arg(long)]
        config: Option<std::path::PathBuf>,
    },
}

/// Entry point invoked from main.rs when argv looks like CLI mode.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    // Commands that don't go through ItemStore (server / admin / local-only):
    match cli.command {
        Command::Serve { bind, config } => return cmd_serve(bind, config),
        Command::Admin { action } => return cmd_admin(action),
        Command::TestPaste { marker, delay_ms } => return cmd_test_paste(&marker, delay_ms),
        _ => {}
    }

    // For everything else the CLI gets a backend (local SQLite by default,
    // remote HTTP when CLIPBOARDER_SERVER + CLIPBOARDER_TOKEN are set).
    let data_dir = data_dir();
    let store = open_store(data_dir).context("open store")?;

    // Re-extract the command (we consumed `cli.command` via the first match).
    // Cheapest way: parse again. clap is fast.
    let cli = Cli::parse();
    match cli.command {
        Command::List { limit, kind, json, agent } => {
            cmd_list(&*store, limit, kind, json, &agent)
        }
        Command::Search { query, limit, kind, json, agent } => {
            cmd_search(&*store, &query, limit, kind, json, &agent)
        }
        Command::Show { id, json } => cmd_show(&*store, id, json),
        Command::Add { text, kind, copy, source, json } => {
            cmd_add(&*store, text, kind, copy, source, json)
        }
        Command::Pin { id } => cmd_set_pin(&*store, id, true),
        Command::Unpin { id } => cmd_set_pin(&*store, id, false),
        Command::Delete { id } => cmd_delete(&*store, id),
        Command::Clear { yes } => cmd_clear(&*store, yes),
        Command::Copy { id } => cmd_copy(&*store, id),
        Command::Stats { json } => cmd_stats(&*store, json),
        Command::Watch { kind } => cmd_watch(&*store, kind),
        Command::Cp { kind, source, no_clipboard, json } => {
            cmd_add(&*store, None, kind, !no_clipboard, Some(source), json)
        }
        Command::P { n, kind, grep, copy, all, json, agent } => {
            cmd_paste(&*store, n, kind, grep, copy, all, json, &agent)
        }
        Command::Pop { kind } => cmd_pop(&*store, kind),
        Command::Doctor => cmd_doctor(&*store),
        // The four above are already handled in the first match — unreachable here.
        Command::Serve { .. } | Command::Admin { .. } | Command::TestPaste { .. } => unreachable!(),
    }
}

// ── server / admin ────────────────────────────────────────────────────────

fn cmd_serve(
    bind: Option<String>,
    config: Option<std::path::PathBuf>,
) -> Result<()> {
    let config_path = config.unwrap_or_else(crate::server_config::default_config_path);
    let runtime = tokio::runtime::Runtime::new().context("tokio runtime")?;
    runtime.block_on(crate::server::run(config_path, bind))
}

fn cmd_admin(action: AdminCommand) -> Result<()> {
    use crate::server_config::{generate_token, ServerConfig, TokenEntry};
    match action {
        AdminCommand::Token { action } => match action {
            TokenAction::Create { namespace, label, config } => {
                let path = config.unwrap_or_else(crate::server_config::default_config_path);
                let mut cfg = ServerConfig::load(&path)?;
                let token = generate_token();
                cfg.tokens.push(TokenEntry {
                    token: token.clone(),
                    namespace: namespace.clone(),
                    label,
                });
                cfg.save(&path)?;
                println!("{token}");
                eprintln!("  ↑ bearer token for namespace `{namespace}` — saved to {}", path.display());
                eprintln!("  Set on the client:");
                eprintln!("    export CLIPBOARDER_SERVER='http://<host>:7474'");
                eprintln!("    export CLIPBOARDER_TOKEN='{token}'");
            }
            TokenAction::List { config, json } => {
                let path = config.unwrap_or_else(crate::server_config::default_config_path);
                let cfg = ServerConfig::load(&path)?;
                if json {
                    let view: Vec<_> = cfg
                        .tokens
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "prefix": &t.token[..t.token.len().min(8)],
                                "namespace": &t.namespace,
                                "label": &t.label,
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&view)?);
                } else if cfg.tokens.is_empty() {
                    eprintln!("(no tokens — run `clipboarder admin token create --namespace …`)");
                } else {
                    println!("{:<12}  {:<24}  LABEL", "PREFIX", "NAMESPACE");
                    for t in &cfg.tokens {
                        let prefix = format!("{}…", &t.token[..t.token.len().min(8)]);
                        println!(
                            "{:<12}  {:<24}  {}",
                            prefix,
                            t.namespace,
                            t.label.as_deref().unwrap_or("")
                        );
                    }
                }
            }
            TokenAction::Revoke { token, config } => {
                let path = config.unwrap_or_else(crate::server_config::default_config_path);
                let mut cfg = ServerConfig::load(&path)?;
                let before = cfg.tokens.len();
                cfg.tokens.retain(|t| t.token != token && !t.token.starts_with(&token));
                let removed = before - cfg.tokens.len();
                if removed == 0 {
                    eprintln!("no token matched `{token}`");
                    std::process::exit(1);
                }
                cfg.save(&path)?;
                eprintln!("revoked {removed} token(s)");
            }
        },
    }
    Ok(())
}

// ── duration parser ─────────────────────────────────────────────────────────

/// Parse `30s`, `5m`, `2h`, `3d`, `1w` into milliseconds (for last_used_at
/// cutoff). Used by `--since`.
pub fn parse_duration_ms(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() { return None; }
    let unit = s.chars().last()?;
    let num_part = &s[..s.len() - unit.len_utf8()];
    let n: i64 = num_part.parse().ok()?;
    let mult: i64 = match unit {
        's' => 1_000,
        'm' => 60_000,
        'h' => 3_600_000,
        'd' => 86_400_000,
        'w' => 604_800_000,
        _ => return None,
    };
    Some(n * mult)
}

fn parse_duration_arg(s: &str) -> Result<i64, String> {
    parse_duration_ms(s).ok_or_else(|| {
        format!("expected duration like `30s`, `5m`, `2h`, `3d`, `1w`; got `{s}`")
    })
}

// ── content transforms ─────────────────────────────────────────────────────

fn truncate_at_char_boundary(s: &str, max: usize) -> String {
    if s.len() <= max { return s.to_string(); }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

fn snippet_around(content: &str, query: &str, window: usize) -> String {
    let lower = content.to_lowercase();
    let token = query
        .split_whitespace()
        .find(|t| !t.is_empty())
        .unwrap_or("");
    if token.is_empty() {
        return truncate_at_char_boundary(content, window.saturating_mul(2));
    }
    let needle = token.to_lowercase();
    if let Some(idx) = lower.find(&needle) {
        let half = window / 2;
        let mut start = idx.saturating_sub(half);
        while start > 0 && !content.is_char_boundary(start) { start -= 1; }
        let mut end = (idx + needle.len() + half).min(content.len());
        while end < content.len() && !content.is_char_boundary(end) { end += 1; }
        let prefix = if start > 0 { "…" } else { "" };
        let suffix = if end < content.len() { "…" } else { "" };
        return format!("{prefix}{}{suffix}", &content[start..end]);
    }
    truncate_at_char_boundary(content, window.saturating_mul(2))
}

#[derive(serde::Serialize)]
struct CompactItem<'a> {
    id: i64,
    kind: &'a str,
    content: &'a str,
    meta: Option<&'a str>,
}

/// Apply --since, --no-secrets, --snippet, --max-bytes to a result set.
fn apply_agent_opts(
    items: &[ClipItem],
    grep: Option<&str>,
    opts: &AgentOpts,
) -> Vec<ClipItem> {
    let now = chrono::Utc::now().timestamp_millis();
    items
        .iter()
        .filter(|it| match opts.since {
            Some(ms) => (now - it.last_used_at) <= ms,
            None => true,
        })
        .map(|it| {
            let mut it = it.clone();
            if opts.no_secrets {
                if let Some(kind) = secrets::detect(&it.content) {
                    it.content = format!("[redacted: {}]", kind.label());
                }
            }
            if let Some(window) = opts.snippet {
                if let Some(q) = grep {
                    it.content = snippet_around(&it.content, q, window);
                }
            }
            if let Some(max) = opts.max_bytes {
                if it.content.len() > max {
                    it.content = truncate_at_char_boundary(&it.content, max);
                }
            }
            it
        })
        .collect()
}

fn cmd_test_paste(marker: &str, delay_ms: u64) -> Result<()> {
    println!("--- paste-back self-test ---");
    println!();
    println!("1. Writing {:?} to the macOS clipboard…", marker);
    write_clipboard_text(marker)?;
    println!("   ✓ clipboard updated");

    println!();
    println!("2. Waiting {} ms then synthesizing ⌘V…", delay_ms);
    println!("   Look at YOUR TERMINAL PROMPT after this command exits.");
    println!("   If the marker shows up there, paste-back is working.");
    println!();
    std::thread::sleep(std::time::Duration::from_millis(delay_ms));

    #[cfg(target_os = "macos")]
    {
        if !crate::macos::is_accessibility_trusted() {
            println!("\x1b[31m✗\x1b[0m Accessibility is NOT granted to this binary.");
            println!("   The clipboard was updated, but ⌘V can't be synthesized.");
            println!("   Run `clipboarder doctor` for the grant instructions.");
            std::process::exit(1);
        }
        match crate::paste::simulate_paste() {
            Ok(()) => println!("\x1b[32m✓\x1b[0m simulate_paste returned Ok"),
            Err(e) => {
                eprintln!("\x1b[31m✗\x1b[0m simulate_paste failed: {e:#}");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn cmd_doctor(store: &dyn ItemStore) -> Result<()> {
    let ok = "\x1b[32m✓\x1b[0m";
    let warn = "\x1b[33m⚠\x1b[0m";
    let bad = "\x1b[31m✗\x1b[0m";

    println!("clipboarder doctor");
    println!("==================\n");

    // 0) Backend
    println!("{ok} backend        {}", store.describe());

    // 1) Data dir / DB (only meaningful for local)
    if store.backend() == StoreBackend::Local {
        let dir = data_dir();
        if dir.exists() {
            println!("{ok} data dir       {}", dir.display());
        } else {
            println!("{bad} data dir       {} (missing — has the GUI ever launched?)", dir.display());
        }
    }

    // 2) Item count via the trait — works for both backends
    let total = store.search("", "all", 1_000_000).map(|v| v.len()).unwrap_or(0);
    println!("{ok} history items  {total} (namespace: {})", store.namespace());

    // 3) GUI process running?
    let running = process_running("clipboarder");
    if running {
        println!("{ok} GUI process    running");
    } else {
        println!("{warn} GUI process    not running — only the CLI can read history right now");
        println!("                       launch with: open -a clipboarder");
    }

    // 4) Accessibility permission (for paste-back synthesis)
    #[cfg(target_os = "macos")]
    {
        let trusted = crate::macos::is_accessibility_trusted();
        if trusted {
            println!("{ok} accessibility  granted to this binary");
        } else {
            println!("{bad} accessibility  NOT granted to this binary");
            println!();
            println!("    Paste-back needs Accessibility permission to synthesize ⌘V");
            println!("    into the previously-focused app after you press Enter on a row.");
            println!();
            println!("    Fix:");
            println!("      1. Open System Settings → Privacy & Security → Accessibility");
            println!("      2. If `clipboarder` is listed, toggle it OFF, then back ON");
            println!("         (this re-grants permission after a binary update)");
            println!("      3. If it's not listed, click + and add:");
            println!("           /Applications/clipboarder.app");
            println!("      4. Re-run `clipboarder doctor` — this line should turn green");
        }
    }

    // 5) Hotkey registration check is hard from CLI (the GUI owns it). Hint instead.
    println!();
    println!("Hotkey  default ⌘⇧V — change in Settings (gear in footer, or ⌘,)");
    println!("Docs    https://shakedaskayo.github.io/clipboarder/");

    Ok(())
}

fn process_running(name: &str) -> bool {
    use std::process::Command;
    let output = Command::new("pgrep").args(["-x", name]).output();
    matches!(output, Ok(o) if !o.stdout.is_empty())
}

fn data_dir() -> PathBuf {
    // Mirror tauri's app_data_dir on macOS: ~/Library/Application Support/<bundle id>.
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("Library/Application Support/com.clipboarder.app")
}

fn normalize_kind(k: Option<String>) -> String {
    k.unwrap_or_else(|| "all".into())
}

// ── list & search ────────────────────────────────────────────────────────────

fn cmd_list(
    store: &dyn ItemStore,
    limit: i64,
    kind: Option<String>,
    json: bool,
    agent: &AgentOpts,
) -> Result<()> {
    let items = store.search("", &normalize_kind(kind), limit)?;
    let items = apply_agent_opts(&items, None, agent);
    emit_items(&items, json, agent.compact);
    Ok(())
}

fn cmd_search(
    store: &dyn ItemStore,
    query: &str,
    limit: i64,
    kind: Option<String>,
    json: bool,
    agent: &AgentOpts,
) -> Result<()> {
    let items = store.search(query, &normalize_kind(kind), limit)?;
    let items = apply_agent_opts(&items, Some(query), agent);
    emit_items(&items, json, agent.compact);
    Ok(())
}

fn cmd_show(store: &dyn ItemStore, id: i64, json: bool) -> Result<()> {
    let Some(item) = store.get(id)? else {
        eprintln!("item #{id} not found");
        std::process::exit(1);
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&item)?);
    } else {
        print!("{}", item.content);
        if !item.content.ends_with('\n') { println!(); }
    }
    Ok(())
}

// ── add ──────────────────────────────────────────────────────────────────────

fn cmd_add(
    store: &dyn ItemStore,
    text: Option<String>,
    kind_override: Option<String>,
    also_copy: bool,
    source: Option<String>,
    json: bool,
) -> Result<()> {
    let body = match text {
        Some(t) => t,
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).context("read stdin")?;
            buf
        }
    };
    if body.is_empty() {
        eprintln!("nothing to add (empty input)");
        std::process::exit(2);
    }

    let reply = store.upsert(&UpsertRequest {
        content: body.clone(),
        kind: kind_override,
        meta: None,
        source_app: source,
    })?;

    if also_copy {
        write_clipboard_text(&body).context("set system clipboard")?;
    }

    if json {
        println!(
            "{}",
            serde_json::json!({"id": reply.id, "inserted": reply.inserted, "kind": reply.kind})
        );
    }
    Ok(())
}

// ── pin / unpin / delete / clear ────────────────────────────────────────────

fn cmd_set_pin(store: &dyn ItemStore, id: i64, want_pinned: bool) -> Result<()> {
    match store.set_pin(id, want_pinned) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn cmd_delete(store: &dyn ItemStore, id: i64) -> Result<()> {
    if store.get(id)?.is_none() {
        eprintln!("item #{id} not found");
        std::process::exit(1);
    }
    store.delete(id)?;
    Ok(())
}

fn cmd_clear(store: &dyn ItemStore, yes: bool) -> Result<()> {
    if !yes {
        eprint!("Clear all non-pinned items? [y/N] ");
        std::io::stderr().flush().ok();
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).ok();
        if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
            eprintln!("aborted");
            std::process::exit(2);
        }
    }
    store.clear()?;
    Ok(())
}

// ── copy ─────────────────────────────────────────────────────────────────────

fn cmd_copy(store: &dyn ItemStore, id: i64) -> Result<()> {
    let Some(item) = store.get(id)? else {
        eprintln!("item #{id} not found");
        std::process::exit(1);
    };
    if item.kind == "image" {
        if let Some(path) = &item.image_path {
            // Image bytes live on the *server's* disk; for remote stores the
            // path won't resolve locally. Fall back to text content for the
            // remote case until we add /v1/items/:id/image.
            if std::path::Path::new(path).exists() {
                let bytes = std::fs::read(path)?;
                write_clipboard_image(&bytes)?;
                return Ok(());
            }
        }
    }
    write_clipboard_text(&item.content)?;
    Ok(())
}

// ── stats ────────────────────────────────────────────────────────────────────

fn cmd_stats(store: &dyn ItemStore, json: bool) -> Result<()> {
    let stats = store.stats()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("backend:   {}", store.describe());
        println!("items:     {}", stats.total);
        println!("pinned:    {}", stats.pinned);
        println!("by kind:");
        for (k, v) in &stats.by_kind {
            println!("  {:8}  {}", k, v);
        }
    }
    Ok(())
}

// ── paste / pop ──────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_paste(
    store: &dyn ItemStore,
    n: usize,
    kind: Option<String>,
    grep: Option<String>,
    also_copy: bool,
    all: bool,
    json: bool,
    agent: &AgentOpts,
) -> Result<()> {
    if n == 0 {
        eprintln!("position N must be >= 1");
        std::process::exit(2);
    }
    let query = grep.clone().unwrap_or_default();
    let kind_str = normalize_kind(kind);
    let limit = if all { 10_000 } else { n as i64 };
    let raw = store.search(&query, &kind_str, limit)?;
    let items = apply_agent_opts(&raw, grep.as_deref(), agent);

    if items.is_empty() {
        eprintln!("no matching items");
        std::process::exit(1);
    }

    if all {
        for item in &items {
            emit_one(item, json, agent.compact);
        }
    } else {
        let idx = n - 1;
        let Some(item) = items.get(idx) else {
            eprintln!("only {} matching item(s) — no #{} to paste", items.len(), n);
            std::process::exit(1);
        };
        emit_one(item, json, agent.compact);
        if also_copy {
            // Pre-truncation original content so the pasteboard gets the full thing.
            let original = &raw[idx];
            if original.kind == "image" {
                if let Some(path) = &original.image_path {
                    if std::path::Path::new(path).exists() {
                        let bytes = std::fs::read(path)?;
                        write_clipboard_image(&bytes)?;
                    } else {
                        // Remote store — image path is on the server, not us.
                        write_clipboard_text(&original.content)?;
                    }
                }
            } else {
                write_clipboard_text(&original.content)?;
            }
        }
    }
    Ok(())
}

fn emit_one(item: &ClipItem, json: bool, compact: bool) {
    if json {
        let output = if compact {
            serde_json::to_string(&CompactItem {
                id: item.id,
                kind: &item.kind,
                content: &item.content,
                meta: item.meta.as_deref(),
            })
        } else {
            serde_json::to_string(item)
        };
        match output {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("serialize: {e}"),
        }
    } else if item.kind == "image" {
        // Images aren't text — emit the on-disk path so callers can pipe it.
        if let Some(p) = &item.image_path {
            println!("{p}");
        }
    } else {
        print!("{}", item.content);
        if !item.content.ends_with('\n') {
            println!();
        }
    }
}

fn cmd_pop(store: &dyn ItemStore, kind: Option<String>) -> Result<()> {
    let kind_str = normalize_kind(kind);
    let items = store.search("", &kind_str, 1)?;
    let Some(item) = items.into_iter().next() else {
        eprintln!("history is empty");
        std::process::exit(1);
    };
    emit_one(&item, false, false);
    store.delete(item.id)?;
    Ok(())
}

// ── watch ────────────────────────────────────────────────────────────────────

fn cmd_watch(store: &dyn ItemStore, kind: Option<String>) -> Result<()> {
    let kind = normalize_kind(kind);
    // Local → polling iterator; Remote → SSE iterator. Both yield items
    // oldest-first as they arrive.
    for result in store.watch(&kind)? {
        match result {
            Ok(item) => {
                println!("{}", serde_json::to_string(&item)?);
                std::io::stdout().flush().ok();
            }
            Err(e) => {
                eprintln!("clipboarder watch: {e}");
                return Err(e);
            }
        }
    }
    Ok(())
}

// ── output helpers ───────────────────────────────────────────────────────────

fn emit_items(items: &[ClipItem], json: bool, compact: bool) {
    if json {
        let output = if compact {
            let view: Vec<CompactItem> = items
                .iter()
                .map(|i| CompactItem {
                    id: i.id,
                    kind: &i.kind,
                    content: &i.content,
                    meta: i.meta.as_deref(),
                })
                .collect();
            serde_json::to_string_pretty(&view)
        } else {
            serde_json::to_string_pretty(items)
        };
        match output {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("serialize: {e}"),
        }
        return;
    }
    if items.is_empty() {
        eprintln!("(no items)");
        return;
    }
    // ID  | KIND | AGE | SOURCE | PREVIEW
    let now = chrono::Utc::now().timestamp_millis();
    println!("{:>5}  {:7}  {:>5}  {:14}  PREVIEW", "ID", "KIND", "AGE", "SOURCE");
    for it in items {
        let preview = truncate(&it.preview, 80);
        let age = ago(now - it.last_used_at);
        let source = it.source_app.as_deref().unwrap_or("");
        let pin = if it.pinned { "★ " } else { "" };
        println!(
            "{:>5}  {:7}  {:>5}  {:14}  {pin}{preview}",
            it.id, it.kind, age, truncate(source, 14)
        );
    }
}

fn truncate(s: &str, n: usize) -> String {
    let mut out = String::new();
    for (count, c) in s.chars().enumerate() {
        if count >= n { out.push('…'); break; }
        out.push(c);
    }
    out
}

fn ago(ms: i64) -> String {
    let s = ms / 1000;
    if s < 60 { return format!("{s}s"); }
    let m = s / 60;
    if m < 60 { return format!("{m}m"); }
    let h = m / 60;
    if h < 24 { return format!("{h}h"); }
    format!("{}d", h / 24)
}

// ── clipboard helpers ────────────────────────────────────────────────────────

fn write_clipboard_text(text: &str) -> Result<()> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    let ctx = ClipboardContext::new().map_err(|e| anyhow::anyhow!("clipboard ctx: {e}"))?;
    ctx.set_text(text.to_string())
        .map_err(|e| anyhow::anyhow!("set text: {e}"))?;
    Ok(())
}

fn write_clipboard_image(png_bytes: &[u8]) -> Result<()> {
    use clipboard_rs::{common::RustImage, Clipboard, ClipboardContext, RustImageData};
    let ctx = ClipboardContext::new().map_err(|e| anyhow::anyhow!("clipboard ctx: {e}"))?;
    let img = RustImageData::from_bytes(png_bytes)
        .map_err(|e| anyhow::anyhow!("decode png: {e}"))?;
    ctx.set_image(img)
        .map_err(|e| anyhow::anyhow!("set image: {e}"))?;
    Ok(())
}

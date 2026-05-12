//! Content classification: detect what kind of data is on the clipboard,
//! produce a short single-line preview, and attach metadata (language, color
//! format, etc.) for the UI to render previews intelligently.

use once_cell::sync::Lazy;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Url,
    Email,
    Code,
    Color,
    Image,
    File,
    Pdf,
    Music,
    Video,
    Repo,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Url => "url",
            Kind::Email => "email",
            Kind::Code => "code",
            Kind::Color => "color",
            Kind::Image => "image",
            Kind::File => "file",
            Kind::Pdf => "pdf",
            Kind::Music => "music",
            Kind::Video => "video",
            Kind::Repo => "repo",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "url" => Kind::Url,
            "email" => Kind::Email,
            "code" => Kind::Code,
            "color" => Kind::Color,
            "image" => Kind::Image,
            "file" => Kind::File,
            "pdf" => Kind::Pdf,
            "music" => Kind::Music,
            "video" => Kind::Video,
            "repo" => Kind::Repo,
            _ => Kind::Text,
        }
    }
}

pub struct Classified {
    pub kind: Kind,
    pub meta: Option<String>,
    pub preview: String,
}

pub fn classify_text(text: &str) -> Classified {
    let trimmed = text.trim();
    let preview = make_preview(text);

    if trimmed.is_empty() {
        return Classified { kind: Kind::Text, meta: None, preview };
    }

    // File path (one of: /abs/path, ~/path, file:// URI)
    if is_file_path(trimmed) {
        return Classified { kind: Kind::File, meta: None, preview };
    }

    // Color
    if let Some(fmt) = detect_color(trimmed) {
        return Classified { kind: Kind::Color, meta: Some(fmt.to_string()), preview };
    }

    // Email
    if is_email(trimmed) {
        return Classified { kind: Kind::Email, meta: None, preview };
    }

    // URL — first see if it's a code-host repo, then a media platform, then a plain URL.
    if is_url(trimmed) {
        if let Ok(parsed) = Url::parse(trimmed) {
            if let Some(host) = parsed.host_str() {
                if let Some(platform) = detect_repo(host, parsed.path()) {
                    return Classified {
                        kind: Kind::Repo,
                        meta: Some(platform.into()),
                        preview,
                    };
                }
                if let Some((kind, platform)) = detect_media(host) {
                    return Classified {
                        kind,
                        meta: Some(platform.into()),
                        preview,
                    };
                }
                return Classified {
                    kind: Kind::Url,
                    meta: Some(host.into()),
                    preview,
                };
            }
        }
        return Classified { kind: Kind::Url, meta: None, preview };
    }

    // Code
    if let Some(lang) = detect_code(text) {
        return Classified { kind: Kind::Code, meta: Some(lang.to_string()), preview };
    }

    Classified { kind: Kind::Text, meta: None, preview }
}

fn make_preview(text: &str) -> String {
    let mut out = String::with_capacity(160);
    let mut prev_space = false;
    let mut started = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if started && !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
            started = true;
        }
        if out.chars().count() >= 160 { break; }
    }
    out.trim().to_string()
}

fn is_file_path(s: &str) -> bool {
    if s.starts_with("file://") {
        return true;
    }
    if s.starts_with('/') || s.starts_with("~/") {
        // Must not contain newlines or look like a sentence.
        return !s.contains('\n') && s.len() < 1024 && !s.contains(' ');
    }
    false
}

fn detect_color(s: &str) -> Option<&'static str> {
    let lower = s.to_lowercase();
    if HEX_RE.is_match(&lower) { return Some("hex"); }
    if RGB_RE.is_match(&lower) { return Some("rgb"); }
    if HSL_RE.is_match(&lower) { return Some("hsl"); }
    None
}

fn is_email(s: &str) -> bool {
    if s.contains(' ') || s.contains('\n') { return false; }
    EMAIL_RE.is_match(s)
}

fn is_url(s: &str) -> bool {
    if s.contains(' ') || s.contains('\n') { return false; }
    if s.starts_with("http://") || s.starts_with("https://") {
        return Url::parse(s).is_ok();
    }
    false
}

/// Heuristic code detection: returns language guess.
fn detect_code(text: &str) -> Option<&'static str> {
    let t = text.trim();
    if t.len() < 6 { return None; }

    // Strong syntactic signals
    let has_braces = (t.contains('{') && t.contains('}'))
        || (t.contains('[') && t.contains(']'));
    let has_semis = t.matches(';').count() >= 2;
    let lines: Vec<&str> = t.lines().collect();
    let indented = lines.iter().filter(|l| l.starts_with("  ") || l.starts_with('\t')).count();
    let multiline = lines.len() >= 2;
    let mut score = 0;
    if has_braces { score += 2; }
    if has_semis { score += 1; }
    if multiline && indented >= 1 { score += 1; }
    if t.contains("=>") || t.contains("->") { score += 1; }
    if t.contains("::") { score += 1; }

    // Single-line that smells like a command
    if !multiline {
        if t.starts_with("$ ") || t.starts_with("> ") || t.starts_with("npm ")
            || t.starts_with("yarn ") || t.starts_with("git ")
            || t.starts_with("cargo ") || t.starts_with("brew ")
            || t.starts_with("docker ") || t.starts_with("kubectl ") {
            return Some("shell");
        }
    }

    if score < 2 { return None; }

    // Language guess
    if t.contains("fn ") && (t.contains("->") || t.contains("let ")) { return Some("rust"); }
    if t.contains("def ") && t.contains(":\n") { return Some("python"); }
    if t.contains("=>") && (t.contains("const ") || t.contains("let ") || t.contains("function")) {
        return Some("javascript");
    }
    if t.contains(": ") && (t.contains("interface ") || t.contains("type ")) {
        return Some("typescript");
    }
    if t.starts_with('{') && (t.contains("\"$schema\"") || lines.iter().any(|l| l.contains(": \""))) {
        return Some("json");
    }
    if t.contains("SELECT ") || t.contains("select ") || t.contains("FROM ") {
        return Some("sql");
    }
    if t.contains("package ") && t.contains("func ") { return Some("go"); }
    if t.contains("<html") || t.contains("</") { return Some("html"); }
    Some("code")
}

static HEX_RE: Lazy<regex_lite::Regex> = Lazy::new(|| {
    regex_lite::Regex::new(r"^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$").unwrap()
});
static RGB_RE: Lazy<regex_lite::Regex> = Lazy::new(|| {
    regex_lite::Regex::new(r"^rgba?\(\s*[\d.]+[\s,]+[\d.]+[\s,]+[\d.]+(?:[\s,/]+[\d.]+%?)?\s*\)$")
        .unwrap()
});
static HSL_RE: Lazy<regex_lite::Regex> = Lazy::new(|| {
    regex_lite::Regex::new(
        r"^hsla?\(\s*[\d.]+(?:deg)?[\s,]+[\d.]+%[\s,]+[\d.]+%(?:[\s,/]+[\d.]+%?)?\s*\)$",
    )
    .unwrap()
});
static EMAIL_RE: Lazy<regex_lite::Regex> = Lazy::new(|| {
    regex_lite::Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$").unwrap()
});

/// Recognize a streaming-music or video URL and return (kind, platform-tag).
fn detect_media(host: &str) -> Option<(Kind, &'static str)> {
    let h = host.to_ascii_lowercase();
    let h = h.strip_prefix("www.").unwrap_or(&h);

    // Music
    if h == "open.spotify.com" || h == "spotify.com" || h.ends_with(".spotify.com") {
        return Some((Kind::Music, "spotify"));
    }
    if h == "music.apple.com" || h == "itunes.apple.com" {
        return Some((Kind::Music, "apple-music"));
    }
    if h == "music.youtube.com" {
        return Some((Kind::Music, "youtube-music"));
    }
    if h == "soundcloud.com" || h.ends_with(".soundcloud.com") {
        return Some((Kind::Music, "soundcloud"));
    }
    if h.ends_with("bandcamp.com") {
        return Some((Kind::Music, "bandcamp"));
    }

    // Video
    if h == "youtube.com" || h == "youtu.be" || h.ends_with(".youtube.com") {
        return Some((Kind::Video, "youtube"));
    }
    if h == "vimeo.com" || h.ends_with(".vimeo.com") {
        return Some((Kind::Video, "vimeo"));
    }
    if h == "twitch.tv" || h.ends_with(".twitch.tv") {
        return Some((Kind::Video, "twitch"));
    }

    None
}

/// Recognize a code-host repository URL.
fn detect_repo(host: &str, path: &str) -> Option<&'static str> {
    let h = host.to_ascii_lowercase();
    let h = h.strip_prefix("www.").unwrap_or(&h);

    if h == "gist.github.com" { return Some("gist"); }

    let platform = match h {
        "github.com" => "github",
        "gitlab.com" => "gitlab",
        "bitbucket.org" => "bitbucket",
        "codeberg.org" => "codeberg",
        _ => return None,
    };

    // Path looks like /owner/repo or /owner/repo/...
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 { return None; }
    let owner = segments[0];
    let repo = segments[1];
    if owner.is_empty() || repo.is_empty() { return None; }

    // Skip well-known non-repo top-level paths.
    const SPECIAL: &[&str] = &[
        "orgs", "sponsors", "marketplace", "features", "settings", "notifications",
        "explore", "trending", "topics", "collections", "events", "search",
        "login", "join", "logout", "pricing", "customer-stories", "security",
        "about", "site", "enterprise", "team", "premium", "readme",
        "users", "groups", "help", "dashboard", "snippets",
    ];
    if SPECIAL.iter().any(|s| owner.eq_ignore_ascii_case(s)) { return None; }

    Some(platform)
}

/// File-extension-driven kind for a single-file clipboard entry.
pub fn kind_for_file(path: &str) -> Kind {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".pdf") {
        return Kind::Pdf;
    }
    Kind::File
}

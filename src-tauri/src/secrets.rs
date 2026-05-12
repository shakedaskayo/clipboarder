//! Heuristic secret/credential detection.
//!
//! Used by the CLI `--no-secrets` flag to keep agents from accidentally
//! pulling API keys, OAuth tokens, JWTs, or similar from the clipboard
//! history into their context.
//!
//! False positives are acceptable — we'd rather over-redact than leak.
//! False negatives are NOT acceptable for the canonical-prefix providers.

use once_cell::sync::Lazy;
use regex::Regex;

/// What we matched on, used for the redacted placeholder label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    OpenAi,
    Anthropic,
    GitHubPat,
    GitHubOauth,
    Slack,
    GitLab,
    AwsAccess,
    AwsSecret,
    Google,
    Stripe,
    Twilio,
    Jwt,
    PrivateKeyHeader,
    HighEntropy,
    NamedAssignment,
}

impl SecretKind {
    pub fn label(self) -> &'static str {
        match self {
            SecretKind::OpenAi            => "openai api key",
            SecretKind::Anthropic         => "anthropic api key",
            SecretKind::GitHubPat         => "github personal access token",
            SecretKind::GitHubOauth       => "github oauth token",
            SecretKind::Slack             => "slack token",
            SecretKind::GitLab            => "gitlab personal access token",
            SecretKind::AwsAccess         => "aws access key id",
            SecretKind::AwsSecret         => "aws secret access key",
            SecretKind::Google            => "google api key",
            SecretKind::Stripe            => "stripe api key",
            SecretKind::Twilio            => "twilio api key",
            SecretKind::Jwt               => "jwt",
            SecretKind::PrivateKeyHeader  => "private key block",
            SecretKind::HighEntropy       => "looks like a credential",
            SecretKind::NamedAssignment   => "key=value with sensitive name",
        }
    }
}

static PROVIDER_PREFIXES: Lazy<Vec<(SecretKind, Regex)>> = Lazy::new(|| {
    let raw: &[(SecretKind, &str)] = &[
        // Anthropic — checked BEFORE the generic `sk-` rule below so it
        // doesn't get mislabeled as OpenAI.
        (SecretKind::Anthropic, r"\bsk-ant-[A-Za-z0-9_-]{30,}\b"),
        // OpenAI
        (SecretKind::OpenAi,    r"\bsk-proj-[A-Za-z0-9_-]{20,}\b"),
        (SecretKind::OpenAi,    r"\bsk-[A-Za-z0-9_-]{20,}\b"),
        // GitHub
        (SecretKind::GitHubPat, r"\bghp_[A-Za-z0-9_]{30,}\b"),
        (SecretKind::GitHubPat, r"\bgithub_pat_[A-Za-z0-9_]{30,}\b"),
        (SecretKind::GitHubOauth, r"\bgho_[A-Za-z0-9_]{30,}\b"),
        (SecretKind::GitHubOauth, r"\bghu_[A-Za-z0-9_]{30,}\b"),
        (SecretKind::GitHubOauth, r"\bghs_[A-Za-z0-9_]{30,}\b"),
        // Slack
        (SecretKind::Slack,     r"\bxox[abprs]-[A-Za-z0-9-]{10,}\b"),
        // GitLab
        (SecretKind::GitLab,    r"\bglpat-[A-Za-z0-9_-]{20,}\b"),
        // AWS
        (SecretKind::AwsAccess, r"\bAKIA[A-Z0-9]{16}\b"),
        (SecretKind::AwsAccess, r"\bASIA[A-Z0-9]{16}\b"),
        // Google
        (SecretKind::Google,    r"\bAIza[A-Za-z0-9_-]{35}\b"),
        // Stripe
        (SecretKind::Stripe,    r"\bsk_live_[A-Za-z0-9]{20,}\b"),
        (SecretKind::Stripe,    r"\brk_live_[A-Za-z0-9]{20,}\b"),
        // Twilio
        (SecretKind::Twilio,    r"\bSK[a-f0-9]{32}\b"),
        // JWT (3 base64 segments separated by .)
        (SecretKind::Jwt,       r"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b"),
        // PEM / OpenSSH private keys
        (SecretKind::PrivateKeyHeader, r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
    ];
    raw.iter()
        .map(|(k, p)| (*k, Regex::new(p).unwrap()))
        .collect()
});

static NAMED_ASSIGNMENT: Lazy<Regex> = Lazy::new(|| {
    // Match common sensitive variable names assigned to a non-trivial value.
    // We don't try to be exhaustive; this is a backstop after the provider
    // prefixes above.
    Regex::new(
        r"(?i)\b(api[_-]?key|access[_-]?token|secret[_-]?key|private[_-]?key|password|passwd|bearer)\s*[:=]\s*['\x22]?[^\s'\x22]{12,}",
    )
    .unwrap()
});

/// Classify `content`. Returns `Some(SecretKind)` if it looks like a credential.
pub fn detect(content: &str) -> Option<SecretKind> {
    // Skip very short content — anything < 12 chars is likely not a secret.
    if content.len() < 12 {
        return None;
    }

    for (kind, re) in PROVIDER_PREFIXES.iter() {
        if re.is_match(content) {
            return Some(*kind);
        }
    }

    if NAMED_ASSIGNMENT.is_match(content) {
        return Some(SecretKind::NamedAssignment);
    }

    // High-entropy single-token check: if the entire content is one token
    // of >= 32 chars, all base64/hex/url-safe, and has high enough entropy.
    if is_high_entropy_token(content) {
        return Some(SecretKind::HighEntropy);
    }

    None
}

fn is_high_entropy_token(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.len() < 32 {
        return false;
    }
    // Must be a single token (no whitespace, no quotes).
    if trimmed.chars().any(|c| c.is_whitespace() || c == '"' || c == '\'') {
        return false;
    }
    // All chars must be in the base64/hex/url-safe alphabet.
    if !trimmed.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '-' | '_' | '.')
    }) {
        return false;
    }
    shannon_entropy(trimmed) > 3.5
}

fn shannon_entropy(s: &str) -> f64 {
    let mut counts = [0u32; 128];
    let mut total = 0u32;
    for b in s.bytes() {
        if (b as usize) < counts.len() {
            counts[b as usize] += 1;
            total += 1;
        }
    }
    if total == 0 {
        return 0.0;
    }
    let total_f = total as f64;
    counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = (c as f64) / total_f;
            -p * p.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_openai() {
        assert!(detect("sk-abcdefghijklmnopqrstuvwx").is_some());
    }
    #[test]
    fn detects_anthropic() {
        assert!(detect("sk-ant-api03-AAA000111222333444555666777888999000-AAA000111222333444555666777888999000").is_some());
    }
    #[test]
    fn detects_jwt() {
        assert!(detect("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c").is_some());
    }
    #[test]
    fn detects_aws() {
        assert!(detect("AKIAIOSFODNN7EXAMPLE").is_some());
    }
    #[test]
    fn detects_named_assignment() {
        assert!(detect("api_key=AKjsdf039280SDFjsdf03928023").is_some());
    }
    #[test]
    fn ignores_short() {
        assert!(detect("hello world").is_none());
    }
    #[test]
    fn ignores_normal_text() {
        assert!(detect("This is just some prose that doesn't look like a credential.").is_none());
    }
    #[test]
    fn ignores_normal_code() {
        assert!(detect("fn main() { println!(\"hello\"); }").is_none());
    }
    #[test]
    fn anthropic_takes_priority_over_openai() {
        // A long sk-ant- string must be labeled anthropic, not openai.
        let key = "sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        assert_eq!(detect(key), Some(SecretKind::Anthropic));
    }
}

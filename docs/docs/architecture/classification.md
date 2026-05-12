# Classification

How clipboarder decides what kind of thing you just copied.

## Entry point

`classify::classify_text(text: &str) -> Classified` runs on every text clipboard event. Returns a `Kind`, an optional `meta` tag, and a single-line `preview`.

Image and file events take separate paths (`Kind::Image`, `Kind::File`/`Kind::Pdf`).

## The decision tree

```
trim text
  ├─ empty            → Kind::Text
  ├─ looks like /abs path or ~/path
  │    └─ no spaces, no newlines, < 1KB
  │           → Kind::File
  ├─ matches hex/rgb/hsl color regex
  │           → Kind::Color, meta = "hex"|"rgb"|"hsl"
  ├─ matches email regex
  │           → Kind::Email
  ├─ matches http(s) URL + Url::parse succeeds
  │    ├─ host is a code-host with owner/repo path
  │    │   → Kind::Repo, meta = "github"|"gitlab"|...
  │    ├─ host is a media platform
  │    │   → Kind::Music or Kind::Video, meta = platform
  │    └─ otherwise
  │       → Kind::Url, meta = host
  ├─ code heuristic score ≥ 2
  │           → Kind::Code, meta = best-guess language
  └─ default
              → Kind::Text
```

## Color detection

Three regexes (case-insensitive):

```
^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$
^rgba?\(\s*[\d.]+[\s,]+[\d.]+[\s,]+[\d.]+(?:[\s,/]+[\d.]+%?)?\s*\)$
^hsla?\(\s*[\d.]+(?:deg)?[\s,]+[\d.]+%[\s,]+[\d.]+%(?:[\s,/]+[\d.]+%?)?\s*\)$
```

## Code detection (heuristic)

```
score = 0
score += 2 if (text has `{...}` or `[...]`)
score += 1 if (text has ≥2 semicolons)
score += 1 if (multi-line AND ≥1 line starts with 2 spaces or tab)
score += 1 if (text contains "=>" or "->")
score += 1 if (text contains "::")

if score >= 2: Kind::Code
```

Plus a fast path for known command prefixes (`git `, `npm `, `cargo `, `docker `, `kubectl `, etc.) → `Kind::Code` with `meta="shell"`.

Language guess (cheap, often right):

- `fn ... ->` or `let ` → `rust`
- `def ...:\n` → `python`
- `=>` + (`const `, `let `, `function`) → `javascript`
- `interface ` or `type ` + `: ` → `typescript`
- starts with `{` and contains `: "` → `json`
- `SELECT ` / `FROM ` → `sql`
- `package ` + `func ` → `go`
- `<html` or `</` → `html`
- otherwise → `code`

## Repo detection

Host must match one of: `github.com`, `gitlab.com`, `bitbucket.org`, `codeberg.org`, `gist.github.com`.

Path must look like `/<owner>/<repo>(/...)?` where `<owner>` is not in a skip-list of known non-repo top-level paths:

```
orgs, sponsors, marketplace, features, settings, notifications,
explore, trending, topics, collections, events, search, login,
join, logout, pricing, customer-stories, security, about, site,
enterprise, team, premium, readme, users, groups, help, dashboard,
snippets
```

The exact resource (Pull Request / Issue / File / etc.) is parsed at preview time by the frontend (`src/lib/repo.ts`), since classification only needs the bucket.

## Media detection

| Host pattern | Kind | meta |
|--------------|------|------|
| `open.spotify.com`, `*.spotify.com` | Music | `spotify` |
| `music.apple.com`, `itunes.apple.com` | Music | `apple-music` |
| `music.youtube.com` | Music | `youtube-music` |
| `soundcloud.com`, `*.soundcloud.com` | Music | `soundcloud` |
| `*.bandcamp.com` | Music | `bandcamp` |
| `youtube.com`, `youtu.be`, `*.youtube.com` | Video | `youtube` |
| `vimeo.com`, `*.vimeo.com` | Video | `vimeo` |
| `twitch.tv`, `*.twitch.tv` | Video | `twitch` |

## Why heuristics, not ML

- Sub-millisecond cost — runs synchronously on every clipboard event
- No external dependencies — no models to ship, no inference to schedule
- Predictable failure modes — if the user disagrees with a classification, they can see exactly why and (in a future version) override via meta tags

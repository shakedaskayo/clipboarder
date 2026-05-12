# Changelog

All notable changes to clipboarder are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-05-12

The first public release.

### Added

- NSPasteboard watcher with text, image (PNG), and file capture
- Smart classification at capture time: text, url, email, code, color, image, file, pdf, music, video
- SQLite storage with FTS5 full-text search and bm25 ranking
- SHA-256-based deduplication and dedup-by-recent-bump on re-copy
- Custom hotkey, default `⌘⇧V`, with rebind via in-app recorder
- Quick-paste with `⌘1`–`⌘9`
- Pinning — pinned items survive `Clear history` and float to the top
- Source-app capture (name + bundle id) and on-demand app-icon extraction via NSWorkspace
- Rich previews: color swatches (HEX/RGB/HSL), inline PDF embed, branded music/video cards (Spotify, Apple Music, YouTube, YouTube Music, SoundCloud, Bandcamp, Vimeo, Twitch)
- Settings panel: hotkey, launch-at-login, max history items, auto-clear after N days, excluded apps
- Privacy exclusions enforced on capture via `NSWorkspace.frontmostApplication.bundleIdentifier`
- Menu-bar tray icon with Show / Settings / Clear history / Quit
- Translucent overlay window that floats above all apps and joins every Space
- macOS `.dmg` installer and one-liner `install.sh`
- MkDocs Material documentation site at <https://shakedaskayo.github.io/clipboarder>

[Unreleased]: https://github.com/shakedaskayo/clipboarder/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/shakedaskayo/clipboarder/releases/tag/v0.1.0

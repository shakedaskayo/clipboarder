# Content types

Every clipboard event is classified by clipboarder at the moment of capture. The kind determines the icon tile, the preview pane layout, and which filter chips count it.

## Text

The default. Plain prose, snippets, anything that didn't match a more specific kind. Preview: a wrapped `<pre>` of the full text.

## URL

A standalone `http://` or `https://` URL.

- Row meta shows the host
- Preview shows a rich website card with the OpenGraph image, title, description, and an **Open in browser** button
- Metadata is fetched lazily on first preview and cached forever at `~/Library/Application Support/com.clipboarder.app/url_meta/<hash>.json`

## Repo

A code-host URL: `github.com/owner/repo`, `gitlab.com/owner/repo`, `bitbucket.org/owner/repo`, `codeberg.org/owner/repo`, or `gist.github.com/owner/id`.

Preview card shows:

- Platform-specific glow (GitHub white, GitLab orange, Bitbucket blue, etc.)
- Big `owner/repo` typography
- A colored pill describing the resource:
  - `Repository`
  - `Pull request #N`
  - `Issue #N`
  - `Discussion #N`
  - `Commit <sha7>`
  - `Release <tag>`
  - `File · <name>`
  - `Folder · <path>`
  - `Actions`
  - `Wiki`
- OG hero image and description (lazy-fetched)
- **Open on GitHub** button

## Email

A bare `user@host.tld`. Preview shows the address big and offers a **Compose email** button that opens `mailto:` in your default mail client.

## Code

Heuristic detection: braces + semicolons + indented multi-line, or a recognized command prefix (`git `, `npm `, `cargo `, `docker `, etc.). Best-effort language tag is stored in `meta` (`rust`, `python`, `javascript`, `typescript`, `json`, `sql`, `go`, `html`, `shell`).

Preview: dark-bg monospace code block.

## Color

Matches `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(...)`, `rgba(...)`, `hsl(...)`, `hsla(...)`.

Preview shows:

- Large color swatch
- All three notations side-by-side (HEX / RGB / HSL)

## Image

Bitmap captured from the clipboard (e.g. `⌘⇧4` screenshot, "Copy image" in Safari). Stored as PNG at `<app_data>/images/<hash16>.png`.

Preview: the full image, fit to the pane.

## PDF

A single `.pdf` file URL on the clipboard.

Preview: WebKit's native inline PDF viewer via `<embed>`, full-pane. Lazy-loaded over IPC (up to 50 MB).

## Music

A URL on a streaming music service:

- Spotify (`open.spotify.com/track`, `/album`, `/playlist`, `/artist`)
- Apple Music (`music.apple.com`)
- YouTube Music (`music.youtube.com`)
- SoundCloud
- Bandcamp

Preview: branded card with platform glow + parsed title/subtitle + **Open in <platform>** button.

## Video

A URL on a video platform:

- YouTube (`youtube.com/watch`, `youtu.be`)
- Vimeo
- Twitch

Same card style as Music.

## File

Any other file path on the clipboard. One file or many. Preview: the path(s).

## How classification works

See [architecture / classification](../architecture/classification.md) for the source-of-truth heuristics.

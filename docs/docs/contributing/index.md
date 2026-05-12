# Contributing

See the canonical [CONTRIBUTING.md](https://github.com/shakedaskayo/clipboarder/blob/main/CONTRIBUTING.md) in the repo root for the full setup.

## Quick orientation

```bash
git clone https://github.com/shakedaskayo/clipboarder.git
cd clipboarder
make dev
```

## Make targets

| Target | What it does |
|--------|--------------|
| `make dev` | `npm install` + `tauri dev` (HMR) |
| `make build` | Full release build (.app + .dmg) |
| `make dmg` | Just the .dmg |
| `make test` | `cargo check` + `tsc --noEmit` |
| `make lint` | `cargo clippy -D warnings` + `cargo fmt --check` |
| `make fmt` | `cargo fmt` |
| `make docs` | Serve MkDocs at `localhost:8000` |
| `make icon` | Regenerate the app icon |
| `make clean` | Remove `target/` and `dist/` |

## Where to start

- **Want a new content kind?** Add a `Kind::Xxx` variant in `src-tauri/src/classify.rs`, mirror it in `src/lib/types.ts`, then add an icon, a row-icon color, a Preview renderer, and a filter chip. The whole flow is ≤ 50 lines.
- **Want a new platform-aware card?** Look at `src/components/RepoCard.tsx` for the pattern.
- **Want to debug a classification?** `sqlite3 ~/Library/Application\ Support/com.clipboarder.app/clipboarder.sqlite` and `SELECT kind, meta, preview FROM items ORDER BY id DESC LIMIT 20;`.

## Commit conventions

We don't enforce a particular commit message style, but keep messages imperative and scoped. `feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...` all work.

## Pull requests

- One feature per PR
- Include a screenshot or short video for UI changes
- Make sure `make test` passes locally
- Reference the issue you're closing in the description

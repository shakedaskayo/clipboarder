#!/usr/bin/env bash
# clipboarder installer — https://github.com/shakedaskayo/clipboarder
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash -s -- --version v0.1.0
#
# Flags:
#   --version, -v  <tag>   pin to a specific release tag (default: latest)
#   --dir,     -d  <dir>   install destination (default: /Applications)
#   --keep-dmg             don't delete the downloaded .dmg after install
#   --no-launch            don't auto-open clipboarder after install
#   --help,    -h          show this help

set -euo pipefail

REPO="shakedaskayo/clipboarder"
APP_NAME="clipboarder.app"
INSTALL_DIR="/Applications"
VERSION=""
KEEP_DMG=0
AUTO_LAUNCH=1

# ── Colors (only when stderr is a tty) ───────────────────────────────
if [ -t 2 ]; then
  CLR_BLUE="\033[34m"; CLR_GREEN="\033[32m"; CLR_RED="\033[31m"
  CLR_BOLD="\033[1m"; CLR_DIM="\033[2m"; CLR_RESET="\033[0m"
else
  CLR_BLUE=""; CLR_GREEN=""; CLR_RED=""; CLR_BOLD=""; CLR_DIM=""; CLR_RESET=""
fi

log()    { printf "${CLR_BLUE}==>${CLR_RESET} %s\n" "$*" >&2; }
ok()     { printf "${CLR_GREEN}✓${CLR_RESET} %s\n" "$*" >&2; }
warn()   { printf "${CLR_BOLD}warning:${CLR_RESET} %s\n" "$*" >&2; }
die()    { printf "${CLR_RED}error:${CLR_RESET} %s\n" "$*" >&2; exit 1; }

usage() {
  cat <<'EOF'
clipboarder installer — https://github.com/shakedaskayo/clipboarder

Usage:
  curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash -s -- --version v0.1.0

Flags:
  --version, -v  <tag>   pin to a specific release tag (default: latest)
  --dir,     -d  <dir>   install destination (default: /Applications)
  --keep-dmg             don't delete the downloaded .dmg after install
  --no-launch            don't auto-open clipboarder after install
  --help,    -h          show this help
EOF
  exit 0
}

# ── Parse args ───────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version|-v) VERSION="$2"; shift 2 ;;
    --dir|-d)     INSTALL_DIR="$2"; shift 2 ;;
    --keep-dmg)   KEEP_DMG=1; shift ;;
    --no-launch)  AUTO_LAUNCH=0; shift ;;
    --help|-h)    usage ;;
    *)            die "Unknown option: $1" ;;
  esac
done

# ── Platform check ───────────────────────────────────────────────────
[[ "$(uname -s)" == "Darwin" ]] || die "clipboarder is macOS-only. (You're on $(uname -s).)"

ARCH=$(uname -m)
case "$ARCH" in
  arm64)   ARCH_TAG="aarch64" ;;
  x86_64)  ARCH_TAG="x64" ;;
  *)       die "Unsupported architecture: $ARCH" ;;
esac

# ── Authenticated curl when GITHUB_TOKEN is set (private-repo support) ──
curl_gh() {
  local args=(-fsSL -H "Accept: application/vnd.github+json")
  if [ -n "${GITHUB_TOKEN:-}" ]; then
    args+=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
  fi
  curl "${args[@]}" "$@"
}

# ── Resolve release info ─────────────────────────────────────────────
if [ -z "$VERSION" ]; then
  log "Looking up the latest clipboarder release…"
  RELEASE_JSON=$(curl_gh "https://api.github.com/repos/${REPO}/releases/latest")
else
  log "Looking up clipboarder release ${VERSION}…"
  RELEASE_JSON=$(curl_gh "https://api.github.com/repos/${REPO}/releases/tags/${VERSION}")
fi

# Pull tag + asset url. Prefer jq when available, otherwise grep our way.
if command -v jq >/dev/null 2>&1; then
  TAG=$(printf '%s' "$RELEASE_JSON" | jq -r '.tag_name // empty')
  DMG_URL=$(printf '%s' "$RELEASE_JSON" \
    | jq -r --arg arch "$ARCH_TAG" '
        .assets[]? | select(.name | endswith(".dmg")) | select(.name | contains($arch)) | .browser_download_url
      ' | head -n1)
else
  TAG=$(printf '%s' "$RELEASE_JSON" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
  DMG_URL=$(printf '%s' "$RELEASE_JSON" \
    | tr ',' '\n' \
    | grep '"browser_download_url"' \
    | grep -i "${ARCH_TAG}.*\.dmg" \
    | head -n1 \
    | sed -E 's/.*"(https:[^"]+)".*/\1/')
fi

[ -n "$TAG" ]      || die "Couldn't find a release tag in the GitHub API response."
[ -n "$DMG_URL" ]  || die "No .dmg asset for ${ARCH_TAG} in release ${TAG}."

log "Installing clipboarder ${CLR_BOLD}${TAG}${CLR_RESET} (${ARCH_TAG})"

# ── Download ─────────────────────────────────────────────────────────
TMPDIR=$(mktemp -d -t clipboarder-install.XXXXXX)
trap 'rm -rf "$TMPDIR"' EXIT

DMG_PATH="${TMPDIR}/$(basename "$DMG_URL")"
log "Downloading $(basename "$DMG_URL")…"
curl_gh -o "$DMG_PATH" "$DMG_URL"

# ── Mount, copy, unmount ─────────────────────────────────────────────
log "Mounting disk image…"
ATTACH_OUT=$(hdiutil attach "$DMG_PATH" -nobrowse -mountrandom "$TMPDIR" -plist 2>/dev/null)
MOUNT_POINT=$(printf '%s' "$ATTACH_OUT" \
  | grep -A 1 '<key>mount-point</key>' \
  | tail -n1 \
  | sed -E 's/.*<string>(.*)<\/string>.*/\1/')

[ -n "$MOUNT_POINT" ] || die "Could not determine DMG mount point."

cleanup() {
  if [ -n "${MOUNT_POINT:-}" ] && [ -d "$MOUNT_POINT" ]; then
    hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null || true
  fi
}
trap cleanup EXIT

SRC_APP="${MOUNT_POINT}/${APP_NAME}"
[ -d "$SRC_APP" ] || die "${APP_NAME} not found inside the DMG."

DEST="${INSTALL_DIR%/}/${APP_NAME}"

if [ -d "$DEST" ]; then
  log "Replacing existing ${DEST}…"
  rm -rf "$DEST" 2>/dev/null || sudo rm -rf "$DEST"
fi

log "Copying to ${DEST}…"
if ! cp -R "$SRC_APP" "$DEST" 2>/dev/null; then
  sudo cp -R "$SRC_APP" "$DEST"
fi

# Remove macOS download quarantine so the user doesn't get the "downloaded
# from the internet" prompt every time. Failure here isn't fatal.
xattr -dr com.apple.quarantine "$DEST" 2>/dev/null || true

cleanup
trap - EXIT
[ "$KEEP_DMG" = "1" ] || rm -f "$DMG_PATH"

ok "Installed clipboarder ${TAG} → ${DEST}"

# ── CLI symlink ──────────────────────────────────────────────────────
# The .app's binary is dual-mode: no args → GUI, subcommand → CLI. Drop
# a symlink somewhere on PATH so users can run `clipboarder list` etc.
APP_BIN="${DEST}/Contents/MacOS/clipboarder"
CLI_LINK=""
for dir in /usr/local/bin "$HOME/.local/bin"; do
  mkdir -p "$dir" 2>/dev/null || true
  if [ -w "$dir" ]; then
    # Both names point at the same binary. `cb` is the short alias for
    # pipe-friendly one-liners (`cb cp` / `cb p` / `echo … | cb cp`).
    if ln -sf "$APP_BIN" "$dir/clipboarder" 2>/dev/null \
       && ln -sf "$APP_BIN" "$dir/cb" 2>/dev/null; then
      CLI_LINK="$dir/clipboarder"
      break
    fi
  fi
done

if [ -n "$CLI_LINK" ]; then
  ok "CLI: ${CLI_LINK} + $(dirname "$CLI_LINK")/cb (alias) → ${APP_BIN}"
  case ":$PATH:" in
    *":${CLI_LINK%/clipboarder}:"*) : ;;
    *)
      warn "${CLI_LINK%/clipboarder} is not on \$PATH — add it to ~/.zshrc to use \`clipboarder\` / \`cb\` from any shell"
      ;;
  esac
else
  warn "Couldn't create a CLI symlink (need /usr/local/bin or ~/.local/bin writable). The GUI still works."
fi

if [ "$AUTO_LAUNCH" = "1" ]; then
  log "Launching clipboarder…"
  open -a "$DEST"
  cat <<EOF

${CLR_BOLD}Next steps:${CLR_RESET}

  1. The window that just opened has a ${CLR_BOLD}banner at the top${CLR_RESET}
     asking for Accessibility permission. Click "Open Settings".
  2. macOS jumps to ${CLR_BOLD}Privacy & Security → Accessibility${CLR_RESET}.
     Toggle ${CLR_BOLD}clipboarder${CLR_RESET} on.
  3. Switch back to clipboarder — the banner auto-detects the change
     and turns green. You're set.

  • ${CLR_BOLD}⌘⇧V${CLR_RESET} from anywhere   summon the overlay
  • ${CLR_BOLD}⌘,${CLR_RESET}                   open Settings

${CLR_BOLD}Pipe ergonomics${CLR_RESET} (the \`cb\` alias is shorter for one-liners):

  • ${CLR_BOLD}echo "anything" | cb cp${CLR_RESET}       stdin → history + pasteboard
  • ${CLR_BOLD}cb p${CLR_RESET}                          print most recent item
  • ${CLR_BOLD}cb p --kind url${CLR_RESET}               most recent URL
  • ${CLR_BOLD}cb p --grep "react"${CLR_RESET}           most recent match for "react"
  • ${CLR_BOLD}cb p --copy${CLR_RESET}                   ↑ and put on pasteboard
  • ${CLR_BOLD}cb --help${CLR_RESET}                     full reference

${CLR_BOLD}For Claude Code:${CLR_RESET}  install the clipboarder skill (auto-loads):

  mkdir -p ~/.claude/skills/clipboarder && \\
    curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/agents/.claude/skills/clipboarder/SKILL.md \\
      -o ~/.claude/skills/clipboarder/SKILL.md

Docs: https://shakedaskayo.github.io/clipboarder
EOF
fi

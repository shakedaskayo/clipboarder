# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| latest  | Yes       |

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Use [GitHub's private vulnerability reporting](https://github.com/shakedaskayo/clipboarder/security/advisories/new), or email the maintainer privately.

Include:

- A description of the vulnerability and its potential impact
- Steps to reproduce or a proof of concept
- Any suggested mitigations

You can expect an acknowledgement within 48 hours and a resolution timeline within 7 days for critical issues.

## Threat model

clipboarder is a local-only macOS app. It does **not**:

- Make outbound network calls
- Sync clipboard data anywhere
- Phone home with telemetry
- Require any account

It **does**:

- Read the system pasteboard (this is the entire point of the app)
- Write captured items to a local SQLite database at `~/Library/Application Support/com.clipboarder.app/`
- Read app bundles via `NSWorkspace` to extract source-app icons (cached locally)
- Synthesize `⌘V` keystrokes via `CGEventPost` (requires Accessibility permission)

### What's in scope

- Logic bugs that allow reading clipboard data clipboarder shouldn't have captured (e.g. an exclusion bypass)
- Local privilege escalation through clipboarder
- Crashes triggered by malformed clipboard content (image bombs, oversized files, etc.)
- IPC command bypasses

### What's out of scope

- macOS itself granting clipboard / Accessibility permissions to clipboarder — those are user-controlled prompts
- Physical access to an unlocked machine reading the local SQLite database

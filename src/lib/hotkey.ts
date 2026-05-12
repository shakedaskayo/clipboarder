// Convert between Tauri accelerator strings and pretty kbd tokens.
//
// Tauri format:  "CommandOrControl+Shift+V"
// Modifier order: CommandOrControl, Meta, Control, Alt, Shift
// Key: a single character (letter/digit) or code-name (KeyV, Digit1, Space, F1).

export interface ParsedAccel {
  mods: { meta: boolean; ctrl: boolean; alt: boolean; shift: boolean };
  key: string;
}

export function parseAccel(accel: string): ParsedAccel | null {
  if (!accel) return null;
  const parts = accel.split("+").map(p => p.trim());
  if (!parts.length) return null;
  const mods = { meta: false, ctrl: false, alt: false, shift: false };
  let key = "";
  for (const p of parts) {
    const lower = p.toLowerCase();
    if (lower === "commandorcontrol" || lower === "cmdorctrl") {
      mods.meta = true; // shown as Cmd on mac
    } else if (lower === "command" || lower === "cmd" || lower === "meta" || lower === "super") {
      mods.meta = true;
    } else if (lower === "control" || lower === "ctrl") {
      mods.ctrl = true;
    } else if (lower === "alt" || lower === "option" || lower === "opt") {
      mods.alt = true;
    } else if (lower === "shift") {
      mods.shift = true;
    } else {
      key = p;
    }
  }
  return { mods, key };
}

export function prettyTokens(accel: string): string[] {
  const p = parseAccel(accel);
  if (!p) return [];
  const out: string[] = [];
  if (p.mods.ctrl) out.push("⌃");
  if (p.mods.alt) out.push("⌥");
  if (p.mods.shift) out.push("⇧");
  if (p.mods.meta) out.push("⌘");
  if (p.key) out.push(prettyKey(p.key));
  return out;
}

function prettyKey(key: string): string {
  // Normalize KeyboardEvent.code shapes (KeyV → V, Digit1 → 1, F12 → F12)
  if (/^Key[A-Z]$/.test(key)) return key.slice(3);
  if (/^Digit\d$/.test(key)) return key.slice(5);
  if (/^Numpad\d$/.test(key)) return key.slice(6);
  switch (key) {
    case "Space": return "Space";
    case "Enter":
    case "Return": return "↵";
    case "Tab": return "⇥";
    case "Escape": return "Esc";
    case "Backspace": return "⌫";
    case "Delete": return "⌦";
    case "ArrowUp": return "↑";
    case "ArrowDown": return "↓";
    case "ArrowLeft": return "←";
    case "ArrowRight": return "→";
    case "Comma": return ",";
    case "Period": return ".";
    case "Slash": return "/";
    case "Semicolon": return ";";
    case "Quote": return "'";
    case "BracketLeft": return "[";
    case "BracketRight": return "]";
    case "Backquote": return "`";
    case "Backslash": return "\\";
    case "Minus": return "-";
    case "Equal": return "=";
    default: return key;
  }
}

// Build a Tauri accelerator string from a KeyboardEvent captured during recording.
// Returns null if the combo is not yet valid (no non-modifier key pressed).
export function accelFromEvent(e: KeyboardEvent): string | null {
  const parts: string[] = [];
  if (e.metaKey) parts.push("CommandOrControl");
  // On macOS, prefer the unified CommandOrControl over a separate Ctrl when
  // both are pressed, but if Ctrl is pressed without Cmd, record Control.
  if (e.ctrlKey && !e.metaKey) parts.push("Control");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");

  // Reject modifier-only events.
  const code = e.code;
  if (
    code === "MetaLeft" || code === "MetaRight" ||
    code === "ControlLeft" || code === "ControlRight" ||
    code === "AltLeft" || code === "AltRight" ||
    code === "ShiftLeft" || code === "ShiftRight"
  ) {
    return null;
  }
  if (!parts.length) return null;
  // Use KeyboardEvent.code for stability (independent of keyboard layout).
  parts.push(code);
  return parts.join("+");
}

import { useEffect, useRef, useState } from "react";
import { accelFromEvent, parseAccel, prettyTokens } from "../lib/hotkey";

interface Props {
  value: string;
  onChange: (accel: string) => void;
}

export function HotkeyRecorder({ value, onChange }: Props) {
  const [recording, setRecording] = useState(false);
  const [draft, setDraft] = useState<string | null>(null);
  const boxRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      // Allow Esc to cancel.
      if (e.code === "Escape" && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
        setRecording(false);
        setDraft(null);
        return;
      }
      const next = accelFromEvent(e);
      if (next) {
        // Require at least one real modifier so we don't bind a bare key.
        const p = parseAccel(next);
        const hasMod = p && (p.mods.meta || p.mods.ctrl || p.mods.alt);
        if (hasMod) {
          setDraft(next);
        } else {
          setDraft(null);
        }
      }
    };
    const onUp = (e: KeyboardEvent) => {
      // Commit on key up if we have a valid draft.
      if (!draft) return;
      // Only commit when the *non-modifier* key is released.
      const code = e.code;
      if (
        code !== "MetaLeft" && code !== "MetaRight" &&
        code !== "ControlLeft" && code !== "ControlRight" &&
        code !== "AltLeft" && code !== "AltRight" &&
        code !== "ShiftLeft" && code !== "ShiftRight"
      ) {
        onChange(draft);
        setRecording(false);
        setDraft(null);
      }
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("keyup", onUp, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("keyup", onUp, true);
    };
  }, [recording, draft, onChange]);

  const displayed = recording && draft ? draft : value;
  const tokens = prettyTokens(displayed);

  return (
    <div className="hotkey-recorder" ref={boxRef}>
      <div className={`hk-display${recording ? " recording" : ""}`}>
        {tokens.length === 0 ? (
          <span className="hk-empty">Press a key combination…</span>
        ) : (
          tokens.map((t, i) => (
            <kbd key={i} className="hk-key">{t}</kbd>
          ))
        )}
      </div>
      {recording ? (
        <button
          className="btn btn-ghost"
          onClick={() => { setRecording(false); setDraft(null); }}
        >
          Cancel
        </button>
      ) : (
        <button className="btn btn-primary" onClick={() => setRecording(true)}>
          Record
        </button>
      )}
    </div>
  );
}

import { useEffect, useRef, useState } from "react";
import { accessibilityTrusted, openAccessibilitySettings } from "../lib/api";
import { ShieldIcon, CheckIcon, CloseIcon } from "../lib/icons";

// Cache the granted state for the lifetime of this process so the banner
// disappears the instant the user grants permission, without re-prompting
// on every re-render.
let lastKnownTrusted: boolean | null = null;

interface Props {
  /** Hide the banner via the "Skip" button. Bypass remains until next launch. */
  onSkip?: () => void;
}

export function PermissionBanner({ onSkip }: Props) {
  const [trusted, setTrusted] = useState<boolean | null>(lastKnownTrusted);
  const [skipped, setSkipped] = useState(false);
  const [checking, setChecking] = useState(false);
  const [justGranted, setJustGranted] = useState(false);
  const pollRef = useRef<number | null>(null);

  async function check() {
    setChecking(true);
    try {
      const v = await accessibilityTrusted();
      lastKnownTrusted = v;
      setTrusted(prev => {
        if (prev === false && v === true) setJustGranted(true);
        return v;
      });
    } catch {
      setTrusted(true); // assume trusted on error (non-macOS)
    } finally {
      setChecking(false);
    }
  }

  useEffect(() => {
    check();
    // Poll every 2s while we don't yet have permission. macOS doesn't notify
    // when the user toggles the switch, so polling is the standard pattern.
    pollRef.current = window.setInterval(() => {
      if (lastKnownTrusted !== true) check();
    }, 2000);
    return () => {
      if (pollRef.current) window.clearInterval(pollRef.current);
    };
  }, []);

  // Auto-dismiss the success indicator after a short delay
  useEffect(() => {
    if (!justGranted) return;
    const t = window.setTimeout(() => setJustGranted(false), 2500);
    return () => window.clearTimeout(t);
  }, [justGranted]);

  if (trusted === null) return null;          // initial check pending
  if (trusted && !justGranted) return null;   // permission good and announcement consumed
  if (skipped && !justGranted) return null;   // user dismissed for the session

  if (justGranted) {
    return (
      <div className="perm-banner perm-success">
        <span className="perm-icon"><CheckIcon /></span>
        <div className="perm-body">
          <strong>Accessibility granted</strong>
          <span> · paste-back is now active.</span>
        </div>
      </div>
    );
  }

  return (
    <div className="perm-banner">
      <span className="perm-icon"><ShieldIcon /></span>
      <div className="perm-body">
        <strong>One more step:</strong>
        <span> grant Accessibility so clipboarder can paste back into your apps.</span>
      </div>
      <div className="perm-actions">
        <button
          className="btn btn-primary"
          onClick={() => openAccessibilitySettings()}
        >
          Open Settings
        </button>
        <button
          className="btn btn-ghost"
          onClick={check}
          disabled={checking}
        >
          {checking ? "Checking…" : "I just granted it"}
        </button>
        <button
          className="icon-btn perm-close"
          title="Skip for this session"
          onClick={() => {
            setSkipped(true);
            onSkip?.();
          }}
        >
          <CloseIcon />
        </button>
      </div>
    </div>
  );
}

import { useEffect, useState } from "react";
import type { Settings as SettingsT } from "../lib/types";
import {
  clearHistory, frontmostAppInfo, getSettings, saveSettings,
} from "../lib/api";
import { HotkeyRecorder } from "./HotkeyRecorder";
import { Toggle } from "./Toggle";
import { Select } from "./Select";

interface Props {
  onClose: () => void;
}

export function SettingsPanel({ onClose }: Props) {
  const [settings, setSettings] = useState<SettingsT | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [newExcluded, setNewExcluded] = useState("");

  useEffect(() => {
    getSettings().then(setSettings);
  }, []);

  if (!settings) {
    return (
      <div className="settings">
        <div className="settings-loading">Loading…</div>
      </div>
    );
  }

  async function update(patch: Partial<SettingsT>) {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    setSaving(true);
    setError(null);
    try {
      const saved = await saveSettings(next);
      setSettings(saved);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function addCurrentApp() {
    const app = await frontmostAppInfo();
    if (!app) return;
    if (settings!.excluded_apps.includes(app.bundle_id)) return;
    update({ excluded_apps: [...settings!.excluded_apps, app.bundle_id] });
  }

  function removeExcluded(id: string) {
    update({ excluded_apps: settings!.excluded_apps.filter(x => x !== id) });
  }

  function addManual() {
    const id = newExcluded.trim();
    if (!id) return;
    if (settings!.excluded_apps.includes(id)) { setNewExcluded(""); return; }
    update({ excluded_apps: [...settings!.excluded_apps, id] });
    setNewExcluded("");
  }

  return (
    <div className="settings">
      <div className="settings-header">
        <button className="back-btn" onClick={onClose}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
          <span>Back</span>
        </button>
        <h2>Settings</h2>
        <div className="settings-status">
          {saving ? "Saving…" : error ? <span className="err">{error}</span> : ""}
        </div>
      </div>

      <div className="settings-body">
        <Section title="Hotkey" hint="Press this combination from anywhere on your Mac to summon clipboarder.">
          <Row label="Open clipboarder">
            <HotkeyRecorder
              value={settings.hotkey}
              onChange={v => update({ hotkey: v })}
            />
          </Row>
        </Section>

        <Section title="General">
          <Row label="Launch at login" hint="Start clipboarder automatically when you log in.">
            <Toggle
              checked={settings.launch_at_login}
              onChange={v => update({ launch_at_login: v })}
            />
          </Row>
        </Section>

        <Section title="History">
          <Row label="Maximum items" hint="Older non-pinned items are removed when this limit is reached.">
            <Select
              value={settings.max_items}
              onChange={v => update({ max_items: Number(v) })}
              options={[
                { value: 100, label: "100" },
                { value: 250, label: "250" },
                { value: 500, label: "500" },
                { value: 1000, label: "1,000" },
                { value: 2500, label: "2,500" },
                { value: 5000, label: "5,000" },
                { value: 0, label: "Unlimited" },
              ]}
            />
          </Row>
          <Row label="Auto-clear after" hint="Forget non-pinned items older than this.">
            <Select
              value={settings.auto_clear_days}
              onChange={v => update({ auto_clear_days: Number(v) })}
              options={[
                { value: 0, label: "Never" },
                { value: 1, label: "1 day" },
                { value: 7, label: "1 week" },
                { value: 30, label: "1 month" },
                { value: 90, label: "3 months" },
                { value: 365, label: "1 year" },
              ]}
            />
          </Row>
          <Row label="">
            <button
              className="btn btn-danger"
              onClick={async () => {
                if (confirm("Clear all non-pinned items? This can't be undone.")) {
                  await clearHistory();
                }
              }}
            >
              Clear all history
            </button>
          </Row>
        </Section>

        <Section
          title="Privacy"
          hint="clipboarder won't capture clipboard activity from these apps. Useful for password managers."
        >
          <Row label="Excluded apps">
            <div className="exclusions">
              {settings.excluded_apps.length === 0 ? (
                <p className="hint-empty">No exclusions yet.</p>
              ) : (
                <ul>
                  {settings.excluded_apps.map(id => (
                    <li key={id}>
                      <code>{id}</code>
                      <button className="link" onClick={() => removeExcluded(id)}>Remove</button>
                    </li>
                  ))}
                </ul>
              )}
              <div className="exclusion-add">
                <input
                  type="text"
                  placeholder="com.example.app"
                  value={newExcluded}
                  onChange={e => setNewExcluded(e.target.value)}
                  onKeyDown={e => {
                    if (e.key === "Enter") addManual();
                  }}
                />
                <button className="btn btn-ghost" onClick={addManual}>Add</button>
                <button className="btn btn-ghost" onClick={addCurrentApp} title="Add the currently-frontmost app">
                  Add frontmost app
                </button>
              </div>
            </div>
          </Row>
        </Section>

        <Section title="About">
          <Row label="Version"><span className="value-text">clipboarder 0.1.0</span></Row>
        </Section>
      </div>
    </div>
  );
}

function Section({
  title,
  hint,
  children,
}: {
  title: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="settings-section">
      <header>
        <h3>{title}</h3>
        {hint && <p className="hint">{hint}</p>}
      </header>
      <div className="section-rows">{children}</div>
    </section>
  );
}

function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="settings-row">
      <div className="settings-row-label">
        {label && <span className="lbl">{label}</span>}
        {hint && <span className="hint">{hint}</span>}
      </div>
      <div className="settings-row-control">{children}</div>
    </div>
  );
}

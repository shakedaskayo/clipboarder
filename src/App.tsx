import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ClipItem } from "./lib/types";
import { searchItems, pasteItem, hideWindow } from "./lib/api";
import { SearchIcon, GearIcon } from "./lib/icons";
import { Row } from "./components/Row";
import { Preview } from "./components/Preview";
import { Chips, type Filter } from "./components/Chips";
import { SettingsPanel } from "./components/Settings";

const PAGE_SIZE = 200;

type View = "search" | "settings";

export default function App() {
  const [view, setView] = useState<View>("search");
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [items, setItems] = useState<ClipItem[]>([]);
  const [counts, setCounts] = useState<Record<string, number>>({});
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const refresh = useCallback(async (q = query, f = filter) => {
    const list = await searchItems({
      query: q,
      kind: f as any,
      limit: PAGE_SIZE,
    });
    setItems(list);
    setActive(prev => Math.min(prev, Math.max(0, list.length - 1)));
  }, [query, filter]);

  const refreshCounts = useCallback(async () => {
    const all = await searchItems({ query: "", kind: "all", limit: 5000 });
    const c: Record<string, number> = { all: all.length, pinned: 0 };
    for (const it of all) {
      c[it.kind] = (c[it.kind] ?? 0) + 1;
      if (it.pinned) c.pinned++;
    }
    setCounts(c);
  }, []);

  useEffect(() => { refresh(); }, [refresh]);
  useEffect(() => { refreshCounts(); }, [refreshCounts, items.length]);

  useEffect(() => {
    const un1 = listen("clipboard:new", () => { refresh(); });
    const un2 = listen("window:shown", () => {
      setQuery("");
      setActive(0);
      setView("search");
      setTimeout(() => inputRef.current?.focus(), 0);
      refresh("", filter);
    });
    const un3 = listen("nav:settings", () => { setView("settings"); });
    return () => { un1.then(f => f()); un2.then(f => f()); un3.then(f => f()); };
  }, [refresh, filter]);

  useEffect(() => {
    if (view === "search") inputRef.current?.focus();
  }, [view]);

  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(`[data-i="${active}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [active]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (view !== "search") {
        if (e.key === "Escape") {
          e.preventDefault();
          setView("search");
        }
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        hideWindow();
        return;
      }
      // Cmd+, opens settings (macOS convention)
      if (e.metaKey && e.key === ",") {
        e.preventDefault();
        setView("settings");
        return;
      }
      if (e.key === "ArrowDown" || (e.key === "n" && e.ctrlKey)) {
        e.preventDefault();
        setActive(a => Math.min(a + 1, items.length - 1));
        return;
      }
      if (e.key === "ArrowUp" || (e.key === "p" && e.ctrlKey)) {
        e.preventDefault();
        setActive(a => Math.max(a - 1, 0));
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const it = items[active];
        if (it) pasteItem(it.id);
        return;
      }
      if (e.metaKey && /^[1-9]$/.test(e.key)) {
        e.preventDefault();
        const idx = parseInt(e.key, 10) - 1;
        const it = items[idx];
        if (it) pasteItem(it.id);
        return;
      }
      if (e.metaKey && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setQuery("");
        inputRef.current?.focus();
        return;
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [items, active, view]);

  useEffect(() => {
    const t = setTimeout(() => { refresh(query, filter); }, 60);
    return () => clearTimeout(t);
  }, [query, filter, refresh]);

  const activeItem = items[active] ?? null;

  return (
    <div className="shell">
      {view === "settings" ? (
        <SettingsPanel onClose={() => setView("search")} />
      ) : (
        <>
          <div className="search">
            <SearchIcon className="search-icon" />
            <input
              ref={inputRef}
              autoFocus
              placeholder="Search clipboard…"
              value={query}
              onChange={e => { setQuery(e.target.value); setActive(0); }}
              spellCheck={false}
              autoComplete="off"
            />
            <div className="kbd-row">
              <kbd>↑</kbd><kbd>↓</kbd>
              <span style={{ marginLeft: 4 }}>navigate</span>
              <kbd style={{ marginLeft: 10 }}>↵</kbd>
              <span>paste</span>
            </div>
          </div>

          <Chips filter={filter} counts={counts} onChange={setFilter} />

          <div className="body">
            <div className="list" ref={listRef} role="listbox">
              {items.length === 0 ? (
                <div className="empty" style={{ height: "100%" }}>
                  <SearchIcon />
                  <h3>{query ? "No matches" : "Your clipboard is empty"}</h3>
                  <p>
                    {query
                      ? "Try a different search or clear filters."
                      : "Copy something and clipboarder will start filling up."}
                  </p>
                </div>
              ) : (
                items.map((item, i) => (
                  <div data-i={i} key={item.id}>
                    <Row
                      item={item}
                      index={i}
                      active={i === active}
                      query={query}
                      onActivate={() => pasteItem(item.id)}
                      onSelect={() => setActive(i)}
                    />
                  </div>
                ))
              )}
            </div>
            <Preview item={activeItem} onAction={() => refresh()} />
          </div>

          <div className="footer">
            <div className="brand">
              <span className="brand-dot" />
              <span>clipboarder</span>
            </div>
            <div className="footer-group"><kbd>⌘1-9</kbd><span>quick paste</span></div>
            <div className="footer-group"><kbd>⌘K</kbd><span>clear</span></div>
            <div className="footer-group"><kbd>esc</kbd><span>close</span></div>
            <div className="spacer" />
            <div className="footer-group">
              <span style={{ color: "var(--text-soft)" }}>
                {items.length} item{items.length === 1 ? "" : "s"}
              </span>
            </div>
            <button
              className="footer-btn"
              title="Settings (⌘,)"
              onClick={() => setView("settings")}
            >
              <GearIcon />
            </button>
          </div>
        </>
      )}
    </div>
  );
}

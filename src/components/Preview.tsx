import { useEffect, useMemo, useState } from "react";
import type { ClipItem } from "../lib/types";
import { relativeTime, bytes, lineCount } from "../lib/format";
import { parseColor, cssColor, toHex, toRgb, toHsl } from "../lib/color";
import {
  copyItem, deleteItem, fetchFileBytes, imageUrl, openUrl, togglePin,
} from "../lib/api";
import { useAppIcon } from "../lib/useAppIcon";
import {
  StarIcon, StarOutlineIcon, CopyIcon, TrashIcon, ClipboardIcon,
  MusicIcon, VideoIcon, PdfIcon, ExternalIcon,
} from "../lib/icons";
import { UrlCard } from "./UrlCard";
import { RepoCard } from "./RepoCard";

interface Props {
  item: ClipItem | null;
  onAction: () => void;
}

export function Preview({ item, onAction }: Props) {
  const sourceIcon = useAppIcon(item?.source_app_id ?? null);

  if (!item) {
    return (
      <div className="preview">
        <div className="empty">
          <ClipboardIcon />
          <h3>Nothing here yet</h3>
          <p>Copy anything — text, links, images, files, PDFs, music — and it'll show up instantly. Press <kbd>⌘⇧V</kbd> anywhere to open clipboarder.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="preview">
      <div className="preview-header">
        <span className="preview-kind">{labelFor(item)}</span>
        <span className="preview-meta">
          {bytes(item.size)} · {relativeTime(item.created_at)}
        </span>
        {item.source_app && (
          <span className="preview-source">
            {sourceIcon && <img src={sourceIcon} alt="" />}
            <span>{item.source_app}</span>
          </span>
        )}
        <div className="preview-actions">
          <button
            className={`icon-btn${item.pinned ? " active" : ""}`}
            title={item.pinned ? "Unpin" : "Pin"}
            onClick={async () => { await togglePin(item.id); onAction(); }}
          >
            {item.pinned ? <StarIcon /> : <StarOutlineIcon />}
          </button>
          <button
            className="icon-btn"
            title="Copy to clipboard"
            onClick={async () => { await copyItem(item.id); }}
          >
            <CopyIcon />
          </button>
          <button
            className="icon-btn"
            title="Delete"
            onClick={async () => { await deleteItem(item.id); onAction(); }}
          >
            <TrashIcon />
          </button>
        </div>
      </div>
      <div className={`preview-body kind-${item.kind}`}>
        {renderBody(item)}
      </div>
    </div>
  );
}

function labelFor(item: ClipItem): string {
  switch (item.kind) {
    case "url": return "Link";
    case "email": return "Email";
    case "code": return item.meta ? `Code · ${item.meta}` : "Code";
    case "image": return "Image";
    case "color": return `Color · ${(item.meta || "hex").toUpperCase()}`;
    case "file": return "File";
    case "pdf": return "PDF";
    case "music": return item.meta ? `Music · ${platformName(item.meta)}` : "Music";
    case "video": return item.meta ? `Video · ${platformName(item.meta)}` : "Video";
    case "repo": return item.meta ? `Repo · ${platformName(item.meta)}` : "Repo";
    default: {
      const lc = lineCount(item.content);
      return lc > 1 ? `Text · ${lc} lines` : "Text";
    }
  }
}

function platformName(p: string): string {
  switch (p) {
    case "spotify": return "Spotify";
    case "apple-music": return "Apple Music";
    case "youtube-music": return "YouTube Music";
    case "soundcloud": return "SoundCloud";
    case "bandcamp": return "Bandcamp";
    case "youtube": return "YouTube";
    case "vimeo": return "Vimeo";
    case "twitch": return "Twitch";
    case "github": return "GitHub";
    case "gitlab": return "GitLab";
    case "bitbucket": return "Bitbucket";
    case "codeberg": return "Codeberg";
    case "gist": return "GitHub Gist";
    default: return p;
  }
}

function renderBody(item: ClipItem) {
  if (item.kind === "image" && item.image_path) {
    return (
      <div className="preview-image">
        <img src={imageUrl(item.image_path)} alt="clipboard image" />
      </div>
    );
  }
  if (item.kind === "color") {
    const c = parseColor(item.content);
    if (c) {
      return (
        <div className="preview-color">
          <div className="swatch" style={{ background: cssColor(c) }} />
          <dl className="values">
            <dt>HEX</dt><dd>{toHex(c)}</dd>
            <dt>RGB</dt><dd>{toRgb(c)}</dd>
            <dt>HSL</dt><dd>{toHsl(c)}</dd>
          </dl>
        </div>
      );
    }
  }
  if (item.kind === "pdf") {
    return <PdfPreview path={item.content.split("\n")[0]} />;
  }
  if (item.kind === "music" || item.kind === "video") {
    return <MediaCard url={item.content} platform={item.meta || ""} kind={item.kind} />;
  }
  if (item.kind === "repo") {
    return <RepoCard url={item.content} platform={item.meta || "github"} />;
  }
  if (item.kind === "url") {
    return <UrlCard url={item.content} />;
  }
  if (item.kind === "email") {
    return <EmailCard address={item.content} />;
  }
  return <pre>{item.content}</pre>;
}

function EmailCard({ address }: { address: string }) {
  return (
    <div className="email-card">
      <div className="email-address">{address}</div>
      <div className="url-actions">
        <button className="btn btn-primary" onClick={() => openUrl(`mailto:${address}`)}>
          <ExternalIcon /> Compose email
        </button>
      </div>
    </div>
  );
}

function PdfPreview({ path }: { path: string }) {
  const [src, setSrc] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let url: string | null = null;
    setSrc(null);
    setErr(null);
    fetchFileBytes(path).then(bytes => {
      const blob = new Blob([bytes], { type: "application/pdf" });
      url = URL.createObjectURL(blob);
      setSrc(url);
    }).catch(e => setErr(String(e)));
    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  }, [path]);

  if (err) {
    return (
      <div className="pdf-error">
        <PdfIcon />
        <h4>Can't open this PDF</h4>
        <p>{err}</p>
        <code>{path}</code>
      </div>
    );
  }
  if (!src) {
    return <div className="pdf-loading">Loading PDF…</div>;
  }
  return (
    <div className="pdf-frame">
      <embed src={src} type="application/pdf" />
    </div>
  );
}

function MediaCard({ url, platform, kind }: { url: string; platform: string; kind: "music" | "video" }) {
  const meta = useMemo(() => parseMediaUrl(url, platform), [url, platform]);
  const Icon = kind === "music" ? MusicIcon : VideoIcon;
  return (
    <div className={`media-card platform-${platform}`}>
      <div className="media-art">
        <Icon />
      </div>
      <div className="media-body">
        <div className="media-platform">{platformName(platform)}</div>
        <div className="media-title">{meta.title}</div>
        {meta.subtitle && <div className="media-subtitle">{meta.subtitle}</div>}
        <button className="media-open" onClick={() => openUrl(url)}>
          Open in {platformName(platform)} <ExternalIcon />
        </button>
        <div className="media-url">{url}</div>
      </div>
    </div>
  );
}

function parseMediaUrl(rawUrl: string, platform: string): { title: string; subtitle?: string } {
  try {
    const u = new URL(rawUrl);
    const parts = u.pathname.split("/").filter(Boolean);
    if (platform === "spotify") {
      // /track/<id> | /album/<id> | /playlist/<id> | /artist/<id>
      const [type, id] = parts;
      if (type && id) return { title: capitalize(type), subtitle: id };
    }
    if (platform === "apple-music") {
      // /us/album/<slug>/<id> or /us/song/<slug>/<id>
      const type = parts[1];
      const slug = parts[2];
      if (type && slug) {
        return { title: prettySlug(slug), subtitle: capitalize(type) };
      }
    }
    if (platform === "youtube" || platform === "youtube-music") {
      const v = u.searchParams.get("v") || (u.hostname === "youtu.be" ? parts[0] : null);
      if (v) return { title: "Video", subtitle: v };
      const list = u.searchParams.get("list");
      if (list) return { title: "Playlist", subtitle: list };
    }
    if (platform === "soundcloud") {
      const [user, slug] = parts;
      if (user && slug) return { title: prettySlug(slug), subtitle: user };
      if (user) return { title: user };
    }
    if (platform === "bandcamp") {
      return { title: u.hostname.replace(".bandcamp.com", ""), subtitle: parts.join("/") };
    }
    if (platform === "vimeo") {
      const id = parts[0];
      if (id && /^\d+$/.test(id)) return { title: "Video", subtitle: id };
    }
    if (platform === "twitch") {
      const channel = parts[0];
      if (channel) return { title: channel, subtitle: "Channel" };
    }
    return { title: u.hostname, subtitle: u.pathname };
  } catch {
    return { title: rawUrl };
  }
}

function prettySlug(s: string): string {
  return s.replace(/[-_+]/g, " ").replace(/\b\w/g, c => c.toUpperCase());
}

function capitalize(s: string): string {
  return s ? s[0].toUpperCase() + s.slice(1) : s;
}

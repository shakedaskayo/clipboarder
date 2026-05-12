import { memo } from "react";
import type { ClipItem, Kind } from "../lib/types";
import { relativeTime, highlight, lineCount } from "../lib/format";
import { parseColor, cssColor } from "../lib/color";
import { useAppIcon } from "../lib/useAppIcon";
import {
  TextIcon, UrlIcon, EmailIcon, CodeIcon, ImageIcon, ColorIcon, FileIcon,
  PdfIcon, MusicIcon, VideoIcon, RepoIcon, StarIcon,
} from "../lib/icons";

interface Props {
  item: ClipItem;
  active: boolean;
  index: number;
  query: string;
  onActivate: () => void;
  onSelect: () => void;
}

function kindIcon(kind: Kind) {
  switch (kind) {
    case "url": return <UrlIcon />;
    case "email": return <EmailIcon />;
    case "code": return <CodeIcon />;
    case "image": return <ImageIcon />;
    case "color": return <ColorIcon />;
    case "file": return <FileIcon />;
    case "pdf": return <PdfIcon />;
    case "music": return <MusicIcon />;
    case "video": return <VideoIcon />;
    case "repo": return <RepoIcon />;
    default: return <TextIcon />;
  }
}

function kindLabel(item: ClipItem): string {
  switch (item.kind) {
    case "url": return "Link";
    case "email": return "Email";
    case "code": return item.meta ? `Code · ${item.meta}` : "Code";
    case "image": return "Image";
    case "color": return `Color · ${(item.meta || "hex").toUpperCase()}`;
    case "file": return "File";
    case "pdf": return "PDF";
    case "music": return item.meta ? `Music · ${platformLabel(item.meta)}` : "Music";
    case "video": return item.meta ? `Video · ${platformLabel(item.meta)}` : "Video";
    case "repo": return item.meta ? `Repo · ${platformLabel(item.meta)}` : "Repo";
    default: {
      const lc = lineCount(item.content);
      return lc > 1 ? `Text · ${lc} lines` : "Text";
    }
  }
}

function platformLabel(p: string): string {
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
    case "gist": return "Gist";
    default: return p;
  }
}

export const Row = memo(function Row({ item, active, index, query, onActivate, onSelect }: Props) {
  const parts = highlight(item.preview, query);
  const isColor = item.kind === "color";
  const swatch = isColor ? parseColor(item.content) : null;
  const appIcon = useAppIcon(item.source_app_id);

  return (
    <div
      className={`row${active ? " active" : ""}`}
      onMouseEnter={onSelect}
      onClick={onActivate}
      role="option"
      aria-selected={active}
    >
      <div className="row-num">{index < 9 ? `⌘${index + 1}` : ""}</div>
      {isColor && swatch ? (
        <div className="color-swatch" style={{ background: cssColor(swatch) }} />
      ) : (
        <div className={`row-icon kind-${item.kind}`}>{kindIcon(item.kind)}</div>
      )}
      <div className="row-content">
        <div className="row-title">
          {parts.map((p, i) =>
            typeof p === "string" ? <span key={i}>{p}</span> : <mark key={i}>{p.mark}</mark>,
          )}
        </div>
        <div className="row-meta">
          <span>{kindLabel(item)}</span>
          <span className="dot" />
          <span>{relativeTime(item.last_used_at)}</span>
          {item.source_app && (
            <>
              <span className="dot" />
              <span className="row-source">
                {appIcon && <img src={appIcon} alt="" className="row-source-icon" />}
                <span>{item.source_app}</span>
              </span>
            </>
          )}
        </div>
      </div>
      {item.pinned && (
        <span className="row-pin" title="Pinned"><StarIcon /></span>
      )}
    </div>
  );
});

import type { Kind } from "../lib/types";
import {
  TextIcon, UrlIcon, EmailIcon, CodeIcon, ImageIcon, ColorIcon, FileIcon,
  PdfIcon, MusicIcon, VideoIcon, RepoIcon, StarIcon,
} from "../lib/icons";

export type Filter = "all" | "pinned" | Kind;

interface Props {
  filter: Filter;
  counts: Record<string, number>;
  onChange: (f: Filter) => void;
}

const FILTERS: { id: Filter; label: string; icon: JSX.Element }[] = [
  { id: "all", label: "All", icon: <TextIcon /> },
  { id: "pinned", label: "Pinned", icon: <StarIcon /> },
  { id: "text", label: "Text", icon: <TextIcon /> },
  { id: "url", label: "Links", icon: <UrlIcon /> },
  { id: "repo", label: "Repos", icon: <RepoIcon /> },
  { id: "code", label: "Code", icon: <CodeIcon /> },
  { id: "image", label: "Images", icon: <ImageIcon /> },
  { id: "color", label: "Colors", icon: <ColorIcon /> },
  { id: "music", label: "Music", icon: <MusicIcon /> },
  { id: "video", label: "Video", icon: <VideoIcon /> },
  { id: "pdf", label: "PDFs", icon: <PdfIcon /> },
  { id: "email", label: "Emails", icon: <EmailIcon /> },
  { id: "file", label: "Files", icon: <FileIcon /> },
];

export function Chips({ filter, counts, onChange }: Props) {
  return (
    <div className="chips">
      {FILTERS.map(f => {
        const n = counts[f.id] ?? 0;
        if (f.id !== "all" && f.id !== "pinned" && n === 0) return null;
        return (
          <button
            key={f.id}
            className={`chip${filter === f.id ? " active" : ""}`}
            onClick={() => onChange(f.id)}
          >
            {f.icon}
            <span>{f.label}</span>
            <span className="count">{n}</span>
          </button>
        );
      })}
    </div>
  );
}

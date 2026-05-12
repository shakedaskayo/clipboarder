export type Kind =
  | "text"
  | "url"
  | "email"
  | "code"
  | "color"
  | "image"
  | "file"
  | "pdf"
  | "music"
  | "video"
  | "repo";

export interface UrlMetadata {
  url: string;
  final_url: string | null;
  title: string | null;
  description: string | null;
  image: string | null;
  site_name: string | null;
  icon: string | null;
  fetched_at: number;
  error: string | null;
}

export interface ClipItem {
  id: number;
  kind: Kind;
  // For text-ish kinds, the full content. For images: empty (use image_path).
  content: string;
  // Truncated single-line preview for the list.
  preview: string;
  // Optional source app display name (localized).
  source_app: string | null;
  // Optional source app bundle identifier ("com.apple.Safari").
  source_app_id: string | null;
  // Detected language (for code) or color format ("hex" | "rgb" | "hsl").
  meta: string | null;
  // Path to PNG on disk (for kind = image).
  image_path: string | null;
  pinned: boolean;
  // Byte length of content (for text) or image bytes.
  size: number;
  // Unix ms.
  created_at: number;
  last_used_at: number;
}

export interface QueryArgs {
  query: string;
  kind: Kind | "all" | "pinned";
  limit: number;
}

export interface Settings {
  hotkey: string;
  launch_at_login: boolean;
  max_items: number;
  auto_clear_days: number;
  excluded_apps: string[];
}

export interface FrontmostApp {
  bundle_id: string;
  name: string;
}

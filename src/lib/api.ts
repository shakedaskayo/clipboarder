import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { ClipItem, FrontmostApp, QueryArgs, Settings, UrlMetadata } from "./types";

export async function searchItems(args: QueryArgs): Promise<ClipItem[]> {
  return await invoke<ClipItem[]>("search_items", { args });
}

export async function pasteItem(id: number): Promise<void> {
  await invoke("paste_item", { id });
}

export async function copyItem(id: number): Promise<void> {
  await invoke("copy_item", { id });
}

export async function togglePin(id: number): Promise<boolean> {
  return await invoke<boolean>("toggle_pin", { id });
}

export async function deleteItem(id: number): Promise<void> {
  await invoke("delete_item", { id });
}

export async function clearHistory(): Promise<void> {
  await invoke("clear_history");
}

export async function hideWindow(): Promise<void> {
  await invoke("hide_window");
}

export function imageUrl(path: string): string {
  return convertFileSrc(path);
}

export async function getSettings(): Promise<Settings> {
  return await invoke<Settings>("get_settings");
}

export async function saveSettings(settings: Settings): Promise<Settings> {
  return await invoke<Settings>("save_settings", { settings });
}

export async function frontmostAppInfo(): Promise<FrontmostApp | null> {
  const tuple = await invoke<[string, string] | null>("frontmost_app_info");
  if (!tuple) return null;
  return { bundle_id: tuple[0], name: tuple[1] };
}

export async function getAppIconPath(bundleId: string): Promise<string | null> {
  return await invoke<string | null>("get_app_icon", { bundleId });
}

export async function fetchFileBytes(path: string): Promise<ArrayBuffer> {
  const arr = await invoke<number[]>("fetch_file_bytes", { path });
  return new Uint8Array(arr).buffer;
}

export async function openUrl(url: string): Promise<void> {
  await invoke("open_url", { url });
}

export async function fetchUrlMetadata(
  url: string,
  options?: { refresh?: boolean },
): Promise<UrlMetadata> {
  return await invoke<UrlMetadata>("fetch_url_metadata", {
    url,
    refresh: options?.refresh ?? false,
  });
}

export async function accessibilityTrusted(): Promise<boolean> {
  return await invoke<boolean>("accessibility_trusted");
}

export async function openAccessibilitySettings(): Promise<void> {
  await invoke("open_accessibility_settings");
}

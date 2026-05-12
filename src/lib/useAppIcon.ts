import { useEffect, useState } from "react";
import { getAppIconPath, imageUrl } from "./api";

// Process-wide cache so multiple rows for the same bundle id share one fetch.
const cache = new Map<string, string | null>();
const inflight = new Map<string, Promise<string | null>>();

async function load(bundleId: string): Promise<string | null> {
  if (cache.has(bundleId)) return cache.get(bundleId) ?? null;
  let p = inflight.get(bundleId);
  if (!p) {
    p = getAppIconPath(bundleId).then(path => {
      const url = path ? imageUrl(path) : null;
      cache.set(bundleId, url);
      inflight.delete(bundleId);
      return url;
    }).catch(() => {
      cache.set(bundleId, null);
      inflight.delete(bundleId);
      return null;
    });
    inflight.set(bundleId, p);
  }
  return p;
}

export function useAppIcon(bundleId: string | null | undefined): string | null {
  const [url, setUrl] = useState<string | null>(() =>
    bundleId ? cache.get(bundleId) ?? null : null,
  );
  useEffect(() => {
    if (!bundleId) { setUrl(null); return; }
    const cached = cache.get(bundleId);
    if (cached !== undefined) { setUrl(cached); return; }
    let cancelled = false;
    load(bundleId).then(v => { if (!cancelled) setUrl(v); });
    return () => { cancelled = true; };
  }, [bundleId]);
  return url;
}

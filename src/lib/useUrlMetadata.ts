import { useEffect, useRef, useState } from "react";
import type { UrlMetadata } from "./types";
import { fetchUrlMetadata } from "./api";

const cache = new Map<string, UrlMetadata>();
const inflight = new Map<string, Promise<UrlMetadata>>();

async function load(url: string, refresh = false): Promise<UrlMetadata> {
  if (!refresh && cache.has(url)) return cache.get(url)!;
  let p = inflight.get(url);
  if (!p) {
    p = fetchUrlMetadata(url, { refresh }).then(meta => {
      cache.set(url, meta);
      inflight.delete(url);
      return meta;
    }).catch(err => {
      inflight.delete(url);
      throw err;
    });
    inflight.set(url, p);
  }
  return p;
}

export function useUrlMetadata(url: string | null) {
  const [meta, setMeta] = useState<UrlMetadata | null>(() =>
    url ? cache.get(url) ?? null : null,
  );
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestId = useRef(0);

  useEffect(() => {
    if (!url) { setMeta(null); setLoading(false); setError(null); return; }
    const cached = cache.get(url);
    if (cached) {
      setMeta(cached);
      setLoading(false);
      setError(cached.error);
      return;
    }
    const id = ++requestId.current;
    setLoading(true);
    setError(null);
    load(url).then(m => {
      if (requestId.current !== id) return;
      setMeta(m);
      setLoading(false);
      setError(m.error);
    }).catch(e => {
      if (requestId.current !== id) return;
      setLoading(false);
      setError(String(e));
    });
  }, [url]);

  async function refresh() {
    if (!url) return;
    cache.delete(url);
    setLoading(true);
    setError(null);
    try {
      const m = await load(url, true);
      setMeta(m);
      setError(m.error);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return { meta, loading, error, refresh };
}

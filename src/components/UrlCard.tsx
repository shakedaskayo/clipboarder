import { useUrlMetadata } from "../lib/useUrlMetadata";
import { openUrl } from "../lib/api";
import { ExternalIcon, RefreshIcon, UrlIcon } from "../lib/icons";

interface Props {
  url: string;
}

export function UrlCard({ url }: Props) {
  const { meta, loading, refresh } = useUrlMetadata(url);

  let hostname = "";
  let path = "";
  try {
    const u = new URL(url);
    hostname = u.host;
    path = u.pathname + u.search + u.hash;
  } catch {
    hostname = url;
  }

  const title = meta?.title?.trim();
  const description = meta?.description?.trim();
  const image = meta?.image;
  const siteName = meta?.site_name?.trim();
  const icon = meta?.icon;

  return (
    <div className="url-card">
      {image && (
        <div className="url-hero">
          <img src={image} alt="" loading="lazy" onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
        </div>
      )}
      <div className="url-card-body">
        <div className="url-card-head">
          <div className="url-favicon">
            {icon ? (
              <img src={icon} alt="" onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
            ) : (
              <UrlIcon />
            )}
          </div>
          <div className="url-host-stack">
            <div className="url-site">{siteName || hostname}</div>
            {siteName && <div className="url-host-faint">{hostname}</div>}
          </div>
          <button
            className="icon-btn"
            title="Refetch preview"
            onClick={refresh}
            disabled={loading}
          >
            <RefreshIcon />
          </button>
        </div>

        {title ? (
          <h2 className="url-title">{title}</h2>
        ) : loading ? (
          <div className="url-title-skeleton" />
        ) : null}

        {description && (
          <p className="url-description">{description}</p>
        )}

        <div className="url-path">{path || "/"}</div>

        <div className="url-actions">
          <button
            className="btn btn-primary"
            onClick={() => openUrl(url)}
          >
            <ExternalIcon /> Open in browser
          </button>
        </div>

        {!meta && !loading && (
          <p className="url-hint">Preview not loaded yet — click Refetch.</p>
        )}
      </div>
    </div>
  );
}

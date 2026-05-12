import { useUrlMetadata } from "../lib/useUrlMetadata";
import { openUrl } from "../lib/api";
import { parseRepoUrl, platformDisplayName, resourceLabel } from "../lib/repo";
import { ExternalIcon, RefreshIcon, RepoIcon } from "../lib/icons";

interface Props {
  url: string;
  platform: string;
}

export function RepoCard({ url, platform }: Props) {
  const parsed = parseRepoUrl(url, platform);
  const { meta, loading, refresh } = useUrlMetadata(url);

  if (!parsed) {
    return null;
  }

  const { owner, repo, resource } = parsed;
  const isGist = platform === "gist";
  const title = meta?.title?.trim();
  const description = meta?.description?.trim();
  const image = meta?.image;
  const icon = meta?.icon;

  return (
    <div className={`repo-card platform-${platform}`}>
      {image && (
        <div className="url-hero">
          <img src={image} alt="" loading="lazy" onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
        </div>
      )}
      <div className="repo-card-body">
        <div className="repo-card-head">
          <div className="url-favicon">
            {icon ? (
              <img src={icon} alt="" onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
            ) : (
              <RepoIcon />
            )}
          </div>
          <div className="repo-platform">{platformDisplayName(platform)}</div>
          <button
            className="icon-btn"
            title="Refetch preview"
            onClick={refresh}
            disabled={loading}
          >
            <RefreshIcon />
          </button>
        </div>

        <div className="repo-id">
          {isGist ? (
            <span className="repo-owner">{owner}</span>
          ) : (
            <>
              <span className="repo-owner">{owner}</span>
              <span className="repo-sep">/</span>
              <span className="repo-name">{repo}</span>
            </>
          )}
        </div>

        <div className="repo-resource-pill">{resourceLabel(resource)}</div>

        {title && title !== `${owner}/${repo}` && (
          <h3 className="url-title repo-title">{title}</h3>
        )}

        {description && (
          <p className="url-description">{description}</p>
        )}

        <div className="url-actions">
          <button className="btn btn-primary" onClick={() => openUrl(url)}>
            <ExternalIcon /> Open on {platformDisplayName(platform)}
          </button>
        </div>
      </div>
    </div>
  );
}

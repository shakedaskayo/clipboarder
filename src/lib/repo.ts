// Parse a GitHub-family URL into owner/repo + the kind of resource it points at.

export type RepoResource =
  | { kind: "repo" }
  | { kind: "pull"; number: string }
  | { kind: "issue"; number: string }
  | { kind: "discussion"; number: string }
  | { kind: "commit"; sha: string }
  | { kind: "release"; tag: string }
  | { kind: "blob"; ref: string; path: string }
  | { kind: "tree"; ref: string; path: string }
  | { kind: "actions" }
  | { kind: "wiki" }
  | { kind: "other" };

export interface ParsedRepo {
  platform: string;          // "github" | "gitlab" | "bitbucket" | "codeberg" | "gist"
  owner: string;
  repo: string;
  resource: RepoResource;
  href: string;
}

export function parseRepoUrl(rawUrl: string, platform: string): ParsedRepo | null {
  try {
    const u = new URL(rawUrl);
    const parts = u.pathname.split("/").filter(Boolean);
    if (platform === "gist") {
      const [owner, id] = parts;
      return {
        platform,
        owner: owner ?? "",
        repo: id ?? "",
        resource: { kind: "other" },
        href: rawUrl,
      };
    }
    if (parts.length < 2) return null;
    const [owner, repo, ...rest] = parts;
    let resource: RepoResource = { kind: "repo" };
    if (rest.length > 0) {
      const head = rest[0].toLowerCase();
      if (head === "pull" || head === "pulls" || head === "-/merge_requests" || head === "merge_requests") {
        resource = { kind: "pull", number: rest[1] ?? "" };
      } else if (head === "issues") {
        resource = { kind: "issue", number: rest[1] ?? "" };
      } else if (head === "discussions") {
        resource = { kind: "discussion", number: rest[1] ?? "" };
      } else if (head === "commit") {
        resource = { kind: "commit", sha: rest[1] ?? "" };
      } else if (head === "releases" && rest[1] === "tag") {
        resource = { kind: "release", tag: rest[2] ?? "" };
      } else if (head === "blob" && rest.length >= 3) {
        resource = { kind: "blob", ref: rest[1], path: rest.slice(2).join("/") };
      } else if (head === "tree" && rest.length >= 2) {
        resource = { kind: "tree", ref: rest[1], path: rest.slice(2).join("/") };
      } else if (head === "actions") {
        resource = { kind: "actions" };
      } else if (head === "wiki") {
        resource = { kind: "wiki" };
      } else {
        resource = { kind: "other" };
      }
    }
    return { platform, owner, repo, resource, href: rawUrl };
  } catch {
    return null;
  }
}

export function resourceLabel(resource: RepoResource): string {
  switch (resource.kind) {
    case "repo": return "Repository";
    case "pull": return `Pull request #${resource.number}`;
    case "issue": return `Issue #${resource.number}`;
    case "discussion": return `Discussion #${resource.number}`;
    case "commit": return `Commit ${resource.sha.slice(0, 7)}`;
    case "release": return `Release ${resource.tag}`;
    case "blob": return `File · ${resource.path.split("/").slice(-1)[0]}`;
    case "tree": return `Folder · ${resource.path || resource.ref}`;
    case "actions": return "Actions";
    case "wiki": return "Wiki";
    default: return "Link";
  }
}

export function platformDisplayName(platform: string): string {
  switch (platform) {
    case "github": return "GitHub";
    case "gitlab": return "GitLab";
    case "bitbucket": return "Bitbucket";
    case "codeberg": return "Codeberg";
    case "gist": return "GitHub Gist";
    default: return platform;
  }
}

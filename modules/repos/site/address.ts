// Where Code's address points, both ways: what a link or a reload lands on,
// and what the address says once the cursor moves, so the two always agree.
// A repository is home at #repo=<owner/name> and a tree at #tree=<its path>,
// as the sites' homes say; #every and #growing are the list's places. Where
// another site is home to a thing comes from the sites' record, by the kinds
// each row claims.

import { isObject, text } from "../../local-sites/kit/records.ts";
import { type Grove, type Repository, type Season, treeAddress } from "./model.ts";

/** The list's places: every repository, or those growing today. */
type Place = "every" | "growing";

/** Where an address lands: a place, a repository, or a tree inside one. */
type Landing =
  | { readonly kind: "place"; readonly place: Place }
  | { readonly kind: "repository"; readonly key: string }
  | { readonly kind: "tree"; readonly key: string; readonly path: string; readonly season: Season };

const GITHUB_KEY = /^github:(?<place>.+)$/u;
// Only the first = parts the kind from its value, since a folder may hold one too.
const SAID = /^#(?<kind>[^=]*)=(?<raw>.*)$/su;

/** Where `hash` lands in `grove`, or nowhere when it names nothing there. */
function landing(hash: string, grove: Grove): Landing | undefined {
  if (hash === "#every" || hash === "#growing") {
    return { kind: "place", place: hash === "#growing" ? "growing" : "every" };
  }
  const [, kind, raw] = SAID.exec(hash) ?? [];
  const said = decoded(raw);
  if (said === undefined) {
    return undefined;
  }
  if (kind === "repo") {
    // Said again as the page says it, so an address typed with a bare slash lands too.
    const address = `#repo=${encodeURIComponent(said)}`;
    const found = grove.repositories.find((each) => each.address === address);
    return found === undefined ? undefined : { kind: "repository", key: found.key };
  }
  if (kind !== "tree") {
    return undefined;
  }
  const path = said;
  for (const repository of grove.repositories) {
    const tree = repository.trees.find((each) => each.path === path);
    if (tree !== undefined) {
      return { kind: "tree", key: repository.key, path, season: tree.season };
    }
  }
  return undefined;
}

/** `raw` decoded, or nothing for an escape that's broken, which names nowhere. */
function decoded(raw: string | undefined): string | undefined {
  try {
    return raw === undefined ? undefined : decodeURIComponent(raw);
  } catch {
    return undefined;
  }
}

/** The address of the cursor: the tree it's on inside `open`, else `open` itself. */
function addressed(open: Repository, inner: string | null): string {
  const tree = inner === null ? undefined : open.trees.find((each) => each.path === inner);
  return tree === undefined ? open.address : treeAddress(tree.path);
}

/** Where `o` goes: the tree's change, else the repository where GitHub keeps it. */
function outward(open: Repository, inner: string | null): string | undefined {
  const change = open.trees.find((each) => each.path === inner)?.change;
  const github = GITHUB_KEY.exec(open.key)?.groups?.["place"];
  return change?.url ?? (github === undefined ? undefined : `https://github.com/${github}`);
}

/**
 * Whether `path` is a folder among the sites', as the system daemon's homes take one: not
 * rooted and never stepping up or in place, so a row can't send a fetch outside them, and
 * ending in a slash, since Code joins the homes' file onto it.
 */
function among(path: string): boolean {
  return (
    path.endsWith("/") &&
    !path.startsWith("/") &&
    path.split("/").every((step) => step !== ".." && step !== ".")
  );
}

/**
 * The path of the site home to `kind`, as `sites`, the sites' record, says by the row that
 * claims it: a row is known by its path, since its name is the words people read.
 */
function homeSiteOf(sites: unknown, kind: string): string | undefined {
  const rows = isObject(sites) ? sites["sites"] : undefined;
  if (!Array.isArray(rows)) {
    return undefined;
  }
  const row = rows.find(
    (each: unknown) =>
      isObject(each) && Array.isArray(each["homes"]) && each["homes"].includes(kind),
  );
  const path = isObject(row) ? text(row, "path") : undefined;
  return path !== undefined && among(path) ? path : undefined;
}

/** Where each thing of `kind` is home, by its key, as a site's `homes` lists them. */
function homesOf(homes: unknown, kind: string): ReadonlyMap<string, string> {
  const things = isObject(homes) && Array.isArray(homes["things"]) ? homes["things"] : [];
  return new Map(
    things.flatMap((each: unknown) => {
      if (!isObject(each) || text(each, "kind") !== kind) {
        return [];
      }
      const key = text(each, "key");
      const address = text(each, "address");
      return key === undefined || address === undefined ? [] : [[key, address] as const];
    }),
  );
}

export { addressed, homeSiteOf, homesOf, type Landing, landing, outward, type Place };

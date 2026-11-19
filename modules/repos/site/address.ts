// Where Code's address points, both ways: what a link or a reload lands on,
// and what the address says once the cursor moves, so the two always agree.
// A repository is home at #repo=<owner/name> and a tree at #tree=<its path>,
// as the sites' homes say; #every and #growing are the list's places. Where
// another site is home to a thing is homes.ts's.

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

/** What the page draws now that a reload's room depends on. */
interface Drawn {
  readonly query: string;
  readonly place: Place;
  /** The repository open in the drawer. */
  readonly cursor: string | null;
  /** Each repository's seasons unfolded, beyond those open from the start. */
  readonly unfolded: ReadonlyMap<string, ReadonlySet<Season>>;
}

/**
 * The parts a reload at `hash` won't draw as they're drawn now, so they keep no room: the list,
 * when it was narrowed by a name found or by a place the address doesn't name, and the drawer,
 * when it shows a folding season's trees that the address doesn't open again by naming one.
 */
function forgotten(hash: string, grove: Grove | undefined, drawn: Drawn): ("list" | "open")[] {
  const said = grove === undefined ? undefined : landing(hash, grove);
  const place = said?.kind === "place" ? said.place : "every";
  const reopens = said?.kind === "tree" && said.key === drawn.cursor ? said.season : undefined;
  const list = drawn.query.trim() !== "" || drawn.place !== place;
  const open = grove?.repositories.find((each) => each.key === drawn.cursor);
  // Only resting and gone fold, and unfolding one with no trees draws nothing more.
  const shows = (season: Season): boolean =>
    (season === "resting" || season === "gone") &&
    season !== reopens &&
    open?.trees.some((each) => each.season === season) === true;
  const drawer = [...(drawn.unfolded.get(drawn.cursor ?? "") ?? [])].some(shows);
  return [...(list ? (["list"] as const) : []), ...(drawer ? (["open"] as const) : [])];
}

export { addressed, type Drawn, forgotten, type Landing, landing, outward, type Place };

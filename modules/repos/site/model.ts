// What Code makes of the repos daemon's records: every repository on the
// machine as one tree of the grove, its own trees (git's) grouped by when
// each was last written, its changes with their checks and review, and the
// cells at work in it, from the mycelium's map. Nothing here touches the
// page, so each reading is tried in Node (tests/model.test.ts).

import {
  type Fields,
  type Held,
  isObject,
  number,
  object,
  text,
  texts,
} from "../../local-sites/kit/records.ts";

/** When a tree was last written, as the grove draws it: in leaf, budding, bare or a stump. */
type Season = "today" | "week" | "resting" | "gone";
type ChecksState = "passing" | "failing" | "pending" | "none";
type ReviewState = "approved" | "changes_asked" | "waiting" | "none";

/** A cell at work on the machine, as the mycelium's map says it. */
interface Cell {
  readonly name: string;
  readonly title: string;
  readonly standing: string;
  readonly cwd: string;
  readonly networks: readonly string[];
}

/** Work put up for review, whichever source holds it. */
interface Change {
  readonly key: string;
  readonly title: string;
  readonly branch: string;
  readonly draft: boolean;
  readonly number: number | undefined;
  readonly url: string | undefined;
  readonly checks: ChecksState;
  readonly failed: readonly string[];
  readonly review: ReviewState;
}

/** A folder holding a repository's files to work in. */
interface Tree {
  readonly path: string;
  readonly snapshot: boolean;
  readonly branch: string | undefined;
  readonly commit: string | undefined;
  readonly writtenAt: number | undefined;
  readonly season: Season;
  readonly change: Change | undefined;
  readonly cells: readonly Cell[];
}

/** A repository's whole copy on the machine, which trees are cut from. */
interface Checkout {
  readonly path: string;
  readonly bare: boolean;
  readonly branch: string | undefined;
  readonly behind: number | undefined;
  readonly ahead: number | undefined;
  readonly cells: readonly Cell[];
}

/** One history of code, listed once, however many copies the machine holds. */
interface Repository {
  readonly key: string;
  readonly name: string;
  readonly owner: string | undefined;
  readonly trunk: string | undefined;
  readonly writtenAt: number | undefined;
  readonly checkouts: readonly Checkout[];
  /** Its trees by season, today's first and the gone last, and by branch within one. */
  readonly trees: readonly Tree[];
  /** The folder most of its trees are cut in, when two or more share one. */
  readonly cutIn: string | undefined;
  readonly changes: readonly Change[];
  /** How many of its trees fall in each season. */
  readonly seasons: { readonly [season in Season]: number };
  /** Every cell at work in it, in a checkout or a tree. */
  readonly cells: readonly Cell[];
  /** The repositories one of its cells works across too, by key. */
  readonly joined: readonly string[];
  /** Where it's home, as the sites' convention addresses it. */
  readonly address: string;
}

/** Something worth a look, as words that follow its repository's name. */
interface Worth {
  readonly weight: number;
  readonly repository: Repository;
  readonly says: string;
}

/** The grove: every repository, and what's worth a look in it, worst first. */
interface Grove {
  readonly machine: string;
  readonly roots: readonly string[];
  readonly unread: readonly string[];
  /** Every repository, by name, so each keeps its place between readings. */
  readonly repositories: readonly Repository[];
  readonly worth: readonly Worth[];
}

const DAY = 24 * 60 * 60 * 1000;
// A tree is resting once it has gone a week unwritten, and a repository
// resting that many of them is worth a look: they hold the disk and clutter
// every list of its trees.
const WEEK = 7 * DAY;
const RESTING_PILE = 10;
const WORTH_SHOWN = 5;
// The cells' standings that wait on Joel.
const WAITING = new Set(["needs-you", "stuck"]);

/** `at` read as milliseconds since the epoch, when it's a time. */
function when(at: string | undefined): number | undefined {
  if (at === undefined) {
    return undefined;
  }
  const ms = Date.parse(at);
  return Number.isNaN(ms) ? undefined : ms;
}

/** The season a tree written at `at` is in, `now` being the reader's moment. */
function seasonOf(present: boolean, at: number | undefined, now: number): Season {
  if (!present) {
    return "gone";
  }
  if (at === undefined || now - at >= WEEK) {
    return "resting";
  }
  return new Date(at).toDateString() === new Date(now).toDateString() ? "today" : "week";
}

/** `count` of `one`, with its plural when it isn't one. */
function plural(count: number, one: string, many = `${one}s`): string {
  return `${count.toLocaleString("en-CA")} ${count === 1 ? one : many}`;
}

/** The cells the mycelium's map holds, from its now.json, as far as each reads. */
function cellsOf(map: unknown): readonly Cell[] {
  const cells = isObject(map) ? map["cells"] : undefined;
  if (!Array.isArray(cells)) {
    return [];
  }
  return cells.flatMap((each: unknown) => {
    if (!isObject(each)) {
      return [];
    }
    const cwd = text(each, "cwd");
    const name = text(each, "name");
    if (cwd === undefined || name === undefined) {
      return [];
    }
    return [
      {
        name,
        cwd,
        title: text(each, "title") ?? "",
        standing: text(each, "standing") ?? "quiet",
        networks: texts(each, "networks") ?? [],
      },
    ];
  });
}

/** Whether `cwd` is the folder `path` or inside it. */
function within(cwd: string, path: string): boolean {
  return cwd === path || cwd.startsWith(`${path}/`);
}

/** Each cell given to the deepest of `folders` it works in, by folder. */
function placed(cells: readonly Cell[], folders: readonly string[]): Map<string, Cell[]> {
  const deepestFirst = [...folders].sort((one, other) => other.length - one.length);
  const at = new Map<string, Cell[]>();
  for (const cell of cells) {
    const folder = deepestFirst.find((path) => within(cell.cwd, path));
    if (folder !== undefined) {
      at.set(folder, [...(at.get(folder) ?? []), cell]);
    }
  }
  return at;
}

const CHECKS: readonly ChecksState[] = ["passing", "failing", "pending", "none"];
const REVIEWS: readonly ReviewState[] = ["approved", "changes_asked", "waiting", "none"];

/** One of `allowed`, or the last of them when `said` isn't one. */
function oneOf<T extends string>(said: string | undefined, allowed: readonly T[]): T {
  return allowed.find((each) => each === said) ?? (allowed.at(-1) as T);
}

function changeOf(record: Fields & { readonly key: string }): Change {
  const checks = object(record, "checks") ?? {};
  const github = object(record, "github");
  return {
    key: record.key,
    title: text(record, "title") ?? "",
    branch: text(record, "branch") ?? "",
    draft: record["draft"] === true,
    number: github === undefined ? undefined : number(github, "number"),
    url: github === undefined ? undefined : text(github, "url"),
    checks: oneOf(text(checks, "state"), CHECKS),
    failed: texts(checks, "failed") ?? [],
    review: oneOf(text(record, "review"), REVIEWS),
  };
}

/** The key a repository is addressed by: its owner and name, or its name alone. */
function addressOf(name: string, owner: string | undefined): string {
  return `#repo=${encodeURIComponent(owner === undefined ? name : `${owner}/${name}`)}`;
}

/** Where the tree at `path` is home. */
function treeAddress(path: string): string {
  return `#tree=${encodeURIComponent(path)}`;
}

/**
 * The folder most of `trees` are cut in, so the drawer can say it once and each tree only its
 * own name; unnamed when no two share one, since saying it apart would save nothing.
 */
function cutIn(trees: readonly Tree[]): string | undefined {
  const counts = new Map<string, number>();
  for (const tree of trees) {
    const parent = tree.path.slice(0, tree.path.lastIndexOf("/"));
    counts.set(parent, (counts.get(parent) ?? 0) + 1);
  }
  const [folder, count] = [...counts].reduce<[string | undefined, number]>(
    (most, each) => (each[1] > most[1] ? each : most),
    [undefined, 0],
  );
  return count >= 2 ? folder : undefined;
}

/** The seasons in the order a repository's trees are drawn. */
const SEASONS: readonly Season[] = ["today", "week", "resting", "gone"];

// A cell's network for a repository is named `repo.<its name>`.
const REPOSITORY_NETWORK = "repo.";
const REPOSITORY_KEY = /^repository:/u;

/** The kinds the daemon tells, each followed by a watch of its own. */
const KINDS = ["reading", "repository", "checkout", "tree", "change"] as const;

/** What each kind's watch holds. */
type Feed = { readonly [kind in (typeof KINDS)[number]]: Held };

/** Where cells work, by folder, and the changes by their keys, as a grove's parts read them. */
interface Parts {
  readonly at: ReadonlyMap<string, readonly Cell[]>;
  readonly changes: ReadonlyMap<string, Change>;
  readonly now: number;
}

/** The repositories named by a cell's networks. */
function networked(cell: Cell): readonly string[] {
  return cell.networks
    .filter((each) => each.startsWith(REPOSITORY_NETWORK))
    .map((each) => each.slice(REPOSITORY_NETWORK.length));
}

/** A checkout from its record, with the cells at work in it. */
function checkoutOf(record: Fields, parts: Parts): Checkout {
  const path = text(record, "path") ?? "";
  const head = object(record, "head");
  const count = head === undefined ? undefined : object(object(head, "upstream") ?? {}, "count");
  return {
    path,
    bare: record["bare"] === true,
    branch: head === undefined ? undefined : text(head, "branch"),
    behind: count === undefined ? undefined : number(count, "behind"),
    ahead: count === undefined ? undefined : number(count, "ahead"),
    cells: parts.at.get(path) ?? [],
  };
}

/** A tree from its record, with its change and the cells at work in it. */
function treeOf(record: Fields, parts: Parts): Tree {
  const path = text(record, "path") ?? "";
  const head = object(record, "head");
  const writtenAt = when(text(record, "written_at"));
  const change = text(record, "change");
  return {
    path,
    snapshot: record["snapshot"] === true,
    branch: head === undefined ? undefined : text(head, "branch"),
    commit: head === undefined ? undefined : text(head, "commit"),
    writtenAt,
    season: seasonOf(record["present"] === true, writtenAt, parts.now),
    change: change === undefined ? undefined : parts.changes.get(change),
    cells: parts.at.get(path) ?? [],
  };
}

/** By season, then by branch, so a tree keeps its place under the cursor while cells write in
 * it, and moves only when it changes season. */
function inPlace(one: Tree, other: Tree): number {
  return (
    SEASONS.indexOf(one.season) - SEASONS.indexOf(other.season) ||
    (one.branch ?? one.path).localeCompare(other.branch ?? other.path, "en-CA")
  );
}

/** A repository from its record, with its own of the checkouts, trees and changes. */
function repositoryOf(
  record: Fields & { readonly key: string },
  feed: Feed,
  parts: Parts,
): Repository {
  const key = record.key.replace(REPOSITORY_KEY, "");
  const mine = (each: Fields): boolean => text(each, "repository") === key;
  const checkouts = feed.checkout
    .all()
    .filter(mine)
    .map((each) => checkoutOf(each, parts));
  const trees = feed.tree
    .all()
    .filter(mine)
    .map((each) => treeOf(each, parts))
    .sort(inPlace);
  const seasons = { today: 0, week: 0, resting: 0, gone: 0 };
  for (const tree of trees) {
    seasons[tree.season] += 1;
  }
  const name = text(record, "name") ?? key;
  const owner = text(record, "owner");
  return {
    key,
    name,
    owner,
    trunk: text(record, "trunk"),
    writtenAt: when(text(record, "written_at")),
    checkouts,
    trees,
    cutIn: cutIn(trees),
    changes: [...parts.changes.values()].filter(
      (change) =>
        change.key.startsWith(`change:${key}#`) || change.key.startsWith(`change:${key}@`),
    ),
    seasons,
    cells: [...checkouts.flatMap((each) => each.cells), ...trees.flatMap((each) => each.cells)],
    joined: [],
    address: addressOf(name, owner),
  };
}

/**
 * The grove the daemon's records make, with `cells` from the mycelium's map
 * placed in the trees and checkouts they work in, at `now`. Undefined until
 * the daemon has said its reading.
 */
function groveOf(feed: Feed, cells: readonly Cell[], now: number): Grove | undefined {
  const reading = feed.reading.get("reading");
  if (reading === undefined) {
    return undefined;
  }
  const at = placed(cells, [
    ...feed.checkout.all().flatMap((each) => text(each, "path") ?? []),
    ...feed.tree
      .all()
      .flatMap((each) => (each["present"] === true ? (text(each, "path") ?? []) : [])),
  ]);
  const changes = new Map(feed.change.all().map((each) => [each.key, changeOf(each)]));
  const parts = { at, changes, now };
  const repositories = feed.repository.all().map((record) => repositoryOf(record, feed, parts));
  const joinedUp = joined(repositories);
  const grove = joinedUp.sort((one, other) => one.name.localeCompare(other.name, "en-CA"));
  return {
    machine: text(reading, "machine") ?? "this machine",
    roots: texts(reading, "roots") ?? [],
    unread: unreadOf(reading),
    repositories: grove,
    worth: worthOf(grove),
  };
}

/** Each repository with the others its cells' networks name too. */
function joined(repositories: readonly Repository[]): Repository[] {
  const byName = new Map(repositories.map((each) => [each.name, each.key]));
  return repositories.map((repository) => {
    const others = new Set<string>();
    for (const cell of repository.cells) {
      for (const name of networked(cell)) {
        const key = byName.get(name);
        if (key !== undefined && key !== repository.key) {
          others.add(key);
        }
      }
    }
    return { ...repository, joined: [...others].sort((one, other) => one.localeCompare(other)) };
  });
}

/** What the reading's sources couldn't read, each a sentence's part. */
function unreadOf(reading: Fields): readonly string[] {
  const told = reading["sources"];
  const sources = Array.isArray(told) ? told : [];
  return sources.flatMap((source: unknown) => {
    const unread = isObject(source) ? (texts(source, "unread") ?? []) : [];
    if (!isObject(source) || unread.length === 0) {
      return [];
    }
    return [`${text(source, "name") ?? "a source"}: ${unread.join("; ")}`];
  });
}

/** What's worth a look, worst first, at most five: never a total. */
function worthOf(repositories: readonly Repository[]): readonly Worth[] {
  const found: Worth[] = [];
  for (const repository of repositories) {
    const waiting = repository.cells.filter((cell) => WAITING.has(cell.standing)).length;
    if (waiting > 0) {
      found.push({
        weight: 1e6 + waiting,
        repository,
        says: `has ${plural(waiting, "cell")} waiting on you.`,
      });
    }
    const failing = repository.changes.filter((change) => change.checks === "failing").length;
    if (failing > 0) {
      found.push({
        weight: 1e5 + failing,
        repository,
        says: `has ${plural(failing, "change")} whose checks fail.`,
      });
    }
    const asked = repository.changes.filter((change) => change.review === "changes_asked").length;
    if (asked > 0) {
      found.push({
        weight: 1e4 + asked,
        repository,
        says: `has ${plural(asked, "change")} with changes asked for.`,
      });
    }
    const { gone, resting } = repository.seasons;
    if (gone > 0) {
      found.push({
        weight: 1e3 + gone,
        repository,
        says: `still lists ${plural(gone, "tree")} whose folders are gone; pruning its worktrees clears them.`,
      });
    }
    if (resting >= RESTING_PILE) {
      found.push({
        weight: resting,
        repository,
        says: `rests ${plural(resting, "tree")} unwritten for over a week.`,
      });
    }
  }
  return found.sort((one, other) => other.weight - one.weight).slice(0, WORTH_SHOWN);
}

/** The index `by` steps to from `at` among `count`, kept inside them; an infinite step goes
 * to the first or the last. */
function stepped(at: number, by: number, count: number): number {
  if (by === Number.POSITIVE_INFINITY) {
    return count - 1;
  }
  if (by === Number.NEGATIVE_INFINITY) {
    return 0;
  }
  return Math.max(0, Math.min(count - 1, at + by));
}

/** The repositories that grew today: a tree of each written since midnight. */
function growing(grove: Grove): readonly Repository[] {
  return grove.repositories.filter((each) => each.seasons.today > 0);
}

export type {
  Cell,
  Change,
  Checkout,
  ChecksState,
  Grove,
  Repository,
  ReviewState,
  Feed,
  Season,
  Tree,
  Worth,
};
export {
  addressOf,
  cellsOf,
  groveOf,
  growing,
  KINDS,
  plural,
  seasonOf,
  stepped,
  treeAddress,
  when,
};

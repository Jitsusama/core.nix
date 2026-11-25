// Who started each cell: the cell whose ask started it, from the book of
// starts kept by the node that started it, else the orchestrator its
// presence names. Each cell's own record says so on every machine, since
// links carry it; a node too old to say it there says it only in its
// forages (the node's `forage` kind), which never cross a link, so only
// this machine's cells are known that way. From that, which network grows
// out of which, as a hypha branches from another.

import { type Fields, type Held, object, text } from "../../local-sites/kit/records.ts";

/** A cell and the cell that started it, each by its address. */
interface Started {
  readonly cell: string;
  readonly by: string;
}

/** A cell and the cell its purpose split from, each by its address. */
interface Split {
  readonly cell: string;
  readonly from: string;
}

/** A network as the map draws it: where the node sets it, and the cells drawn in it. */
interface Network {
  readonly key: string;
  readonly name: string;
  readonly within: string | null;
  readonly cells: readonly string[];
}

/** Where a cell came from: the cell that started it, or that its purpose split from. */
interface Origin {
  readonly cell: string;
  readonly by: string;
  readonly split?: true;
}

/**
 * What a network grew out of, the cells there that started its own, how
 * many of its cells grew it, and, where any split from one there rather
 * than being said to be started, those cells and how many split.
 */
interface Grown {
  readonly from: string;
  readonly by: readonly string[];
  readonly cells: number;
  readonly split?: { readonly from: readonly string[]; readonly cells: number };
}

const collate = new Intl.Collator("en-CA").compare;

/**
 * Whether `copy` of a cell stands over `had`, the copy held so far, when
 * two machines list one cell while it moves between them: only a copy
 * started strictly later does, a start not said as words reading as none,
 * so of two started alike the copy told first stands. Only oneCopyEach
 * asks it, so that nothing else can choose a copy by a rule of its own;
 * it is exported so its test can pin the tie and the start not as words.
 */
function isLaterCopy(copy: Fields, had: Fields): boolean {
  return (text(copy, "since") ?? "") > (text(had, "since") ?? "");
}

/** The networks `cell` names, none when it names them as anything but a list. */
function networksOf(cell: Fields): readonly unknown[] {
  const networks = cell["networks"];
  return Array.isArray(networks) ? networks : [];
}

/**
 * Each cell of `cells` once, by name, in the order each name was first
 * told: of two copies, the one that stands by isLaterCopy, naming the
 * networks either copy names, since it is one cell under one address and
 * the machine it left may still list groups it heard there. A record with
 * no name is no cell. The drawing, its reach and its starters all take a
 * cell's copy from here, so none can say a copy another doesn't.
 */
function oneCopyEach<Cell extends Fields>(cells: readonly Cell[]): Cell[] {
  const once = new Map<string, Cell>();
  for (const cell of cells) {
    const name = text(cell, "name");
    const had = name ? once.get(name) : undefined;
    if (name && !had) {
      once.set(name, cell);
    } else if (name && had) {
      const [keep, other] = isLaterCopy(cell, had) ? [cell, had] : [had, cell];
      const networks = [...new Set([...networksOf(keep), ...networksOf(other)])];
      once.set(name, { ...keep, networks });
    }
  }
  return [...once.values()];
}

/**
 * Who started each cell: its own record's word first, which reaches every
 * machine, then its forage's for a cell whose record says nothing, as an
 * older node's does. The two never disagree on a node that says both,
 * since it makes them from the same book and presence and leaves the
 * record's out where nothing names a starter, so the forage only fills in
 * for a node too old to say it on the record. A cell said to have started
 * itself names nobody, and a node that says neither reads as one where
 * nobody started anybody, which grows no branch either way. `cells` are
 * the records the map draws as cells, so a record it never draws can't
 * name the starter of one it does.
 */
function startedOf(cells: readonly Fields[], forages: Held | undefined): Started[] {
  const by = new Map<string, string>();
  const say = (cell: string | undefined, starter: string | undefined): void => {
    if (cell && starter && starter !== cell && !by.has(cell)) {
      by.set(cell, starter);
    }
  };
  // A cell moving between machines can be listed by both for a while, the
  // one it left still saying what it last knew, so its starter is the
  // copy's the map draws.
  for (const record of oneCopyEach(cells)) {
    say(text(record, "name"), text(record, "started_by"));
  }
  for (const record of forages?.all() ?? []) {
    say(text(record, "cell"), text(object(record, "lineage") ?? {}, "started_by"));
  }
  return [...by].map(([cell, starter]) => ({ cell, by: starter }));
}

/**
 * What each cell split from, which only its forage says, and so only for
 * this machine's cells: its own split_from, else the first forage here
 * listing it in split_into, as the forage page reads either end. A split
 * naming the cell itself, a blank or anything but words names nothing.
 */
function splitsOf(forages: Held | undefined): Split[] {
  const from = new Map<string, string>();
  const listed = new Map<string, string>();
  for (const record of forages?.all() ?? []) {
    const cell = text(record, "cell");
    if (!cell) {
      continue;
    }
    const lineage = object(record, "lineage") ?? {};
    const own = text(lineage, "split_from");
    if (own && own !== cell && !from.has(cell)) {
      from.set(cell, own);
    }
    const into = lineage["split_into"];
    for (const each of Array.isArray(into) ? into : []) {
      if (typeof each === "string" && each !== "" && each !== cell && !listed.has(each)) {
        listed.set(each, cell);
      }
    }
  }
  for (const [cell, by] of listed) {
    if (!from.has(cell)) {
      from.set(cell, by);
    }
  }
  return [...from].map(([cell, by]) => ({ cell, from: by }));
}

/**
 * Where each cell came from, for growing branches: who started it, else
 * what its purpose split from, one edge a cell. A sprout's start often
 * names the cell it split from, but not always: the node's book lets a
 * start go after a day and finds none where no session, or two, claim the
 * asker, while a split stays said for as long as its forage is.
 */
function originsOf(started: readonly Started[], splits: readonly Split[]): Origin[] {
  const named = new Set(started.map((start) => start.cell));
  const split = splits.filter((each) => !named.has(each.cell));
  const splitOff = split.map((each) => ({ cell: each.cell, by: each.from, split: true as const }));
  return [...started, ...splitOff];
}

/**
 * Every cell a forage is held for, which the forage page lists and so
 * can open; none at all when the node serves no forages, as for starters.
 */
function foragedOf(held: Held | undefined): string[] | null {
  if (held === undefined) {
    return null;
  }
  return held.all().flatMap((record) => text(record, "cell") ?? []);
}

/** The cells of one network that a network's own came from, kept apart by how they came. */
interface Came {
  readonly starters: Set<string>;
  readonly splitters: Set<string>;
  readonly cells: Set<string>;
  readonly split: Set<string>;
}

/** Nothing come from a network yet. */
function came(): Came {
  return { starters: new Set(), splitters: new Set(), cells: new Set(), split: new Set() };
}

/**
 * Which networks grow out of which, from where their cells came from, each
 * drawn network's cells known by `hubOf` (a cell's address to the key of
 * the network it is drawn in, none for a receptor or a cell working
 * alone). A network the node sets within none grows out of the network
 * whose cells started more of its cells than any other, ties going to the
 * first by name. Where the node says a network sits always stands, a start
 * from inside the network or what sits in it counts for nothing, and the
 * strongest starts are settled first, so of two networks that started
 * each other's cells the one that started more is the trunk, and no
 * branch is grown that would loop back round to itself. A split counts as
 * a start does, and is said apart, since nobody is said to have started it.
 */
function grownOf(
  networks: readonly Network[],
  hubOf: ReadonlyMap<string, string>,
  origins: readonly Origin[],
): Map<string, Grown> {
  const named = new Map(networks.map((network) => [network.key, network]));
  const originOf = new Map(origins.map((origin) => [origin.cell, origin]));
  const grown = new Map<string, Grown>();
  const within = (key: string): string | null =>
    named.get(key)?.within ?? grown.get(key)?.from ?? null;
  const sitsIn = (key: string, of: string): boolean => {
    const seen = new Set<string>();
    for (let at: string | null = key; at !== null && !seen.has(at); at = within(at)) {
      if (at === of) {
        return true;
      }
      seen.add(at);
    }
    return false;
  };
  const offered: { readonly key: string; readonly grown: Grown }[] = [];
  for (const network of networks) {
    if (network.within !== null) {
      continue;
    }
    const from = new Map<string, Came>();
    for (const cell of network.cells) {
      const origin = originOf.get(cell);
      const hub = origin === undefined ? undefined : hubOf.get(origin.by);
      if (origin === undefined || hub === undefined) {
        continue;
      }
      if (!named.has(hub) || sitsIn(hub, network.key)) {
        continue;
      }
      const had = from.get(hub) ?? came();
      (origin.split ? had.splitters : had.starters).add(origin.by);
      if (origin.split) {
        had.split.add(cell);
      }
      had.cells.add(cell);
      from.set(hub, had);
    }
    const nameOf = (key: string): string => named.get(key)?.name ?? key;
    const [best] = [...from].sort(
      ([one, ones], [other, others]) =>
        others.cells.size - ones.cells.size || collate(nameOf(one), nameOf(other)),
    );
    if (best) {
      const [key, { starters, splitters, cells, split }] = best;
      const by = [...starters].sort(collate);
      const splits =
        split.size > 0 ? { split: { from: [...splitters].sort(collate), cells: split.size } } : {};
      offered.push({ key: network.key, grown: { from: key, by, cells: cells.size, ...splits } });
    }
  }
  offered.sort((one, other) => other.grown.cells - one.grown.cells || collate(one.key, other.key));
  for (const { key, grown: offer } of offered) {
    if (!sitsIn(offer.from, key)) {
      grown.set(key, offer);
    }
  }
  return grown;
}

export type { Grown, Network, Origin, Split, Started };
export { foragedOf, grownOf, isLaterCopy, oneCopyEach, originsOf, splitsOf, startedOf };

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

/** A network as the map draws it: where the node sets it, and the cells drawn in it. */
interface Network {
  readonly key: string;
  readonly name: string;
  readonly within: string | null;
  readonly cells: readonly string[];
}

/** What a network grew out of, the cells there that started its own, and how many they started. */
interface Grown {
  readonly from: string;
  readonly by: readonly string[];
  readonly cells: number;
}

const collate = new Intl.Collator("en-CA").compare;

/**
 * Whether `copy` of a cell stands over `had`, the copy held so far, when
 * two machines list one cell while it moves between them: only a copy
 * started strictly later does, a start not said as words reading as none,
 * so of two started alike the copy told first stands. The map draws the
 * copy that stands and says who started it from the same one, so both
 * ask this.
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
 * Every cell a forage is held for, which the forage page lists and so
 * can open; none at all when the node serves no forages, as for starters.
 */
function foragedOf(held: Held | undefined): string[] | null {
  if (held === undefined) {
    return null;
  }
  return held.all().flatMap((record) => text(record, "cell") ?? []);
}

/**
 * Which networks grow out of which, from who started their cells, each
 * drawn network's cells known by `hubOf` (a cell's address to the key of
 * the network it is drawn in, none for a receptor or a cell working
 * alone). A network the node sets within none grows out of the network
 * whose cells started more of its cells than any other, ties going to the
 * first by name. Where the node says a network sits always stands, a start
 * from inside the network or what sits in it counts for nothing, and the
 * strongest starts are settled first, so of two networks that started
 * each other's cells the one that started more is the trunk, and no
 * branch is grown that would loop back round to itself.
 */
function grownOf(
  networks: readonly Network[],
  hubOf: ReadonlyMap<string, string>,
  started: readonly Started[],
): Map<string, Grown> {
  const named = new Map(networks.map((network) => [network.key, network]));
  const startedBy = new Map(started.map((start) => [start.cell, start.by]));
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
    const from = new Map<string, { readonly starters: Set<string>; readonly cells: Set<string> }>();
    for (const cell of network.cells) {
      const by = startedBy.get(cell);
      const hub = by === undefined ? undefined : hubOf.get(by);
      if (by === undefined || hub === undefined || !named.has(hub) || sitsIn(hub, network.key)) {
        continue;
      }
      const had = from.get(hub) ?? { starters: new Set<string>(), cells: new Set<string>() };
      had.starters.add(by);
      had.cells.add(cell);
      from.set(hub, had);
    }
    const nameOf = (key: string): string => named.get(key)?.name ?? key;
    const [best] = [...from].sort(
      ([one, ones], [other, others]) =>
        others.cells.size - ones.cells.size || collate(nameOf(one), nameOf(other)),
    );
    if (best) {
      const [key, { starters, cells }] = best;
      const by = [...starters].sort(collate);
      offered.push({ key: network.key, grown: { from: key, by, cells: cells.size } });
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

export type { Grown, Network, Started };
export { foragedOf, grownOf, isLaterCopy, oneCopyEach, startedOf };

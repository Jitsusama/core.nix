// Who started each cell: the cell whose ask started it, from the book of
// starts kept by the node that started it, else the orchestrator its
// presence names. Each cell's own record says so on every machine, since
// links carry it; a node too old to say it there says it only in its
// forages (the node's `forage` kind), which never cross a link, so only
// this machine's cells are known that way. What each cell's purpose split
// from, which only forages say, so only of this machine's cells. From both,
// which network grows out of which, as a hypha branches from another, and
// how the drawers say where a branch and a cell came from.

import { type Fields, type Held, object, text } from "../../local-sites/kit/records.ts";
import { NUMBER, plural } from "./words.ts";

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
 * many of its cells grew it and how many of those were started, and,
 * where any split from one there rather than being said to be started,
 * those cells and how many split. `started` and the split's `cells` add
 * up to `cells`, so a drawer can say each without counting them itself.
 */
interface Grown {
  readonly from: string;
  readonly by: readonly string[];
  readonly cells: number;
  readonly started: number;
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
 * listing it in split_into, as the forage page reads either end. A cell
 * listed is one only with a forage of its own here, so a forage can't say
 * what a cell on another machine, or one it never held, came from. A
 * split naming the cell itself, a blank or anything but words names
 * nothing.
 */
function splitsOf(forages: Held | undefined): Split[] {
  const from = new Map<string, string>();
  const listed = new Map<string, string>();
  const here = new Set<string>();
  for (const record of forages?.all() ?? []) {
    const cell = text(record, "cell");
    if (!cell) {
      continue;
    }
    here.add(cell);
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
    if (here.has(cell) && !from.has(cell)) {
      from.set(cell, by);
    }
  }
  return [...from].map(([cell, by]) => ({ cell, from: by }));
}

/**
 * Where each cell came from, for growing branches: who started it, then
 * what its purpose split from, the starts first, so where a start and a
 * split come from one network the cell counts there once, as started. Each
 * counts toward its own network in grownOf, so a split still grows a
 * branch where the start grows none, as one from a receptor or from inside
 * its own network. A split naming the cell its start names is said once. A
 * sprout's start often names the cell it split from, but not always: the
 * node's book lets a start go after a day and finds none where no
 * session, or two, claim the asker, while a split stays said for as long
 * as its forage is.
 */
function originsOf(started: readonly Started[], splits: readonly Split[]): Origin[] {
  const startOf = new Map(started.map((start) => [start.cell, start.by]));
  const split = splits.filter((each) => startOf.get(each.cell) !== each.from);
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

/** What a network grew out of `key` says, from what came from there. */
function grownFrom(key: string, { starters, splitters, cells, split }: Came): Grown {
  const by = [...starters].sort(collate);
  const splits =
    split.size > 0 ? { split: { from: [...splitters].sort(collate), cells: split.size } } : {};
  return { from: key, by, cells: cells.size, started: cells.size - split.size, ...splits };
}

/**
 * Which networks grow out of which, from where their cells came from, each
 * drawn network's cells known by `hubOf` (a cell's address to the key of
 * the network it is drawn in, none for a receptor or a cell working
 * alone). A network the node sets within none grows out of the network
 * whose cells started or split more of its cells than any other, ties
 * going to the one that started more, then to the first by name. Where the
 * node says a network sits always stands, a start from inside the network or what sits
 * in it counts for nothing, and every network's choices are settled
 * together, strongest first, so of two networks that started each other's
 * cells the one that started more is the trunk, and no branch is grown
 * that would loop back round to itself. A network whose strongest choice
 * would loop grows out of its next when that comes, never ahead of a
 * stronger choice elsewhere. A split counts as a start does, and is said
 * apart, since nobody is said to have started it. Each of a cell's origins
 * counts toward the network it came from, once a network and its start
 * before its split, so a cell started from one network and split from
 * another counts toward both, and where its branch grows from says how it
 * came from there.
 */
function grownOf(
  networks: readonly Network[],
  hubOf: ReadonlyMap<string, string>,
  origins: readonly Origin[],
): Map<string, Grown> {
  const named = new Map(networks.map((network) => [network.key, network]));
  // A cell's start goes before its split however they were handed, so
  // where both come from one network the cell counts there as started.
  const originsOfCell = new Map<string, Origin[]>();
  for (const origin of origins) {
    const had = originsOfCell.get(origin.cell) ?? [];
    originsOfCell.set(origin.cell, origin.split ? [...had, origin] : [origin, ...had]);
  }
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
  const nameOf = (key: string): string => named.get(key)?.name ?? key;
  const cameFrom = (network: Network): Grown[] => {
    const from = new Map<string, Came>();
    for (const cell of network.cells) {
      for (const origin of originsOfCell.get(cell) ?? []) {
        const hub = hubOf.get(origin.by);
        const had = hub === undefined ? undefined : (from.get(hub) ?? came());
        if (hub === undefined || had === undefined || had.cells.has(cell)) {
          continue;
        }
        if (!named.has(hub) || sitsIn(hub, network.key)) {
          continue;
        }
        (origin.split ? had.splitters : had.starters).add(origin.by);
        if (origin.split) {
          had.split.add(cell);
        }
        had.cells.add(cell);
        from.set(hub, had);
      }
    }
    return [...from].map(([key, had]) => grownFrom(key, had));
  };
  const choices = networks
    .filter((network) => network.within === null)
    .flatMap((network) => cameFrom(network).map((choice) => ({ key: network.key, choice })));
  choices.sort(
    (one, other) =>
      other.choice.cells - one.choice.cells ||
      other.choice.started - one.choice.started ||
      collate(one.key, other.key) ||
      collate(nameOf(one.choice.from), nameOf(other.choice.from)),
  );
  for (const { key, choice } of choices) {
    if (!(grown.has(key) || sitsIn(choice.from, key))) {
      grown.set(key, choice);
    }
  }
  return grown;
}

/** How a branch grew, as grownOf counts it: whose cells there started or split its own. */
interface Grew<T> {
  readonly by: readonly T[];
  readonly started: number;
  readonly split: { readonly from: readonly T[]; readonly cells: number } | null;
}

/**
 * What a branch says of the network it grew out of, each a line of its own:
 * the cells there that started its own and how many, then those its own
 * split from and how many, out of its `cells`, since a split is not a
 * start. `who` says a list of cells as the page shows them.
 */
function grewWords<T>(
  grew: Grew<T>,
  cells: number,
  who: (cells: readonly T[]) => string,
): string[] {
  const ofIts = (n: number): string => {
    if (n !== cells) {
      return `${NUMBER.format(n)} of its ${plural(cells, "cell")}`;
    }
    return n === 1 ? "its cell" : `all ${NUMBER.format(n)} of its cells`;
  };
  const lines: string[] = [];
  if (grew.started > 0 && grew.by.length > 0) {
    lines.push(`Started by ${who(grew.by)}: ${ofIts(grew.started)}`);
  }
  if (grew.split && grew.split.cells > 0) {
    lines.push(`Split from ${who(grew.split.from)}: ${ofIts(grew.split.cells)}`);
  }
  return lines;
}

/**
 * The cell a cell's drawer says it split from: none where the split names
 * the cell that started it, since that is said once, as Started, the way a
 * network's drawer and originsOf say it.
 */
function splitSaid(split: string | undefined, start: string | undefined): string | undefined {
  return split && split !== start ? split : undefined;
}

export type { Grew, Grown, Network, Origin, Split, Started };
export {
  foragedOf,
  grewWords,
  grownOf,
  isLaterCopy,
  oneCopyEach,
  originsOf,
  splitSaid,
  splitsOf,
  startedOf,
};

// How forages join one another, read from the node's forage records
// (mycelium's PROTOCOL.md, Forages): the needs one serves and relies on,
// the cell it split from or into, and whether the node's book holds its
// start. A join the page couldn't make sense of is said rather than drawn,
// while the rest still draw.

import type { Fields } from "../../local-sites/kit/records.ts";
import { isObject, text } from "../../local-sites/kit/records.ts";

/**
 * One need joining two forages: the cell at its other end, what the need
 * was about, and since when, as an ISO time, when it says.
 */
interface Join {
  readonly cell: string;
  readonly need: string;
  readonly since: string | undefined;
}

/** What a split says of one cell: the cell it split from, and those it split into. */
interface Split {
  readonly cell: string;
  readonly splitFrom: string | undefined;
  readonly splitInto: readonly string[];
}

/** The join `value` describes for `self`, or none when it names no other cell. */
function joinOf(value: unknown, self: string): Join | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const cell = text(value, "cell");
  if (cell === undefined || cell === "" || cell === self) {
    return undefined;
  }
  return { cell, need: text(value, "need") ?? "", since: text(value, "since") };
}

/**
 * The joins `listed` under `field`, serves or relies, of `cell`, saying
 * any it couldn't read while the rest still draw.
 */
function joinsOf(
  listed: unknown,
  field: "serves" | "relies",
  cell: string,
  unread: string[],
): readonly Join[] {
  // The node sends null, as for lineage's other fields, when a cell has none.
  if (listed === undefined || listed === null) {
    return [];
  }
  if (!Array.isArray(listed)) {
    unread.push(`${cell}'s ${field} listed no needs`);
    return [];
  }
  const read = listed.map((each) => joinOf(each, cell)).filter((each) => each !== undefined);
  if (read.length < listed.length) {
    unread.push(`${listed.length - read.length} of ${cell}'s ${field} named no other cell`);
  }
  return read;
}

/**
 * The cells `lineage` says `cell` split from and into, saying any address
 * it couldn't read while the rest still draw.
 */
function splitOf(lineage: Fields, cell: string, unread: string[]): Omit<Split, "cell"> {
  // The node sends null, as for lineage's other fields, when a cell split from none.
  const from = lineage["split_from"] ?? undefined;
  const splitFrom = typeof from === "string" && from !== "" && from !== cell ? from : undefined;
  if (from !== undefined && splitFrom === undefined) {
    unread.push(`${cell}'s split_from named no other cell`);
  }
  const into = lineage["split_into"] ?? undefined;
  const listed = Array.isArray(into) ? into : [];
  const splitInto = listed.filter(
    (each): each is string => typeof each === "string" && each !== "" && each !== cell,
  );
  if (into !== undefined && !Array.isArray(into)) {
    unread.push(`${cell}'s split_into listed no cells`);
  } else if (splitInto.length < listed.length) {
    unread.push(`${listed.length - splitInto.length} of ${cell}'s split_into named no other cell`);
  }
  return { splitFrom, splitInto };
}

/**
 * `found`, each split read from either end so both say it: a cell's
 * split_into takes in each cell here naming it as split_from, and a cell
 * naming none takes the first cell here listing it in split_into. Where
 * the two ends disagree the split cell's own word stands, so a cell here
 * naming another ancestor, or taken by an earlier lister, drops out of a
 * split_into listing it.
 */
function bothEnds<Each extends Split>(found: readonly Each[]): readonly Each[] {
  const naming = new Map<string, string[]>();
  const listing = new Map<string, string>();
  const named = new Map(found.map((each) => [each.cell, each.splitFrom]));
  for (const each of found) {
    if (each.splitFrom !== undefined) {
      naming.set(each.splitFrom, [...(naming.get(each.splitFrom) ?? []), each.cell]);
    }
    for (const into of each.splitInto) {
      listing.set(into, listing.get(into) ?? each.cell);
    }
  }
  return found.map((each) => {
    // A cell here grows under one ancestor, so only that one says it split into it.
    const ancestor = (cell: string) => named.get(cell) ?? listing.get(cell) ?? each.cell;
    const listed = each.splitInto.filter(
      (cell) => !named.has(cell) || ancestor(cell) === each.cell,
    );
    const into = [...new Set([...listed, ...(naming.get(each.cell) ?? [])])];
    return {
      ...each,
      splitFrom: each.splitFrom ?? listing.get(each.cell),
      splitInto: into.sort((one, other) => one.localeCompare(other)),
    };
  });
}

/**
 * Each pair of cells among `forages` a need joins, once however both ends
 * keep it and however many needs they keep it by, since the floor draws
 * one arc a pair: the cell found first, then the other. A join to a cell
 * not drawn here isn't a pair.
 */
function joined(
  forages: readonly {
    readonly cell: string;
    readonly serves: readonly Join[];
    readonly relies: readonly Join[];
  }[],
): readonly { readonly from: string; readonly to: string }[] {
  const here = new Set(forages.map((each) => each.cell));
  const pairs = new Map<string, { from: string; to: string }>();
  for (const forage of forages) {
    for (const each of [...forage.serves, ...forage.relies]) {
      const [from, to] = [forage.cell, each.cell];
      const key = from < to ? `${from}\n${to}` : `${to}\n${from}`;
      if (from !== to && here.has(to) && !pairs.has(key)) {
        pairs.set(key, { from, to });
      }
    }
  }
  return [...pairs.values()];
}

/** Where the node said starts came from, over every cell, as a node-wide fault is said once. */
interface StartsTold {
  said: boolean;
  readonly odd: string[];
  readonly unsourced: string[];
}

/** Whether the node's book holds `cell`'s start, by `started_by_from`; else it's claimed. */
function bookedOf(lineage: Fields, cell: string, told: StartsTold): boolean {
  const from = lineage["started_by_from"];
  if (from !== undefined && from !== null) {
    told.said = true;
    if (from !== "book" && from !== "presence") {
      told.odd.push(cell);
    }
  } else if (text(lineage, "started_by") !== undefined) {
    told.unsourced.push(cell);
  }
  return from === "book";
}

/** How many cells a fault names before counting the rest, so its line stays short. */
const MOST_NAMED = 6;

/**
 * The cells a fault of several is about, in brackets, so whoever reads it knows where to
 * look; one more is named rather than counted, as "and 1 more" says less than its name.
 */
function which(cells: readonly string[]): string {
  if (cells.length < 2) {
    return "";
  }
  const named = cells.length > MOST_NAMED + 1 ? cells.slice(0, MOST_NAMED) : [...cells];
  const rest = cells.length - named.length;
  const last = rest === 0 ? named.pop() : `${rest} more`;
  return ` (${named.join(", ")} and ${last})`;
}

/**
 * Why starts read as claimed: a source neither word, or a starter with none, said by cell
 * unless no cell's start had one, as a node from before the field, said once for all.
 */
function startsUnread(told: StartsTold): string[] {
  const whose = (cells: string[]) =>
    cells.length === 1 ? `${cells[0]}'s` : `${cells.length} cells'`;
  const unread: string[] = [];
  if (told.odd.length > 0) {
    const odd = `${whose(told.odd)} started_by_from said neither book nor presence`;
    unread.push(odd + which(told.odd));
  }
  if (told.unsourced.length > 0 && told.said) {
    const starts = told.unsourced.length === 1 ? "start" : "starts";
    unread.push(`where ${whose(told.unsourced)} ${starts} came from${which(told.unsourced)}`);
  } else if (told.unsourced.length > 0) {
    unread.push("where any start came from, as the node sends none yet, so each reads as claimed");
  }
  return unread;
}

export type { Join, StartsTold };
export { bookedOf, bothEnds, joined, joinsOf, splitOf, startsUnread };

// What the forage page draws, read from the mycelium node's forage records
// (mycelium's PROTOCOL.md, Forages): one for each cell of this machine that
// works on a quest, keyed forage:<cell>, with the purpose its quest states,
// the documents it keeps for it, what its quest waits on its person
// for, and how it joins other forages: the needs it serves and relies
// on, and the cell it split from or into, read in joins.ts. Nothing here
// touches the page, so every case is proven in node, and a record the
// page couldn't make sense of is said rather than drawn as if it were
// whole.

import type { Fields, Held } from "../../local-sites/kit/records.ts";
import { isObject, number, object, text } from "../../local-sites/kit/records.ts";
import {
  bookedOf,
  bothEnds,
  type Join,
  joinsOf,
  type StartsTold,
  splitOf,
  startsUnread,
} from "./joins.ts";

/** One document a cell keeps for its purpose: a plan, research, a brief or a report. */
interface Document {
  readonly id: string;
  readonly kind: string;
  readonly title: string;
  readonly stage: string;
  readonly ticked: number;
  readonly open: number;
}

/** One thing a cell's quest waits on its person for, as the launcher keeps it. */
interface Wait {
  /** What tells it apart in its quest: its id, or its title when it has none, as the launcher counts it. */
  readonly id: string;
  /** What the person is asked to do, in a line. */
  readonly title: string;
  /** Why it waits on them; empty when it doesn't say. */
  readonly why: string;
  /** About how many minutes of theirs it takes. */
  readonly minutes: number | undefined;
  /** When it's for, in words, such as "today". */
  readonly when: string | undefined;
  /** Whether it can be done now. */
  readonly ready: boolean;
}

/** One cell's forage, as the page draws its row and its panel. */
interface Forage {
  readonly cell: string;
  readonly machine: string;
  readonly quest: string;
  /** The first sentence of its quest's Summary; empty when none is written. */
  readonly purpose: string;
  readonly status: string;
  readonly documents: readonly Document[];
  /** What its quest waits on its person for, oldest first as the node says. */
  readonly waits: readonly Wait[];
  /** The cell that started it, by address, when one did. */
  readonly startedBy: string | undefined;
  /**
   * Whether the node's own start book holds who started it. Otherwise it's
   * the cell's own word, which a cell may claim of any other.
   */
  readonly booked: boolean;
  /** The quest its own sits under, when it sits under one. */
  readonly parent: string | undefined;
  /** The cell whose purpose it took a part of, when it was split from one. */
  readonly splitFrom: string | undefined;
  /** The cells it split its purpose into, by address, as it ended. */
  readonly splitInto: readonly string[];
  /** The needs it took from other cells, and works on for them. */
  readonly serves: readonly Join[];
  /** The needs it handed other cells, which work on them for it. */
  readonly relies: readonly Join[];
  /** How many cells above it it grew from, here, started or split: 0 for none here. */
  readonly depth: number;
}

/** Every forage held, ordered as the cells started each other, and what couldn't be read. */
interface Forages {
  readonly forages: readonly Forage[];
  readonly unread: readonly string[];
}

/** The documents a forage keeps, in the order its kinds are said. */
const KINDS: readonly string[] = ["plan", "research", "brief", "report"];

/** Each kind of document as a count says it, one and many. */
const COUNTED: { readonly [kind: string]: readonly [string, string] } = {
  plan: ["plan", "plans"],
  research: ["research", "research"],
  brief: ["brief", "briefs"],
  report: ["report", "reports"],
};

/** The document `value` describes, or none when it isn't one. */
function documentOf(value: unknown): Document | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const id = text(value, "id");
  if (id === undefined) {
    return undefined;
  }
  return {
    id,
    kind: text(value, "kind") ?? "",
    title: text(value, "title") ?? id,
    stage: text(value, "stage") ?? "",
    ticked: number(value, "ticked") ?? 0,
    open: number(value, "open") ?? 0,
  };
}

/** The documents `record` keeps, in kind order, saying any it couldn't read. */
function documentsOf(record: Fields, cell: string, unread: string[]): readonly Document[] {
  const listed = record["documents"];
  if (!Array.isArray(listed)) {
    return [];
  }
  const read = listed.map(documentOf).filter((each) => each !== undefined);
  if (read.length < listed.length) {
    const skipped = listed.length - read.length;
    unread.push(`${skipped} of ${cell}'s documents had no id`);
  }
  const rank = (kind: string): number => {
    const at = KINDS.indexOf(kind);
    return at === -1 ? KINDS.length : at;
  };
  return [...read].sort((one, other) => rank(one.kind) - rank(other.kind));
}

/**
 * Whether a wait can be done now, as the launcher reads it: as its owner
 * says, or, in a file older than `ready`, when its when is "now".
 */
function readyOf(value: Fields): boolean {
  const ready = value["ready"];
  if (typeof ready === "boolean") {
    return ready;
  }
  return text(value, "when")?.trim().toLowerCase() === "now";
}

/** The wait `value` describes, or none without a title. */
function waitOf(value: unknown): Wait | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const title = text(value, "title");
  if (title === undefined) {
    return undefined;
  }
  return {
    id: text(value, "id") ?? title,
    title,
    why: text(value, "why") ?? "",
    minutes: number(value, "minutes"),
    when: text(value, "when"),
    ready: readyOf(value),
  };
}

/** What `record`'s quest waits on its person for, saying any it couldn't read. */
function waitsOf(record: Fields, cell: string, unread: string[]): readonly Wait[] {
  const listed = record["waits"];
  if (!Array.isArray(listed)) {
    return [];
  }
  const read = listed.map(waitOf).filter((each) => each !== undefined);
  if (read.length < listed.length) {
    unread.push(`${listed.length - read.length} of ${cell}'s waits had no title`);
  }
  // The launcher counts a wait once however often its file repeats it.
  const seen = new Set<string>();
  return read.filter((each) => !seen.has(each.id) && seen.add(each.id));
}

/** The forage in `record`, without its depth, or none when it names no cell. */
function forageOf(
  record: Fields,
  unread: string[],
  told: StartsTold,
): Omit<Forage, "depth"> | undefined {
  const cell = text(record, "cell");
  if (cell === undefined) {
    unread.push(`${text(record, "key") ?? "a forage"} named no cell`);
    return undefined;
  }
  const lineage = object(record, "lineage") ?? {};
  return {
    cell,
    machine: text(record, "machine") ?? "",
    quest: text(record, "quest") ?? "",
    purpose: text(record, "purpose") ?? "",
    status: text(record, "status") ?? "",
    documents: documentsOf(record, cell, unread),
    waits: waitsOf(record, cell, unread),
    startedBy: text(lineage, "started_by"),
    booked: bookedOf(lineage, cell, told),
    parent: text(lineage, "parent"),
    ...splitOf(lineage, cell, unread),
    serves: joinsOf(record["serves"], "serves", cell, unread),
    relies: joinsOf(record["relies"], "relies", cell, unread),
  };
}

/**
 * The cell `forage` grew from, among `here`: the one that started it, else
 * the one it split from, since a split's new cells may be started by
 * another. None when neither is here, or when it names itself.
 */
function grewFrom(
  forage: Pick<Forage, "cell" | "startedBy" | "splitFrom">,
  here: ReadonlySet<string>,
): string | undefined {
  return [forage.startedBy, forage.splitFrom].find(
    (each) => each !== undefined && each !== forage.cell && here.has(each),
  );
}

/**
 * Whether how `forage` grew from a cell among `here` is only claimed, so the
 * node can't vouch for it: every split is, a start is unless the node's book
 * holds it, and so is a cell that grew from none here, failing closed. One
 * that both started and split from the same cell grew from it by the book,
 * though the panel still says the split as claimed.
 */
function grewClaimed(
  forage: Pick<Forage, "cell" | "startedBy" | "splitFrom" | "booked">,
  here: ReadonlySet<string>,
): boolean {
  const from = grewFrom(forage, here);
  return !(forage.booked && from !== undefined && from === forage.startedBy);
}

/**
 * Every forage `held` holds, each cell after the one it grew from (its
 * starter, else the cell it split from), as a tree is read: those that
 * grew from nothing here first, by address, and under each the cells that
 * grew from it, by address. A cell that grew from one not drawn here
 * stands at the top, and a split is read from either end.
 */
function foragesOf(held: Held): Forages {
  const unread: string[] = [];
  const told: StartsTold = { said: false, odd: [], unsourced: [] };
  const read = bothEnds(
    held
      .all()
      .map((record) => forageOf(record, unread, told))
      .filter((each) => each !== undefined)
      .sort((one, other) => one.cell.localeCompare(other.cell)),
  );
  unread.push(...startsUnread(told));
  const here = new Set(read.map((each) => each.cell));
  const started = new Map<string, (typeof read)[number][]>();
  const tops: (typeof read)[number][] = [];
  for (const each of read) {
    const by = grewFrom(each, here);
    if (by === undefined) {
      tops.push(each);
    } else {
      started.set(by, [...(started.get(by) ?? []), each]);
    }
  }
  const forages: Forage[] = [];
  const placed = new Set<string>();
  const place = (each: (typeof read)[number], depth: number): void => {
    if (placed.has(each.cell)) {
      return;
    }
    placed.add(each.cell);
    forages.push({ ...each, depth });
    for (const under of started.get(each.cell) ?? []) {
      place(under, depth + 1);
    }
  };
  for (const top of tops) {
    place(top, 0);
  }
  // Cells that started each other in a ring have no top; each still shows.
  for (const each of read) {
    place(each, 0);
  }
  return { forages, unread };
}

/**
 * The cells `cell` started among `forages`, by address: the other end of each
 * one's "Started by". A cell that names itself as its starter isn't counted.
 */
function startedFrom(forages: readonly Forage[], cell: string): readonly Forage[] {
  return forages
    .filter((each) => each.startedBy === cell && each.cell !== cell)
    .sort((one, other) => one.cell.localeCompare(other.cell));
}

/** How many of `forage`'s steps are done, and how many there are. */
function steps(forage: Forage): { readonly ticked: number; readonly all: number } {
  const ticked = forage.documents.reduce((sum, each) => sum + each.ticked, 0);
  const open = forage.documents.reduce((sum, each) => sum + each.open, 0);
  return { ticked, all: ticked + open };
}

/** `count` of `kind`, as a count says it: one plan, 3 plans. */
function counted(count: number, kind: string): string {
  const [one, many] = COUNTED[kind] ?? [kind, kind];
  return count === 1 ? `one ${one}` : `${count} ${many}`;
}

/** What `forage` keeps, said short: "2 plans and one report", or "Keeps nothing yet". */
function keeps(forage: Forage): string {
  const kinds = [...new Set(forage.documents.map((each) => each.kind))];
  const said = kinds.map((kind) =>
    counted(forage.documents.filter((each) => each.kind === kind).length, kind),
  );
  if (said.length === 0) {
    return "Keeps nothing yet";
  }
  const sentence = and(said);
  return `${sentence.charAt(0).toUpperCase()}${sentence.slice(1)}`;
}

/** `words` joined as a sentence lists them: a, b and c. */
function and(words: readonly string[]): string {
  if (words.length < 2) {
    return words.join("");
  }
  return `${words.slice(0, -1).join(", ")} and ${words.at(-1) ?? ""}`;
}

/** `count` with its thousands set apart, as 3,000. */
function grouped(count: number): string {
  return count.toLocaleString("en-CA");
}

/** The whole as the headline says it: a plain lead and its turn. */
interface Headline {
  readonly cells: number;
  readonly without: number;
  /** "38 cells here at work on a quest," or "One cell here ...," or "No cells here". */
  readonly lead: string;
  /** "4 without purpose." or "each with its purpose." */
  readonly turn: string;
}

/**
 * The whole in a sentence's two parts: how many cells, and how many have no
 * purpose written. The count names the set it counts, this machine's cells
 * at work on a quest, since "cells" alone means every session a node holds.
 */
function headline(forages: readonly Forage[]): Headline {
  const cells = forages.length;
  const without = forages.filter(unwritten).length;
  if (cells === 0) {
    return { cells, without, lead: "No cells here", turn: "at work on a quest." };
  }
  const many = cells === 1 ? "One cell" : `${grouped(cells)} cells`;
  const lead = `${many} here at work on a quest,`;
  if (without === 0) {
    return {
      cells,
      without,
      lead,
      turn: cells === 1 ? "with its purpose." : "each with its purpose.",
    };
  }
  if (without === cells) {
    return {
      cells,
      without,
      lead,
      turn: cells === 1 ? "without purpose." : "all without purpose.",
    };
  }
  return { cells, without, lead, turn: `${grouped(without)} without purpose.` };
}

/** `cells` named in a sentence, at most three by name and the rest counted. */
function named(cells: readonly string[]): string {
  const most = 3;
  if (cells.length <= most) {
    return and(cells);
  }
  const rest = cells.length - (most - 1);
  return `${cells.slice(0, most - 1).join(", ")} and ${grouped(rest)} more`;
}

/** The cell with a plan furthest from done, and how far it has come, when one has steps left. */
function furthest(
  forages: readonly Forage[],
): { readonly cell: string; readonly ticked: number; readonly all: number } | undefined {
  let found: { cell: string; ticked: number; all: number; left: number } | undefined;
  for (const forage of forages) {
    const { ticked, all } = steps(forage);
    const left = all - ticked;
    if (left > 0 && (found === undefined || left > found.left)) {
      found = { cell: forage.cell, ticked, all, left };
    }
  }
  return found === undefined
    ? undefined
    : { cell: found.cell, ticked: found.ticked, all: found.all };
}

/**
 * What waits on its person, said first: how many things, each counted once
 * however many cells work on its quest, and the cells waiting; nothing when
 * none waits.
 */
function waited(forages: readonly Forage[]): string {
  const things = new Set(
    forages.flatMap((each) => each.waits.map((wait) => `${each.quest}\n${wait.id}`)),
  );
  if (things.size === 0) {
    return "";
  }
  const cells = forages.filter((each) => each.waits.length > 0).map((each) => each.cell);
  const count =
    things.size === 1 ? "One thing waits on you" : `${grouped(things.size)} things wait on you`;
  return `${count}, from ${named(cells)}. `;
}

/**
 * What the lede says: what waits on its person, then the cells with no
 * purpose written, then the plan furthest from done.
 */
function lede(forages: readonly Forage[]): string {
  if (forages.length === 0) {
    return "When a cell here takes up a quest, its purpose shows here, with how far its plans have come.";
  }
  return `${waited(forages)}${purposes(forages)}`;
}

/** The lede's account of purposes and plans. */
function purposes(forages: readonly Forage[]): string {
  const hollow = forages.filter(unwritten).map((each) => each.cell);
  const first =
    hollow.length === 0
      ? "Every cell has its purpose written."
      : `${named(hollow)} ${hollow.length === 1 ? "has" : "have"} no purpose written yet.`;
  const far = furthest(forages);
  if (far === undefined) {
    return `${first} No plan has steps left.`;
  }
  return `${first} ${far.cell} has furthest to go, ${far.ticked} of ${far.all} steps done.`;
}

/** Whether `forage` holds `query` in its cell, quest, purpose, a document's or a wait's title. */
function holds(forage: Forage, query: string): boolean {
  const sought = query.trim().toLowerCase();
  if (sought === "") {
    return true;
  }
  return [
    forage.cell,
    forage.quest,
    forage.purpose,
    ...forage.documents.map((each) => each.title),
    ...forage.waits.map((each) => each.title),
  ]
    .map((each) => each.toLowerCase())
    .some((each) => each.includes(sought));
}

/**
 * A purpose that shows nothing: spaces and breaks, controls, the characters
 * Unicode says draw nothing by default (a zero-width space, a word joiner, a
 * soft hyphen, a Hangul filler, a variation selector) and the blank braille
 * cell, which a pasted or written purpose can carry and `trim` leaves. The
 * marks written before a number, such as the end of an ayah, show a glyph,
 * so a purpose of them is written.
 */
const SAYS_NOTHING = /^[\s\p{Cc}\p{Default_Ignorable_Code_Point}\u2800]*$/u;

/**
 * Whether `forage` has no purpose written yet: what the headline counts,
 * the lede names, the row and the floor show hollow and the view of cells
 * without purpose takes in, so none of them can disagree with another. A
 * purpose that shows nothing says nothing, so it counts as unwritten.
 */
function unwritten(forage: Forage): boolean {
  return SAYS_NOTHING.test(forage.purpose);
}

/**
 * Whether the list's view takes `forage` in before a find narrows it: every
 * cell does, but while `hollowOnly` asks for the cells with no purpose
 * written, only those.
 */
function inView(forage: Forage, hollowOnly: boolean): boolean {
  return !hollowOnly || unwritten(forage);
}

/** The cells of `forages` the list shows for the find `sought` in its view, in its order. */
function shownBy(
  forages: readonly Forage[],
  sought: string,
  hollowOnly: boolean,
): readonly Forage[] {
  return forages.filter((forage) => holds(forage, sought) && inView(forage, hollowOnly));
}

/**
 * The cell the cursor rests on once the list shows what the find `sought`
 * holds of `forages` in its view: `cursor` while the list shows it, else
 * its first row. A find that matches nothing keeps it while its cell lasts
 * and the view still takes it in, so ending the find finds it again; a cell
 * that ends, or that the view no longer takes in, takes the cursor with it.
 * It works the rows out itself, as the list does, so the two can't be given
 * different views.
 */
function cursorAfter(
  forages: readonly Forage[],
  sought: string,
  hollowOnly: boolean,
  cursor: string | null,
): string | null {
  const rows = shownBy(forages, sought, hollowOnly);
  if (rows.some((each) => each.cell === cursor)) {
    return cursor;
  }
  const [first] = rows;
  if (first !== undefined) {
    return first.cell;
  }
  const open = forages.find((each) => each.cell === cursor);
  return open !== undefined && inView(open, hollowOnly) ? cursor : null;
}

export type { Document, Forage, Forages, Headline, Join, Wait };
export {
  counted,
  cursorAfter,
  foragesOf,
  grewFrom,
  grewClaimed,
  headline,
  holds,
  inView,
  keeps,
  lede,
  shownBy,
  startedFrom,
  steps,
  unwritten,
};

// What the forage page draws, read from the mycelium node's forage records
// (mycelium's PROTOCOL.md, Forages): one for each cell of this machine that
// works on a quest, keyed forage:<cell>, with the purpose its quest states
// and the documents it keeps for it. Nothing here touches the page, so
// every case is proven in node, and a record the page couldn't make sense
// of is said rather than drawn as if it were whole.

import type { Fields, Held } from "../../local-sites/kit/records.ts";
import { isObject, number, object, text } from "../../local-sites/kit/records.ts";

/** One document a cell keeps for its purpose: a plan, research, a brief or a report. */
interface Document {
  readonly id: string;
  readonly kind: string;
  readonly title: string;
  readonly stage: string;
  readonly ticked: number;
  readonly open: number;
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
  /** The cell that started it, by address, when one did. */
  readonly startedBy: string | undefined;
  /** The quest its own sits under, when it sits under one. */
  readonly parent: string | undefined;
  /** How many cells above it started it, here: 0 for one nothing here started. */
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
  const listed = record.documents;
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

/** The forage in `record`, without its depth, or none when it names no cell. */
function forageOf(record: Fields, unread: string[]): Omit<Forage, "depth"> | undefined {
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
    startedBy: text(lineage, "started_by"),
    parent: text(lineage, "parent"),
  };
}

/**
 * Every forage `held` holds, each cell after the one that started it, as a
 * tree is read: those nothing here started first, by address, and under
 * each the cells it started, by address. A cell started by one on another
 * machine stands at the top, since its starter isn't drawn here.
 */
function foragesOf(held: Held): Forages {
  const unread: string[] = [];
  const read = held
    .all()
    .map((record) => forageOf(record, unread))
    .filter((each) => each !== undefined)
    .sort((one, other) => one.cell.localeCompare(other.cell));
  const here = new Set(read.map((each) => each.cell));
  const started = new Map<string, (typeof read)[number][]>();
  const tops: (typeof read)[number][] = [];
  for (const each of read) {
    const by = each.startedBy;
    if (by !== undefined && by !== each.cell && here.has(by)) {
      started.set(by, [...(started.get(by) ?? []), each]);
    } else {
      tops.push(each);
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

/** What `forage` keeps, said short: "2 plans and one report", or "No documents yet". */
function keeps(forage: Forage): string {
  const kinds = [...new Set(forage.documents.map((each) => each.kind))];
  const said = kinds.map((kind) =>
    counted(forage.documents.filter((each) => each.kind === kind).length, kind),
  );
  if (said.length === 0) {
    return "No documents yet";
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

/** The whole in one sentence: how many cells, and how many have no purpose written. */
function judged(forages: readonly Forage[], machine: string): string {
  const where = machine === "" ? "here" : `on ${machine}`;
  if (forages.length === 0) {
    return `No cell ${where} works on a quest right now.`;
  }
  const cells = forages.length === 1 ? "One cell" : `${forages.length} cells`;
  const works = forages.length === 1 ? "works" : "work";
  const without = forages.filter((each) => each.purpose === "").length;
  const purposes =
    without === 0
      ? `${forages.length === 1 ? "" : "each "}with its purpose written`
      : `${without === 1 ? "one" : String(without)} with no purpose written`;
  return `${cells} ${where} ${works} on a quest, ${purposes}.`;
}

/** Whether `forage` holds `query` in its cell, quest, purpose or a document's title. */
function holds(forage: Forage, query: string): boolean {
  const sought = query.trim().toLowerCase();
  if (sought === "") {
    return true;
  }
  return [forage.cell, forage.quest, forage.purpose, ...forage.documents.map((each) => each.title)]
    .map((each) => each.toLowerCase())
    .some((each) => each.includes(sought));
}

export type { Document, Forage, Forages };
export { counted, foragesOf, holds, judged, keeps, steps };

// What waits on a cell and what holds it has standing, as the drawer says
// them: who asked, how long ago and whether anything has answered, where a
// hold went in the words the page names each thing by, and which cells to
// list first. Times are said against the reading's own, as when.ts says them.

import { dayClock, minutesWord } from "./when.ts";
import type { Cell, Hold, Network, Owed } from "./words.ts";
import { SHARED_WORD, waitingWords } from "./words.ts";

/** A network the node shares rather than draws: a quest, a repository or a group. */
export interface Shared {
  readonly key: string;
  readonly short: string;
  readonly shape: string;
}

/** The reading, of what the words for waits and holds read. */
export interface Seen {
  readonly byAddress: ReadonlyMap<string, Cell>;
  readonly hubs: ReadonlyMap<string, Network>;
  readonly shared: readonly Shared[];
}

const PEER = /^@(.+)$/u;
const LEADING_HASH = /^#/u;

/**
 * A shared network as the panel names it, "the backwater repository", or
 * its key where the reading holds none by it. The model passes only shapes
 * with a word; one with none, or a name every object inherits, is said as
 * itself rather than as whatever the lookup finds.
 */
export const sharedWords = (key: string, seen: Seen | null): string => {
  const n = seen?.shared.find((x) => x.key === key);
  if (!n) {
    return key;
  }
  const word = Object.hasOwn(SHARED_WORD, n.shape) ? SHARED_WORD[n.shape] : undefined;
  return `the ${n.short} ${word ?? n.shape}`;
};

/**
 * Where a hold went, as the mesh writes it (@PEER, #machine, #quest/x), said
 * as the page names each: a cell by its short name, a network as the drawers
 * name it, the machine as every cell on it.
 */
export const holdTarget = (to: string, seen: Seen | null): string => {
  const peer = PEER.exec(to)?.[1]?.toUpperCase();
  if (peer) {
    return seen?.byAddress.get(peer)?.short || peer;
  }
  if (to === "#machine") {
    return "every cell on its machine";
  }
  const key = to.replace(LEADING_HASH, "").replace("/", ".");
  const hub = seen?.hubs.get(key);
  if (hub) {
    return hub.name;
  }
  return sharedWords(key, seen);
};

/** A hold a cell has standing, in words: where it went, since when, and until when at most. */
export const holdWords = (h: Hold, seen: Seen | null, at: string): string =>
  `holding ${h.to.map((to) => holdTarget(to, seen)).join(" and ") || "its peers"} ` +
  `since ${dayClock(h.at, at)}${h.until ? `, until ${dayClock(h.until, at)} at most` : ""}`;

/**
 * What the page says of a question waiting, by how it knows: the node's
 * judgement, or its own guess from the latest messages, where any word since
 * counts as an answer.
 */
export const unansweredWords = (judged: boolean): string =>
  judged ? "with no answer yet" : "with no word since";

/** Who asked a question, by the short name the page shows, or by address. */
export const askerOf = (o: Owed, seen: Seen): string => seen.byAddress.get(o.from)?.short || o.from;

/** How long the oldest of a cell's questions has waited, in milliseconds; none is none. */
export const ageOf = (owes: readonly Owed[], at: string): number => {
  const oldest = owes[0];
  return oldest ? Math.max(0, Date.parse(at) - Date.parse(oldest.at)) : 0;
};

/** One question waiting on a cell, in words: whose, and since when. */
export const owedWords = (o: Owed, seen: Seen, at: string): string =>
  `${askerOf(o, seen)}'s question waiting since ${dayClock(o.at, at)}`;

/** How the page says what holds a cell up: whether the node judged it, and how a name fits. */
export interface HeldSaid {
  readonly judged: boolean;
  readonly named?: ((name: string) => string) | undefined;
}

/**
 * What holds a cell up, in words: how many wait, the oldest's asker and age,
 * and how it knows; `named` fits the asker's name to the room the row has.
 */
export const heldWords = (
  s: Cell,
  seen: Seen,
  at: string,
  { judged, named = (name) => name }: HeldSaid,
): string => {
  const oldest = s.owes[0];
  const asker = oldest ? named(askerOf(oldest, seen)) : "";
  return (
    `${waitingWords(s.owes)}; ${s.owes.length === 1 ? "it came" : "the oldest came"} ` +
    `from ${asker} ${minutesWord(ageOf(s.owes, at))} ago, ${unansweredWords(judged)}`
  );
};

/** The cells with questions waiting on them, the oldest question first. */
export const heldFirst = (cells: readonly Cell[]): readonly Cell[] =>
  cells
    .filter((s) => s.owes.length > 0)
    .sort((a, b) => (a.owes[0]?.at ?? "").localeCompare(b.owes[0]?.at ?? ""));

/** The cells with holds standing, the oldest hold first. */
export const holdingFirst = (cells: readonly Cell[]): readonly Cell[] =>
  cells
    .filter((s) => s.holds.length > 0)
    .sort((a, b) => (a.holds[0]?.at ?? "").localeCompare(b.holds[0]?.at ?? ""));

/**
 * The cells that need the reader: every one asking him something first,
 * since only he can answer it, then every one stuck, each in the order
 * given. Now lists them before anything else.
 */
export const needsFirst = (cells: readonly Cell[]): readonly Cell[] => [
  ...cells.filter((s) => s.standing === "asks"),
  ...cells.filter((s) => s.standing === "stuck"),
];

/**
 * The address of the cell that needs the reader after the one at `from`,
 * in the order Now lists them, round to the first after the last; the
 * first when `from` is none or needs nothing; null when none needs him.
 */
export const nextNeeding = (cells: readonly Cell[], from: string | null): string | null => {
  const needing = needsFirst(cells);
  const at = needing.findIndex((s) => s.address === from);
  return needing[(at + 1) % needing.length]?.address ?? null;
};

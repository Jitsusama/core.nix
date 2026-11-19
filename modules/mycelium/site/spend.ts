// What the cells spent today, as the drawer's Spent Today fold opens it: the
// cells and the parts of a sum in spend order, what each part and cell says
// it spent, how each list is headed, and what the parts past a list's end
// spent between them. The sums are the sessions' own, as words.ts reads them.

import { allCells, type Cell, money, type Network, plural, spentBy, spentOf } from "./words.ts";

/** How many rows a list of the fold shows before it counts the rest. */
export const SPEND_SHOWN = 10;

/** What a list of the fold splits its sum by, beside the list of its cells. */
export type Split = "machine" | "network" | "branch";

/** What a list of the fold counts, its cells included. */
export type Part = Split | "cell";

// Each split's words, as a list is headed by one and as it names those that
// spent most; the cells' list has headings of its own.
const HEADING_WORDS: Readonly<Record<Split, readonly [string, string]>> = {
  machine: ["Machine", "Machines"],
  network: ["Network", "Networks"],
  branch: ["Branch", "Branches"],
};

// Each part's words for more than one, as the rest past a list's end are counted.
const MANY: Readonly<Record<Part, string>> = {
  machine: "machines",
  network: "networks",
  branch: "branches",
  cell: "cells",
};

// The order is kept per list, and each reading brings new lists.
const BY_SPEND = new WeakMap<readonly Cell[], readonly Cell[]>();

/** The cells that spent anything today, the most first. */
export const bySpend = (cells: readonly Cell[]): readonly Cell[] => {
  let ordered = BY_SPEND.get(cells);
  if (ordered === undefined) {
    ordered = cells.filter((s) => spentBy(s) > 0).sort((a, b) => spentBy(b) - spentBy(a));
    BY_SPEND.set(cells, ordered);
  }
  return ordered;
};

/** The parts that spent anything today, each with its cells, the most first. */
export const partsBySpend = <T>(
  parts: readonly T[],
  cellsOf: (part: T) => readonly Cell[],
): readonly (readonly [T, readonly Cell[]])[] =>
  parts
    .map((part): readonly [T, readonly Cell[]] => [part, cellsOf(part)])
    .filter(([, cells]) => spentOf(cells) > 0)
    .sort(([, a], [, b]) => spentOf(b) - spentOf(a));

/**
 * A network's cells, its branches' with its own, so a list of networks adds
 * up to the sum above it; on a machine, only the cells that run there.
 */
export const networkCells = (h: Network, machine: string | null = null): readonly Cell[] =>
  machine === null || machine === ""
    ? allCells(h)
    : allCells(h).filter((s) => s.machine === machine);

/** A list cut where the fold stops showing rows: those it shows, and the rest it counts. */
export const shownAndRest = <T>(rows: readonly T[]): readonly [readonly T[], readonly T[]] => [
  rows.slice(0, SPEND_SHOWN),
  rows.slice(SPEND_SHOWN),
];

/** What a part of a sum spent, over how many cells. */
export const partWords = (cells: readonly Cell[]): string =>
  `${money(spentOf(cells))}, ${plural(cells.length, "cell")}`;

/** What one cell spent, and in how many turns where its session says. */
export const cellSpendWords = (s: Cell): string => {
  const turns = s.usage?.turns_today;
  return [money(spentBy(s)), turns === null || turns === undefined ? "" : plural(turns, "turn")]
    .filter(Boolean)
    .join(", ");
};

/**
 * Whether a split says anything the sum above it doesn't: one part is the
 * whole sum again.
 */
export const splits = (count: number): boolean => count > 1;

/** How a list of `count` parts is headed: by its part, or those that spent most. */
export const partHeading = (part: Split, count: number): string =>
  count > SPEND_SHOWN
    ? `The ${SPEND_SHOWN} ${HEADING_WORDS[part][1]} That Spent Most`
    : `By ${HEADING_WORDS[part][0]}`;

/** How the list of `count` cells is headed: its cells, or those that spent most. */
export const cellsHeading = (count: number): string =>
  count > SPEND_SHOWN ? `The ${SPEND_SHOWN} Cells That Spent Most` : "Its Cells";

/** What the parts past a list's end spent between them, or nothing where none are. */
export const restWords = (rest: readonly (readonly Cell[])[], part: Part): string =>
  rest.length === 0
    ? ""
    : `${plural(rest.length, `more ${part}`, `more ${MANY[part]}`)} spent ` +
      `${money(rest.reduce((t, cells) => t + spentOf(cells), 0))} between them.`;

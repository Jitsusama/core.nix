// Forages for the floor's tests, as the model hands them to the floor, and
// ways to read what the floor drew of them: its compartments in order, what
// stands along its main line, and the runs that line is broken into.

import { floorFor, floorOf } from "../draw.ts";
import type { Forage } from "../model.ts";

/** A forage for `cell` with `more` over its fields, its documents none. */
function forage(cell: string, more: Partial<Forage> = {}): Forage {
  return {
    cell,
    machine: "kestrel",
    quest: `QEST-20261116-${cell}`,
    purpose: `${cell} learns to hover.`,
    status: "active",
    documents: [],
    waits: [],
    startedBy: undefined,
    booked: false,
    parent: undefined,
    splitFrom: undefined,
    splitInto: [],
    serves: [],
    relies: [],
    depth: 0,
    ...more,
  };
}

/** The cell at `at` in the tests' lists. */
const named = (at: number): string => `C${String(at).padStart(2, "0")}`;

/** The cells from `first` to `last`, both drawn. */
const span = (first: number, last: number): string[] =>
  Array.from({ length: last - first + 1 }, (_, at) => named(first + at));

/** The cells `floor` draws a compartment for, in the order it draws them. */
const compartments = (floor: string): string[] =>
  [...floor.matchAll(/<rect class="[^"]*" data-cell="([^"]+)"/gu)].map((match) => match[1] ?? "");

/**
 * What `floor` draws along its main line, left to right: each compartment's
 * cell, and an ellipsis for each gap where it skips cells of its list.
 */
const along = (floor: string): string[] =>
  [
    ...[...floor.matchAll(/<rect class="[^"]*" data-cell="([^"]+)" x="([^"]+)"/gu)].map(
      (match) => ({ mark: match[1] ?? "", x: Number(match[2]) + 11 }),
    ),
    ...[...floor.matchAll(/<text class="gap" x="([^"]+)"/gu)].map((match) => ({
      mark: "\u2026",
      x: Number(match[1]),
    })),
  ]
    .sort((one, other) => one.x - other.x)
    .map((each) => each.mark);

/** The main line, the one hypha naming no cell. */
const MAIN = /<path class="hypha" d="([^"]+)"/u;

/** The runs `floor`'s main line is drawn in, broken at each gap. */
const runs = (floor: string): number => (floor.match(MAIN)?.[1] ?? "").split("M").length - 1;

/** The floor of `list` drawn around `around`, ringing `cursor`. */
const floorAt = (
  list: readonly Forage[],
  cursor: string | null,
  around: string | null = cursor,
): string => floorOf(floorFor(list, around), cursor, "kestrel");

export { along, compartments, floorAt, forage, MAIN, named, runs, span };

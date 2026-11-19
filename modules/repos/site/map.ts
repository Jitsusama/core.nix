// The map of the mycelium between repositories, as Joel asked to see "which repositories have
// mycelial connections to get stuff done": a row for each repository a cell's work ties to
// another, in the grove's order, and a thread for each cell tying two rows, which mapview.ts
// draws as an arc beside them. The rows are walked one way, up and down, since an arc diagram
// lays every repository on one line. Nothing here touches the page, so each is tried in Node
// (tests/map.test.ts).

import { type Tie, ties } from "./joins.ts";
import type { Repository } from "./model.ts";

// As many threads as a crown in the grove draws under its ground; the words count the rest.
const THREADS_A_TIE = 6;

/** A repository on the map, at its row. */
interface Row {
  readonly key: string;
  readonly name: string;
  readonly address: string;
}

/** One cell's thread between two rows, in the `lane` it takes among its tie's. */
interface Thread {
  readonly from: number;
  readonly to: number;
  readonly cell: string;
  readonly lane: number;
}

/** The rows, the threads between them and the ties they're drawn from. */
interface MycelialMap {
  readonly rows: readonly Row[];
  readonly threads: readonly Thread[];
  readonly ties: readonly Tie[];
}

/** Where a key moves the walk: a row up or down, sideways, or to either end. */
type Way = "up" | "down" | "left" | "right" | "first" | "last";

/** The map of `repositories`, which come in the grove's order and keep it. */
function mapOf(repositories: readonly Repository[]): MycelialMap {
  const all = ties(repositories);
  const tied = new Set(all.flatMap((tie) => tie.ends));
  const rows = repositories
    .filter((each) => tied.has(each.key))
    .map(({ key, name, address }) => ({ key, name, address }));
  const at = new Map(rows.map((row, index) => [row.key, index]));
  const threads = all.flatMap((tie) => {
    const [from = 0, to = 0] = tie.ends.map((key) => at.get(key) ?? 0).sort((a, b) => a - b);
    const drawn = tie.cells.slice(0, THREADS_A_TIE);
    return drawn.map((cell, lane) => ({ from, to, cell, lane }));
  });
  threads.sort((one, other) => one.from - other.from || one.to - other.to || one.lane - other.lane);
  return { rows, threads, ties: all };
}

/**
 * Where the keys rest on `now` when they rested at `at` on `was`: on the same repository, however
 * the rows around it moved, or as near the row they left as `now` reaches once it's gone.
 */
function kept(was: MycelialMap, at: number, now: MycelialMap): number {
  const key = was.rows[at]?.key;
  const same = now.rows.findIndex((each) => each.key === key);
  return same === -1 ? Math.min(at, Math.max(0, now.rows.length - 1)) : same;
}

/** The row `way` from the row at `from`, or nothing when there's none that way. */
function walked(map: MycelialMap, from: number, way: Way): number | undefined {
  const last = map.rows.length - 1;
  const steps: { readonly [way in Way]: number | undefined } = {
    up: from - 1,
    down: from + 1,
    left: undefined,
    right: undefined,
    first: 0,
    last,
  };
  const to = steps[way];
  return to === undefined || to < 0 || to > last ? undefined : to;
}

/**
 * How deep a thread's bend is set into the gutter beside `rows` rows, out of the gutter's 100,
 * the curve itself reaching three quarters as far: a farther tie deeper, until ties spanning
 * nearly every row bow alike at the deepest a tie may, and each cell's lane a little more, never
 * past the gutter's edge. A near tie's last lanes may bow past a farther tie's first, as the lanes
 * fan past each tie's own bow.
 */
function bowed(thread: Thread, rows: number): number {
  const span = (thread.to - thread.from) / Math.max(1, rows - 1);
  // The lanes are kept apart past the tie's own bow, which leaves room for every lane.
  const lanes = 4 * (THREADS_A_TIE - 1);
  return Math.min(96 - lanes, 14 + 70 * span) + 4 * thread.lane;
}

const counted = (count: number): string => `${count}\u00a0${count === 1 ? "cell" : "cells"}`;

/**
 * What the row at `index` is tied to, the most cells first, as a sentence, each other row's
 * name written by `named`: as it is for a reader's label, or as a link home on the page.
 */
function saidOf(
  map: MycelialMap,
  index: number,
  named: (row: Row) => string = (row) => row.name,
): string {
  const row = map.rows[index];
  if (row === undefined) {
    return "";
  }
  const rowOf = (key: string): Row =>
    map.rows.find((each) => each.key === key) ?? { key, name: key, address: "" };
  const others = map.ties
    .filter((tie) => tie.ends.includes(row.key))
    .map((tie) => ({
      row: rowOf(tie.ends[0] === row.key ? tie.ends[1] : tie.ends[0]),
      cells: tie.cells.length,
    }))
    .sort(
      (one, other) =>
        other.cells - one.cells || one.row.name.localeCompare(other.row.name, "en-CA"),
    )
    .map((other) => `${named(other.row)} by ${counted(other.cells)}`);
  const last = others.pop();
  const listed = others.length === 0 ? last : `${others.join(", ")}, and ${last}`;
  return `Joined to ${listed}.`;
}

export {
  bowed,
  kept,
  type MycelialMap,
  mapOf,
  type Row,
  saidOf,
  THREADS_A_TIE,
  type Thread,
  type Way,
  walked,
};

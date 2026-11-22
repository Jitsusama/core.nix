// Which cells the forage page's floor draws when the list holds more than
// it can: those around the cell it's drawn around, with the cells that grew
// from that one, and each cell drawn with every cell it grew from, so no
// branch hangs from a starter left off and a booked start far down a long
// list still draws solid; and when a step draws the floor again.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { floorOf, keptAround, stepsOffFloor } from "../draw.ts";
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

/** The cell at `at` in the lists below. */
const named = (at: number): string => `C${String(at).padStart(2, "0")}`;

/** The cells from `first` to `last`, both drawn. */
const span = (first: number, last: number): string[] =>
  Array.from({ length: last - first + 1 }, (_, at) => named(first + at));

/** Sixty top cells, C00 to C59, with a starter at C10 and the cell it booked at C55. */
const LONG: readonly Forage[] = Array.from({ length: 60 }, (_, at) =>
  at === 55 ? forage(named(at), { startedBy: "C10", booked: true, depth: 1 }) : forage(named(at)),
);

/** `LONG` with `cell` started by `by`. */
const startedOn = (list: readonly Forage[], cell: string, by: string): readonly Forage[] =>
  list.map((each) => (each.cell === cell ? { ...each, startedBy: by, depth: 1 } : each));

/** The cells `floor` draws a compartment for, in the order it draws them. */
const compartments = (floor: string): string[] =>
  [...floor.matchAll(/<rect class="[^"]*" data-cell="([^"]+)"/gu)].map((match) => match[1] ?? "");

/** A ringed compartment, naming its cell. */
const RINGED = /<rect class="[^"]* on" data-cell="([^"]+)"/u;

/** The cell whose compartment `floor` rings. */
const ringed = (floor: string): string | undefined => floor.match(RINGED)?.[1];

it("draws every cell of a list that fits, whatever the cursor", () => {
  assert.deepEqual(compartments(floorOf(LONG.slice(0, 5), "C04", "kestrel")), span(0, 4));
});

it("draws the list's first 48 with no cursor, and counts the rest", () => {
  const floor = floorOf(LONG, null, "kestrel");
  assert.deepEqual(compartments(floor), span(0, 47));
  assert.ok(floor.includes("and 12 more in the list"), floor);
  // C55's starter is drawn, but C55 isn't, so no branch is.
  assert.ok(!floor.includes('data-cell="C55"'), floor);
});

it("draws a cursor past the 48th with its starter, so its booked branch is solid", () => {
  const floor = floorOf(LONG, "C55", "kestrel");
  // Its starter, then the 47 nearest it, kept in the list's order.
  assert.deepEqual(compartments(floor), ["C10", ...span(13, 59)]);
  assert.ok(floor.includes('<path class="hypha" data-cell="C55"'), floor);
  assert.equal(ringed(floor), "C55");
});

it("draws the cells a cursor started, however far down the list", () => {
  const floor = floorOf(LONG, "C10", "kestrel");
  assert.deepEqual(compartments(floor), [...span(0, 46), "C55"]);
  assert.ok(floor.includes('<path class="hypha" data-cell="C55"'), floor);
});

it("draws a cell near the cursor only with every cell it grew from", () => {
  const list = startedOn(LONG, "C30", "C02");
  const floor = floorOf(list, "C40", "kestrel");
  const drawn = compartments(floor);
  assert.equal(drawn.length, 48);
  assert.ok(drawn.includes("C30") && drawn.includes("C02"), drawn.join(" "));
  assert.ok(floor.includes('<path class="hypha claimed" data-cell="C30"'), floor);
  // C55 is drawn, so its starter is too.
  assert.ok(drawn.includes("C55") && drawn.includes("C10"), drawn.join(" "));
  assert.ok(floor.includes('<path class="hypha" data-cell="C55"'), floor);
});

it("follows a ring of starts once round, drawing each cell in it", () => {
  const two = startedOn(LONG, "C10", "C55");
  const three = startedOn(startedOn(LONG, "C10", "C30"), "C30", "C55");
  for (const [ring, cells] of [
    [two, ["C10", "C55"]],
    [three, ["C10", "C30", "C55"]],
  ] as const) {
    const drawn = compartments(floorOf(ring, "C55", "kestrel"));
    assert.equal(new Set(drawn).size, 48, drawn.join(" "));
    assert.ok(
      cells.every((cell) => drawn.includes(cell)),
      drawn.join(" "),
    );
  }
  // Followed round more than once, the ring beside the cursor would ask for
  // more room than is left, and go undrawn.
  const beside = startedOn(startedOn(LONG, "C51", "C52"), "C52", "C51");
  const drawn = compartments(floorOf(beside, "C50", "kestrel"));
  assert.ok(drawn.includes("C51") && drawn.includes("C52"), drawn.join(" "));
});

/** Ninety cells, C00 to C89, grown from each other in chains of every depth. */
const GROWN: readonly Forage[] = Array.from({ length: 90 }, (_, at) => {
  if (at > 0 && at % 4 === 0) {
    return forage(named(at), { startedBy: named(Math.floor(at * 0.6)), booked: true, depth: 1 });
  }
  return at % 4 === 1 && at > 1
    ? forage(named(at), { startedBy: named(at - 1), booked: true, depth: 2 })
    : forage(named(at));
});

it("draws at most 48 cells around any cursor, each with every cell it grew from", () => {
  const starter = new Map(GROWN.map((each) => [each.cell, each.startedBy]));
  for (const { cell } of GROWN) {
    const drawn = compartments(floorOf(GROWN, cell, "kestrel"));
    assert.ok(drawn.length <= 48, `${cell}: ${drawn.length}`);
    assert.ok(drawn.includes(cell), `${cell}: ${drawn.join(" ")}`);
    for (const each of drawn) {
      const by = starter.get(each);
      assert.ok(by === undefined || drawn.includes(by), `${cell}: ${each} without ${by}`);
    }
  }
});

it("draws around the cell the page keeps it at, ringing the cursor there", () => {
  const floor = floorOf(LONG, "C30", "kestrel", "C55");
  assert.deepEqual(compartments(floor), ["C10", ...span(13, 59)]);
  assert.equal(ringed(floor), "C30");
});

it("draws the floor again only for a step to a cell it leaves off, or whose starts it does", () => {
  const marked = new Set(span(0, 47));
  assert.equal(stepsOffFloor(LONG, "C55", marked, null), true);
  assert.equal(stepsOffFloor(LONG, "C47", marked, null), false);
  assert.equal(stepsOffFloor(LONG, null, marked, null), false);
  // C10 is drawn, but the cell it booked, C55, isn't.
  assert.equal(stepsOffFloor(LONG, "C10", marked, null), true);
  // Once the floor is drawn around a cell, it stays, even one it can't mark.
  assert.equal(stepsOffFloor(LONG, "C\u0000", marked, null), true);
  assert.equal(stepsOffFloor(LONG, "C\u0000", marked, "C\u0000"), false);
  // A floor drawing the whole list never redraws.
  assert.equal(stepsOffFloor(LONG.slice(0, 5), "C\u0000", new Set(span(0, 4)), null), false);
});

it("keeps the floor around its cell while the list holds it, else the cursor", () => {
  assert.equal(keptAround(LONG, "C55", "C02"), "C55");
  assert.equal(keptAround(LONG, null, "C02"), "C02");
  assert.equal(keptAround(LONG, "C99", "C02"), "C02");
});

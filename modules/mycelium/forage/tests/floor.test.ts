// Which cells the forage page's floor draws when the list holds more than
// it can: those around the cell it's drawn around, with the cells that grew
// from that one, and each cell drawn with every cell it grew from, so no
// branch hangs from a starter left off and a booked start far down a long
// list still draws solid; and when a step draws the floor again.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { floorAfter, floorFor, floorOf, stepsOff } from "../draw.ts";
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
    assert.equal(drawn.length, 48, drawn.join(" "));
    assert.equal(new Set(drawn).size, 48, drawn.join(" "));
    assert.ok(
      cells.every((cell) => drawn.includes(cell)),
      drawn.join(" "),
    );
  }
  // The ring at the cursor fits however often it's followed round, so only a
  // ring beside it shows the walk stops once round: followed round more than
  // once, it would ask for more room than is left, and go undrawn.
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

it("draws 48 cells around any cursor, with those it started, each with every cell it grew from", () => {
  const starter = new Map(GROWN.map((each) => [each.cell, each.startedBy]));
  for (const { cell } of GROWN) {
    const drawn = compartments(floorOf(GROWN, cell, "kestrel"));
    assert.equal(drawn.length, 48, `${cell}: ${drawn.join(" ")}`);
    assert.ok(drawn.includes(cell), `${cell}: ${drawn.join(" ")}`);
    for (const each of GROWN.filter((one) => one.startedBy === cell)) {
      assert.ok(drawn.includes(each.cell), `${cell}: leaves off ${each.cell}`);
    }
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

/** Sixty cells, C00 to C59, each started by the one before it. */
const CHAIN: readonly Forage[] = Array.from({ length: 60 }, (_, at) =>
  at === 0
    ? forage(named(at))
    : forage(named(at), { startedBy: named(at - 1), depth: Math.min(at, 3) }),
);

it("cuts a line deeper than the floor at its top, keeping the 48 nearest the cursor", () => {
  const floor = floorOf(CHAIN, "C59", "kestrel");
  assert.deepEqual(compartments(floor), span(12, 59));
  // C12's starter isn't drawn, so no branch leads to it; C13's is.
  assert.ok(!floor.includes('data-cell="C12" d='), floor);
  assert.ok(floor.includes('data-cell="C13" d='), floor);
  // Its line fills the floor, so the cell C50 started goes undrawn, and a
  // step to C50 keeps the floor drawn around it.
  assert.deepEqual(compartments(floorOf(CHAIN, "C50", "kestrel")), span(3, 50));
  assert.equal(stepsOff(floorFor(CHAIN, "C50"), "C50"), false);
});

it("draws a cell split from another with it, as one it started", () => {
  const split = LONG.map((each) =>
    each.cell === "C57" ? { ...each, splitFrom: "C03", depth: 1 } : each,
  );
  const floor = floorOf(split, "C57", "kestrel");
  assert.ok(compartments(floor).includes("C03"), floor);
  assert.ok(floor.includes('data-cell="C57" d='), floor);
  // C03 is drawn, but the cell split from it isn't.
  assert.equal(stepsOff(floorFor(split, null), "C03"), true);
});

it("draws the floor again only for a step to a cell it leaves off, or one that grew a cell it does", () => {
  const first = floorFor(LONG, null);
  assert.equal(stepsOff(first, "C55"), true);
  assert.equal(stepsOff(first, "C47"), false);
  assert.equal(stepsOff(first, null), false);
  // C10 is drawn, but the cell it booked, C55, isn't.
  assert.equal(stepsOff(first, "C10"), true);
  // Kept around C55, the floor draws C10 and C13 to C59.
  const kept = floorFor(LONG, "C55");
  assert.equal(stepsOff(kept, "C10"), false);
  assert.equal(stepsOff(kept, "C30"), false);
  assert.equal(stepsOff(kept, "C12"), true);
  // A floor drawing the whole list never redraws.
  assert.equal(stepsOff(floorFor(LONG.slice(0, 5), null), "C04"), false);
});

it("keeps the floor where it is while it holds the cursor, else draws it around the cursor", () => {
  assert.equal(floorAfter(LONG, "C55", "C30").around, "C55");
  assert.equal(floorAfter(LONG, null, "C02").around, "C02");
  assert.equal(floorAfter(LONG, "C99", "C02").around, "C02");
  // A record puts thirty cells between the kept cell and the cursor.
  const newcomers = Array.from({ length: 30 }, (_, at) =>
    forage(`N${String(at).padStart(2, "0")}`),
  );
  const pushed = [...LONG.slice(0, 31), ...newcomers, ...LONG.slice(31)];
  assert.equal(floorAfter(pushed, "C55", "C30").around, "C30");
  // A record grows a cell from the cursor that the kept floor leaves off.
  assert.equal(floorAfter(startedOn(LONG, "C05", "C20"), "C55", "C20").around, "C20");
});

it("draws the cursor after a redraw around any kept cell, keeping it there while it can", () => {
  for (const { cell: kept } of GROWN.filter((_, at) => at % 9 === 0)) {
    const there = floorFor(GROWN, kept);
    for (const { cell: cursor } of GROWN) {
      const floor = floorAfter(GROWN, kept, cursor);
      const drawn = compartments(floorOf(GROWN, cursor, "kestrel", floor.around));
      assert.ok(drawn.includes(cursor), `${kept}, ${cursor}: ${drawn.join(" ")}`);
      assert.equal(stepsOff(floor, cursor), false, `${kept}, ${cursor}`);
      // Kept there, it holds still.
      assert.ok(!there.holds.has(cursor) || floor.around === kept, `${kept}, ${cursor}`);
    }
  }
});

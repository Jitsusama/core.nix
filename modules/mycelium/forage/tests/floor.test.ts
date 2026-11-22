// Which cells the forage page's floor draws when the list holds more than
// it can: those around the cell the cursor is on, with every cell that one
// grew from, so a booked start far down a long list still draws solid.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { floorOf } from "../draw.ts";
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

/** Sixty top cells, C00 to C59, with a starter at C10 and the cell it booked at C55. */
const LONG: readonly Forage[] = Array.from({ length: 60 }, (_, at) => {
  const cell = `C${String(at).padStart(2, "0")}`;
  return at === 55 ? forage(cell, { startedBy: "C10", booked: true, depth: 1 }) : forage(cell);
});

/** The cells `floor` draws a compartment for, in the order it draws them. */
const compartments = (floor: string): string[] =>
  [...floor.matchAll(/<rect class="[^"]*" data-cell="([^"]+)"/gu)].map((match) => match[1] ?? "");

it("draws every cell of a list that fits, whatever the cursor", () => {
  const short = LONG.slice(0, 5);
  assert.deepEqual(compartments(floorOf(short, "C04", "kestrel")), [
    "C00",
    "C01",
    "C02",
    "C03",
    "C04",
  ]);
});

it("draws the list's first 48 with no cursor, and counts the rest", () => {
  const floor = floorOf(LONG, null, "kestrel");
  const drawn = compartments(floor);
  assert.equal(drawn.length, 48);
  assert.equal(drawn[0], "C00");
  assert.equal(drawn.at(-1), "C47");
  assert.ok(floor.includes("and 12 more in the list"), floor);
  // C55's starter is drawn, but C55 isn't, so no branch is.
  assert.ok(!floor.includes('data-cell="C55"'), floor);
});

it("draws a cursor past the 48th with its starter, so its booked branch is solid", () => {
  const floor = floorOf(LONG, "C55", "kestrel");
  const drawn = compartments(floor);
  assert.equal(drawn.length, 48);
  assert.ok(drawn.includes("C55") && drawn.includes("C10"), drawn.join(" "));
  // The rest are the cells nearest it, kept in the list's order.
  assert.equal(drawn.at(-1), "C59");
  assert.deepEqual(
    drawn,
    [...drawn].sort((one, other) => one.localeCompare(other)),
  );
  assert.ok(floor.includes('<path class="hypha" data-cell="C55"'), floor);
  assert.ok(floor.includes('<rect class="whole on" data-cell="C55"'), floor);
});

it("follows a ring of starts only once round", () => {
  const ring = LONG.map((each) =>
    each.cell === "C10" ? { ...each, startedBy: "C55", depth: 1 } : each,
  );
  const drawn = compartments(floorOf(ring, "C55", "kestrel"));
  assert.equal(drawn.length, 48);
  assert.equal(new Set(drawn).size, 48);
});

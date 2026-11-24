// Which skipped cells the floor folds under a drawn cell: only all that is
// left of its brood, grandchildren among them, with the next cell drawn not
// of it, and never a cell that only shares a ring with it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { kinOf } from "../lineage.ts";
import { grewFrom } from "../model.ts";
import { forage, named } from "./floors.ts";

/**
 * In the list's order: C00 started C01 and C03, and C01 started C02; C04
 * started C05; C06 to C08 started each other in a ring, read from C06,
 * and C06 started C09; C10 started on its own.
 */
const STARTERS = [
  undefined,
  "C00",
  "C01",
  "C00",
  undefined,
  "C04",
  "C08",
  "C06",
  "C07",
  "C06",
  undefined,
];
const { rest } = kinOf(STARTERS.map((startedBy, at) => forage(named(at), { startedBy })), grewFrom);

it("folds the rest of a brood, grandchildren and all, up to a cell not of it", () => {
  assert.equal(rest(0, 1, 4), true);
  assert.equal(rest(1, 2, 3), true);
  assert.equal(rest(4, 5, 6), true);
});

it("keeps a gap holding a cell outside the brood, wherever in it", () => {
  assert.equal(rest(0, 1, 5), false);
  assert.equal(rest(0, 4, 5), false);
});

it("keeps a gap before a cell of the same brood, so the brood isn't closed", () => {
  assert.equal(rest(0, 1, 3), false);
});

it("keeps a gap before a cell that only shares a ring with the drawn one", () => {
  assert.equal(rest(8, 9, 10), false);
});

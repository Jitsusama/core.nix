// What a folded list shows: the first seven, the rows the page keeps
// wherever they rank, how many more there are, and all of them once open.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { folded, foldSaid, SHOWN } from "../fold.ts";

const rows = Array.from({ length: 20 }, (_, at) => at);

it("a folded list shows the first seven and counts the rest", () => {
  const fold = folded({ rows, open: false });
  assert.deepEqual(fold.rows, [0, 1, 2, 3, 4, 5, 6]);
  assert.equal(fold.more, 13);
  assert.equal(SHOWN, 7);
});

it("a short list shows whole, with nothing folded", () => {
  const fold = folded({ rows: [1, 2, 3], open: false });
  assert.deepEqual(fold.rows, [1, 2, 3]);
  assert.equal(fold.more, 0);
});

it("a kept row shows wherever it ranks, in its place, and takes one of the seven", () => {
  const fold = folded({ rows, open: false, kept: (row) => row === 15 });
  assert.deepEqual(fold.rows, [0, 1, 2, 3, 4, 5, 15]);
  assert.equal(fold.more, 13);
});

it("rows the page counts but never lists are folded with the rest", () => {
  const fold = folded({ rows, open: false, count: 124 });
  assert.equal(fold.rows.length, 7);
  assert.equal(fold.more, 117);
});

it("a count under the rows held is read as the rows, so more is never negative", () => {
  assert.equal(folded({ rows, open: false, count: 3 }).more, 13);
});

it("open, every row held shows, and only those never listed stay counted", () => {
  assert.deepEqual(folded({ rows, open: true }), { rows, more: 0 });
  assert.equal(folded({ rows, open: true, count: 124 }).more, 104);
});

it("the button says how many more, of what kind, and how to fold them again", () => {
  assert.equal(foldSaid(13, false), "And 13 more");
  assert.equal(foldSaid(117, false, "snapshot"), "And 117 more snapshots");
  assert.equal(foldSaid(1, false, "snapshot"), "And 1 more snapshot");
  assert.equal(foldSaid(13, true), "Fold them away");
});

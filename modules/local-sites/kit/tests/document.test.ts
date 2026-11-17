// Which part of a document its reader is in, and which part a step goes
// from. Where the step lands is steps.test.ts's.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { partAt, partFrom } from "../document.ts";

const tops = [120, 900, 1800];

it("the part being read is the last one starting at or above the reading line", () => {
  assert.equal(partAt(tops, 120), 0, "a part starting on the line is the one read");
  assert.equal(partAt(tops, 1799), 1, "the line short of the third part is still in the second");
  assert.equal(partAt(tops, 5000), 2, "past the last part's top is the last part");
});

it("above the first part, no part is being read", () => {
  assert.equal(partAt(tops, 40), -1);
  assert.equal(partAt([], 40), -1, "a document with no parts reads none");
});

it("a step goes from the part last stepped to while it's on screen", () => {
  // The page is scrolled to its end: the last two parts can't reach the line.
  const atTheEnd = [-900, 30, 400, 700];
  assert.equal(partFrom(2, atTheEnd, 800), 2, "a part short of the line is still where j left off");
  assert.equal(partFrom(-1, atTheEnd, 800), 1, "with none stepped to, the part at the line");
});

it("once the part stepped to is scrolled away, a step goes from the reading line", () => {
  const scrolledOn = [-2000, -1200, -300, 40];
  assert.equal(partFrom(1, scrolledOn, 800), 3, "a part above the window is left behind");
  assert.equal(partFrom(3, [60, 900, 1700, 2500], 800), 0, "and so is one below it");
});

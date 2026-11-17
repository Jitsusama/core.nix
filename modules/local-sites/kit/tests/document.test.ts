// Which part of a document its reader is in, where a step takes them, and
// when two presses of g come close enough to go to the top.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { partAt, partFrom, stepped, twice } from "../document.ts";

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

it("a step goes to the next or the last part and stops at the ends", () => {
  assert.equal(stepped(0, 1, 3), 1, "j from the first goes to the second");
  assert.equal(stepped(2, 1, 3), 2, "j from the last stays there");
  assert.equal(stepped(1, -1, 3), 0, "k from the second goes to the first");
  assert.equal(stepped(0, -1, 3), 0, "k from the first stays there");
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

it("from above the first part, j reaches the first", () => {
  assert.equal(stepped(-1, 1, 3), 0);
});

it("g counts as twice only when its second press comes soon after the first", () => {
  assert.equal(twice(1000, 1500), true, "half a second on is twice");
  assert.equal(twice(1000, 1700), false, "most of a second on is a fresh first press");
  assert.equal(twice(Number.NEGATIVE_INFINITY, 1000), false, "the first press ever is a first");
});

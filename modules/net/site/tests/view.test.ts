// Where the map scrolls to bring a device into view, one way at a time.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { shownFrom } from "../view.ts";

it("a device already in view leaves the map where it is", () => {
  assert.equal(shownFrom({ start: 100, end: 200 }, 50, 400), 50);
});

it("a device past the near edge is shown from its start", () => {
  assert.equal(shownFrom({ start: 100, end: 200 }, 150, 400), 100);
});

it("a device past the far edge is shown to its end", () => {
  assert.equal(shownFrom({ start: 500, end: 600 }, 0, 400), 200);
});

it("a device wider than the room is shown from its start, where its name is", () => {
  assert.equal(shownFrom({ start: 100, end: 700 }, 0, 400), 100);
});

// How long Code waits before drawing again, so a slow draw never takes the whole page.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { IDLE_PER_DRAW, UNDRAWN, waitBeforeDrawing } from "../pace.ts";

const GAP = 500;

it("draws at once when nothing has been drawn yet", () => {
  assert.equal(waitBeforeDrawing(UNDRAWN, 1000, GAP, true), 0, "the first draw waited");
  assert.equal(waitBeforeDrawing(UNDRAWN, 1000, GAP, false), 0, "the first draw waited");
});

it("keeps the gap from the last draw's end once pacing", () => {
  const drawn = { end: 1000, cost: 10 };
  assert.equal(waitBeforeDrawing(drawn, 1100, GAP, true), 400, "the gap wasn't kept");
  assert.equal(waitBeforeDrawing(drawn, 1600, GAP, true), 0, "a gap already past was waited");
});

it("leaves a few times a slow draw's cost idle, paced or not", () => {
  const slow = { end: 1000, cost: 300 };
  assert.equal(
    waitBeforeDrawing(slow, 1000, GAP, false),
    300 * IDLE_PER_DRAW,
    "a slow draw before the feeds arrived was followed at once",
  );
  assert.equal(
    waitBeforeDrawing(slow, 1000, GAP, true),
    300 * IDLE_PER_DRAW,
    "the gap won over a slow draw's own rest",
  );
});

it("draws on the next frame before the feeds arrive when the last draw was cheap", () => {
  assert.equal(
    waitBeforeDrawing({ end: 1000, cost: 2 }, 1016, GAP, false),
    0,
    "a cheap draw held the next back",
  );
});

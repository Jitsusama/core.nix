// How wide the map draws a line as it steps in and out.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { drawnWidth, HAIRLINE, TOUCHED_GROWS } from "../lines.ts";

/** Widths a line carries, a hairline's and lighter among them, lightest first. */
const CARRIED = [0.5, 1, 1.25, 1.5, 2.4, 2.5, 4, 6];

/** The zoom at the map's own size, the furthest the map steps in, and out. */
const OWN_SIZE = 1;
const STEPPED_IN = 3;
const STEPPED_OUT = 0.005;

/** How near two widths are to count as one, far under a device pixel. */
const SAME_PX = 1e-9;

it("draws every line as it carries it at the map's own size and stepped in", () => {
  for (const k of [OWN_SIZE, STEPPED_IN]) {
    assert.deepEqual(
      CARRIED.map((carried) => drawnWidth(carried, k)),
      CARRIED,
      `at zoom ${k} no line should thin`,
    );
  }
});

it("thins what a line carries past a hairline as the map steps out", () => {
  assert.equal(drawnWidth(4, 0.5), 2.5, "half the zoom should halve the 3 px past a hairline");
  assert.ok(drawnWidth(4, 0.999) < 4, "just past its own size a line should start thinning");
});

it("keeps a heavier line heavier than a lighter one however far the map steps out", () => {
  for (const k of [0.5, 0.34, 0.15, STEPPED_OUT]) {
    const drawn = CARRIED.filter((carried) => carried > HAIRLINE).map((carried) =>
      drawnWidth(carried, k),
    );
    for (const [at, width] of drawn.entries()) {
      assert.ok(
        at === 0 || width > (drawn[at - 1] ?? Number.POSITIVE_INFINITY),
        `at zoom ${k}, ${width} px should be wider than the lighter line's ${drawn[at - 1]} px`,
      );
    }
  }
});

it("draws no line under a hairline, and one at a hairline or under it as it is", () => {
  for (const k of [0.34, STEPPED_OUT]) {
    assert.deepEqual(
      [drawnWidth(0.5, k), drawnWidth(HAIRLINE, k)],
      [0.5, HAIRLINE],
      `at zoom ${k} a line no wider than a hairline should stay as it is`,
    );
    assert.ok(drawnWidth(6, k) > HAIRLINE, `at zoom ${k} a heavy line should stay over a hairline`);
  }
});

it("grows the line under the pointer by the same amount at every zoom", () => {
  for (const k of [STEPPED_IN, OWN_SIZE, 0.34, STEPPED_OUT]) {
    for (const carried of CARRIED) {
      // Widths add in floating point, so they match to far under a pixel.
      assert.ok(
        Math.abs(drawnWidth(carried, k, true) - drawnWidth(carried, k) - TOUCHED_GROWS) < SAME_PX,
        `at zoom ${k} a touched ${carried} px line should grow by ${TOUCHED_GROWS} px`,
      );
    }
  }
});

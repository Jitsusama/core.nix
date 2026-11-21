// How the map's motion is timed: a CSS time read, a flight's easing and a
// legend sample's keyframes.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { ARRIVE_DEFAULT_MS, durationOf, easedInOut, type Frame, goneAndBack } from "../motion.ts";

/** A sample's timing, the steps few enough to read. */
const TIMING = { ms: 1200, holdMs: 400, arriveMs: 150, steps: 4, ease: "ease-out" };

/** Where a mark rests, and each step of its way there. */
const REST: Frame = { translate: "0px 0px" };
const stepOf = (t: number): Frame => ({ translate: `${t * 10}px 0px`, opacity: t < 1 ? 1 : 0 });

it("reads a CSS time in milliseconds or in seconds", () => {
  assert.equal(durationOf("150ms"), 150, "milliseconds were misread");
  assert.equal(durationOf(" 0.25s "), 250, "seconds were misread");
  assert.equal(durationOf(".3s"), 300, "seconds without a leading zero were misread");
});

it("takes the theme's arriving duration when the stylesheet says none it can read", () => {
  for (const said of ["", "var(--arrive)", "fast", "-1s"]) {
    assert.equal(durationOf(said), ARRIVE_DEFAULT_MS, `${JSON.stringify(said)} gave no default`);
  }
});

it("eases a flight in and out, leaving and arriving still", () => {
  assert.equal(easedInOut(0), 0, "a flight should start where it leaves");
  assert.equal(easedInOut(0.5), 0.5, "a flight should be halfway at half its time");
  assert.equal(easedInOut(1), 1, "a flight should end where it lands");
  assert.ok(easedInOut(0.1) < 0.1, "a flight should leave slower than it goes");
  assert.ok(easedInOut(0.9) > 0.9, "a flight should arrive slower than it goes");
});

it("runs a sample's keyframes forward from its start to its end", () => {
  const { frames, total } = goneAndBack(TIMING, stepOf, REST);
  const offsets = frames.map((frame) => Number(frame["offset"]));
  assert.equal(offsets[0], 0, "a sample should start at its first keyframe");
  assert.equal(offsets.at(-1), 1, "a sample should end at its last keyframe");
  const backward = offsets.findIndex((offset, i) => i > 0 && offset < (offsets[i - 1] ?? 0));
  assert.equal(backward, -1, `keyframes ran backward at ${backward}: ${offsets.join(", ")}`);
  assert.equal(total, 1750, "a sample should take its move, its hold and its arrival");
});

it("moves a sample step by step, then holds it gone, then brings it back to rest", () => {
  const { frames } = goneAndBack(TIMING, stepOf, REST);
  const moving = frames.slice(0, TIMING.steps + 1).map(({ offset: _offset, ...frame }) => frame);
  const expected = [0, 0.25, 0.5, 0.75, 1].map(stepOf);
  assert.deepEqual(moving, expected, "a sample should move as its steps say");
  const landed = Number(frames[TIMING.steps]?.["offset"]);
  assert.equal(landed, 1200 / 1750, "a sample should land once its move's time is up");
  assert.deepEqual(
    frames.at(-2),
    { offset: 1600 / 1750, ...REST, opacity: 0, easing: "ease-out" },
    "a sample should wait gone at rest, then arrive on the theme's easing",
  );
  assert.deepEqual(
    frames.at(-1),
    { offset: 1, ...REST, opacity: 1 },
    "a sample should end at rest",
  );
});

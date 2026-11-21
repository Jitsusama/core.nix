// The geometry of the map's ground layer: what lies off it, the inks it
// reads, lines knocked out round names, the lines it says, and which line
// or ground a point touches.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  type Box,
  dashOf,
  GROUND_PAD,
  type Ground,
  groundAt,
  holeClips,
  KNOCK_PX,
  knockedOut,
  type Line,
  LINE_STEPS,
  lineAt,
  lineSteps,
  offCanvas,
  onScreen,
  overlap,
  paintOf,
  quadAt,
  saidLines,
  TOUCH_PX,
  type Touchable,
} from "../canvas.ts";

/** The view at the map's own size, its origin at the canvas's corner. */
const OWN = { k: 1, tx: 0, ty: 0 };

/** A straight line along y = `y` from x = 0 to x = `to`. */
const across = (y: number, to = 100): Line => [
  [0, y],
  [to / 2, y],
  [to, y],
];

/** A line that opens `id`, inside a ground or not. */
const opening = (line: Line, id: string, within = false): Touchable =>
  Object.assign([line[0], line[1], line[2]] as const, { opens: id, within });

it("finds a bowed line's points along it", () => {
  const bowed: Line = [
    [0, 0],
    [50, 100],
    [100, 0],
  ];
  assert.deepEqual(quadAt(bowed, 0), [0, 0], "a line should start at its start");
  assert.deepEqual(quadAt(bowed, 1), [100, 0], "a line should end at its end");
  assert.deepEqual(quadAt(bowed, 0.5), [50, 50], "a line should bow halfway to its control");
});

it("puts a line in map units on screen at the view's zoom and offset", () => {
  const view = { k: 2, tx: 10, ty: -5 };
  assert.deepEqual(onScreen(across(3, 10), view), [
    [10, 1],
    [20, 1],
    [30, 1],
  ]);
});

it("paints what reaches into the canvas or its pad, and nothing wholly past them", () => {
  assert.equal(offCanvas([10, 10, 20, 20], 100, 100), false, "a shape inside should paint");
  const edge = -GROUND_PAD + 1;
  assert.equal(
    offCanvas([edge - 5, 10, edge, 20], 100, 100),
    false,
    "a shape in the pad should paint",
  );
  assert.equal(offCanvas([200, 10, 210, 20], 100, 100), true, "a shape past the right should not");
  assert.equal(offCanvas([10, -40, 20, -20], 100, 100), true, "a shape above should not");
});

it("reads a dash and a paint as the canvas takes them", () => {
  assert.deepEqual(dashOf("none"), [], "no dash should be a solid line");
  assert.deepEqual(dashOf("3px, 3px"), [3, 3], "a dash should keep its lengths");
  assert.deepEqual(dashOf("1 2"), [1, 2], "a dash spaced apart should read too");
  assert.equal(paintOf("none"), null, "no paint should be none");
  assert.equal(paintOf("transparent"), null, "a transparent paint should be none");
  // The browser writes a computed paint's alpha last; the paint is named
  // plainly here, since a colour is the theme's to spell.
  assert.equal(paintOf("paint(1, 2, 3, 0)"), null, "a clear paint should be none");
  assert.equal(paintOf("paint(1, 2, 3)"), "paint(1, 2, 3)", "a paint should be kept as said");
});

it("knocks a line out round a name, with room to spare", () => {
  const plate: Box = [10, 10, 20, 10];
  assert.ok(knockedOut(plate, 15, 15), "a point on the plate should be knocked out");
  assert.ok(knockedOut(plate, 10 - KNOCK_PX + 1, 15), "a point in the room should be too");
  assert.ok(!knockedOut(plate, 10 - KNOCK_PX - 1, 15), "a point past the room should not");
});

it("puts overlapping holes in clips of their own, and apart ones together", () => {
  const clips = holeClips([
    [0, 0, 10, 10],
    [5, 5, 10, 10],
    [100, 100, 10, 10],
  ]);
  assert.equal(clips.length, 2, `holes went in ${clips.length} clips`);
  assert.deepEqual(
    clips[0],
    [
      [-KNOCK_PX, -KNOCK_PX, 10 + 2 * KNOCK_PX, 10 + 2 * KNOCK_PX],
      [100 - KNOCK_PX, 100 - KNOCK_PX, 10 + 2 * KNOCK_PX, 10 + 2 * KNOCK_PX],
    ],
    "holes overlapping none should share the first clip, each with its room",
  );
  for (const clip of clips) {
    const crossing = clip.some((a, i) => clip.slice(i + 1).some((b) => overlap(a, b)));
    assert.ok(!crossing, "no clip should hold two holes that overlap");
  }
});

it("says a line in more points the longer it runs, never fewer than its least", () => {
  assert.equal(lineSteps(across(0, 6)), LINE_STEPS, "a short line should take the fewest");
  assert.equal(lineSteps(across(0, 300)), 100, "a long line should step at most 3 px");
});

it("says each line on the canvas, split where a name knocks it out", () => {
  assert.deepEqual(saidLines([across(-50)], 100, 100, []), [], "a line off the canvas is not said");
  const [whole] = saidLines([across(50)], 100, 100, []);
  assert.equal(whole?.length, LINE_STEPS + 1, "a line with no names on it takes the fewest points");
  const split = saidLines([across(50)], 100, 100, [[40, 45, 20, 10]]);
  assert.equal(split.length, 2, `a knocked-out line was said as ${split.length}`);
  const [left, right] = split;
  const ends = [left?.at(-1)?.[0] ?? 0, right?.[0]?.[0] ?? 0];
  assert.ok(ends[0] < 40 - KNOCK_PX && ends[1] > 60 + KNOCK_PX, `the halves end at ${ends}`);
});

it("touches the nearest line within reach, from the uppermost set holding one", () => {
  const lower = opening(across(10), "lower");
  const upper = opening(across(14), "upper");
  const touched = lineAt([[upper], [lower]], OWN, [50, 11], false);
  assert.equal(touched, upper, "a line over another should be touched first, though further");
  assert.equal(
    lineAt([[], [lower]], OWN, [50, 11], false),
    lower,
    "with none above, the one below",
  );
  const far = lineAt([[lower]], OWN, [50, 10 + TOUCH_PX + 1], false);
  assert.equal(far, null, "a line out of reach should not be touched");
  const inert: Touchable = across(10);
  assert.equal(lineAt([[inert]], OWN, [50, 10], false), null, "a line that opens nothing is not");
});

it("touches the nearest of a set's lines, wherever it is listed", () => {
  const further = opening(across(10), "further");
  const nearer = opening(across(13), "nearer");
  for (const layer of [
    [further, nearer],
    [nearer, further],
  ]) {
    const touched = lineAt([layer], OWN, [50, 12], false);
    assert.equal(touched, nearer, `of ${layer.map((l) => l.opens)}, the nearer should be touched`);
  }
});

it("finds a line in map units under a point on screen, at any zoom", () => {
  const view = { k: 2, tx: 100, ty: 40 };
  const line = opening(across(10), "line");
  const [x, y] = [100 + 50 * 2, 40 + 10 * 2 + 1];
  assert.equal(lineAt([[line]], view, [x, y], false), line, "the line on screen should be touched");
  const unzoomed = lineAt([[line]], view, [50, 11], false);
  assert.equal(unzoomed, null, "the line's place in map units should not be, on screen");
  const past = [100 + 50 * 2, 40 + 10 * 2 + TOUCH_PX + 1] as const;
  assert.equal(lineAt([[line]], view, past, false), null, "reach should be px, at any zoom");
});

it("leaves a network's own lines to its ground", () => {
  const inside = opening(across(10), "inside", true);
  assert.equal(lineAt([[inside]], OWN, [50, 10], true), null, "on its ground the ground opens");
  assert.equal(lineAt([[inside]], OWN, [50, 10], false), inside, "off it, the line opens");
});

it("finds the innermost ground holding a point", () => {
  const grounds: Ground[] = [
    ["outer", 0, 0, 100],
    ["inner", 10, 0, 20],
  ];
  assert.equal(groundAt(grounds, 12, 0), "inner", "the innermost ground should hold a point");
  assert.equal(groundAt(grounds, -50, 0), "outer", "the outer alone should hold its own");
  assert.equal(groundAt(grounds, 500, 0), null, "a point on no ground should find none");
});

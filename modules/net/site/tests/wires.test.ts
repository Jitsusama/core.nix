// Where the network map's wires run beside its boxes: their lanes kept to a
// phone's width while they fit, the boxes moved right once even the closest
// lanes don't.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { networkOf } from "../model.ts";
import { lanesWithin, type Wire } from "../wires.ts";
import { told } from "./house.ts";

/** The stacked plan's box: room for three lanes at their usual step. */
const STACKED_BOX = 40;
/** The plan across's box, far right of its lanes. */
const ACROSS_BOX = 200;

/** `lanes` wires up the map's left, each taking a lane, and `down` cables taking none. */
function wires(lanes: number, down = 0): Wire[] {
  const { watched, kinds } = told();
  const link = networkOf(watched, kinds)?.links[0];
  assert.ok(link, "the test house holds a link to draw");
  const up = Array.from({ length: lanes }, (_, n) => ({ from: `eero:${n}`, to: "modem", link }));
  const cable = { from: "eero:1", to: "modem", link, down: true };
  return [...up, ...Array.from({ length: down }, () => cable)];
}

it("three lanes keep their usual step within the stacked box", () => {
  assert.deepEqual(lanesWithin(wires(3, 1), STACKED_BOX), { box: STACKED_BOX, step: 10 });
});

it("a fourth lane runs them closer rather than widening the map", () => {
  const { box, step } = lanesWithin(wires(4), STACKED_BOX);
  assert.equal(box, STACKED_BOX);
  assert.ok(step < 10 && step >= 6, `four lanes run ${step} apart`);
});

it("lanes run no closer than a thick wire's gap, the boxes moving right instead", () => {
  assert.deepEqual(lanesWithin(wires(5), STACKED_BOX), { box: 44, step: 6 });
  assert.deepEqual(lanesWithin(wires(8), STACKED_BOX), { box: 62, step: 6 });
});

it("one lane, or none, keeps the box and the usual step", () => {
  assert.deepEqual(lanesWithin(wires(1), STACKED_BOX), { box: STACKED_BOX, step: 10 });
  assert.deepEqual(lanesWithin(wires(0, 1), STACKED_BOX), { box: STACKED_BOX, step: 10 });
});

it("across, the box leaves a dozen lanes their usual step", () => {
  assert.deepEqual(lanesWithin(wires(12), ACROSS_BOX), { box: ACROSS_BOX, step: 10 });
});

// The shape of a reading: what changes it, so the map draws whole, and
// what leaves it, so a reading is only restyled.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import type { Point } from "../layout.ts";
import {
  type ShapedCell,
  type ShapedHub,
  type ShapedReading,
  type ShapeReads,
  SWELL_STEP,
  shapeOf,
} from "../shape.ts";

const HUB: ShapedHub = { key: "repo.kestrel", cells: [1, 2], shape: "repository", feeds: [] };
const CELL: ShapedCell = { id: "c1", address: "TERN01", standing: "works" };
const KNOT = { key: "gull", shape: "repository", cells: [CELL] };
const READING: ShapedReading = { hubs: [HUB], cells: [CELL], shared: [KNOT] };
const PLACES = new Map<string, Point>([["c1", [10, 20]]]);
const READS: ShapeReads = {
  live: () => true,
  swollen: () => 0,
  sown: () => ["c1", null],
  at: (id) => PLACES.get(id),
  home: () => "repo.kestrel",
  net: () => "repo.kestrel",
  machines: ["kestrel"],
};
const SHAPE = shapeOf(READING, READS);

it("reads one reading as one shape each time", () => {
  assert.equal(shapeOf({ ...READING }, { ...READS }), SHAPE, "a copy should read the same");
});

it("leaves out what a restyle reads afresh", () => {
  const turned = { ...CELL, standing: "waits", spent: 4, doing: "reading" };
  const shape = shapeOf({ ...READING, cells: [turned] }, READS);
  assert.equal(shape, SHAPE, "a cell's turn, spend and words should leave the shape");
});

it("changes with each thing the whole draw reads", () => {
  const changed: [string, ShapedReading, ShapeReads][] = [
    ["a hub's key", { ...READING, hubs: [{ ...HUB, key: "repo.osprey" }] }, READS],
    ["a hub's glyph", { ...READING, hubs: [{ ...HUB, shape: "project" }] }, READS],
    ["a hub at the edge", { ...READING, hubs: [{ ...HUB, edge: true }] }, READS],
    ["a hub working alone", { ...READING, hubs: [{ ...HUB, alone: true }] }, READS],
    ["a hub's cells", { ...READING, hubs: [{ ...HUB, cells: [1] }] }, READS],
    ["a hub's tree", { ...READING, hubs: [{ ...HUB, within: "quest.osprey" }] }, READS],
    ["a hub's machine", { ...READING, hubs: [{ ...HUB, machine: "osprey" }] }, READS],
    ["a hub's feeds", { ...READING, hubs: [{ ...HUB, feeds: [["heron", 1]] }] }, READS],
    ["a hub's liveness", READING, { ...READS, live: () => false }],
    ["a hub's seeds", READING, { ...READS, sown: () => ["c1", "c2"] }],
    ["a cell's id", { ...READING, cells: [{ ...CELL, id: "c2" }] }, READS],
    ["a cell's address", { ...READING, cells: [{ ...CELL, address: "TERN02" }] }, READS],
    ["a cell asking", { ...READING, cells: [{ ...CELL, standing: "asks" }] }, READS],
    ["a cell's home", READING, { ...READS, home: () => "repo.osprey" }],
    ["a cell's network", READING, { ...READS, net: () => undefined }],
    ["a cell's place", READING, { ...READS, at: () => [10, 21] }],
    ["a cell unplaced", READING, { ...READS, at: () => undefined }],
    ["a cell's extra networks", { ...READING, cells: [{ ...CELL, also: [HUB] }] }, READS],
    ["a knot gone", { ...READING, shared: [] }, READS],
    ["a knot's glyph", { ...READING, shared: [{ ...KNOT, shape: "group" }] }, READS],
    ["a knot's cells", { ...READING, shared: [{ ...KNOT, cells: [{ id: "c2" }] }] }, READS],
    ["the machines drawn", READING, { ...READS, machines: [] }],
  ];
  for (const [what, reading, reads] of changed) {
    assert.notEqual(shapeOf(reading, reads), SHAPE, `${what} should change the shape`);
  }
});

it("draws again once a waits ring has grown past half a step, and once a step after", () => {
  const swollen = (by: number): string => shapeOf(READING, { ...READS, swollen: () => by });
  const half = SWELL_STEP / 2;
  assert.equal(swollen(half * 0.9), SHAPE, "a ring grown just under half a step should not");
  const once = swollen(half * 1.1);
  assert.notEqual(once, SHAPE, "a ring grown just past half a step should");
  assert.equal(swollen(half * 2.9), once, "growing on short of the next middle should not");
  assert.notEqual(swollen(half * 3.1), once, "passing the next step's middle should");
});

it("keeps a place to a tenth of a unit", () => {
  const nudged = shapeOf(READING, { ...READS, at: () => [10.01, 20.02] });
  assert.equal(nudged, SHAPE, "a nudge under a tenth should leave the shape");
});

it("can't be fooled by a separator in a name", () => {
  const hubbed = (key: string, within: string): string =>
    shapeOf({ ...READING, hubs: [{ ...HUB, key, within }] }, READS);
  const shapes = [hubbed("gull", "tern,heron"), hubbed("gull,tern", "heron")];
  assert.notEqual(shapes[0], shapes[1], "a separator moved between names should change the shape");
});

// What the network page makes of the mycelium node's cells and machines,
// told as the node's feed tells them: each record its key and kind, a
// machine's room nested as the node measures it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { homeOf, myceliumOf } from "../mycelium.ts";

const machine = {
  key: "machine:skiff",
  kind: "machine",
  name: "skiff",
  memory: 64_000_000_000,
  disk: 1_000_000_000_000,
  disk_free: 180_000_000_000,
  room: {
    memory_available: 25_000_000_000,
    pressure: { cpu: 28.8, memory: 8.4, io: 11.9 },
    over: ["cpu"],
  },
};

const cell = (name: string, standing: string, on = "skiff", type = "session") => ({
  key: `cell:${name}`,
  kind: "cell",
  name,
  type,
  machine: on,
  standing,
});

it("a machine's room is read from where the node nests it", () => {
  const [skiff] = myceliumOf([], [machine]).machines;
  assert.deepEqual(
    {
      available: skiff?.available,
      pressure: skiff?.pressure,
      over: skiff?.over,
      diskFree: skiff?.diskFree,
    },
    {
      available: 25_000_000_000,
      pressure: { cpu: 28.8, memory: 8.4, io: 11.9 },
      over: ["cpu"],
      diskFree: 180_000_000_000,
    },
  );
});

it("a machine's cells are counted by how each stands, its own cells alone", () => {
  const cells = [
    cell("OSPREY", "needs-you"),
    cell("HERON", "moving"),
    cell("TERN", "moving"),
    cell("LOON", "quiet", "dory"),
  ];
  const [skiff] = myceliumOf(cells, [machine]).machines;
  assert.deepEqual(
    [...(skiff?.cells ?? [])],
    [
      ["needs-you", 1],
      ["moving", 2],
    ],
  );
});

it("the machine's own cell isn't one of the cells running on it", () => {
  const [skiff] = myceliumOf([cell("skiff", "quiet", "skiff", "machine")], [machine]).machines;
  assert.equal(skiff?.cells.size, 0);
});

it("a machine that measured no room reads as having none", () => {
  const [bare] = myceliumOf([], [{ key: "machine:dory", kind: "machine", name: "dory" }]).machines;
  assert.deepEqual([bare?.available, bare?.pressure, bare?.over], [undefined, undefined, []]);
});

it("a machine's home is its place on the mycelium map", () => {
  assert.equal(homeOf("skiff"), "mycelium/#machine=skiff");
});

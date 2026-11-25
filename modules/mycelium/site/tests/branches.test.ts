// Which network grows out of which, from where each cell came from: who
// started it and what its purpose split from, as lineage.test.ts proves the
// map keeps them. These prove the choice of trunk, what a branch says of how
// its cells came, and that no branch loops back round to itself.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { grownOf, type Network, type Origin, originsOf, type Started } from "../lineage.ts";

/** A network the node sets within `within`, named `name`, its cells drawn in it. */
function network(
  key: string,
  cells: readonly string[],
  within: string | null = null,
  name = key,
): Network {
  return { key, name, within, cells };
}

/** What grows out of what among `networks`, each cell drawn in the network that lists it. */
function grown(networks: readonly Network[], origins: readonly Origin[]) {
  const hubOf = new Map(networks.flatMap((each) => each.cells.map((cell) => [cell, each.key])));
  return Object.fromEntries(grownOf(networks, hubOf, origins));
}

/** Each of `cells` started by `by`. */
function startedBy(by: string, ...cells: string[]): Started[] {
  return cells.map((cell) => ({ cell, by }));
}

it("a network whose cell split from another's, no start said, grows out of it", () => {
  const networks = [network("skiff", ["SKIFF1"]), network("heron", ["HERON1", "HERON2"])];
  const origins = originsOf([], [{ cell: "HERON1", from: "SKIFF1" }]);
  assert.deepEqual(grown(networks, origins), {
    heron: { from: "skiff", by: [], cells: 1, started: 0, split: { from: ["SKIFF1"], cells: 1 } },
  });
});

it("a start from inside or from a receptor leaves the cell's split to grow its branch", () => {
  const networks = [network("skiff", ["SKIFF1"]), network("heron", ["HERON1", "HERON2"])];
  const origins = originsOf(
    [...startedBy("HERON2", "HERON1"), ...startedBy("calls@kestrel", "HERON2")],
    [
      { cell: "HERON1", from: "SKIFF1" },
      { cell: "HERON2", from: "SKIFF1" },
    ],
  );
  assert.deepEqual(grown(networks, origins), {
    heron: { from: "skiff", by: [], cells: 2, started: 0, split: { from: ["SKIFF1"], cells: 2 } },
  });
});

it("a cell started from one network and split from the trunk counts as split from it", () => {
  const networks = [
    network("skiff", ["SKIFF1"]),
    network("gull", ["GULL01"]),
    network("heron", ["HERON1", "HERON2"]),
  ];
  const origins = originsOf(startedBy("SKIFF1", "HERON1"), [
    { cell: "HERON1", from: "GULL01" },
    { cell: "HERON2", from: "GULL01" },
  ]);
  assert.deepEqual(grown(networks, origins), {
    heron: { from: "gull", by: [], cells: 2, started: 0, split: { from: ["GULL01"], cells: 2 } },
  });
});

it("a start whose branch would loop leaves the network to grow from its split", () => {
  const networks = [
    network("aaa", ["AUK001", "AUK002"]),
    network("bbb", ["BRANT1", "BRANT2", "BRANT3", "BRANT4"]),
    network("ccc", ["CRANE1"]),
  ];
  const origins = originsOf(
    [...startedBy("AUK001", "BRANT1", "BRANT2", "BRANT3"), ...startedBy("BRANT4", "AUK002")],
    [{ cell: "AUK002", from: "CRANE1" }],
  );
  assert.deepEqual(grown(networks, origins), {
    aaa: { from: "ccc", by: [], cells: 1, started: 0, split: { from: ["CRANE1"], cells: 1 } },
    bbb: { from: "aaa", by: ["AUK001"], cells: 3, started: 3 },
  });
});

it("a start and a split of one weight grow from the start, whatever their names", () => {
  const networks = [
    network("skiff", ["SKIFF1"]),
    network("gull", ["GULL01"]),
    network("heron", ["HERON1"]),
  ];
  const origins = originsOf(startedBy("SKIFF1", "HERON1"), [{ cell: "HERON1", from: "GULL01" }]);
  assert.deepEqual(grown(networks, origins), {
    heron: { from: "skiff", by: ["SKIFF1"], cells: 1, started: 1 },
  });
});

it("a network whose strongest start would loop grows out of its next", () => {
  const networks = [
    network("aaa", ["AUK001", "AUK002", "AUK003"]),
    network("bbb", ["BRANT1", "BRANT2", "BRANT3"]),
    network("ccc", ["CRANE1"]),
  ];
  const origins = [
    ...startedBy("AUK001", "BRANT1", "BRANT2", "BRANT3"),
    ...startedBy("BRANT1", "AUK001", "AUK002"),
    ...startedBy("CRANE1", "AUK003"),
  ];
  assert.deepEqual(
    grown(networks, origins),
    {
      bbb: { from: "aaa", by: ["AUK001"], cells: 3, started: 3 },
      aaa: { from: "ccc", by: ["CRANE1"], cells: 1, started: 1 },
    },
    "aaa's two starts from bbb would loop, so its one from ccc should grow it",
  );
});

it("a choice left over from a loop settles only after every stronger choice", () => {
  const networks = [
    network("aaa", ["AUK001", "AUK002", "AUK003", "AUK004"]),
    network("bbb", ["BRANT1", "BRANT2", "BRANT3", "BRANT4", "BRANT5"]),
    network("ccc", ["CRANE1", "CRANE2"]),
  ];
  const origins = [
    ...startedBy("AUK001", "BRANT1", "BRANT2", "BRANT3", "BRANT4", "BRANT5"),
    ...startedBy("BRANT1", "AUK001", "AUK002", "AUK003"),
    ...startedBy("CRANE1", "AUK004"),
    ...startedBy("AUK002", "CRANE1", "CRANE2"),
  ];
  assert.deepEqual(
    grown(networks, origins),
    {
      bbb: { from: "aaa", by: ["AUK001"], cells: 5, started: 5 },
      ccc: { from: "aaa", by: ["AUK002"], cells: 2, started: 2 },
    },
    "aaa's one cell from ccc must not settle ahead of ccc's two from aaa and make it loop",
  );
});

it("a network some of whose cells were started there and some split from it says each", () => {
  const networks = [
    network("skiff", ["SKIFF1", "SKIFF2"]),
    network("heron", ["HERON1", "HERON2", "HERON3"]),
  ];
  const origins = originsOf(startedBy("SKIFF1", "HERON1"), [
    { cell: "HERON1", from: "SKIFF2" },
    { cell: "HERON2", from: "SKIFF2" },
  ]);
  assert.deepEqual(grown(networks, origins), {
    heron: {
      from: "skiff",
      by: ["SKIFF1"],
      cells: 2,
      started: 1,
      split: { from: ["SKIFF2"], cells: 1 },
    },
  });
});

it("a network whose cell another network's cell started grows out of that network", () => {
  const networks = [network("skiff", ["SKIFF1"]), network("heron", ["HERON1", "HERON2"])];
  assert.deepEqual(grown(networks, startedBy("SKIFF1", "HERON1")), {
    heron: { from: "skiff", by: ["SKIFF1"], cells: 1, started: 1 },
  });
});

it("nothing grows where nobody is said to have started anybody", () => {
  assert.deepEqual(grown([network("skiff", ["SKIFF1"]), network("heron", ["HERON1"])], []), {});
});

it("where the node sets a network stands, whoever started its cells", () => {
  const networks = [
    network("skiff", ["SKIFF1"]),
    network("tern", ["TERN01"]),
    network("heron", ["HERON1"], "tern"),
  ];
  assert.deepEqual(grown(networks, startedBy("SKIFF1", "HERON1")), {});
});

it("a start from inside, from what sits in it or by a cell in no network grows nothing", () => {
  const networks = [
    network("skiff", ["SKIFF1", "SKIFF2", "SKIFF3"]),
    network("tern", ["TERN01"], "skiff"),
  ];
  const started = [
    ...startedBy("SKIFF1", "SKIFF2"),
    ...startedBy("TERN01", "SKIFF1"),
    ...startedBy("calls@kestrel", "SKIFF3"),
  ];
  assert.deepEqual(grown(networks, started), {});
});

it("a network grows out of the one that started more of its cells, a tie by name", () => {
  const networks = [
    network("aaa", ["AUK001"], null, "Zephyr"),
    network("bbb", ["BRANT1", "BRANT2"], null, "Anchor"),
    network("ccc", ["CRANE1", "CRANE2", "CRANE3"]),
    network("ddd", ["DUNLN1", "DUNLN2"]),
  ];
  const started = [
    ...startedBy("AUK001", "CRANE1"),
    ...startedBy("BRANT1", "CRANE2"),
    ...startedBy("BRANT2", "CRANE3"),
    ...startedBy("AUK001", "DUNLN1"),
    ...startedBy("BRANT1", "DUNLN2"),
  ];
  assert.deepEqual(grown(networks, started), {
    ccc: { from: "bbb", by: ["BRANT1", "BRANT2"], cells: 2, started: 2 },
    ddd: { from: "bbb", by: ["BRANT1"], cells: 1, started: 1 },
  });
});

it("of two that started each other's cells, the one that started more is the trunk", () => {
  const networks = [
    network("aaa", ["AUK001", "AUK002"]),
    network("bbb", ["BRANT1", "BRANT2", "BRANT3", "BRANT4"]),
  ];
  const started = [
    ...startedBy("AUK001", "BRANT1", "BRANT2", "BRANT3"),
    ...startedBy("BRANT4", "AUK002"),
  ];
  assert.deepEqual(grown(networks, started), {
    bbb: { from: "aaa", by: ["AUK001"], cells: 3, started: 3 },
  });
});

it("no branch is grown that would loop back round to the network it grows from", () => {
  const networks = [
    network("aaa", ["AUK001"]),
    network("bbb", ["BRANT1"]),
    network("ccc", ["CRANE1"]),
  ];
  const started = [
    ...startedBy("BRANT1", "AUK001"),
    ...startedBy("CRANE1", "BRANT1"),
    ...startedBy("AUK001", "CRANE1"),
  ];
  assert.deepEqual(grown(networks, started), {
    aaa: { from: "bbb", by: ["BRANT1"], cells: 1, started: 1 },
    bbb: { from: "ccc", by: ["CRANE1"], cells: 1, started: 1 },
  });
});

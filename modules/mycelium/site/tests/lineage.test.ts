// Each cell of this machine that works on a quest has a forage (the node's
// `forage` kind), whose lineage says who started it: these prove what the
// map keeps of that, that a node too old to serve forages reads as one
// that says nothing of who started whom, and which network grows out of
// which from it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { grownOf, type Network, type Started } from "../lineage.ts";
import { nowOf } from "../model.ts";

const AT = Date.parse("2026-11-19T01:00:00Z");

/** A forage as the node serves it, for `cell`, its lineage as given. */
function forage(
  cell: string,
  lineage: Record<string, unknown>,
  more: Record<string, unknown> = {},
) {
  return {
    key: `forage:${cell}`,
    kind: "forage",
    heard: AT,
    cell,
    quest: `QEST-20261119-${cell}`,
    lineage,
    ...more,
  };
}

/** The four watches the map waits on, empty, and the forage kind's, told `these`. */
function told(these: readonly object[] | undefined) {
  const heard = { network: new Held(), cell: new Held(), machine: new Held(), message: new Held() };
  if (these === undefined) {
    return heard;
  }
  const forages = new Held();
  forages.took({ records: these });
  return { ...heard, forage: forages };
}

it("a node that serves no forage says nothing of who started whom, not that nobody did", () => {
  assert.equal(nowOf(told(undefined), AT).started, null);
  assert.deepEqual(nowOf(told([]), AT).started, []);
});

it("a cell's starter is kept as the two addresses, never the rest of its forage", () => {
  const one = forage(
    "KESTRL",
    { started_by: "OSPREY", split_from: "TERNSS", parent: "QEST-20261101-OSPREY" },
    { purpose: "words" },
  );
  assert.deepEqual(nowOf(told([one]), AT).started, [{ cell: "KESTRL", by: "OSPREY" }]);
});

it("every cell with a forage is known, so its drawer can open it there, and none without", () => {
  const these = [
    forage("KESTRL", { started_by: "OSPREY" }),
    forage("ALONE1", {}),
    { key: "forage:NOCELL", kind: "forage", heard: AT },
  ];
  assert.deepEqual(
    [nowOf(told(these), AT).foraged, nowOf(told(undefined), AT).foraged],
    [["KESTRL", "ALONE1"], null],
    "each forage naming a cell should be known, and a node serving none should know none",
  );
});

it("a cell nobody started, or one said to have started itself, names no starter", () => {
  const these = [
    forage("ALONE1", { started_by: null, split_from: null, parent: null }),
    forage("SELF01", { started_by: "SELF01" }),
    forage("BLANK1", { started_by: "" }),
    forage("NOLINE", {}),
    { key: "forage:NOLIN2", kind: "forage", heard: AT, cell: "NOLIN2" },
  ];
  assert.deepEqual(nowOf(told(these), AT).started, []);
});

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
function grown(networks: readonly Network[], started: readonly Started[] | null) {
  const hubOf = new Map(networks.flatMap((each) => each.cells.map((cell) => [cell, each.key])));
  return Object.fromEntries(grownOf(networks, hubOf, started));
}

/** Each of `cells` started by `by`. */
function startedBy(by: string, ...cells: string[]): Started[] {
  return cells.map((cell) => ({ cell, by }));
}

it("a network whose cell another network's cell started grows out of that network", () => {
  const networks = [network("skiff", ["SKIFF1"]), network("heron", ["HERON1", "HERON2"])];
  assert.deepEqual(grown(networks, startedBy("SKIFF1", "HERON1")), {
    heron: { from: "skiff", by: ["SKIFF1"], cells: 1 },
  });
});

it("nothing grows where the node serves no forages", () => {
  assert.deepEqual(grown([network("skiff", ["SKIFF1"]), network("heron", ["HERON1"])], null), {});
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
    ccc: { from: "bbb", by: ["BRANT1", "BRANT2"], cells: 2 },
    ddd: { from: "bbb", by: ["BRANT1"], cells: 1 },
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
    bbb: { from: "aaa", by: ["AUK001"], cells: 3 },
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
    aaa: { from: "bbb", by: ["BRANT1"], cells: 1 },
    bbb: { from: "ccc", by: ["CRANE1"], cells: 1 },
  });
});

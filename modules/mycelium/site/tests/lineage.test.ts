// Each cell's record says who started it, on every machine, and each cell
// of this machine that works on a quest has a forage (the node's `forage`
// kind), whose lineage says so too: these prove what the map keeps of
// that, that the record's word comes first, that a node saying neither
// names nobody, and which network grows out of which from it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { grownOf, isLaterCopy, type Network, type Started } from "../lineage.ts";
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

/** A cell's record as the node serves it, on `machine`, its fields as given. */
function record(name: string, machine: string, more: Record<string, unknown> = {}) {
  return { key: `cell:${name}`, kind: "cell", heard: AT, name, type: "session", machine, ...more };
}

/** The four watches the map waits on, the cells told `cells`, and the forages told `these`. */
function told(these: readonly object[] | undefined, cells: readonly object[] = []) {
  const heard = { network: new Held(), cell: new Held(), machine: new Held(), message: new Held() };
  heard.cell.took({ records: cells });
  if (these === undefined) {
    return heard;
  }
  const forages = new Held();
  forages.took({ records: these });
  return { ...heard, forage: forages };
}

it("a node whose records name no starter and that serves no forage names nobody", () => {
  assert.deepEqual(nowOf(told(undefined), AT).started, []);
  assert.deepEqual(nowOf(told([]), AT).started, []);
});

it("a cell's own record names its starter on any machine, where no forage is served", () => {
  const cells = [
    record("HERON1", "osprey", { started_by: "SKIFF1", started_by_from: "book" }),
    record("SKIFF1", "kestrel"),
  ];
  assert.deepEqual(nowOf(told(undefined, cells), AT).started, [{ cell: "HERON1", by: "SKIFF1" }]);
});

it("a cell's own record is believed before its forage, which fills in where it is silent", () => {
  const cells = [record("HERON1", "osprey", { started_by: "SKIFF1" }), record("TERN03", "kestrel")];
  const these = [
    forage("HERON1", { started_by: "GULL01" }),
    forage("TERN03", { started_by: "TERN01" }),
  ];
  const byCell = new Map(nowOf(told(these, cells), AT).started.map((s) => [s.cell, s.by]));
  assert.deepEqual(
    Object.fromEntries(byCell),
    { HERON1: "SKIFF1", TERN03: "TERN01" },
    "the record should name HERON1's starter over its forage, and TERN03's forage fill in",
  );
});

it("a cell two machines list while it moves names the starter of the copy started latest", () => {
  const stale = record("HERON1", "osprey", {
    key: "cell:HERON1@osprey",
    since: "2026-11-19T00:00:00Z",
    started_by: "GULL01",
  });
  const moved = record("HERON1", "kestrel", {
    since: "2026-11-19T00:30:00Z",
    started_by: "SKIFF1",
  });
  assert.deepEqual(nowOf(told(undefined, [stale, moved]), AT).started, [
    { cell: "HERON1", by: "SKIFF1" },
  ]);
  assert.deepEqual(
    nowOf(told(undefined, [moved, stale]), AT).started,
    [{ cell: "HERON1", by: "SKIFF1" }],
    "the copy told first should not win over the one started latest",
  );
});

it("of two copies of a cell started alike, or both silent on it, the first told stands", () => {
  const at = { since: "2026-11-19T00:00:00Z" };
  const first = record("HERON1", "osprey", {
    ...at,
    key: "cell:HERON1@osprey",
    started_by: "GULL01",
  });
  const second = record("HERON1", "kestrel", { ...at, started_by: "SKIFF1" });
  assert.deepEqual(nowOf(told(undefined, [first, second]), AT).started, [
    { cell: "HERON1", by: "GULL01" },
  ]);
  const { since: _first, ...unsaidFirst } = first;
  const { since: _second, ...unsaidSecond } = second;
  assert.deepEqual(nowOf(told(undefined, [unsaidFirst, unsaidSecond]), AT).started, [
    { cell: "HERON1", by: "GULL01" },
  ]);
});

it("a copy said to have started stands over one that doesn't say, whichever is told first", () => {
  const unsaid = record("HERON1", "osprey", { key: "cell:HERON1@osprey", started_by: "GULL01" });
  const said = record("HERON1", "kestrel", {
    since: "2026-11-19T00:00:00Z",
    started_by: "SKIFF1",
  });
  for (const order of [
    [unsaid, said],
    [said, unsaid],
  ]) {
    assert.deepEqual(nowOf(told(undefined, order), AT).started, [{ cell: "HERON1", by: "SKIFF1" }]);
  }
});

it("a record the map doesn't draw as a cell names no starter for one it does", () => {
  const drawn = record("HERON1", "osprey", {
    since: "2026-11-19T00:00:00Z",
    started_by: "SKIFF1",
  });
  const undrawn = record("HERON1", "kestrel", {
    key: "cell:HERON1@kestrel",
    type: "machine",
    since: "2099-01-01T00:00:00Z",
    started_by: "GULL01",
  });
  assert.deepEqual(nowOf(told(undefined, [drawn, undrawn]), AT).started, [
    { cell: "HERON1", by: "SKIFF1" },
  ]);
});

it("a copy stands over another only when started strictly later, a start as no words none", () => {
  const early = { since: "2026-11-19T00:00:00Z" };
  const late = { since: "2026-11-19T00:30:00Z" };
  assert.equal(isLaterCopy(late, early), true);
  assert.equal(isLaterCopy(early, late), false);
  assert.equal(isLaterCopy(early, { ...early }), false, "a tie should keep the copy held");
  assert.equal(isLaterCopy(early, {}), true);
  assert.equal(isLaterCopy({}, early), false);
  assert.equal(isLaterCopy({ since: 1e15 }, early), false, "a start as a number reads as none");
});

it("a record naming itself, a blank or nobody, with no forage served, names nobody", () => {
  const cells = [
    record("SELF01", "kestrel", { started_by: "SELF01" }),
    record("BLANK1", "kestrel", { started_by: "" }),
    record("ALONE1", "osprey"),
  ];
  assert.deepEqual(nowOf(told(undefined, cells), AT).started, []);
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
function grown(networks: readonly Network[], started: readonly Started[]) {
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

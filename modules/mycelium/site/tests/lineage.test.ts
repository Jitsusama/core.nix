// Each cell's record says who started it, on every machine, and each cell
// of this machine that works on a quest has a forage (the node's `forage`
// kind), whose lineage says so too: these prove what the map keeps of
// that, that the record's word comes first, that a node saying neither
// names nobody, and what each cell's forage says it split from;
// branches.test.ts proves which network grows out of which from them.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { isLaterCopy, oneCopyEach, originsOf } from "../lineage.ts";
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

it("each cell is kept once, as its copy that stands, with the networks either copy names", () => {
  const stale = { name: "HERON1", machine: "osprey", since: "2026-11-19T00:00:00Z" };
  const moved = { name: "HERON1", machine: "kestrel", since: "2026-11-19T00:30:00Z" };
  const alone = { name: "SKIFF1", networks: ["repo.harbour"] };
  const kept = oneCopyEach([
    { ...stale, networks: ["orchestration.heron1", "repo.harbour"] },
    alone,
    { ...moved, networks: ["orchestration.heron1", "repo.skiff"] },
    { machine: "kestrel" },
  ]);
  assert.deepEqual(kept, [
    { ...moved, networks: ["orchestration.heron1", "repo.skiff", "repo.harbour"] },
    alone,
  ]);
  const [tied] = oneCopyEach([
    { ...stale, networks: [] },
    { ...stale, machine: "kestrel", networks: [] },
  ]);
  assert.equal(tied?.machine, "osprey", "of two copies started alike, the first told stands");
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

it("a cell's forage says what it split from, else the first forage listing it split into", () => {
  const these = [
    forage("KESTRL", { split_into: ["TERNAA", "SKIFFS", "GULL01", "AWAY01"] }),
    forage("OSPREY", { split_into: ["SKIFFS"] }),
    forage("TERNAA", { split_from: "KESTRL" }),
    forage("GULL01", { split_from: "OSPREY" }),
    forage("SKIFFS", {}),
  ];
  const split = nowOf(told(these), AT).split;
  assert.deepEqual(
    Object.fromEntries(split.map((each) => [each.cell, each.from])),
    { TERNAA: "KESTRL", GULL01: "OSPREY", SKIFFS: "KESTRL" },
    "a cell's own split_from should stand, a cell naming none take its first lister, and a cell" +
      " with no forage here, as one on another machine, take none",
  );
});

it("a split naming the cell itself, a blank or anything but words names nothing", () => {
  const these = [
    forage("SELF01", { split_from: "SELF01", split_into: ["SELF01", "", 7] }),
    forage("BLANK1", { split_from: "", split_into: "T" }),
    forage("NUMBER", { split_from: 7, split_into: null }),
    forage("ALONE1", { split_from: null }),
    { key: "forage:NOCELL", kind: "forage", heard: AT, lineage: { split_into: ["GULL01"] } },
    // The cells listed have forages here, so only how they were listed keeps them out:
    // a string read as a list would name T. A blank or a number can't have a forage, so
    // those two in SELF01's list only show that they come to nothing.
    forage("T", {}),
    forage("GULL01", {}),
  ];
  assert.deepEqual(nowOf(told(these), AT).split, []);
  assert.deepEqual(nowOf(told(undefined), AT).split, [], "a node serving no forage says no split");
});

it("a cell's origins are its start, then its split, said once where both name one cell", () => {
  const started = [
    { cell: "HERON1", by: "SKIFF1" },
    { cell: "TERN01", by: "SKIFF1" },
  ];
  const splits = [
    { cell: "HERON1", from: "GULL01" },
    { cell: "TERNAA", from: "KESTRL" },
    { cell: "TERN01", from: "SKIFF1" },
  ];
  assert.deepEqual(originsOf(started, splits), [
    { cell: "HERON1", by: "SKIFF1" },
    { cell: "TERN01", by: "SKIFF1" },
    { cell: "HERON1", by: "GULL01", split: true },
    { cell: "TERNAA", by: "KESTRL", split: true },
  ]);
});

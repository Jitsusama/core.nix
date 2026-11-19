// Each cell of this machine that works on a quest has a forage (the node's
// `forage` kind), whose lineage says who started it: these prove what the
// map keeps of that, and that a node too old to serve forages reads as one
// that says nothing of who started whom.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
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

it("the cells come in the order of their addresses, whatever order the node said them", () => {
  const these = [
    forage("TERN03", { started_by: "OSPREY" }),
    forage("AUK001", { started_by: "TERN03" }),
  ];
  assert.deepEqual(
    nowOf(told(these), AT).started?.map((each) => each.cell),
    ["AUK001", "TERN03"],
  );
});

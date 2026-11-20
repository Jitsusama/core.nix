// A basin lends machines by the hour, under names it makes up, so the map
// draws each basin once, as the one stand model.ts makes of its machines:
// these prove which names are a basin's, what the stand is called, where it
// stands and what it says of the machines in it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { nowOf } from "../model.ts";

const AT = Date.parse("2026-11-16T07:30:00Z");

/** The watches, the machines' told `these` whole, in the order given. */
function told(these: readonly { readonly key: string; readonly kind: string }[]) {
  const heard = { network: new Held(), cell: new Held(), machine: new Held(), message: new Held() };
  heard.machine.took({ records: these });
  return heard;
}

/** A machine called `name`, as the node keys it, with `more` to it. */
function bare(name: string, more: Record<string, unknown> = {}) {
  return { key: `machine:${name}`, kind: "machine", heard: AT, name, state: "ready", ...more };
}

/** A machine called `name` lent by the basin `basin`, with `more` to it. */
function lent(name: string, basin: unknown, more: Record<string, unknown> = {}) {
  return bare(name, { declared: { basin }, ...more });
}

/** The names of the machines the map draws from `these`, in order. */
function standing(these: readonly { readonly key: string; readonly kind: string }[]) {
  return nowOf(told(these), AT).machines.map((each) => each.name);
}

// The node reads a basin by a DNS label's letters and drops any other
// shape, so the map holds a name to the same letters and never draws one
// a node of another build let through. Each wrong name breaks one rule.
it("a basin named in any shape but a name reads as no basin", () => {
  const misshapen = [
    7,
    { name: "shoal" },
    "",
    "Shoal",
    "shoal basin",
    "shoal\n",
    "shoal<script>",
    "-",
    "-shoal",
    "shoal-",
    "b".repeat(64),
  ];
  for (const basin of misshapen) {
    const [machine] = nowOf(told([lent("skiff", basin)]), AT).machines;
    assert.equal(machine?.basin, null, `${JSON.stringify(basin)} named a basin`);
  }
  for (const basin of ["a", "7", "basin-1", "b".repeat(63)]) {
    const [machine] = nowOf(told([lent("skiff", basin)]), AT).machines;
    assert.equal(machine?.basin, basin, `${basin} is a basin's name`);
  }
});

it("a basin's machines stand as one, named by the basin, after the machines of none", () => {
  const { machines } = nowOf(
    told([
      lent("pool-b", "shoal", { cpu: { cores: 4 }, memory: 16 }),
      bare("osprey"),
      lent("pool-a", "shoal", { cpu: { cores: 8 }, memory: 32 }),
      bare("heron"),
    ]),
    AT,
  );
  assert.deepEqual(
    machines.map((each) => each.name),
    ["osprey", "heron", "shoal basin"],
  );
  const [, , basin] = machines;
  assert.deepEqual(
    Array.isArray(basin?.["members"]) && basin["members"].map((each) => each.name),
    ["pool-a", "pool-b"],
    "a basin lists its machines by name",
  );
  assert.deepEqual([basin?.["basin"], basin?.["cores"], basin?.["memory"]], ["shoal", 12, 48]);
});

it("a basin keeps its place when it gives back the machine it was first told of", () => {
  assert.deepEqual(
    standing([bare("osprey"), bare("heron"), lent("pool-b", "shoal")]),
    standing([bare("osprey"), lent("pool-a", "shoal"), bare("heron"), lent("pool-b", "shoal")]),
  );
});

it("basins stand by name, whichever lent a machine first", () => {
  assert.deepEqual(standing([lent("pool-a", "tarn"), lent("pool-b", "shoal")]), [
    "shoal basin",
    "tarn basin",
  ]);
});

it("a basin is never named as a machine is, even one named like it", () => {
  assert.deepEqual(standing([bare("shoal"), lent("pool-a", "shoal")]), ["shoal", "shoal basin"]);
});

it("a basin runs while any of its machines does, and carries the worst of them", () => {
  const { machines } = nowOf(
    told([
      lent("pool-a", "shoal", { room: { pressure: { cpu: 10, memory: 20, io: 1 }, over: [] } }),
      lent("pool-b", "shoal", {
        state: "off",
        disk_band: "filling",
        room: { pressure: { cpu: 90, memory: 5, io: 3 }, over: ["memory"] },
      }),
    ]),
    AT,
  );
  const [basin] = machines;
  assert.deepEqual(
    [basin?.["state"], basin?.["pressure"], basin?.["over"], basin?.["disk_band"]],
    ["ready", { cpu: 90, memory: 20, io: 3 }, ["memory"], "filling"],
  );
});

it("a basin's disk is the fullest of its machines', and none while none fills", () => {
  const bands = (...said: (string | null)[]): unknown =>
    nowOf(told(said.map((band, at) => lent(`pool-${at}`, "shoal", { disk_band: band }))), AT)
      .machines[0]?.["disk_band"];
  assert.deepEqual(
    [
      bands("healthy", "nearly-full", "filling"),
      bands("critically-full", "filling"),
      bands("healthy", null),
    ],
    ["nearly-full", "critically-full", null],
    "a basin should carry its fullest disk's band, so a nearly full one isn't hidden",
  );
});

it("a basin with every machine off is off, and one that measured nothing is pressed by none", () => {
  const [basin] = nowOf(
    told([lent("pool-a", "shoal", { state: "off" }), lent("pool-b", "shoal", { state: "off" })]),
    AT,
  ).machines;
  assert.deepEqual([basin?.["state"], basin?.["pressure"], basin?.["over"]], ["off", null, []]);
});

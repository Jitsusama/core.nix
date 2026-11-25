// What the network page's head says stands now, in one sentence.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { networkOf } from "../model.ts";
import { type Context, standsNow } from "../panel.ts";
import { study } from "../shape.ts";
import { records, type Told, told } from "./house.ts";

/** The internet being the narrowest point, which is how a home line is, not a fault. */
const ceiling: Told = {
  key: "concern:ceiling:modem:down",
  kind: "concern",
  concern: "ceiling",
  device: "modem",
  on: "down",
  says: "The internet is the narrowest point down.",
  rank: 3,
};

/** The house with only the ceiling for a concern, so its concerns are still told. */
const calm: readonly Told[] = [...records.filter((record) => record.kind !== "concern"), ceiling];

/** What the lede is said from, for the house told as `these`. */
function contextOf(these: readonly Told[] = records): Context {
  const { watched, kinds } = told(these);
  const network = networkOf(watched, kinds);
  assert.ok(network, "the whole house was told, so the page draws it");
  return {
    network,
    shape: study(network),
    now: 1_700_000_300,
    mycelium: { machines: [] },
    open: () => undefined,
    letGo: () => undefined,
  };
}

it("the lede counts the devices using the network, the eeros and how much is worth a look", () => {
  assert.equal(
    standsNow(contextOf()),
    "1 device on 2 eeros across 2 floors, and 3 things worth a look.",
  );
});

it("the lede counts a source that didn't answer as worth a look", () => {
  const quietEero = calm.map((record) =>
    record.key === "source:eero" ? { ...record, state: "unreachable" } : record,
  );
  assert.match(standsNow(contextOf(quietEero)), /, and 1 thing worth a look\.$/u);
});

it("the lede says nothing needs a look when only the internet's ceiling is told", () => {
  assert.match(standsNow(contextOf(calm)), /, and nothing needs a look\.$/u);
});

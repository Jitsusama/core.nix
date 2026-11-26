// What the network page's head says stands now, in one sentence.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { KINDS, networkOf } from "../model.ts";
import { type Context, ledeOf, standsNow, UNANSWERED } from "../panel.ts";
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

/** How the lede ends with one source quiet, and with nothing but the ceiling told. */
const ONE_WORTH = /, and 1 thing worth a look\.$/u;
const NOTHING_WORTH = /, and nothing needs a look\.$/u;

/** What the lede is said from, for the house told as `these`, every kind told when `whole`. */
function contextOf(these: readonly Told[] = records, whole = false): Context {
  const { watched, kinds } = told(these);
  const network = networkOf(watched, whole ? new Set(KINDS) : kinds);
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
  assert.match(standsNow(contextOf(quietEero)), ONE_WORTH);
});

it("the lede says nothing needs a look when only the internet's ceiling is told", () => {
  assert.match(standsNow(contextOf(calm)), NOTHING_WORTH);
});

// A page joining a watch the server holds as broken is told every kind
// empty, before any feed was live.
const empty = contextOf([], true);

it("the lede says net hasn't answered once its feeds stop reaching it before any was live", () => {
  assert.equal(ledeOf({ answered: false, worst: "broken" }, undefined), UNANSWERED);
  assert.equal(ledeOf({ answered: false, worst: "stale" }, undefined), UNANSWERED);
});

it("the lede says net hasn't answered over an empty net no feed was live for", () => {
  assert.equal(ledeOf({ answered: false, worst: "broken" }, empty), UNANSWERED);
});

it("the lede is left as it is while net is still being reached", () => {
  assert.equal(ledeOf({ answered: false, worst: "connecting" }, undefined), null);
  assert.equal(ledeOf({ answered: false, worst: undefined }, empty), null);
});

// A page opened or reloaded while net is down is given the records the
// server still holds, and the map draws them.
// A feed coming back says it's live before its records arrive.
it("the lede keeps its words over an empty net though a feed is live", () => {
  assert.equal(ledeOf({ answered: true, worst: "live" }, empty), null);
  assert.equal(ledeOf({ answered: true, worst: "broken" }, empty), null);
});

it("the lede counts the devices the map draws from held records though no feed was live", () => {
  for (const worst of ["broken", "stale", "connecting", undefined]) {
    assert.equal(ledeOf({ answered: false, worst }, contextOf()), standsNow(contextOf()), worst);
  }
});

it("the lede counts what stands once a feed was live, and keeps its words once net goes", () => {
  assert.equal(ledeOf({ answered: true, worst: "live" }, contextOf()), standsNow(contextOf()));
  assert.equal(ledeOf({ answered: true, worst: "broken" }, contextOf()), standsNow(contextOf()));
  assert.equal(ledeOf({ answered: true, worst: "broken" }, undefined), null);
});

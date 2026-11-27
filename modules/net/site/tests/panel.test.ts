// What the network page's head says stands now, in one sentence.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { KINDS, networkOf } from "../model.ts";
import {
  type Context,
  ledeOf,
  NAMELESS,
  REACHING,
  standsNow,
  UNANSWERED,
  UNREACHED,
} from "../panel.ts";
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
const WORTH_A_LOOK = /needs a look|worth a look/u;

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

// A concern the page couldn't read is said apart as unread, so the lede
// can't count it as an all-clear.
it("the lede counts a concern it couldn't read as worth a look", () => {
  const unreadable: Told = { key: "concern:unreadable", kind: "concern" };
  assert.match(standsNow(contextOf([...calm, unreadable])), ONE_WORTH);
});

// A page joining a watch the server holds as broken is told every kind
// empty, before any feed was live.
const empty = contextOf([], true);

it("the lede says net isn't answering once the server says it can't reach net", () => {
  assert.equal(ledeOf("broken", undefined), UNANSWERED);
  assert.equal(ledeOf("broken", empty), UNANSWERED);
});

// A feed goes stale while the sites' server itself can't be reached, which
// says nothing of net.
it("the lede blames the sites' server, not net, while the page can't reach it", () => {
  assert.equal(ledeOf("stale", undefined), UNREACHED);
  assert.equal(ledeOf("stale", empty), UNREACHED);
});

it("the lede says net is being reached while it is, as the page first does", () => {
  assert.equal(ledeOf("connecting", undefined), REACHING);
  assert.equal(ledeOf(undefined, empty), REACHING);
});

// Every feed may say live before the last kind's records come, or before the
// first drawing: until every kind is read, no network says net named nothing.
it("the lede says net is still being reached while live feeds have told no network yet", () => {
  assert.equal(ledeOf("live", undefined), REACHING);
});

// Net serves every kind empty until its first reading is done.
it("the lede says net has named no device over an empty net it has read", () => {
  assert.equal(ledeOf("live", empty), NAMELESS);
});

it("the lede counts what stands, and what is worth a look, while net's feeds are live", () => {
  assert.equal(ledeOf("live", contextOf()), standsNow(contextOf()));
});

// The server says live before the records that bring it, so a page that
// held records from before has none current until they come.
it("the lede counts held records as net last told them until each live feed's records come", () => {
  assert.equal(
    ledeOf("live", contextOf(calm), { fresh: false, unheard: 0 }),
    "1 device on 2 eeros across 2 floors, as net last told them.",
  );
});

// A line that isn't JSON, or a record with no key, never reaches the
// network, but may have been a concern worth a look.
it("the lede counts what a concern or source watch couldn't read as worth a look", () => {
  assert.match(ledeOf("live", contextOf(calm), { fresh: true, unheard: 1 }), ONE_WORTH);
});

// A page opened or reloaded while net is down is given the records the
// server still holds, and the map draws them, each source still answering
// as it last said: their counts stand, but what they ask would read as an
// all-clear.
//
// A page reconnecting to a sites' server that restarted is told connecting
// and no records, so the records it holds are from before until it's live.
it("the lede counts held records as net last told them until its feeds are live", () => {
  for (const worst of ["broken", "stale", "connecting", undefined]) {
    assert.equal(
      ledeOf(worst, contextOf(calm)),
      "1 device on 2 eeros across 2 floors, as net last told them.",
      worst,
    );
    assert.doesNotMatch(ledeOf(worst, contextOf()), WORTH_A_LOOK, worst);
  }
});

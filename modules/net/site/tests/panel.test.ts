// What the network page's head says stands now, in one sentence.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { KINDS, networkOf } from "../model.ts";
import {
  type Context,
  HELD,
  ledeOf,
  NAMELESS,
  NOTHING,
  REACHING,
  standsNow,
  toldOf,
  UNANSWERED,
  UNREACHED,
  unreadSaid,
  worthSaid,
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
const TWO_WORTH = /, and 2 things worth a look\.$/u;
const THREE_WORTH = /, and 3 things worth a look\.$/u;
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

it("the lede counts a source it couldn't read as worth a look", () => {
  const unreadable: Told = { key: "source:unreadable", kind: "source" };
  const context = contextOf([...calm, unreadable]);
  assert.ok(context.network.unread.includes("source source:unreadable"));
  assert.match(standsNow(context), ONE_WORTH);
  assert.deepEqual(worthSaid(context), [unreadSaid(1)]);
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

// Empty records held from before a reconnect aren't what net says now.
it("the lede says net is still being reached until a live feed's records come", () => {
  assert.equal(ledeOf("live", empty, { fresh: false, unheard: 0 }), REACHING);
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

// The panel's Worth a Look says what the lede counts, rather than nothing.
it("the panel counts what couldn't be read as the lede does", () => {
  const unreadable: Told = { key: "concern:unreadable", kind: "concern" };
  const context = { ...contextOf([...calm, unreadable]), unheard: 1 };
  assert.match(ledeOf("live", context, { fresh: true, unheard: 1 }), TWO_WORTH);
  assert.deepEqual(
    worthSaid(context),
    ["2 things net told couldn't be read, and may be worth a look."],
    "the panel's Worth a Look counted otherwise than the lede",
  );
  assert.equal(unreadSaid(1), "1 thing net told couldn't be read, and may be worth a look.");
});

it("the panel says nothing needs a look only when the lede does", () => {
  const context = contextOf(calm);
  assert.match(ledeOf("live", context), NOTHING_WORTH);
  assert.deepEqual(worthSaid(context), [NOTHING]);
  assert.match(ledeOf("live", contextOf()), THREE_WORTH);
  assert.deepEqual(worthSaid(contextOf()), [], "the panel spoke beside rows it drew");
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

it("the panel says nothing needs a look only while net's feeds are live", () => {
  assert.deepEqual(worthSaid(contextOf(calm)), [NOTHING]);
  for (const worst of ["broken", "stale", "connecting", undefined]) {
    const live = toldOf(worst);
    assert.deepEqual(worthSaid({ ...contextOf(calm), told: live }), [HELD], worst);
  }
  const held = toldOf("live", { fresh: false, unheard: 0 });
  assert.deepEqual(
    worthSaid({ ...contextOf(calm), told: held }),
    [HELD],
    "live, records not yet come",
  );
});

// What it lists, and what it couldn't read, are still as net last told them.
it("the panel says what's worth a look is as net last told it while its feeds aren't live", () => {
  const unreadable: Told = { key: "concern:unreadable", kind: "concern" };
  const context = { ...contextOf([...calm, unreadable]), told: false };
  assert.deepEqual(worthSaid(context), [unreadSaid(1), HELD]);
  assert.deepEqual(worthSaid({ ...contextOf(), told: false }), [HELD]);
  assert.deepEqual(worthSaid({ ...contextOf(), told: true }), []);
});

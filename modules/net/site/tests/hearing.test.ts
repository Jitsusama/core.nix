// What the network page heard of net's watches beside their records.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { type Heard, Hearing, heeding } from "../hearing.ts";

const KINDS = ["device", "concern"];

/** A hearing with every kind live and its records come whole. */
function heardLive(): Hearing {
  const hearing = new Hearing(KINDS);
  for (const kind of KINDS) {
    hearing.stands(kind, "live");
    hearing.moved(kind, true);
  }
  return hearing;
}

it("holds the records current only once each kind's have come since it went live", () => {
  const hearing = heardLive();
  assert.equal(hearing.now().fresh, true);
  hearing.stands("concern", "connecting");
  hearing.stands("concern", "live");
  assert.equal(hearing.now().fresh, false, "live came before the records that bring it");
  hearing.moved("concern", false);
  assert.equal(hearing.now().fresh, true);
});

it("counts a concern watch's unread answer until a whole one comes", async () => {
  const hearing = heardLive();
  hearing.problem("concern", "the feed sent a line that isn't JSON");
  assert.deepEqual(hearing.now(), { fresh: true, unheard: 1 });
  await Promise.resolve();
  hearing.moved("concern", true);
  assert.deepEqual(hearing.now(), { fresh: true, unheard: 0 });
});

// A change tells only the records it names, so a record lost from an
// earlier line may still be missing after it.
it("keeps counting a concern watch's unread answer over a change that isn't whole", async () => {
  const hearing = heardLive();
  hearing.problem("concern", "the feed sent a line that isn't JSON");
  await Promise.resolve();
  hearing.moved("concern", false);
  assert.deepEqual(hearing.problems(), ["concern: the feed sent a line that isn't JSON"]);
  assert.equal(hearing.now().unheard, 1);
});

// The kit's feed says a record had no key, then hands over the rest of that
// answer in the same turn.
it("keeps what a watch couldn't read of the answer it hands over with it", async () => {
  const hearing = heardLive();
  hearing.problem("concern", "1 of what the feed told had no key");
  hearing.moved("concern", true);
  assert.deepEqual(hearing.problems(), ["concern: 1 of what the feed told had no key"]);
  assert.equal(hearing.now().unheard, 1);
  await Promise.resolve();
  hearing.moved("concern", true);
  assert.deepEqual(hearing.problems(), []);
});

it("says what a device watch couldn't read without counting it as worth a look", () => {
  const hearing = heardLive();
  hearing.problem("device", "the feed sent a line that isn't JSON");
  assert.deepEqual(hearing.problems(), ["device: the feed sent a line that isn't JSON"]);
  assert.equal(hearing.now().unheard, 0);
});

// An answer to another ask was never the watch's, so it hid none of its records.
it("says a concern watch's stray answer without counting it as worth a look", () => {
  const hearing = heardLive();
  hearing.problem("concern", 'an answer to no ask of this page: {"id":9}');
  assert.deepEqual(hearing.problems(), ['concern: an answer to no ask of this page: {"id":9}']);
  assert.equal(hearing.now().unheard, 0);
});

it("keeps counting a line that may hide a record when a stray answer follows it", () => {
  const hearing = heardLive();
  hearing.problem("concern", "the feed sent a line that isn't JSON");
  hearing.problem("concern", 'an answer to no ask of this page: {"id":9}');
  assert.equal(hearing.now().unheard, 1);
});

/** What the page heard each time a watch's heeding asked it to draw or say. */
function heededBy(hearing: Hearing, kind: string) {
  const asked: { said: string; told: number; heard: Heard }[] = [];
  let told = 0;
  const as = (said: string) => () => asked.push({ said, told, heard: hearing.now() });
  const heed = heeding(hearing, kind, {
    told: () => {
      told += 1;
    },
    stood: () => undefined,
    drawSoon: as("draw"),
    sayStands: as("stands"),
  });
  return { heed, asked };
}

const HELD = new Held();
const SINCE = 1_700_000_000_000;

// The page draws and says from what the hearing holds, so each watch's
// heeding tells the hearing first.
it("tells the hearing what a watch heard before the page draws or says it", async () => {
  const hearing = heardLive();
  const { heed, asked } = heededBy(hearing, "concern");
  heed.stands({ state: "connecting", since: SINCE });
  heed.problem("the feed sent a line that isn't JSON");
  await Promise.resolve();
  heed.stands({ state: "live", since: SINCE });
  heed.moved(HELD, { changed: [], gone: [], whole: true });
  assert.deepEqual(asked, [
    { said: "stands", told: 0, heard: { fresh: false, unheard: 0 } },
    { said: "draw", told: 0, heard: { fresh: false, unheard: 1 } },
    { said: "stands", told: 0, heard: { fresh: false, unheard: 1 } },
    { said: "draw", told: 1, heard: { fresh: true, unheard: 0 } },
  ]);
});

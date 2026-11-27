// What the network page heard of net's watches beside their records.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Hearing } from "../hearing.ts";

const KINDS = ["device", "concern"];

/** A hearing with every kind live and its records come. */
function heardLive(): Hearing {
  const hearing = new Hearing(KINDS);
  for (const kind of KINDS) {
    hearing.stands(kind, "live");
    hearing.moved(kind);
  }
  return hearing;
}

it("holds the records current only once each kind's have come since it went live", () => {
  const hearing = heardLive();
  assert.equal(hearing.now().fresh, true);
  hearing.stands("concern", "connecting");
  hearing.stands("concern", "live");
  assert.equal(hearing.now().fresh, false, "live came before the records that bring it");
  hearing.moved("concern");
  assert.equal(hearing.now().fresh, true);
});

it("counts a concern watch's unread answer until it reads another", async () => {
  const hearing = heardLive();
  hearing.problem("concern", "the feed sent a line that isn't JSON");
  assert.deepEqual(hearing.now(), { fresh: true, unheard: 1 });
  await Promise.resolve();
  hearing.moved("concern");
  assert.deepEqual(hearing.now(), { fresh: true, unheard: 0 });
});

// The kit's feed says a record had no key, then hands over the rest of that
// answer in the same turn.
it("keeps what a watch couldn't read of the answer it hands over with it", async () => {
  const hearing = heardLive();
  hearing.problem("concern", "1 of what the feed told had no key");
  hearing.moved("concern");
  assert.deepEqual(hearing.problems(), ["concern: 1 of what the feed told had no key"]);
  assert.equal(hearing.now().unheard, 1);
  await Promise.resolve();
  hearing.moved("concern");
  assert.deepEqual(hearing.problems(), []);
});

it("says what a device watch couldn't read without counting it as worth a look", () => {
  const hearing = heardLive();
  hearing.problem("device", "an answer to no ask of this page");
  assert.deepEqual(hearing.problems(), ["device: an answer to no ask of this page"]);
  assert.equal(hearing.now().unheard, 0);
});

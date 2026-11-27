// What the network page heard of net's watches beside their records.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Hearing } from "../hearing.ts";

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

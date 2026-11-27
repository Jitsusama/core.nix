// Whether a name may draw as an act where it holds a letter from beyond
// Latin, whichever script lends it, as one of the act's letters or as
// nothing.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { endsOf, MARKS, pointsIn } from "../borrowed.ts";

it("a letter beyond Latin where an act's letter stands may draw as it", () => {
  assert.deepEqual(endsOf("\u0441lear it", "clear it"), [8]);
  assert.deepEqual(endsOf("\ua4da\ua4e1\ua4f0\ua4ee\ua4e3 \ua4f2\ua4d4", "clear it"), [8]);
  assert.deepEqual(endsOf("clear \u2170t", "clear it"), [8]);
  assert.deepEqual(endsOf("\u{1d5b3}alk", "talk"), [4]);
});

it("a Latin letter draws only as itself, and a space only as a space", () => {
  assert.deepEqual(endsOf("clear at", "clear it"), []);
  assert.deepEqual(endsOf("clear\u0441it", "clear it"), []);
  assert.deepEqual(endsOf("\u0441lear", "clear it"), []);
});

it("a stem or a zero draws as the Latin letter it draws like", () => {
  assert.deepEqual(endsOf("clear lt", "clear it"), [8]);
  assert.deepEqual(endsOf("ta1k", "talk"), [4]);
  assert.deepEqual(endsOf("explain \u0131t", "explain it"), [10]);
  assert.deepEqual(endsOf("g0 t0", "go to"), [5]);
});

// A mark MARKS doesn't name, Cyrillic's, Arabic's, kana's or music's,
// combines with the letter before it as MARKS' do, so it may draw as
// nothing between an act's letters, or after them (#6562, 211).
it("a letter beyond Latin may draw as nothing, between an act's letters or after", () => {
  assert.deepEqual(endsOf("ta\u0485lk", "talk"), [5]);
  assert.deepEqual(endsOf("tal\u064bk", "talk"), [4, 5]);
  assert.deepEqual(endsOf("t\u3099alk", "talk"), [5]);
  assert.deepEqual(endsOf("talk\u{1d167} now", "talk"), [4, 5]);
  assert.deepEqual(endsOf("\u0434\u0430", "talk"), []);
});

it("a combined mark is one the read takes out", () => {
  assert.equal("tal\u0327k".replace(MARKS, ""), "talk");
});

it("a letter past U+FFFF counts once", () => {
  assert.equal(pointsIn("\u{1d5b3}alk"), 4);
});

// Whether a name may draw as an act where it holds a letter from beyond
// Latin, whichever script lends it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { drawsLike, MARKS, pointsIn } from "../borrowed.ts";

it("a letter beyond Latin where an act's letter stands may draw as it", () => {
  assert.equal(drawsLike("\u0441lear it", "clear it"), true);
  assert.equal(drawsLike("\ua4da\ua4e1\ua4f0\ua4ee\ua4e3 \ua4f2\ua4d4", "clear it"), true);
  assert.equal(drawsLike("clear \u2170t", "clear it"), true);
  assert.equal(drawsLike("\u{1d5b3}alk", "talk"), true);
});

it("a Latin letter draws only as itself, and a space only as a space", () => {
  assert.equal(drawsLike("clear at", "clear it"), false);
  assert.equal(drawsLike("clear\u0441it", "clear it"), false);
  assert.equal(drawsLike("\u0441lear", "clear it"), false);
});

it("a stem or a zero draws as the Latin letter it draws like", () => {
  assert.equal(drawsLike("clear lt", "clear it"), true);
  assert.equal(drawsLike("ta1k", "talk"), true);
  assert.equal(drawsLike("explain \u0131t", "explain it"), true);
  assert.equal(drawsLike("g0 t0", "go to"), true);
});

it("a combined mark is one the read takes out", () => {
  assert.equal("tal\u0327k".replace(MARKS, ""), "talk");
});

it("a letter past U+FFFF counts once", () => {
  assert.equal(pointsIn("\u{1d5b3}alk"), 4);
});

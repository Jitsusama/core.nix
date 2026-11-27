// What a name reads as once each letter it borrows from another script is
// read as the Latin one it draws as.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { unborrowed } from "../borrowed.ts";

it("a Cyrillic or Greek letter drawing as a Latin one reads as it", () => {
  assert.equal(unborrowed("\u0421lear \u0406t"), "clear it");
  assert.equal(unborrowed("\u03a4alk"), "talk");
});

it("a fullwidth letter reads as its Latin one, its case kept", () => {
  assert.equal(unborrowed("\uff34alk \uff43"), "Talk c");
});

it("a letter drawing as no Latin one, and a Latin name, are left as they are", () => {
  assert.equal(unborrowed("\u0414\u0430"), "\u0414a");
  assert.equal(unborrowed("Clear It"), "Clear It");
});

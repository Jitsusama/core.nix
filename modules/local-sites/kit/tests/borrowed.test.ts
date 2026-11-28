// Whether a name holds only the closed alphabet a choice is trusted in, how
// wide each of its characters draws, and how its letters that draw alike
// read as an act.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { COMBINING, MARKS, opensWith, reads, trusted, widthOf } from "../borrowed.ts";

/**
 * The closed set by how wide each draws, as LookForm.js weighs them, the
 * marks sessions write as the widest, an ellipsis among them (#6562, 230).
 */
const WEIGHTS: readonly (readonly [string, number])[] = [
  ["abcdefghijklnopqrstuvxyz .,'\"-!?:;()/\u2018\u2019\u201c\u201d\u2013", 1],
  ["ABCDEFGHIJKLNOPQRSTUVXYZ0123456789", 1.5],
  ["mwMW&#+\u2014`_@%=*[]<>~$|\u2026", 2],
];

it("every character of the closed set is trusted, weighed as it draws", () => {
  for (const [characters, width] of WEIGHTS) {
    for (const one of characters.split("")) {
      assert.ok(trusted(one), `${one} wasn't trusted`);
      assert.equal(widthOf(one), width, `${one} was weighed wrong`);
    }
  }
});

// A letter beyond Latin may draw as an act's (#6562, F170, 177 to 187), a
// kra or a times sign as k or x (218 and 224), and a mark no list names as
// nothing (211); a caret, a brace or a backslash no session was seen to
// write in a choice stays out too.
it("a character outside the closed set isn't trusted", () => {
  const outside = ["\u0441", "\ua4da", "\u2170", "\u{1d5b3}", "\u0138", "\u00d7", "\u0485"];
  for (const one of [...outside, "^", "{", "\\", "\t", "\u00a0", "\u200b", "\u00f8", "\u0301"]) {
    assert.equal(trusted(`clear ${one}t`), false, `${one} was trusted`);
  }
  assert.equal(widthOf("^"), 2, "a character outside the set wasn't weighed widest");
});

it("a stem, a zero, two v's, r and n, or c and l read as the letter they draw as", () => {
  assert.ok(reads("clear lt", "clear it"), "a small l didn't read as an i");
  assert.ok(reads("ta1k", "talk"), "a one didn't read as an l");
  assert.ok(reads("ta|k", "talk"), "a bar didn't read as an l");
  assert.ok(reads("g0 t0", "go to"), "a zero didn't read as an o");
  assert.ok(reads("ansvver", "answer"), "two v's didn't read as a w");
  assert.ok(reads("rnerge", "merge"), "r and n didn't read as an m");
  assert.ok(reads("dear it", "clear it"), "c and l didn't read as a d");
  assert.ok(reads("cle@r it", "clear it"), "an @ didn't read as an a");
  assert.ok(reads("$ave", "save"), "a $ didn't read as an s");
  assert.ok(reads("don`t", "don't"), "a backtick didn't read as an apostrophe");
  assert.equal(reads("clear at", "clear it"), false, "an a read as an i");
});

it("a name opens with an act only where a break follows its words", () => {
  assert.ok(opensWith("clear it, now", "clear it"), "a stop wasn't a break");
  assert.ok(opensWith("ansvver later", "answer"), "two v's didn't open with answer");
  assert.ok(opensWith("talk|now", "talk"), "a bar wasn't a break");
  assert.equal(opensWith("clearance", "clear"), false, "a letter was a break");
  assert.equal(opensWith("talk2 me", "talk"), false, "a digit was a break");
  assert.equal(opensWith("talk", "talk"), false, "an act alone opened with itself");
});

it("an accent decomposed is one the read takes out", () => {
  assert.equal("ta\u013ck".normalize("NFD").replace(COMBINING, ""), "talk");
  assert.equal("tal\u0327k".replace(MARKS, ""), "talk");
});

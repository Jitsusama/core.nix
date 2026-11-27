// What the murmurs page's markup makes of a murmur whose words, cell, ties
// and settling could hold markup of their own: every one is escaped before
// it reaches the page, in text and in attributes alike.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { headlineOf, murmurRow, skyOf } from "../draw.ts";
import type { Murmur } from "../model.ts";

const hostile = '<img src=x onerror="alert(1)">';

/** A murmur whose every word read from the node is `hostile`. */
const sly: Murmur = {
  key: `murmur:${hostile}:100`,
  writer: hostile,
  quest: hostile,
  words: hostile,
  ties: [hostile],
  settled: hostile,
  at: 100,
};

it("a murmur's row escapes every word read from the node", () => {
  const row = murmurRow(sly, 200);
  assert.ok(!row.includes("<img"), `no markup of the murmur's should reach the row: ${row}`);
  assert.ok(
    row.includes("&lt;img src=x onerror=&quot;alert(1)&quot;&gt;"),
    `the words should be shown as text: ${row}`,
  );
});

it("a murmur's row holds its words' direction and its quest's to them", () => {
  const row = murmurRow({ ...sly, quest: "Watch the shore", words: "The tide turned." }, 200);
  assert.ok(row.includes('<span class="words" dir="auto">The tide turned.</span>'), row);
  assert.ok(row.includes("from <bdi>Watch the shore</bdi> "), row);
});

it("the headline links its count once anything was said, and its turn while some is open", () => {
  const said = (count: number, open: number) => ({
    lead: "Lead.",
    turn: "turn.",
    said: count,
    open,
  });
  assert.equal(headlineOf(said(0, 0)), "Lead. <em>turn.</em>", "nothing said links nothing");
  const all = '<a href="#all" data-show="all">Lead.</a>';
  assert.equal(headlineOf(said(3, 0)), `${all} <em>turn.</em>`, "all settled links the count");
  assert.equal(headlineOf(said(3, 3)), `${all} <em>turn.</em>`, "all open links the count");
  assert.equal(
    headlineOf(said(3, 1)),
    `${all} <em><a href="#air" data-show="air">turn.</a></em>`,
    "a part open links the turn to it",
  );
  assert.ok(!headlineOf({ ...said(3, 1), lead: hostile }).includes("<img"), "the lead is escaped");
});

it("the sky escapes a murmur's key where it marks the bird", () => {
  const sky = skyOf([sly, { ...sly, key: "murmur:TERNAA:90", settled: null }], null);
  assert.ok(!sky.includes("<img"), `no markup of the murmur's should reach the sky: ${sky}`);
});

it("the sky is a bare wire once no murmur is left", () => {
  const sky = skyOf([], null);
  assert.ok(sky.includes('class="wire"'), `the wire should still be drawn: ${sky}`);
  assert.ok(!sky.includes('class="bird'), `no bird should be left: ${sky}`);
});

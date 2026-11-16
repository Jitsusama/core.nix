// What the murmurs page's markup makes of a murmur whose words, cell, ties
// and settling could hold markup of their own: every one is escaped before
// it reaches the page, in text and in attributes alike.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { murmurRow, skyOf } from "../draw.ts";
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

it("the sky escapes a murmur's key where it marks the bird", () => {
  const sky = skyOf([sly, { ...sly, key: "murmur:TERNAA:90", settled: null }], null);
  assert.ok(!sky.includes("<img"), `no markup of the murmur's should reach the sky: ${sky}`);
});

it("the sky is a bare wire once no murmur is left", () => {
  const sky = skyOf([], null);
  assert.ok(sky.includes('class="wire"'), `the wire should still be drawn: ${sky}`);
  assert.ok(!sky.includes('class="bird'), `no bird should be left: ${sky}`);
});

// Which trees the cursor stood on a drawer keeps listed, so a row it stepped off stays put.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Listing } from "../listed.ts";

const CORE = "github:owl/core";
const NOTES = "github:owl/notes";

it("keeps every tree the cursor stood on while its repository stays open", () => {
  const listing = new Listing();
  listing.kept(CORE, "/home/ada/trees/a");
  listing.kept(CORE, null);
  const kept = listing.kept(CORE, "/home/ada/trees/b");
  assert.deepEqual([...kept], ["/home/ada/trees/a", "/home/ada/trees/b"], "a tree stood on went");
});

it("forgets them once another repository opens", () => {
  const listing = new Listing();
  listing.kept(CORE, "/home/ada/trees/a");
  assert.deepEqual([...listing.kept(NOTES, null)], [], "core's trees stayed with notes open");
  assert.deepEqual([...listing.kept(CORE, null)], [], "core's trees came back on reopening");
});

it("forgets them when every tree folds again, so the twelve come back", () => {
  const listing = new Listing();
  listing.listWhole(CORE, true);
  listing.kept(CORE, "/home/ada/trees/a");
  listing.kept(CORE, "/home/ada/trees/b");
  listing.listWhole(CORE, false);
  assert.deepEqual([...listing.kept(CORE, null)], [], "folding kept the trees stepped through");
  assert.ok(!listing.whole.has(CORE), "folding still listed every tree");
});

// How Code reads a checkout's weight from its record and says it, in a row and in the drawer.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { bytes, weighedBriefly, weighedFully, weightOf, weightOfAll } from "../weight.ts";

const heavy = {
  weight: {
    loose: 203_776,
    estimated: true,
    loose_bytes: 2_393_000_000,
    packs: 11,
    packed_bytes: 104_000_000,
    due: true,
  },
};
const light = {
  weight: { loose: 0, loose_bytes: 0, packs: 1, packed_bytes: 4_200_000, due: false },
};
// Past 4096 yet counted in every folder, as when the one the daemon samples holds few.
const counted = {
  weight: { loose: 4500, estimated: false, loose_bytes: 0, packs: 0, packed_bytes: 0, due: false },
};

it("reads a weight as its loose and packed bytes together", () => {
  assert.deepEqual(weightOf(heavy), {
    bytes: 2_497_000_000,
    loose: 203_776,
    estimated: true,
    packs: 11,
    due: true,
  });
});

it("reads no weight from a record the daemon couldn't weigh", () => {
  assert.equal(weightOf({ path: "/home/ada/src/core" }), undefined);
  // The daemon's feed says an unweighed store as null.
  assert.equal(weightOf({ path: "/home/ada/src/core", weight: null }), undefined);
  assert.equal(weighedBriefly(undefined), "");
  assert.equal(weighedFully(undefined), "");
});

it("says a size in the largest unit that leaves it one or more, as a disk is sold", () => {
  assert.equal(bytes(0), "0 bytes");
  assert.equal(bytes(999), "999 bytes");
  assert.equal(bytes(4_200_000), "4.2 MB");
  assert.equal(bytes(104_000_000), "104 MB");
  assert.equal(bytes(2_497_000_000), "2.5 GB");
});

it("says a size that rounds up to the next unit or past ten in that unit's own words", () => {
  assert.equal(bytes(999_999), "1.0 MB");
  assert.equal(bytes(999_499), "999 kB");
  assert.equal(bytes(9_960_000), "10 MB");
  assert.equal(bytes(9_940_000), "9.9 MB");
});

it("says in a row its size, and that upkeep is due only when it is", () => {
  assert.equal(weighedBriefly(weightOf(heavy)), "about 2.5 GB, upkeep due");
  assert.equal(weighedBriefly(weightOf(light)), "4.2 MB");
});

it("says in the drawer what a store is made of, and why upkeep is due", () => {
  assert.equal(
    weighedFully(weightOf(heavy)),
    "about 2.5 GB in git: 203,776 loose, 11 packs; git's upkeep is due",
  );
  // A store of few loose objects has each counted, so its count isn't about.
  assert.equal(weighedFully(weightOf(light)), "4.2 MB in git: 0 loose, 1 pack");
  // About is said only where the daemon says it reckoned.
  assert.equal(weighedFully(weightOf(counted)), "0 bytes in git: 4,500 loose, 0 packs");
});

it("weighs a repository's checkouts together, due when any is", () => {
  assert.deepEqual(weightOfAll([weightOf(heavy), undefined, weightOf(light)]), {
    bytes: 2_501_200_000,
    loose: 203_776,
    estimated: true,
    packs: 12,
    due: true,
  });
  assert.equal(weightOfAll([undefined]), undefined);
});

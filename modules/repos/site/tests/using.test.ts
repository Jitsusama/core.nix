// How Code reads what works in a repository from its record and says it, in a row and in the
// drawer.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { footed, usedBriefly, usedFully, usingOf } from "../using.ts";

const record = {
  key: "using:github:owl/core",
  kind: "using",
  repository: "github:owl/core",
  processes: 3,
  memory: 1_300_000_000,
  cpu: 0.4,
  folders: [
    { path: "/home/ada/src/core", processes: 2, memory: 300_000_000, cpu: 0.1 },
    { path: "/home/ada/trees/fresh", processes: 1, memory: 1_000_000_000, cpu: 0.3 },
    { processes: 9, memory: 9, cpu: 9 },
  ],
};

it("reads a repository's use whole and by folder, skipping a folder with no path", () => {
  const using = usingOf(record);
  assert.equal(using.repository, "github:owl/core");
  assert.deepEqual(using.whole, { processes: 3, memory: 1_300_000_000, cpu: 0.4 });
  assert.deepEqual([...using.folders.keys()], ["/home/ada/src/core", "/home/ada/trees/fresh"]);
});

it("says a use as a share of a core and the memory held", () => {
  assert.equal(usedBriefly(usingOf(record).whole), "40% of a core, 1.3 GB of memory");
  assert.equal(
    usedFully({ processes: 1, memory: 2_400_000, cpu: 2.25 }),
    "1 process: 2.3 cores, 2.4 MB of memory",
  );
});

it("says a little processor time as little, and none as none", () => {
  assert.equal(
    usedBriefly({ processes: 1, memory: 2_000_000, cpu: 0.004 }),
    "under 1% of a core, 2.0 MB of memory",
  );
  assert.equal(
    usedBriefly({ processes: 1, memory: 2_000_000, cpu: 0 }),
    "0% of a core, 2.0 MB of memory",
  );
});

it("says only the memory until two readings measure the processor", () => {
  const first = usingOf({ ...record, cpu: null });
  assert.equal(first.whole.cpu, undefined);
  assert.equal(usedFully(first.whole), "3 processes: 1.3 GB of memory");
});

it("never lets an older daemon's refusal of what works call the page broken", () => {
  const standing = new Map([
    ["tree", "live"],
    ["using", "broken"],
    ["change", "stale"],
  ]);
  assert.deepEqual(footed(standing), ["live", "stale"]);
});

it("says nothing where nothing works, so a quiet row stays quiet", () => {
  assert.equal(usedBriefly({ processes: 0, memory: 0, cpu: 0 }), "");
  assert.equal(usedFully(undefined), "");
});

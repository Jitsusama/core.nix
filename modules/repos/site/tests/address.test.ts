// Where Code's address lands and what it says as the cursor moves, over a
// grove of one repository with a tree today and one resting.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { addressed, forgotten, landing, outward } from "../address.ts";
import type { Grove, Repository, Season, Tree } from "../model.ts";

function tree(path: string, season: Tree["season"], url?: string): Tree {
  return {
    path,
    snapshot: false,
    branch: path.split("/").at(-1),
    commit: undefined,
    writtenAt: undefined,
    season,
    change:
      url === undefined
        ? undefined
        : {
            key: "change:github:owl/core#41",
            number: 41,
            title: "Grow",
            draft: false,
            url,
            checks: "passing",
            failed: [],
            review: "none",
          },
    cells: [],
  };
}

const core: Repository = {
  key: "github:owl/core",
  name: "core",
  owner: "owl",
  trunk: "main",
  writtenAt: undefined,
  checkouts: [],
  trees: [
    tree("/home/ada/trees/fresh", "today", "https://github.com/owl/core/pull/41"),
    tree("/home/ada/trees/old", "resting"),
  ],
  cutIn: "/home/ada/trees",
  changes: [],
  seasons: { today: 1, week: 0, resting: 1, gone: 0 },
  cells: [],
  joined: [],
  address: "#repo=owl%2Fcore",
};

const grove: Grove = { machine: "kestrel", roots: [], unread: [], repositories: [core], worth: [] };

it("lands a place, a repository and a tree in its season", () => {
  assert.deepEqual(landing("#growing", grove), { kind: "place", place: "growing" });
  assert.deepEqual(landing("#repo=owl%2Fcore", grove), {
    kind: "repository",
    key: "github:owl/core",
  });
  assert.deepEqual(landing("#repo=owl/core", grove), {
    kind: "repository",
    key: "github:owl/core",
  });
  assert.deepEqual(landing("#tree=%2Fhome%2Fada%2Ftrees%2Fold", grove), {
    kind: "tree",
    key: "github:owl/core",
    path: "/home/ada/trees/old",
    season: "resting",
  });
});

it("lands nowhere on an address naming nothing in the grove", () => {
  assert.equal(landing("#repo=owl%2Fnest", grove), undefined, "a repository not held landed");
  assert.equal(landing("#tree=%2Fnowhere", grove), undefined, "a tree not held landed");
  assert.equal(landing("#cell=E2ZIAQ", grove), undefined, "a cell landed on Code");
  assert.equal(landing("", grove), undefined, "a bare address landed");
  assert.equal(landing("#tree=%E0%A4%A", grove), undefined, "a broken escape landed");
});

it("lands a tree whose folder holds an equals sign", () => {
  const odd = { ...core, trees: [tree("/home/ada/trees/a=b", "today")] };
  assert.equal(
    landing("#tree=%2Fhome%2Fada%2Ftrees%2Fa%3Db", { ...grove, repositories: [odd] })?.kind,
    "tree",
  );
  assert.equal(
    landing("#tree=/home/ada/trees/a=b", { ...grove, repositories: [odd] })?.kind,
    "tree",
  );
});

it("says the tree the cursor is on, else its repository, as landing reads it back", () => {
  assert.equal(addressed(core, null), "#repo=owl%2Fcore");
  const said = addressed(core, "/home/ada/trees/fresh");
  assert.equal(said, "#tree=%2Fhome%2Fada%2Ftrees%2Ffresh");
  assert.equal(landing(said, grove)?.kind, "tree", "the address said didn't land back");
});

it("opens a tree's change, else the repository where GitHub keeps it", () => {
  assert.equal(outward(core, "/home/ada/trees/fresh"), "https://github.com/owl/core/pull/41");
  assert.equal(outward(core, "/home/ada/trees/old"), "https://github.com/owl/core");
  assert.equal(outward({ ...core, key: "heron:src/notes" }, null), undefined);
});

// Core again with a tree gone, so z opens two seasons that each draw something.
const felled: Repository = {
  ...core,
  trees: [...core.trees, tree("/home/ada/trees/lost", "gone")],
  seasons: { ...core.seasons, gone: 1 },
};
const cleared: Grove = { ...grove, repositories: [felled] };

const untouched = { query: "", place: "every", cursor: core.key, unfolded: new Map() } as const;

/** What's drawn with `seasons` unfolded in core. */
function unfolding(...seasons: Season[]): typeof untouched & { unfolded: Map<string, Set<Season>> } {
  return { ...untouched, unfolded: new Map([[core.key, new Set(seasons)]]) };
}

it("forgets nothing a reload draws again as it's drawn now", () => {
  assert.deepEqual(forgotten("#repo=owl%2Fcore", grove, untouched), []);
  assert.deepEqual(forgotten("#growing", grove, { ...untouched, place: "growing" }), []);
  const old = addressed(core, "/home/ada/trees/old");
  const reopened = unfolding("resting");
  assert.deepEqual(forgotten(old, grove, reopened), [], "a tree's address opens its season");
});

it("forgets no unfolding that draws nothing more", () => {
  const here = "#repo=owl%2Fcore";
  // A season that never folds, as an address naming a tree written today leaves unfolded.
  assert.deepEqual(forgotten(here, grove, unfolding("today")), [], "today never folds");
  const old = addressed(core, "/home/ada/trees/old");
  const both = unfolding("resting", "gone");
  assert.deepEqual(forgotten(old, grove, both), [], "core has no gone tree to fold");
  const elsewhere = { ...both, cursor: "github:owl/notes" };
  assert.deepEqual(forgotten(here, grove, elsewhere), [], "core's seasons aren't the cursor's");
});

it("forgets the list a name or a place narrowed, and seasons only a key unfolded", () => {
  const here = "#repo=owl%2Fcore";
  assert.deepEqual(forgotten(here, grove, { ...untouched, query: "core" }), ["list"]);
  assert.deepEqual(forgotten(here, grove, { ...untouched, place: "growing" }), ["list"]);
  const folded = unfolding("resting", "gone");
  assert.deepEqual(forgotten(here, grove, folded), ["open"]);
  const old = addressed(felled, "/home/ada/trees/old");
  assert.deepEqual(forgotten(old, cleared, folded), ["open"], "gone stays folded on a reload");
  assert.deepEqual(forgotten(here, undefined, { ...untouched, place: "growing" }), ["list"]);
});

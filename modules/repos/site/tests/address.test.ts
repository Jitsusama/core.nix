// Where Code's address lands and what it says as the cursor moves, over a
// grove of one repository with a tree today and one resting.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { addressed, landing, outward } from "../address.ts";
import type { Grove, Repository, Tree } from "../model.ts";

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
  assert.equal(outward({ ...core, key: "equilibrius:src/acceluary" }, null), undefined);
});

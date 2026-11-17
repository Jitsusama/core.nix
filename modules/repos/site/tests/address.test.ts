// Where Code's address lands and what it says as the cursor moves, over a
// grove of one repository with a tree today and one resting.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { addressed, homeSiteOf, homesOf, landing, outward } from "../address.ts";
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
  assert.equal(outward({ ...core, key: "heron:src/notes" }, null), undefined);
});

it("finds where a kind is home by the row that claims it, whatever the row is named", () => {
  const sites = {
    sites: [
      { path: "code/", name: "Code", homes: ["repository", "tree"] },
      { path: "mycelium/", name: "Your mycelium, now", homes: ["cell"] },
      { path: "notes/", name: "mycelium" },
    ],
  };
  assert.equal(homeSiteOf(sites, "cell"), "mycelium/");
  assert.equal(homeSiteOf(sites, "tree"), "code/");
});

it("finds no home for a kind no row claims, or in a record that isn't one", () => {
  const unclaimed = { sites: [{ path: "mycelium/", name: "Your mycelium, now" }] };
  assert.equal(homeSiteOf(unclaimed, "cell"), undefined);
  assert.equal(homeSiteOf({ sites: [{ homes: ["cell"] }] }, "cell"), undefined);
  assert.equal(homeSiteOf({ sites: "none" }, "cell"), undefined);
  assert.equal(homeSiteOf(null, "cell"), undefined);
});

it("finds no home in a row whose path leaves the sites' own folders", () => {
  for (const path of ["../../elsewhere/", "/elsewhere/", "./mycelium/", "mycelium", ""]) {
    const sites = { sites: [{ path, name: "Your mycelium, now", homes: ["cell"] }] };
    assert.equal(homeSiteOf(sites, "cell"), undefined, `${path} was taken as a home`);
  }
});

it("takes from a site's homes only the things of the kind asked, each by its key", () => {
  const homes = {
    things: [
      { kind: "cell", key: "E2ZIAQ", name: "E2ZIAQ", address: "mycelium/#cell=E2ZIAQ" },
      { kind: "network", key: "repo.core", name: "repo.core", address: "mycelium/#n=core" },
      { kind: "cell", key: "NOWHERE" },
    ],
  };
  assert.deepEqual([...homesOf(homes, "cell")], [["E2ZIAQ", "mycelium/#cell=E2ZIAQ"]]);
  assert.equal(homesOf({ things: "none" }, "cell").size, 0);
});

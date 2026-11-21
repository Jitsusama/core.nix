// What Code makes of the repos daemon's records, told as the daemon tells
// them, a watch per kind, with cells from the mycelium's map.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { cellsOf, type Feed, groveOf, growing, seasonOf, stepped } from "../model.ts";

// A season is told by the reader's own day, so the cases are read in one zone, wherever they run.
process.env.TZ = "UTC";
const NOW = Date.parse("2026-11-16T15:00:00Z");
const MINUTE = 60 * 1000;
const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;
const at = (ago: number): string => new Date(NOW - ago).toISOString();

/** A watch per kind, each told its kind's records whole. */
function told(these: readonly { readonly key: string; readonly kind: string }[]): Feed {
  const kept = (kind: string): Held => {
    const held = new Held();
    held.took({ records: these.filter((record) => record.kind === kind) });
    return held;
  };
  return {
    reading: kept("reading"),
    repository: kept("repository"),
    checkout: kept("checkout"),
    tree: kept("tree"),
    change: kept("change"),
    using: kept("using"),
  };
}

const reading = {
  key: "reading",
  kind: "reading",
  machine: "kestrel",
  roots: ["/home/ada/src"],
  sources: [
    { name: "git", what: "the disk", unread: [] },
    { name: "github", what: "changes", unread: ["owl/nest: not found"] },
  ],
};

const core = {
  key: "repository:github:owl/core",
  kind: "repository",
  name: "core",
  owner: "owl",
  trunk: "main",
  written_at: at(HOUR),
};

const world = {
  key: "repository:github:owl/world",
  kind: "repository",
  name: "world",
  owner: "owl",
  trunk: "main",
  written_at: at(2 * DAY),
};

const checkout = {
  key: "checkout:/home/ada/src/core",
  kind: "checkout",
  repository: "github:owl/core",
  path: "/home/ada/src/core",
  bare: false,
  head: {
    branch: "main",
    commit: "a1",
    upstream: { branch: "main", count: { behind: 2, ahead: 0 } },
  },
};

/** A tree of core's at `path`, written `ago` before now. */
function tree(path: string, ago: number, present = true, change: string | null = null) {
  return {
    key: `tree:${path}`,
    kind: "tree",
    repository: "github:owl/core",
    checkout: "/home/ada/src/core",
    path,
    snapshot: false,
    present,
    head: { branch: path.split("/").at(-1), commit: "b2" },
    written_at: at(ago),
    change,
  };
}

const failing = {
  key: "change:github:owl/core#41",
  kind: "change",
  repository: "github:owl/core",
  title: "Say the grove",
  branch: "say-grove",
  draft: false,
  checks: { state: "failing", passed: 3, failed: ["lint"], pending: [] },
  review: "waiting",
  github: { number: 41, url: "https://github.com/owl/core/pull/41" },
};

const records = [
  reading,
  world,
  core,
  checkout,
  tree("/home/ada/trees/old", 10 * DAY),
  tree("/home/ada/trees/fresh", HOUR, true, failing.key),
  tree("/home/ada/trees/lost", DAY, false),
  tree("/home/ada/trees/tuesday", 3 * DAY),
  failing,
];

const map = {
  cells: [
    {
      name: "E2ZIAQ",
      title: "Draw the grove",
      standing: "needs-you",
      cwd: "/home/ada/trees/fresh/modules/repos",
      networks: ["repo.world", "repo.core", "harness"],
    },
    { name: "QUIET1", title: "Rest", standing: "quiet", cwd: "/home/ada/src/core" },
    { name: "AWAY01", title: "Elsewhere", standing: "moving", cwd: "/home/ada/notes" },
    { name: "NOWHERE", title: "No folder", standing: "moving" },
  ],
};

it("draws nothing until the daemon has said its reading", () => {
  assert.equal(groveOf(told([core]), [], NOW), undefined, "a grove came without a reading");
});

it("keeps each repository in its place by name, whatever grew", () => {
  const grove = groveOf(told(records), [], NOW);
  assert.deepEqual(
    grove?.repositories.map((each) => each.name),
    ["core", "world"],
  );
  assert.equal(grove?.machine, "kestrel");
});

it("groups trees by when each was written, today's first and the gone last", () => {
  const drawn = groveOf(told(records), [], NOW)?.repositories[0];
  assert.deepEqual(
    drawn?.trees.map((each) => [each.path.split("/").at(-1), each.season]),
    [
      ["fresh", "today"],
      ["tuesday", "week"],
      ["old", "resting"],
      ["lost", "gone"],
    ],
  );
  assert.deepEqual(drawn?.seasons, { today: 1, week: 1, resting: 1, gone: 1 });
});

it("keeps trees of one season in their places by branch, whichever was written last", () => {
  const two = [
    reading,
    core,
    tree("/home/ada/trees/zebra", 2 * MINUTE),
    tree("/home/ada/trees/apple", HOUR),
  ];
  const trees = groveOf(told(two), [], NOW)?.repositories[0]?.trees;
  assert.deepEqual(
    trees?.map((each) => each.branch),
    ["apple", "zebra"],
    "a tree written moved its place among today's",
  );
});

it("names the folder most of a repository's trees are cut in, so each says only its own name", () => {
  const elsewhere = [...records, tree("/home/ada/scratch/odd", HOUR)];
  const [many, none] = groveOf(told(elsewhere), [], NOW)?.repositories ?? [];
  assert.equal(many?.cutIn, "/home/ada/trees", "the folder four of five trees share was missed");
  assert.equal(none?.cutIn, undefined, "a repository with no trees was given a folder");
  const lone = groveOf(told([reading, core, tree("/home/ada/trees/only", HOUR)]), [], NOW);
  assert.equal(
    lone?.repositories[0]?.cutIn,
    undefined,
    "a lone tree's folder was said apart from it",
  );
});

it("calls a tree written yesterday this week's, not today's", () => {
  const evening = Date.parse("2026-11-16T23:30:00");
  assert.equal(seasonOf(true, Date.parse("2026-11-16T00:10:00"), evening), "today");
  assert.equal(seasonOf(true, Date.parse("2026-11-15T23:50:00"), evening), "week");
  assert.equal(seasonOf(true, undefined, evening), "resting");
});

it("gives each cell to the deepest folder it works in, and none outside them", () => {
  const drawn = groveOf(told(records), cellsOf(map), NOW)?.repositories[0];
  assert.deepEqual(
    drawn?.trees.find((each) => each.path.endsWith("fresh"))?.cells.map((each) => each.name),
    ["E2ZIAQ"],
  );
  assert.deepEqual(
    drawn?.checkouts[0]?.cells.map((each) => each.name),
    ["QUIET1"],
  );
  assert.deepEqual(
    drawn?.cells.map((each) => each.name),
    ["QUIET1", "E2ZIAQ"],
  );
});

it("joins repositories one cell works across, by the networks it's in", () => {
  const grove = groveOf(told(records), cellsOf(map), NOW);
  assert.deepEqual(grove?.repositories[0]?.joined, ["github:owl/world"]);
  assert.deepEqual(grove?.repositories[1]?.joined, [], "world has no cell of its own at work");
});

it("joins nothing by a cell that's quiet or resting, as the page counts only cells at work", () => {
  for (const standing of ["quiet", "dormant"]) {
    const resting = { ...map.cells[0], standing };
    const grove = groveOf(told(records), cellsOf({ cells: [resting] }), NOW);
    assert.deepEqual(grove?.repositories[0]?.joined, [], `a ${standing} cell joined core`);
  }
});

/**
 * What core is joined to, with world named `name`, core's cell in `networks`, and `more` read
 * before every other record, or after them when `last`.
 */
function coreJoined(
  name: string,
  networks: string[],
  more: typeof records = [],
  last = false,
): readonly string[] {
  const named = records.map((each) => (each === world ? { ...each, name } : each));
  const cell = { ...map.cells[0], networks: [...networks, "repo.core"] };
  const all = last ? [...named, ...more] : [...more, ...named];
  const grove = groveOf(told(all), cellsOf({ cells: [cell] }), NOW);
  const drawn = grove?.repositories.find((each) => each.key === "github:owl/core");
  assert.ok(drawn !== undefined, "core wasn't drawn");
  return drawn.joined;
}

it("joins a repository by its name as the mesh spells it, lower-cased and hyphened", () => {
  const spellings = [
    ["_World..Nix", "world-nix"],
    ["World-Nix-", "world-nix"],
    [".World.Nix.", "world-nix"],
    ["world2.nix", "world2-nix"],
    ["agentic-harness.pi", "agentic-harness-pi"],
  ];
  for (const [name = "", network] of spellings) {
    assert.deepEqual(coreJoined(name, [`repo.${network}`]), ["github:owl/world"], name);
  }
});

it("joins nothing by a spelling two repositories share, or by none", () => {
  // Each read before and after the rest, so neither the first read nor the last can win.
  for (const last of [false, true]) {
    const twin = { ...world, key: "repository:github:fox/world", owner: "fox" };
    assert.deepEqual(coreJoined("world", ["repo.world"], [twin], last), [], "a world was joined");
    const spelledAlike = { ...twin, name: "World.Nix" };
    const both = coreJoined("world-nix", ["repo.world-nix"], [spelledAlike], last);
    assert.deepEqual(both, [], "one of two names spelled alike was joined");
    const alike = { ...core, key: "repository:github:fox/core", owner: "fox" };
    assert.deepEqual(coreJoined("world", [], [alike], last), [], "core was joined to its namesake");
  }
  assert.deepEqual(coreJoined("...", ["repo."]), [], "a name the mesh spells as nothing joined");
});

it("says what works in a repository whole, and in each checkout and tree apart", () => {
  const using = {
    key: "using:github:owl/core",
    kind: "using",
    repository: "github:owl/core",
    processes: 3,
    memory: 4000,
    cpu: 1.75,
    folders: [
      { path: "/home/ada/src/core", processes: 2, memory: 3000, cpu: 0.25 },
      { path: "/home/ada/trees/fresh", processes: 1, memory: 1000, cpu: 1.5 },
    ],
  };
  const grove = groveOf(told([...records, using]), [], NOW);
  const [measured, unmeasured] = grove?.repositories ?? [];
  assert.deepEqual(measured?.using, { processes: 3, memory: 4000, cpu: 1.75 });
  assert.deepEqual(measured?.checkouts[0]?.using, { processes: 2, memory: 3000, cpu: 0.25 });
  const fresh = measured?.trees.find((each) => each.path === "/home/ada/trees/fresh");
  assert.deepEqual(fresh?.using, { processes: 1, memory: 1000, cpu: 1.5 });
  const old = measured?.trees.find((each) => each.path === "/home/ada/trees/old");
  assert.equal(old?.using, undefined, "a tree nothing works in says nothing");
  assert.equal(
    unmeasured?.using,
    undefined,
    "a repository the daemon hasn't measured says nothing",
  );
});

it("hangs each change on its repository and on the tree that names it", () => {
  const drawn = groveOf(told(records), [], NOW)?.repositories[0];
  assert.deepEqual(
    drawn?.changes.map((each) => [each.number, each.checks, each.failed, each.review]),
    [[41, "failing", ["lint"], "waiting"]],
  );
  assert.equal(drawn?.trees[0]?.change?.number, 41);
});

it("says what's worth a look worst first, never a total", () => {
  const grove = groveOf(told(records), cellsOf(map), NOW);
  assert.deepEqual(
    grove?.worth.map((each) => `${each.repository.name} ${each.says}`),
    [
      "core has 1 cell waiting on you.",
      "core has 1 change whose checks fail.",
      "core still lists 1 tree whose folders are gone; pruning its worktrees clears them.",
    ],
  );
});

it("says a pile of resting trees once it reaches ten", () => {
  const resting = Array.from({ length: 10 }, (_, index) =>
    tree(`/home/ada/trees/r${index}`, 9 * DAY),
  );
  const grove = groveOf(told([reading, core, ...resting]), [], NOW);
  assert.deepEqual(
    grove?.worth.map((each) => each.says),
    ["rests 10 trees unwritten for over a week."],
  );
});

it("addresses a repository by its owner and name, encoded", () => {
  const grove = groveOf(told(records), [], NOW);
  assert.equal(grove?.repositories[0]?.address, "#repo=owl%2Fcore");
});

it("says what a source couldn't read", () => {
  assert.deepEqual(groveOf(told(records), [], NOW)?.unread, ["github: owl/nest: not found"]);
});

it("counts as growing only a repository with a tree written today", () => {
  const grove = groveOf(told(records), [], NOW);
  assert.deepEqual(grove === undefined ? [] : growing(grove).map((each) => each.name), ["core"]);
});

it("reads no cell from a map it can't read", () => {
  assert.deepEqual(cellsOf({ cells: "none" }), []);
  assert.deepEqual(cellsOf(null), []);
});

it("steps the cursor within the rows, an endless step to the first or the last", () => {
  assert.equal(stepped(2, 1, 5), 3, "j didn't step one down");
  assert.equal(stepped(4, 1, 5), 4, "j stepped past the last row");
  assert.equal(stepped(0, -1, 5), 0, "k stepped above the first row");
  assert.equal(stepped(-1, 1, 5), 0, "j from no cursor didn't land on the first");
  assert.equal(stepped(2, Number.POSITIVE_INFINITY, 5), 4, "G didn't go to the last");
  assert.equal(stepped(2, Number.NEGATIVE_INFINITY, 5), 0, "g didn't go to the first");
});

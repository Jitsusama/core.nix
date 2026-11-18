// How the map of the mycelium between repositories is worked out: which repositories a cell's
// work ties together, the rows and threads drawn of them, the walk through the rows and what each
// row says. The drawing itself is tried live, in code-reads-live-test.py.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { ties } from "../joins.ts";
import { mapOf, saidOf, walked } from "../map.ts";
import type { Cell, Repository } from "../model.ts";

/** A cell at work, in the networks named. */
function cell(name: string, networks: readonly string[]): Cell {
  return { name, title: "", standing: "moving", cwd: "/home/ada", networks };
}

/** A repository named `name`, with `cells` at work in it and nothing else. */
function repository(name: string, cells: readonly Cell[] = []): Repository {
  return {
    key: `github:owl/${name}`,
    name,
    owner: "owl",
    trunk: "main",
    writtenAt: undefined,
    checkouts: [],
    trees: [],
    cutIn: undefined,
    changes: [],
    seasons: { today: 0, week: 0, resting: 0, gone: 0 },
    cells,
    joined: [],
    address: `#repo=owl%2F${name}`,
  };
}

// Core's cells work across the world and the harness, and a world cell across core; the lone
// repository works with none, and the quiet one only within itself.
const grove = [
  repository("core", [
    cell("A", ["repo.world", "repo.core"]),
    cell("B", ["repo.world", "repo.harness", "harness"]),
  ]),
  repository("harness"),
  repository("lone"),
  repository("quiet", [cell("D", ["repo.quiet"])]),
  repository("world", [cell("C", ["repo.core"]), cell("A", ["repo.world", "repo.core"])]),
];

it("ties two repositories once, with every cell whose work spans both", () => {
  assert.deepEqual(ties(grove), [
    { ends: ["github:owl/core", "github:owl/harness"], cells: ["B"] },
    { ends: ["github:owl/core", "github:owl/world"], cells: ["A", "B", "C"] },
  ]);
});

it("ties nothing by a spelling two repositories share, though the other side may", () => {
  // Core's cells name a world two repositories are spelled as, so only world's own cells,
  // which name core, tie the two.
  const twin = { ...repository("world"), key: "github:fox/world", owner: "fox" };
  assert.deepEqual(ties([...grove, twin]), [
    { ends: ["github:owl/core", "github:owl/harness"], cells: ["B"] },
    { ends: ["github:owl/core", "github:owl/world"], cells: ["A", "C"] },
  ]);
});

it("draws a row for each tied repository, in the grove's order, and a thread for each cell", () => {
  const map = mapOf(grove);
  assert.deepEqual(
    map.rows.map((each) => each.key),
    ["github:owl/core", "github:owl/harness", "github:owl/world"],
  );
  assert.deepEqual(
    map.threads.map((each) => [each.from, each.to, each.cell, each.lane, each.lanes]),
    [
      [0, 1, "B", 0, 1],
      [0, 2, "A", 0, 3],
      [0, 2, "B", 1, 3],
      [0, 2, "C", 2, 3],
    ],
  );
});

it("draws at most six threads for a tie, while its words count every cell", () => {
  const many = Array.from({ length: 9 }, (_, at) => cell(`M${at}`, ["repo.world"]));
  const map = mapOf([repository("core", many), repository("world")]);
  assert.equal(map.threads.length, 6, "a tie drew a thread past six");
  assert.equal(saidOf(map, 0), "Joined to world by 9\u00a0cells.");
});

it("says each row's ties, the most cells first, and its cells by name", () => {
  const map = mapOf(grove);
  assert.equal(saidOf(map, 0), "Joined to world by 3\u00a0cells, and harness by 1\u00a0cell.");
  assert.equal(saidOf(map, 1), "Joined to core by 1\u00a0cell.");
});

it("steps up and down the rows, stays at either end, and never goes sideways", () => {
  const map = mapOf(grove);
  assert.equal(walked(map, 0, "down"), 1);
  assert.equal(walked(map, 1, "up"), 0);
  assert.equal(walked(map, 0, "up"), undefined, "the walk went past the first row");
  assert.equal(walked(map, 2, "down"), undefined, "the walk went past the last row");
  assert.equal(walked(map, 1, "left"), undefined, "the walk went sideways");
  assert.equal(walked(map, 1, "right"), undefined, "the walk went sideways");
  assert.equal(walked(map, 1, "first"), 0);
  assert.equal(walked(map, 0, "last"), 2);
});

it("draws nothing when no cell ties two repositories", () => {
  const map = mapOf([repository("lone"), repository("quiet", [cell("D", ["repo.quiet"])])]);
  assert.deepEqual([map.rows, map.threads], [[], []]);
});

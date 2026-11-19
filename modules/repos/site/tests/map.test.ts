// How the map of the mycelium between repositories is worked out: which repositories a cell's
// work ties together, the rows and threads drawn of them, the walk through the rows and what each
// row says. The drawing itself is tried live, in code-reads-live-test.py.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { leads, ties } from "../joins.ts";
import { bowed, kept, mapOf, saidOf, THREADS_A_TIE, walked } from "../map.ts";
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

it("keeps two ties apart whatever their keys hold, a local folder's line breaks too", () => {
  // A local repository is keyed by its folder, which may hold a line break, so these two ties'
  // ends read alike when joined by one.
  const keyed = (key: string, name: string, working: readonly Cell[] = []): Repository => ({
    ...repository(name, working),
    key,
  });
  const folders = [
    keyed("local:x", "xa", [cell("C1", ["repo.ya"])]),
    keyed("local:y\nz", "ya"),
    keyed("local:x\nlocal:y", "ra", [cell("C2", ["repo.sa"])]),
    keyed("z", "sa"),
  ];
  const tying = ties(folders).map((tie) => tie.cells.join());
  assert.deepEqual(
    tying.sort((one, other) => one.localeCompare(other)),
    ["C1", "C2"],
  );
});

it("ties two folders once, though their keys read alike to a reader of words", () => {
  // One folder's name is written composed and the other's decomposed, as a Mac writes it, so a
  // comparison of words calls them equal and each side would put its own first.
  const keyed = (key: string, name: string, working: readonly Cell[]): Repository => ({
    ...repository(name, working),
    key,
  });
  const folders = [
    keyed("local:~/caf\u00e9", "aa", [cell("C1", ["repo.bb"])]),
    keyed("local:~/cafe\u0301", "bb", [cell("C1", ["repo.aa"])]),
  ];
  assert.deepEqual(
    ties(folders).map((tie) => tie.cells),
    [["C1"]],
  );
});

it("orders ties by their first end, then their second, as their keys read", () => {
  // Joined whole, a and z read after ab and c, which a comparison that skips the null between
  // them would put first.
  const spread = [
    repository("a", [cell("C1", ["repo.z"])]),
    repository("ab", [cell("C2", ["repo.c"])]),
    repository("c"),
    repository("z"),
  ];
  assert.deepEqual(
    ties(spread).map((tie) => tie.ends),
    [
      ["github:owl/a", "github:owl/z"],
      ["github:owl/ab", "github:owl/c"],
    ],
  );
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
    map.threads.map((each) => [each.from, each.to, each.cell, each.lane]),
    [
      [0, 1, "B", 0],
      [0, 2, "A", 0],
      [0, 2, "B", 1],
      [0, 2, "C", 2],
    ],
  );
});

it("draws at most six threads for a tie, while its words count every cell", () => {
  const many = Array.from({ length: 9 }, (_, at) => cell(`M${at}`, ["repo.world"]));
  const map = mapOf([repository("core", many), repository("world")]);
  assert.equal(map.threads.length, 6, "a tie drew a thread past six");
  assert.equal(saidOf(map, 0), "Joined to world by 9\u00a0cells.");
});

it("says each row's ties, the most cells first, and ties of as many by name", () => {
  const map = mapOf(grove);
  assert.equal(saidOf(map, 0), "Joined to world by 3\u00a0cells, and harness by 1\u00a0cell.");
  assert.equal(saidOf(map, 1), "Joined to core by 1\u00a0cell.");
  // Their keys sort the other way to their names, so only the names can put alpha first.
  const alike = mapOf([
    repository("core", [cell("Z", ["repo.zeta"]), cell("A", ["repo.alpha"])]),
    { ...repository("zeta"), key: "github:owl/a-zeta" },
    { ...repository("alpha"), key: "github:owl/z-alpha" },
  ]);
  assert.equal(saidOf(alike, 0), "Joined to alpha by 1\u00a0cell, and zeta by 1\u00a0cell.");
});

it("keeps the grove's order of the rows, whatever their names", () => {
  const map = mapOf([repository("zeta", [cell("Z", ["repo.alpha"])]), repository("alpha")]);
  assert.deepEqual(
    map.rows.map((each) => each.name),
    ["zeta", "alpha"],
  );
});

it("bows each of a tie's threads apart, the farthest tie's too, inside the gutter", () => {
  const many = Array.from({ length: THREADS_A_TIE }, (_, at) => cell(`M${at}`, ["repo.world"]));
  const map = mapOf([repository("core", many), repository("world")]);
  const bows = map.threads.map((each) => bowed(each, map.rows.length));
  assert.equal(new Set(bows).size, THREADS_A_TIE, `two threads bowed alike: ${bows}`);
  assert.ok(Math.max(...bows) <= 96, `a thread bowed past the gutter: ${bows}`);
});

it("bows a farther tie deeper than a nearer one", () => {
  const map = mapOf(grove);
  const [near, far] = map.threads.map((each) => bowed(each, map.rows.length));
  assert.ok((near ?? 0) < (far ?? 0), `the tie past a row bowed ${far}, the next row's ${near}`);
});

it("ties nothing by a cell that's quiet or resting, as the page counts only cells at work", () => {
  const resting = ["quiet", "dormant"].map((standing, at) => ({
    ...cell(`R${at}`, ["repo.world"]),
    standing,
  }));
  const map = mapOf([repository("core", resting), repository("world")]);
  assert.deepEqual(map.ties, [], "a cell not at work tied two repositories");
});

it("leads each cell at work on to the others it works across, once each and in order", () => {
  const resting = { ...cell("R", ["repo.world"]), standing: "dormant" };
  const core = repository("core", [
    cell("A", ["repo.world", "repo.harness", "repo.world"]),
    resting,
  ]);
  const led = leads(core, [core, repository("world"), repository("harness")]);
  assert.deepEqual([...led], [["A", ["github:owl/harness", "github:owl/world"]]]);
});

it("keeps the keys on their repository as rows come and go around it", () => {
  // Harness ties in above world, so world moves a row down, and the keys move with it.
  const before = mapOf([
    repository("core", [cell("A", ["repo.world"])]),
    repository("harness"),
    repository("world"),
  ]);
  const after = mapOf(grove);
  assert.equal(after.rows[kept(before, 1, after)]?.name, "world", "the keys stayed on the row");
  // World's tie goes, so the keys rest on the row nearest the one they left.
  const gone = mapOf([repository("core", [cell("B", ["repo.harness"])]), repository("harness")]);
  assert.equal(kept(after, 2, gone), 1, "the keys left the map");
  assert.equal(kept(after, 0, mapOf([])), 0, "the keys rested past an empty map");
});

it("writes each other row's name as it's asked, so the page can link it home", () => {
  const map = mapOf(grove);
  const linked = saidOf(map, 1, (row) => `[${row.name}](${row.address})`);
  assert.equal(linked, `Joined to [core](${map.rows[0]?.address}) by 1\u00a0cell.`);
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

// Where the map puts everything: seeds kept while the page is open, discs
// that keep room, the ring, its branches, the receptors and builders, and
// the hyphae a network grows.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  CORD_BUNDLE_OVER,
  colonyRadius,
  control,
  type Hub,
  HUB_R,
  type Layout,
  layOut,
  layoutKeyOf,
  MACHINE_ORBIT,
  type Point,
  placeCells,
  R,
  type Realm,
  roomFor,
  type Seeded,
  type Seeds,
  seedAt,
  shortestTree,
  sow,
} from "../layout.ts";

const collate = new Intl.Collator("en-CA").compare;

/** Seeds the sunflower lays out, enough for a busy network. */
const MANY_SEEDS = 300;

/** A network's hub, with the cells given hanging from it. */
const hubOf = (key: string, fields: Partial<Hub> = {}): Hub => ({
  key,
  name: key,
  children: [],
  cells: [],
  ...fields,
});

/** A cell on `hub`, come at `since`. */
const cellOn = (id: string, hub: string, since = ""): Seeded => ({ id, since, hub: { key: hub } });

/** A cell working alone or a receptor, loose on `hub`, come at `since`. */
const looseOn = (id: string, hub: string, since = ""): Seeded => ({
  id,
  since,
  loose: { key: hub },
});

/** A realm of the hubs given, their cells, and the machines and builders named. */
const realmOf = (hubs: readonly Hub[], machines: string[] = [], builders: string[] = []): Realm => {
  const cells = hubs.flatMap((h) => h.cells);
  return {
    hubs: new Map(hubs.map((h) => [h.key, h])),
    cells,
    byId: new Map(cells.map((s) => [s.id, s])),
    machines: machines.map((name) => ({ name })),
    builders: builders.map((name) => ({ name })),
  };
};

const distance = ([x1, y1]: Point, [x2, y2]: Point): number => Math.hypot(x2 - x1, y2 - y1);

/** Where `key` stands in `layout`, which must place it. */
const where = (layout: Layout, key: string): Point => {
  const at = layout.at.get(key);
  assert.ok(at, `${key} was never placed`);
  return at;
};

/** How far `key`'s disc reaches in `layout`. */
const discOf = (layout: Layout, key: string): number => layout.disc.get(key) ?? 0;

it("keeps each cell's seed, giving a newcomer the seed an ended cell left", () => {
  const seeds: Seeds = new Map();
  sow(seeds, realmOf([hubOf("h", { cells: [cellOn("b", "h", "2"), cellOn("a", "h", "1")] })]));
  assert.deepEqual(seeds.get("h"), ["a", "b"], "cells should be sown oldest first");
  sow(seeds, realmOf([hubOf("h", { cells: [cellOn("b", "h", "2"), cellOn("c", "h", "3")] })]));
  assert.deepEqual(seeds.get("h"), ["c", "b"], "a newcomer should take the ended cell's seed");
  const cells = [cellOn("b", "h", "2"), cellOn("c", "h", "3"), cellOn("d", "h", "4")];
  sow(seeds, realmOf([hubOf("h", { cells })]));
  assert.deepEqual(seeds.get("h"), ["c", "b", "d"], "with no seed free, one past the last");
});

it("keeps a loose cell's seed, as a receptor's or a cell's working alone", () => {
  const seeds: Seeds = new Map();
  sow(seeds, realmOf([hubOf("h", { cells: [looseOn("a", "h", "1"), looseOn("b", "h", "2")] })]));
  sow(seeds, realmOf([hubOf("h", { cells: [looseOn("b", "h", "2")] })]));
  assert.deepEqual(seeds.get("h"), [null, "b"], "a loose cell should keep its seed");
});

it("sows cells come at once in the order of their ids", () => {
  const seeds: Seeds = new Map();
  sow(seeds, realmOf([hubOf("h", { cells: [cellOn("b", "h", "1"), cellOn("a", "h", "1")] })]));
  assert.deepEqual(seeds.get("h"), ["a", "b"], "a tie should go by id, so a redraw keeps it");
});

it("frees a cell's seed in a hub it has left", () => {
  const seeds: Seeds = new Map();
  sow(seeds, realmOf([hubOf("h", { cells: [cellOn("a", "h")] }), hubOf("g")]));
  sow(seeds, realmOf([hubOf("h"), hubOf("g", { cells: [cellOn("a", "g")] })]));
  assert.deepEqual(seeds.get("h"), [null], "a cell that moved should leave its old seed free");
  assert.deepEqual(seeds.get("g"), ["a"], "a cell that moved should take a seed where it went");
});

it("keeps a disc's room in steps, so a cell arriving moves no hub", () => {
  const rooms = [0, 1, 2, 3, 4].map(roomFor);
  assert.deepEqual(new Set(rooms), new Set([8]), `up to four cells kept ${rooms}`);
  assert.equal(roomFor(5), 16, "a fifth cell should step the room up");
  const seeds: Seeds = new Map();
  const cells = [cellOn("a", "h")];
  sow(seeds, realmOf([hubOf("h", { cells })], ["kestrel"]));
  const before = layoutKeyOf(realmOf([hubOf("h", { cells })], ["kestrel"]), seeds);
  const more = [...cells, cellOn("b", "h")];
  sow(seeds, realmOf([hubOf("h", { cells: more })], ["kestrel"]));
  const after = layoutKeyOf(realmOf([hubOf("h", { cells: more })], ["kestrel"]), seeds);
  assert.equal(after, before, "a cell arriving within the room should leave the layout as it is");
});

it("lays out afresh when anything the layout reads changes", () => {
  const keyOf = (hubs: Hub[], machines = ["kestrel"], builders: string[] = []): string => {
    const realm = realmOf(hubs, machines, builders);
    const seeds: Seeds = new Map();
    sow(seeds, realm);
    return layoutKeyOf(realm, seeds);
  };
  const base = keyOf([hubOf("h", { machine: "kestrel" })]);
  const five = Array.from({ length: 5 }, (_, i) => cellOn(`c${i}`, "h"));
  const changed: [string, string][] = [
    ["a hub added", keyOf([hubOf("h", { machine: "kestrel" }), hubOf("g")])],
    ["a hub's machine", keyOf([hubOf("h", { machine: "osprey" })])],
    ["a hub's parent", keyOf([hubOf("h", { machine: "kestrel", within: "g" })])],
    ["a room stepping up", keyOf([hubOf("h", { machine: "kestrel", cells: five })])],
    ["a machine", keyOf([hubOf("h", { machine: "kestrel" })], ["kestrel", "osprey"])],
    ["a builder", keyOf([hubOf("h", { machine: "kestrel" })], ["kestrel"], ["heron"])],
  ];
  for (const [what, key] of changed) {
    assert.notEqual(key, base, `${what} should lay the map out afresh`);
  }
  const two = keyOf([hubOf("x"), hubOf("y")], []);
  for (const name of ["x:8::|y", 'x",8,"",""],["y']) {
    const one = keyOf([hubOf(name)], []);
    assert.notEqual(one, two, `a hub named ${name} should not read as two hubs`);
  }
});

it("packs seeds clear of the hub and of each other", () => {
  const points = Array.from({ length: MANY_SEEDS }, (_, i) => seedAt(i));
  const nearest = Math.min(...points.map((p) => distance(p, [0, 0])));
  assert.ok(nearest > HUB_R + R, `a seed stood ${nearest} from its hub, on top of it`);
  let closest = Number.POSITIVE_INFINITY;
  points.forEach((p, i) => {
    for (const q of points.slice(0, i)) {
      closest = Math.min(closest, distance(p, q));
    }
  });
  assert.ok(closest > 2 * R, `two seeds stood ${closest} apart, their membranes overlapping`);
});

it("rings the networks round the machines, branches out past their parent", () => {
  const branch = hubOf("branch", { machine: "kestrel", within: "root" });
  const root = hubOf("root", { machine: "kestrel", children: [branch] });
  const other = hubOf("other", { machine: "kestrel" });
  const seeds: Seeds = new Map();
  const realm = realmOf([root, branch, other], ["kestrel"]);
  sow(seeds, realm);
  const layout = layOut(realm, seeds);
  assert.deepEqual(
    layout.machines.get("kestrel"),
    [0, 0],
    "one machine should stand at the middle",
  );
  for (const key of ["root", "other"]) {
    const d = distance(where(layout, key), [0, 0]);
    assert.ok(d >= layout.ring, `${key} stood ${d} out, inside the ring at ${layout.ring}`);
  }
  const apart = distance(where(layout, "root"), where(layout, "other"));
  const reach = discOf(layout, "root") + discOf(layout, "other");
  assert.ok(apart >= reach, `two networks' discs overlap: ${apart} apart, reaching ${reach}`);
  const out = distance(where(layout, "branch"), [0, 0]);
  const parent = distance(where(layout, "root"), [0, 0]);
  assert.ok(out > parent, `a branch stood ${out} out, no further than its parent at ${parent}`);
});

it("puts the receptors above everything and the builders below", () => {
  const edge = hubOf("#outside", { edge: true });
  const net = hubOf("net", { machine: "kestrel" });
  const seeds: Seeds = new Map();
  const realm = realmOf([edge, net], ["kestrel", "osprey"], ["heron", "tern"]);
  sow(seeds, realm);
  const layout = layOut(realm, seeds);
  const [ex, ey] = where(layout, "#outside");
  const [, ny] = where(layout, "net");
  assert.equal(ex, 0, "the receptors should stand over the middle");
  assert.ok(ey < ny - discOf(layout, "net"), `the receptors at ${ey} stood amid the networks`);
  const [heron, tern] = ["heron", "tern"].map((b) => layout.builders.get(b) ?? [0, 0]);
  const lowest = Math.max(...[...layout.machines.values()].map(([, y]) => y));
  assert.ok((heron?.[1] ?? 0) > lowest, "the builders should stand under the machines");
  assert.ok(
    (tern?.[1] ?? 0) > (heron?.[1] ?? 0),
    "each builder should stand a step under the last",
  );
  const [kestrel, osprey] = ["kestrel", "osprey"].map((x) => layout.machines.get(x) ?? [0, 0]);
  assert.equal(kestrel?.[0], osprey?.[0], "machines should stand in a column");
});

it("turns the top machine's arc to the top, the next machines' after, and lone ones last", () => {
  const hubs = [
    hubOf("kestrel-net", { machine: "kestrel" }),
    hubOf("alone", { machine: "kestrel", alone: true }),
    hubOf("osprey-net", { machine: "osprey" }),
  ];
  const seeds: Seeds = new Map();
  const realm = realmOf(hubs, ["kestrel", "osprey"]);
  sow(seeds, realm);
  const layout = layOut(realm, seeds);
  const angle = (key: string): number => layout.angle.get(key) ?? Number.NaN;
  const top = angle("kestrel-net");
  assert.ok(Math.abs(top + Math.PI / 2) < 1e-9, `the top machine's arc sat at ${top}, not the top`);
  assert.ok(angle("osprey-net") > top, "the next machine's arc should come round after");
  assert.ok(angle("alone") > angle("osprey-net"), "a network working alone should come last");
});

it("moves each machine out to its own arc once its cords bundle", () => {
  const many = Array.from({ length: CORD_BUNDLE_OVER + 1 }, (_, i) =>
    hubOf(`net${String(i).padStart(2, "0")}`, { machine: "kestrel" }),
  );
  const seeds: Seeds = new Map();
  const realm = realmOf([...many, hubOf("lone", { machine: "osprey" })], ["kestrel", "osprey"]);
  sow(seeds, realm);
  const layout = layOut(realm, seeds);
  // kestrel's arc is turned to the top, so osprey's lone network stands
  // straight below, and each machine moves out toward its own.
  const [kestrel, osprey] = ["kestrel", "osprey"].map((x) => layout.machines.get(x) ?? [0, 0]);
  const out = layout.ring * MACHINE_ORBIT;
  assert.ok((kestrel?.[1] ?? 0) < 0, `kestrel stood at ${kestrel}, not up toward its arc`);
  assert.ok((osprey?.[1] ?? 0) > 0, `osprey stood at ${osprey}, not down toward its arc`);
  const reached = distance(osprey ?? [0, 0], [0, 0]);
  assert.ok(Math.abs(reached - out) < 1e-9, `osprey moved ${reached} out, not ${out}`);
});

it("places each cell at its hub's place plus its seed's, leaving ended seeds empty", () => {
  const seeds: Seeds = new Map([["h", [null, "b"]]]);
  const realm = realmOf([hubOf("h", { cells: [cellOn("b", "h")] })], ["kestrel"]);
  const layout = layOut(realm, seeds);
  const pos = placeCells(realm, layout, seeds);
  const [hx, hy] = where(layout, "h");
  const [dx, dy] = seedAt(1);
  assert.deepEqual([...pos.keys()], ["b"], "only cells still in the reading should be placed");
  assert.deepEqual(pos.get("b"), [hx + dx, hy + dy], "a cell should stand at its own seed");
  assert.equal(
    colonyRadius(seeds, "nobody"),
    HUB_R,
    "a hub with no cells should reach its own edge",
  );
  const last = distance(seedAt(1), [0, 0]) + R;
  assert.ok(
    Math.abs(colonyRadius(seeds, "h") - last) < 1e-9,
    `a ground reached ${colonyRadius(seeds, "h")}, not its last seed's membrane at ${last}`,
  );
});

it("grows the shortest tree joining a network's cells", () => {
  assert.deepEqual(shortestTree([[0, 0]]), [], "one cell should grow no hypha");
  const tree = shortestTree([
    [0, 0],
    [30, 0],
    [10, 0],
    [20, 0],
  ]);
  const joined = tree.map(([a, b]) => [Math.min(a, b), Math.max(a, b)].join("-")).sort(collate);
  assert.deepEqual(joined, ["0-2", "1-3", "2-3"], "cells in a row should join their neighbours");
});

it("bows a line to one side of the straight way, by a share of its length", () => {
  assert.deepEqual(control([0, 0], [10, 0], 0.5), [5, 5], "a line along x should bow down");
  assert.deepEqual(control([0, 0], [10, 0], 0), [5, 0], "an unbowed line should stay straight");
});

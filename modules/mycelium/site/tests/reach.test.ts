// Whether the map reaches every cell: each miss Joel found, named, and a
// map that misses nothing answering with nothing.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { type Model, type Reading, asked, reachOf } from "../reach.ts";

const AT = "2026-11-16T22:00:00Z";

const kestrel = { name: "orchestration.kestrl", shape: "orchestration" };
const leaderless = { name: "orchestration.osprey", shape: "orchestration" };
const tern = { name: "orchestration.ternss", shape: "orchestration" };

/**
 * The map's hubs: kestrel's, led by KESTRL; osprey's, drawn by its
 * address in another case; tern's, under a parent the map doesn't draw;
 * and those working alone.
 */
const hubs = new Map([
  [
    "orchestration.kestrl",
    { key: "orchestration.kestrl", name: "Kestrel", within: null, lead: { address: "KESTRL" } },
  ],
  [
    "orchestration.osprey",
    { key: "orchestration.osprey", name: "OSPREY", within: null, lead: null },
  ],
  [
    "orchestration.ternss",
    { key: "orchestration.ternss", name: "Tern", within: "orchestration.gone", lead: null },
  ],
  [
    "#loose.skiff",
    { key: "#loose.skiff", name: "Working alone on skiff", within: null, alone: true },
  ],
]);

/** A map that drew each of `cells` in the hub of its key. */
function drew(cells: Record<string, string>): Model {
  const byAddress = new Map(
    Object.entries(cells).map(([address, key]) => {
      const hub = hubs.get(key) ?? null;
      return [address, hub?.alone ? { address, hub: null, loose: hub } : { address, hub }];
    }),
  );
  return { hubs, byAddress };
}

function reading(cells: Reading["cells"]): Reading {
  return { at: AT, networks: [kestrel, leaderless, tern], cells };
}

it("a map that draws every cell where it belongs misses nothing", () => {
  const now = reading([
    { name: "KESTRL", type: "session", networks: ["orchestration.kestrl"] },
    { name: "TERN01", type: "session", networks: ["repo.skiff"] },
  ]);
  const reach = reachOf(now, drew({ KESTRL: "orchestration.kestrl", TERN01: "#loose.skiff" }));
  assert.deepEqual(reach, { at: AT, cells: 2, count: 0, failures: [] });
});

it("a cell of an orchestration drawn alone is named, with its lead to tell", () => {
  const now = reading([
    { name: "TERN02", type: "session", networks: ["orchestration.kestrl"] },
    { name: "TERN03", type: "session", networks: ["quest.kestrl.ternss"] },
  ]);
  const reach = reachOf(now, drew({ TERN02: "#loose.skiff", TERN03: "#loose.skiff" }));
  assert.deepEqual(reach.failures, [
    { cell: "TERN02", orchestration: "kestrl", owner: "KESTRL", why: "alone" },
    { cell: "TERN03", orchestration: "kestrl", owner: "KESTRL", why: "alone" },
  ]);
});

it("a receptor drawn alone is named, since it sits at the realm's edge", () => {
  const now = reading([{ name: "mail@skiff", type: "receptor", networks: ["outside.mail"] }]);
  assert.deepEqual(reachOf(now, drew({ "mail@skiff": "#loose.skiff" })).failures, [
    { cell: "mail@skiff", orchestration: null, owner: "mail@skiff", why: "alone" },
  ]);
});

it("a cell in a network with nothing but its address to go by is named", () => {
  const now = reading([{ name: "OSPREY", type: "session", networks: ["orchestration.osprey"] }]);
  assert.deepEqual(reachOf(now, drew({ OSPREY: "orchestration.osprey" })).failures, [
    { cell: "OSPREY", orchestration: "osprey", owner: "OSPREY", why: "unnamed network" },
  ]);
});

it("a cell under a network whose parent the map doesn't draw is unreachable", () => {
  const now = reading([{ name: "TERN04", type: "session", networks: ["orchestration.ternss"] }]);
  assert.deepEqual(reachOf(now, drew({ TERN04: "orchestration.ternss" })).failures, [
    { cell: "TERN04", orchestration: "ternss", owner: "TERN04", why: "unreachable" },
  ]);
});

it("a cell failing is told of the network the map drew it in", () => {
  const now = reading([
    { name: "OSPRY2", type: "session", networks: ["orchestration.kestrl", "orchestration.osprey"] },
  ]);
  assert.deepEqual(reachOf(now, drew({ OSPRY2: "orchestration.osprey" })).failures, [
    { cell: "OSPRY2", orchestration: "osprey", owner: "OSPRY2", why: "unnamed network" },
  ]);
});

it("a cell two machines both list is judged once", () => {
  const twice = { name: "TERN05", type: "session", networks: ["orchestration.kestrl"] };
  const reach = reachOf(reading([twice, twice]), drew({ TERN05: "#loose.skiff" }));
  assert.equal(reach.cells, 1);
  assert.equal(reach.count, 1);
});

it("a cell two machines list is judged as the map keeps it, by either copy's networks", () => {
  // The copy told last is the stale one, and alone it names no orchestration.
  const moved = {
    name: "TERN05",
    type: "session",
    since: "2026-11-19T00:30:00Z",
    networks: ["orchestration.kestrl"],
  };
  const stale = { ...moved, since: "2026-11-19T00:00:00Z", networks: ["repo.skiff"] };
  const reach = reachOf(reading([moved, stale]), drew({ TERN05: "#loose.skiff" }));
  assert.deepEqual(reach.failures, [
    { cell: "TERN05", orchestration: "kestrl", owner: "KESTRL", why: "alone" },
  ]);
});

it("a cell the map holds nowhere is unreachable", () => {
  const now = reading([{ name: "SKIFFS", type: "session", networks: ["orchestration.kestrl"] }]);
  const reach = reachOf(now, drew({}));
  assert.equal(reach.count, 1);
  assert.deepEqual(reach.failures[0], {
    cell: "SKIFFS",
    orchestration: "kestrl",
    owner: "KESTRL",
    why: "unreachable",
  });
});

it("only a page opened with reach says it", () => {
  assert.equal(asked("?reach"), true);
  assert.equal(asked("?look=dark&reach="), true);
  assert.equal(asked(""), false);
});

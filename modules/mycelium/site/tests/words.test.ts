// What each thing on the map is, in words: a cell by how it stands and what
// waits on it, a network by its cells' counts and what it and its branches
// spent, a machine by whether it's ready, and the label each goes by. The
// words are written out whole, so an edit to any of them shows here.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  allCells,
  type Cell,
  cellWords,
  countsOf,
  countsWords,
  described,
  hubWords,
  type Machine,
  machineWords,
  money,
  type Network,
  offWords,
  plural,
  type Reading,
  spends,
  spentOf,
  spentWords,
} from "../words.ts";

/** A cell on kestrel, quiet with nothing waiting, but for what `more` says. */
const cell = (address: string, more: Partial<Cell> = {}): Cell => ({
  address,
  short: address,
  standing: "quiet",
  owes: [],
  holds: [],
  machine: "kestrel",
  ...more,
});

/** A network on kestrel with no branches, holding `cells`, but for what `more` says. */
const network = (key: string, cells: readonly Cell[], more: Partial<Network> = {}): Network => ({
  key,
  name: key,
  within: null,
  machine: "kestrel",
  children: [],
  cells,
  ...more,
});

/** A reading of these networks, cells and machines. */
const reading = (
  hubs: readonly Network[],
  cells: readonly Cell[],
  machines: readonly Machine[],
): Reading => ({
  hubs: new Map(hubs.map((h) => [h.key, h])),
  byAddress: new Map(cells.map((s) => [s.address, s])),
  machines,
  cells,
});

it("counts in Canadian English and names more than one as told", () => {
  assert.equal(plural(1, "cell"), "1 cell", "one is said alone");
  assert.equal(plural(12_345, "cell"), "12,345 cells", "thousands take a comma");
  assert.equal(plural(2, "branch", "branches"), "2 branches", "a plural given is used");
  assert.equal(plural(0, "cell"), "0 cells", "none is plural");
});

it("says dollars to the cent below a hundred and whole above it", () => {
  assert.equal(money(3.456), "$3.46", "cents tell small sums apart");
  assert.equal(money(0), "$0.00", "nothing spent is a sum");
  assert.equal(money(99.994), "$99.99", "just under a hundred keeps cents");
  assert.equal(money(1234.5), "$1,235", "a hundred and up is whole");
  assert.equal(money(null), "", "no sum says nothing");
  assert.equal(money(undefined), "", "an unknown sum says nothing");
});

it("sums what cells spent, counting only those that say", () => {
  const cells = [
    cell("A", { usage: { cost_today: 1.5 } }),
    cell("B", { usage: { cost_today: 0 } }),
    cell("C", { usage: null }),
    cell("D", { usage: { cost_today: null } }),
  ];
  assert.equal(spentOf(cells), 1.5, "the sum of what was said");
  assert.equal(spends(cells), true, "a cell that spent nothing still says");
  assert.equal(
    spends([cells[2] as Cell, cells[3] as Cell]),
    false,
    "a null or missing cost says nothing",
  );
  assert.equal(spentWords(cells), ", $1.50 today", "the sum follows other words");
  assert.equal(spentWords([cells[2] as Cell]), "", "cells that say nothing add no words");
});

it("counts cells by standing, worst first, and those with questions waiting", () => {
  const cells = [
    cell("A", { standing: "asks", owes: [{}] }),
    cell("B", { standing: "quiet" }),
    cell("C", { standing: "quiet", owes: [{}, {}] }),
    cell("D", { standing: "stuck" }),
  ];
  assert.deepEqual(
    countsOf(cells),
    { asks: 1, stuck: 1, moving: 0, quiet: 2, dormant: 0 },
    "each standing counted",
  );
  assert.equal(
    countsWords(cells),
    "4 cells: 1 needs you, 1 stuck, 2 quiet; 2 with questions waiting",
    "worst first, then what waits",
  );
  assert.equal(
    countsWords([cells[0] as Cell]),
    "1 cell: 1 needs you; 1 with a question waiting",
    "one waiting is a question",
  );
  assert.equal(countsWords([]), "0 cells", "no cells, no standings");
});

it("says what a cell is doing and what waits on it or it holds", () => {
  assert.equal(
    cellWords(cell("A", { standing: "asks", wants: "a login" })),
    "needs you: a login",
    "what it needs, when said",
  );
  assert.equal(
    cellWords(cell("A", { standing: "asks" })),
    "needs you",
    "asking without saying what",
  );
  assert.equal(
    cellWords(cell("A", { standing: "moving", doing: "a build" })),
    "moving: a build",
    "what its turn does",
  );
  assert.equal(
    cellWords(cell("A", { standing: "moving" })),
    "moving: in a turn",
    "a turn not said",
  );
  assert.equal(cellWords(cell("A", { standing: "dormant" })), "dormant", "the rest by their word");
  assert.equal(
    cellWords(cell("A", { standing: "stuck", owes: [{}], holds: [{}] })),
    "stuck; a question waiting; a hold standing",
    "one question and one hold",
  );
  assert.equal(
    cellWords(cell("A", { owes: [{}, {}, {}], holds: [{}, {}] })),
    "quiet; 3 questions waiting; 2 holds standing",
    "several of each",
  );
});

it("says a network's own cells and what it spent, with its branches' where it has any", () => {
  const branch = network("quest.b", [cell("B", { usage: { cost_today: 2 } })], {
    within: "quest.a",
  });
  const top = network("quest.a", [cell("A", { standing: "stuck", usage: { cost_today: 1 } })], {
    children: [branch],
  });
  assert.deepEqual(
    allCells(top).map((s) => s.address),
    ["A", "B"],
    "its own cells, then its branches'",
  );
  assert.equal(
    hubWords(top),
    "1 cell: 1 stuck, $3.00 today with its branches",
    "branches' spending counted",
  );
  assert.equal(hubWords(branch), "1 cell: 1 quiet, $2.00 today", "a network alone says its own");
  assert.equal(
    hubWords(network("quest.c", [cell("C")])),
    "1 cell: 1 quiet",
    "nothing spent, nothing said",
  );
});

it("says whether a machine is ready, how many of a basin's are off, and its cells", () => {
  const cells = [cell("A", { usage: { cost_today: 4 } }), cell("B", { machine: "osprey" })];
  const harbour: Machine = {
    name: "harbour",
    state: "ready",
    members: [
      { name: "skiff", state: "ready" },
      { name: "tern", state: "off" },
    ],
  };
  const now = reading([], cells, [
    { name: "kestrel", state: "ready" },
    { name: "osprey", state: "off" },
    harbour,
  ]);
  assert.equal(
    machineWords(now, "kestrel"),
    "ready, 1 cell, $4.00 today",
    "ready, with what its cells spent",
  );
  assert.equal(machineWords(now, "osprey"), "off, 1 cell", "off");
  assert.equal(
    machineWords(now, "harbour"),
    "ready, 1 machine off, 0 cells",
    "a basin says which of its machines are off",
  );
  assert.equal(
    machineWords(now, "gull"),
    "off, 0 cells",
    "a machine the reading lacks reads as off",
  );
  assert.equal(offWords(harbour), "1 machine off", "one of the basin's is off");
  assert.equal(offWords({ name: "kestrel", state: "ready" }), "", "a machine alone has none off");
});

it("labels each thing by its name and then the words its row shows", () => {
  const a = cell("A", { short: "Tide", standing: "asks", wants: "a yes" });
  const branch = network("quest.b", [], { name: "Nets", within: "quest.a" });
  const top = network("quest.a", [a], { name: "Harbour Works", children: [branch] });
  const now = reading(
    [top, branch],
    [a],
    [
      { name: "kestrel", state: "ready" },
      { name: "harbour", state: "ready", members: [{ name: "skiff" }, { name: "tern" }] },
    ],
  );
  assert.equal(described(now, "c:A"), "Tide, needs you: a yes", "a cell by its short name");
  assert.equal(
    described(now, "h:quest.a"),
    "Harbour Works, 1 cell: 1 needs you, a network with 1 branch",
    "a network with its branches",
  );
  assert.equal(
    described(now, "h:quest.b"),
    "Nets, 0 cells, a network",
    "a branch with no branches of its own",
  );
  assert.equal(described(now, "m:kestrel"), "kestrel, ready, 1 cell, a machine", "a machine");
  assert.equal(
    described(now, "m:harbour"),
    "harbour, ready, 2 machines off, 0 cells, a basin of 2 machines",
    "a basin and how many it holds",
  );
  assert.equal(described(now, "c:GONE"), "GONE", "a cell gone by its address");
  assert.equal(described(now, "h:gone"), "gone", "a network gone by its key");
});

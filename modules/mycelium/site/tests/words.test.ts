// What each thing on the map is, in words: a cell by how it stands and what
// waits on it, a network by its cells' counts and what it and its branches
// spent, a machine by whether it's ready, and the label each goes by. The
// words are written out whole, so an edit to any of them shows here.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  allCells,
  type Builder,
  builderSays,
  builderTone,
  builderWords,
  builds,
  type Cell,
  cellsOn,
  cellWords,
  countsOf,
  countsWords,
  described,
  grewWords,
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
  splitSaid,
  toldApart,
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
  assert.equal(money(99.996), "$100.00", "and keeps them rounding up to a hundred");
  assert.equal(money(100), "$100", "a hundred is whole");
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
  assert.equal(
    spentWords([cells[1] as Cell]),
    ", $0.00 today",
    "a cell that spent nothing still says so alone",
  );
});

it("counts cells by standing, worst first, and the answers they owe", () => {
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
    "4 cells: 1 needs you, 1 stuck, 2 quiet; 3 answers owed",
    "worst first, then the answers owed",
  );
  assert.equal(
    countsWords([cells[0] as Cell]),
    "1 cell: 1 needs you; 1 answer owed",
    "one answer owed",
  );
  assert.equal(
    countsWords([cells[2] as Cell]),
    "1 cell: 1 quiet; 2 answers owed",
    "one cell owing two is two answers, as the drawer counts them",
  );
  assert.equal(countsWords([]), "0 cells", "no cells, no standings");
  const each: Cell["standing"][] = ["dormant", "quiet", "moving", "stuck", "asks"];
  assert.equal(
    countsWords(each.map((standing, i) => cell(`S${i}`, { standing }))),
    "5 cells: 1 needs you, 1 stuck, 1 moving, 1 quiet, 1 dormant",
    "every standing in its place, however the cells come",
  );
});

it("says a branch's starts and splits apart, each out of all its cells", () => {
  const who = (cells: readonly string[]): string => cells.join(" and ");
  assert.deepEqual(
    grewWords({ by: ["SKIFF1"], started: 1, split: { from: ["GULL01"], cells: 1 } }, 3, who),
    ["Started by SKIFF1: 1 of its 3 cells", "Split from GULL01: 1 of its 3 cells"],
    "a branch both started and split says each line with its own count",
  );
  assert.deepEqual(
    grewWords({ by: [], started: 0, split: { from: ["GULL01", "TERN01"], cells: 2 } }, 2, who),
    ["Split from GULL01 and TERN01: all 2 of its cells"],
    "a branch only split says no one started it",
  );
  assert.deepEqual(
    grewWords({ by: ["SKIFF1"], started: 1, split: null }, 1, who),
    ["Started by SKIFF1: its cell"],
    "a branch only started says no split",
  );
});

it("a cell says what it split from only where its start doesn't name it", () => {
  assert.equal(splitSaid("SKIFF1", undefined), "SKIFF1", "a split with no start should be said");
  assert.equal(splitSaid("GULL01", "SKIFF1"), "GULL01", "a split from another cell is said");
  assert.equal(splitSaid("SKIFF1", "SKIFF1"), undefined, "a split from its starter is said once");
  assert.equal(splitSaid(undefined, "SKIFF1"), undefined, "no split, nothing said");
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
    "stuck; owes an answer; a hold standing",
    "one question and one hold",
  );
  assert.equal(
    cellWords(cell("A", { owes: [{}, {}, {}], holds: [{}, {}] })),
    "quiet; owes 3 answers; 2 holds standing",
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
  // A machine with no basin reaches the reading as the node's own record,
  // whatever it holds.
  const record: Machine = JSON.parse('{"name": "kestrel", "state": "ready", "members": false}');
  assert.equal(offWords(record), "", "nor one whose members are no list");
  assert.deepEqual(
    cellsOn(now, "kestrel").map((s) => s.address),
    ["A"],
    "a machine's cells, none of another's",
  );
  assert.equal(
    cellsOn(now, "kestrel"),
    cellsOn(now, "kestrel"),
    "the same list all reading, so what it spent is summed once",
  );
  assert.notEqual(
    cellsOn(reading([], [...cells], now.machines), "kestrel"),
    cellsOn(now, "kestrel"),
    "and a new list for the next reading",
  );
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

/** A builder on x86_64-linux that answered, but for what `more` says. */
const builder = (name: string, more: Partial<Builder> = {}): Builder => ({
  name,
  state: "answering",
  systems: ["x86_64-linux"],
  jobs: 4,
  lent: false,
  said: null,
  ...more,
});

it("a builder says whether it's lent, how it stands and how much it takes at once", () => {
  assert.equal(builderWords(builder("heron")), "answering, 4 builds at once", "a standing one");
  assert.equal(
    builderWords(builder("tern", { state: "running", jobs: 1, lent: true })),
    "lent, running, 1 build at once",
    "a lent one, one build at a time",
  );
  assert.equal(
    builderWords(builder("gull", { state: "unknown", jobs: 0 })),
    "unheard",
    "one nobody heard, taking nothing known",
  );
  assert.deepEqual(
    ["answering", "running", "not answering", "not running", "unknown"].map((state) =>
      builds(builder("heron", { state })),
    ),
    [true, true, false, false, false],
    "only one reached or run by its lender takes builds",
  );
  assert.deepEqual(
    ["answering", "running", "not running", "unknown", "draining", "not answering"].map((state) =>
      builderTone(builder("heron", { state })),
    ),
    [0, 0, 0, 1, 1, 2],
    "a lent builder at rest asks nothing, one nobody heard some, one that didn't answer most",
  );
});

it("a builder's drawer says how it stands, who said so and why", () => {
  const says = (more: Partial<Builder>): string => builderSays(builder("heron", more));
  assert.equal(says({}), "It answered when last reached, so it takes builds.", "answering");
  assert.equal(
    says({ state: "running", said: "passing" }),
    "Its lender runs it, so it takes builds; the lender's word for it is passing.",
    "running, with its lender's word",
  );
  assert.equal(
    says({ state: "running" }),
    "Its lender runs it, so it takes builds.",
    "running, with none",
  );
  assert.equal(
    says({ state: "not answering", said: "refused" }),
    "It didn't answer when last reached: refused.",
    "not answering, and why",
  );
  assert.equal(
    says({ state: "not running" }),
    "Its lender doesn't run it now.",
    "not running, with no reason",
  );
  assert.equal(
    says({ state: "unknown", said: "it is reached through a jump host, which isn't dialled" }),
    "Nobody has heard how it stands: it is reached through a jump host, which isn't dialled.",
    "unheard, and why",
  );
  assert.equal(
    says({ state: "draining", said: "its lender gives it back" }),
    "The node says it is draining: its lender gives it back.",
    "a state the map doesn't know, in the node's own word",
  );
});

it("networks sharing a name are told apart by what each carries, then by their ids", () => {
  const told = toldApart([
    { key: "orchestration.skiff1", name: "Rig the Skiff", carries: "Sails" },
    { key: "orchestration.skiff2", name: "Rig the Skiff", carries: "Hull" },
    { key: "orchestration.tern01", name: "Chart the Harbour", carries: "Chart the Harbour" },
    { key: "orchestration.tern02", name: "Chart the Harbour", carries: "Chart the Harbour" },
    { key: "orchestration.heron1", name: "Mend the Nets", carries: "Before the Season" },
  ]);
  assert.deepEqual(
    [...told.values()],
    [
      "Rig the Skiff: Sails",
      "Rig the Skiff: Hull",
      "Chart the Harbour (TERN01)",
      "Chart the Harbour (TERN02)",
      "Mend the Nets",
    ],
    "a name alone keeps it; shared, what it carries, else its id, tells it apart",
  );
});

it("networks whose whole titles repeat are told apart by their ids", () => {
  const told = toldApart([
    { key: "orchestration.gull01", name: "Mend the Sails", carries: "Mainsail" },
    { key: "orchestration.gull02", name: "Mend the Sails", carries: "Mainsail" },
  ]);
  assert.deepEqual(
    [...told.values()],
    ["Mend the Sails: Mainsail (GULL01)", "Mend the Sails: Mainsail (GULL02)"],
    "a title shared whole, with what it carries, is told by its id",
  );
});

it("a title written as another network's told name is told apart by its whole key", () => {
  const whole = (key: string, name: string) => ({ key, name, carries: name });
  const told = toldApart([
    whole("orchestration.skiff1", "Rig the Skiff"),
    whole("orchestration.skiff2", "Rig the Skiff"),
    whole("orchestration.skiff3", "Rig the Skiff (SKIFF1)"),
  ]);
  assert.deepEqual(
    Object.fromEntries(told),
    {
      "orchestration.skiff1": "Rig the Skiff (SKIFF1) [orchestration.skiff1]",
      "orchestration.skiff2": "Rig the Skiff (SKIFF2)",
      "orchestration.skiff3": "Rig the Skiff (SKIFF1) [orchestration.skiff3]",
    },
    "no two read alike, whatever a title says",
  );
});

it("a builder is labelled by its name, its words and what it is", () => {
  const now: Reading = { ...reading([], [], []), builders: [builder("heron")] };
  assert.equal(
    described(now, "b:heron"),
    "heron, answering, 4 builds at once, a builder",
    "a builder",
  );
  assert.equal(described(now, "b:gone"), "gone", "a builder gone by its name");
});

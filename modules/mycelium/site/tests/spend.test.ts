// What the cells spent today, as the drawer's Spent Today fold says it. The
// words are written out whole, so an edit to any of them shows here.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  bySpend,
  cellSpendWords,
  cellsHeading,
  networkCells,
  partHeading,
  partsBySpend,
  partWords,
  restWords,
  shownAndRest,
  splits,
} from "../spend.ts";
import type { Cell, Network, Usage } from "../words.ts";

/** A quiet cell on `machine` that used what `usage` says today. */
const cell = (address: string, usage: Usage | null, machine = "kestrel"): Cell => ({
  address,
  short: address,
  standing: "quiet",
  owes: [],
  holds: [],
  machine,
  usage,
});

/** A cell on `machine` that spent `usd` today, its turns unsaid. */
const spending = (address: string, usd: number | null, machine = "kestrel"): Cell =>
  cell(address, { cost_today: usd }, machine);

/** A network of its own cells and branches. */
const network = (
  key: string,
  cells: readonly Cell[],
  children: readonly Network[] = [],
): Network => ({ key, name: key, within: null, children, cells });

const addresses = (cells: readonly Cell[]): readonly string[] => cells.map((s) => s.address);

it("orders the cells that spent anything, the most first", () => {
  const cells = [
    spending("A", 1.5),
    spending("B", 0),
    spending("C", null),
    spending("D", 7.25),
    spending("E", 3),
    cell("F", null),
  ];
  assert.deepEqual(
    addresses(bySpend(cells)),
    ["D", "E", "A"],
    "a cell that spent nothing, or said nothing, should be left out, and the rest most first",
  );
  assert.deepEqual(
    addresses(bySpend([spending("A", 2), spending("B", 5), spending("C", 2)])),
    ["B", "A", "C"],
    "cells that spent the same should keep the order they came in",
  );
});

it("orders the parts that spent anything, each with its cells", () => {
  const cells = [
    spending("A", 2, "kestrel"),
    spending("B", 5, "osprey"),
    spending("C", 0, "skiff"),
    spending("D", 2, "tern"),
  ];
  const on = (machine: string): readonly Cell[] => cells.filter((s) => s.machine === machine);
  assert.deepEqual(
    partsBySpend(["kestrel", "osprey", "skiff", "tern"], on).map(([m, its]) => [m, addresses(its)]),
    [
      ["osprey", ["B"]],
      ["kestrel", ["A"]],
      ["tern", ["D"]],
    ],
    "a part whose cells spent nothing should be left out, the rest most first, ties as they came",
  );
});

it("counts a network's branches with it, or only the cells on one machine", () => {
  const branch = network("tern", [spending("C", 1, "osprey")]);
  const hub = network("weir", [spending("A", 1), spending("B", 1, "osprey")], [branch]);
  assert.deepEqual(
    addresses(networkCells(hub)),
    ["A", "B", "C"],
    "a network should count its branches' cells with its own",
  );
  assert.deepEqual(
    addresses(networkCells(hub, "osprey")),
    ["B", "C"],
    "on a machine, a network should count only the cells running there",
  );
  assert.deepEqual(
    addresses(networkCells(hub, "")),
    ["A", "B", "C"],
    "a machine with no name should count every cell, as no machine does",
  );
});

it("says what a part of a sum spent, over how many cells", () => {
  assert.equal(
    partWords([spending("A", 1.5), spending("B", 0)]),
    "$1.50, 2 cells",
    "a part should say its sum to the cent and count every cell in it",
  );
  assert.equal(
    partWords([spending("A", 120.4)]),
    "$120, 1 cell",
    "a part past a hundred dollars should say whole dollars, and one cell as one",
  );
});

it("says what one cell spent, and its turns where its session says them", () => {
  const said = [1, 3, 0].map((turns) =>
    cellSpendWords(cell("A", { cost_today: 2, turns_today: turns })),
  );
  assert.deepEqual(
    said,
    ["$2.00, 1 turn", "$2.00, 3 turns", "$2.00, 0 turns"],
    "a cell's turns should be counted in their number, none as none",
  );
  assert.deepEqual(
    [
      cellSpendWords(cell("A", { cost_today: 2, turns_today: null })),
      cellSpendWords(cell("A", { cost_today: 2 })),
    ],
    ["$2.00", "$2.00"],
    "a cell whose session says no turns, or leaves them out, should say only its spend",
  );
});

it("splits a sum only into more than one part", () => {
  assert.deepEqual(
    [splits(0), splits(1), splits(2)],
    [false, false, true],
    "one part is the sum again, so only two or more should be shown",
  );
});

it("heads a list by its part while ten or fewer fit it", () => {
  assert.deepEqual(
    [partHeading("machine", 2), partHeading("network", 10), partHeading("branch", 2)],
    ["By Machine", "By Network", "By Branch"],
    "a list of ten or fewer should be headed by its part",
  );
  assert.equal(cellsHeading(10), "Its Cells", "ten or fewer cells should be its cells");
});

it("heads a list of more than ten by the ten that spent most", () => {
  assert.deepEqual(
    [partHeading("machine", 11), partHeading("network", 11), partHeading("branch", 11)],
    [
      "The 10 Machines That Spent Most",
      "The 10 Networks That Spent Most",
      "The 10 Branches That Spent Most",
    ],
    "the eleventh part should turn a list into the ten that spent most",
  );
  assert.equal(
    cellsHeading(11),
    "The 10 Cells That Spent Most",
    "the eleventh cell should turn its cells into the ten that spent most",
  );
});

it("shows ten of a list and counts the rest", () => {
  const rows = Array.from({ length: 11 }, (_, i) => i);
  assert.deepEqual(
    shownAndRest(rows),
    [rows.slice(0, 10), [10]],
    "a list of eleven should show its first ten and count the last",
  );
  assert.deepEqual(
    shownAndRest(rows.slice(0, 10)),
    [rows.slice(0, 10), []],
    "a list of ten should show them all and count none",
  );
});

it("says what the parts past a list's end spent between them", () => {
  const two = [[spending("A", 2)], [spending("B", 1.5), spending("C", 0)]];
  const said = [
    restWords([[spending("A", 2)]], "machine"),
    restWords(two, "machine"),
    restWords(two, "network"),
    restWords(two, "branch"),
    restWords([[spending("A", 0.25)], [spending("B", 0.5)]], "cell"),
  ];
  assert.deepEqual(
    said,
    [
      "1 more machine spent $2.00 between them.",
      "2 more machines spent $3.50 between them.",
      "2 more networks spent $3.50 between them.",
      "2 more branches spent $3.50 between them.",
      "2 more cells spent $0.75 between them.",
    ],
    "the rest should be counted in each part's own plural and summed",
  );
  assert.equal(restWords([], "machine"), "", "no rest should say nothing");
});

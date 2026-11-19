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
  splits,
} from "../spend.ts";
import type { Cell, Network } from "../words.ts";

/** A quiet cell on `machine` that spent `usd` today, or said nothing where it is null. */
const cell = (
  address: string,
  usd: number | null,
  machine = "kestrel",
  turns: number | null = null,
): Cell => ({
  address,
  short: address,
  standing: "quiet",
  owes: [],
  holds: [],
  machine,
  usage: { cost_today: usd, turns_today: turns },
});

/** A network of its own cells and branches. */
const network = (
  key: string,
  cells: readonly Cell[],
  children: readonly Network[] = [],
): Network => ({ key, name: key, within: null, children, cells });

it("orders the cells that spent anything, the most first", () => {
  const cells = [cell("A", 1.5), cell("B", 0), cell("C", null), cell("D", 7.25), cell("E", 3)];
  assert.deepEqual(
    bySpend(cells).map((s) => s.address),
    ["D", "E", "A"],
    "a cell that spent nothing, or said nothing, should be left out, and the rest most first",
  );
});

it("orders the parts that spent anything, each with its cells", () => {
  const cells = [cell("A", 2, "kestrel"), cell("B", 5, "osprey"), cell("C", 0, "skiff")];
  const on = (machine: string): readonly Cell[] => cells.filter((s) => s.machine === machine);
  assert.deepEqual(
    partsBySpend(["kestrel", "osprey", "skiff"], on).map(([m, its]) => [m, its.length]),
    [
      ["osprey", 1],
      ["kestrel", 1],
    ],
    "a part whose cells spent nothing should be left out, and the rest most first",
  );
});

it("counts a network's branches with it, or only the cells on one machine", () => {
  const branch = network("tern", [cell("C", 1, "osprey")]);
  const hub = network("weir", [cell("A", 1), cell("B", 1, "osprey")], [branch]);
  assert.deepEqual(
    [networkCells(hub).map((s) => s.address), networkCells(hub, "osprey").map((s) => s.address)],
    [
      ["A", "B", "C"],
      ["B", "C"],
    ],
    "a network should count its branches' cells, and on a machine only those running there",
  );
});

it("says what a part and a cell spent", () => {
  const said = [
    partWords([cell("A", 1.5), cell("B", 0)]),
    partWords([cell("A", 120.4)]),
    cellSpendWords(cell("A", 2, "kestrel", 1)),
    cellSpendWords(cell("A", 2, "kestrel", 3)),
    cellSpendWords(cell("A", 2, "kestrel", 0)),
    cellSpendWords(cell("A", 2)),
  ];
  assert.deepEqual(
    said,
    [
      "$1.50, 2 cells",
      "$120, 1 cell",
      "$2.00, 1 turn",
      "$2.00, 3 turns",
      "$2.00, 0 turns",
      "$2.00",
    ],
    "a sum should say its cells, and a cell its turns only where its session says them",
  );
});

it("splits a sum only into more than one part", () => {
  assert.deepEqual(
    [splits(0), splits(1), splits(2)],
    [false, false, true],
    "one part is the sum again, so only two or more should be shown",
  );
});

it("heads a list by its part, or by the ten that spent most", () => {
  const said = [
    partHeading("machine", 10),
    partHeading("machine", 11),
    partHeading("branch", 2),
    partHeading("branch", 11),
    partHeading("network", 11),
    cellsHeading(10),
    cellsHeading(11),
  ];
  assert.deepEqual(
    said,
    [
      "By Machine",
      "The 10 Machines That Spent Most",
      "By Branch",
      "The 10 Branches That Spent Most",
      "The 10 Networks That Spent Most",
      "Its Cells",
      "The 10 Cells That Spent Most",
    ],
    "ten fit a list whole, and the eleventh should turn it into the ten that spent most",
  );
});

it("says what the parts past a list's end spent between them", () => {
  const said = [
    restWords([], "machine"),
    restWords([[cell("A", 2)]], "machine"),
    restWords([[cell("A", 2)], [cell("B", 1.5), cell("C", 0)]], "branch"),
    restWords([[cell("A", 0.25)], [cell("B", 0.5)]], "cell"),
  ];
  assert.deepEqual(
    said,
    [
      "",
      "1 more machine spent $2.00 between them.",
      "2 more branches spent $3.50 between them.",
      "2 more cells spent $0.75 between them.",
    ],
    "the rest should be counted in their own plural and summed, and no rest say nothing",
  );
});

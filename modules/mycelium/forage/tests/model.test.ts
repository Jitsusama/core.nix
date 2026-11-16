// What the forage page makes of the mycelium node's forage records, told as
// the node tells them: one forage:<cell> record for each cell of this
// machine that works on a quest.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { foragesOf, headline, holds, keeps, lede, startedFrom, steps } from "../model.ts";

const plan = {
  id: "PLAN-20261116-HOVERS",
  kind: "plan",
  title: "Hover Over the Field",
  stage: "build",
  ticked: 2,
  open: 3,
};
const report = {
  id: "REPT-20261116-LANDED",
  kind: "report",
  title: "Where It Landed",
  stage: "concluded",
  ticked: 1,
  open: 0,
};

/** A forage record for `cell`, on kestrel, with `more` over its fields. */
function forage(cell: string, more: { readonly [field: string]: unknown } = {}) {
  return {
    key: `forage:${cell}`,
    kind: "forage",
    heard: 0,
    cell,
    session: `0000-${cell.toLowerCase()}`,
    machine: "kestrel",
    quest: `QEST-20261116-${cell}`,
    purpose: `${cell} learns to hover over one field.`,
    status: "active",
    documents: [report, plan],
    lineage: { started_by: null, parent: null },
    ...more,
  };
}

/** A watch told `records` whole. */
function told(records: readonly { readonly key: string }[]): Held {
  const held = new Held();
  held.took({ records });
  return held;
}

it("each cell comes after the one that started it, by address", () => {
  const { forages } = foragesOf(
    told([
      forage("TERNAA", { lineage: { started_by: "KESTRL" } }),
      forage("OSPREY"),
      forage("KESTRL"),
      forage("SKIFFS", { lineage: { started_by: "KESTRL" } }),
    ]),
  );
  assert.deepEqual(
    forages.map((each) => [each.cell, each.depth]),
    [
      ["KESTRL", 0],
      ["SKIFFS", 1],
      ["TERNAA", 1],
      ["OSPREY", 0],
    ],
  );
});

it("a cell started from another machine stands at the top", () => {
  const { forages } = foragesOf(told([forage("KESTRL", { lineage: { started_by: "OSPREY" } })]));
  assert.equal(forages[0]?.depth, 0);
  assert.equal(forages[0]?.startedBy, "OSPREY");
});

it("cells that started each other in a ring each still show once", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", { lineage: { started_by: "OSPREY" } }),
      forage("OSPREY", { lineage: { started_by: "KESTRL" } }),
    ]),
  );
  assert.deepEqual(forages.map((each) => each.cell).sort(), ["KESTRL", "OSPREY"]);
});

it("a cell's panel names the cells it started, by address, never itself", () => {
  const { forages } = foragesOf(
    told([
      forage("TERNAA", { lineage: { started_by: "KESTRL" } }),
      forage("KESTRL", { lineage: { started_by: "KESTRL" } }),
      forage("OSPREY", { lineage: { started_by: "SKIFFS" } }),
      forage("SKIFFS", { lineage: { started_by: "KESTRL" } }),
    ]),
  );
  assert.deepEqual(startedFrom(forages, "KESTRL"), ["SKIFFS", "TERNAA"]);
  assert.deepEqual(startedFrom(forages, "SKIFFS"), ["OSPREY"]);
  assert.deepEqual(startedFrom(forages, "TERNAA"), []);
});

it("documents come plans first, and their steps add up", () => {
  const { forages } = foragesOf(told([forage("KESTRL")]));
  const [kestrel] = forages;
  assert.ok(kestrel);
  assert.deepEqual(
    kestrel.documents.map((each) => each.kind),
    ["plan", "report"],
  );
  assert.deepEqual(steps(kestrel), { ticked: 3, all: 6 });
  assert.equal(keeps(kestrel), "One plan and one report");
});

it("a forage with no documents says so", () => {
  const { forages } = foragesOf(told([forage("KESTRL", { documents: [] })]));
  assert.equal(forages[0] && keeps(forages[0]), "Keeps nothing yet");
});

it("what can't be read is said, and the rest still drawn", () => {
  const { forages, unread } = foragesOf(
    told([
      { key: "forage:NONAME", kind: "forage" },
      forage("KESTRL", { documents: [plan, { title: "no id" }] }),
    ]),
  );
  assert.deepEqual(
    forages.map((each) => each.cell),
    ["KESTRL"],
  );
  assert.deepEqual(unread, ["forage:NONAME named no cell", "1 of KESTRL's documents had no id"]);
});

it("the headline says how many cells, and how many have no purpose written", () => {
  const { forages } = foragesOf(told([forage("KESTRL"), forage("OSPREY", { purpose: "" })]));
  assert.deepEqual(headline(forages), {
    cells: 2,
    without: 1,
    lead: "2 cells,",
    turn: "1 without purpose.",
  });
  assert.equal(headline(forages.slice(0, 1)).turn, "with its purpose.");
  assert.equal(headline(forages.slice(1)).turn, "without purpose.");
  assert.deepEqual(headline([]), {
    cells: 0,
    without: 0,
    lead: "No cells",
    turn: "at work on a quest.",
  });
});

it("the headline sets a count's thousands apart", () => {
  const cells = Array.from({ length: 3000 }, (_, at) =>
    forage(`C${String(at).padStart(5, "0")}`, at < 1200 ? { purpose: "" } : {}),
  );
  const { forages } = foragesOf(told(cells));
  const said = headline(forages);
  assert.equal(`${said.lead} ${said.turn}`, "3,000 cells, 1,200 without purpose.");
});

it("the lede names the cells with no purpose, then the plan furthest from done", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL"),
      forage("OSPREY", { purpose: "" }),
      forage("MERLIN", { documents: [{ ...plan, ticked: 1, open: 9 }] }),
    ]),
  );
  assert.equal(
    lede(forages),
    "OSPREY has no purpose written yet. MERLIN has furthest to go, 1 of 10 steps done.",
  );
  const many = foragesOf(
    told(
      ["A", "B", "C", "D", "E"].map((cell) =>
        forage(`CELL${cell}`, { purpose: "", documents: [] }),
      ),
    ),
  ).forages;
  assert.equal(
    lede(many),
    "CELLA, CELLB and 3 more have no purpose written yet. No plan has steps left.",
  );
});

it("a find matches a cell, its quest, its purpose or a document's title", () => {
  const { forages } = foragesOf(told([forage("KESTRL")]));
  const [kestrel] = forages;
  assert.ok(kestrel);
  for (const query of ["kestrl", "QEST-20261116", "hover over one", "where it landed", " "]) {
    assert.ok(holds(kestrel, query), query);
  }
  assert.ok(!holds(kestrel, "osprey"));
});

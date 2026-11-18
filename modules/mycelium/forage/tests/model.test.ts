// What the forage page makes of the mycelium node's forage records, told as
// the node tells them: one forage:<cell> record for each cell of this
// machine that works on a quest.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { joined } from "../joins.ts";
import { foragesOf, grewFrom, headline, holds, keeps, lede, startedFrom, steps } from "../model.ts";

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

it("the cells a cell started are those naming it their starter, by address, never itself", () => {
  const { forages } = foragesOf(
    told([
      forage("TERNAA", { lineage: { started_by: "KESTRL" } }),
      forage("KESTRL", { lineage: { started_by: "KESTRL" } }),
      forage("OSPREY", { lineage: { started_by: "SKIFFS" } }),
      forage("SKIFFS", { lineage: { started_by: "KESTRL" } }),
    ]),
  );
  // Given in any order, they still come by address.
  assert.deepEqual(startedFrom([...forages].reverse(), "KESTRL"), ["SKIFFS", "TERNAA"]);
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

/** A wait the launcher keeps for a quest, as the node passes it on. */
const sitting = {
  id: "field-sitting",
  title: "Watch the kestrel hover",
  why: "Only someone at the field can see it.",
  minutes: 30,
  when: "Wednesday or later",
  ready: false,
};

it("reads what a quest waits on its person for, keeping what it can of each", () => {
  const { forages, unread } = foragesOf(
    told([
      forage("KESTRL", {
        waits: [
          { ...sitting, raisedAt: "2026-11-10T22:14:48.007Z" },
          { id: "look", title: "Look at the field", minutes: "soon" },
          { id: "x" },
        ],
      }),
      forage("OSPREY"),
    ]),
  );
  const [kestrel, osprey] = forages;
  assert.deepEqual(kestrel?.waits, [
    sitting,
    {
      id: "look",
      title: "Look at the field",
      why: "",
      minutes: undefined,
      when: undefined,
      ready: false,
    },
  ]);
  assert.deepEqual(osprey?.waits, []);
  assert.deepEqual(unread, ["1 of KESTRL's waits had no title"]);
});

it("reads a wait as the launcher does: ready as it says, else when it's now", () => {
  const readyOf = (wait: object): boolean | undefined =>
    foragesOf(told([forage("KESTRL", { waits: [{ title: "Look", ...wait }] })])).forages[0]
      ?.waits[0]?.ready;
  assert.equal(readyOf({ ready: true }), true);
  assert.equal(readyOf({ ready: true, when: "Friday" }), true);
  assert.equal(readyOf({ ready: false, when: "now" }), false, "its owner's word first");
  assert.equal(readyOf({ when: " Now " }), true, "a file older than ready");
  assert.equal(readyOf({ when: "tomorrow" }), false);
  assert.equal(readyOf({ ready: "yes" }), false);
});

it("tells a wait by its id, else its title, and counts each once", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", {
        waits: [{ title: "Look" }, sitting, { title: "Look" }, { ...sitting, title: "Again" }],
      }),
    ]),
  );
  assert.deepEqual(
    forages[0]?.waits.map((wait) => [wait.id, wait.title]),
    [
      ["Look", "Look"],
      ["field-sitting", "Watch the kestrel hover"],
    ],
  );
});

it("the lede first says what waits on its person, and from which cells", () => {
  const look = { id: "look", title: "Look at the field" };
  const { forages } = foragesOf(
    told([
      forage("KESTRL", { waits: [sitting, look] }),
      // A second cell on kestrel's quest waits on the same two things.
      forage("KESTR2", { quest: "QEST-20261116-KESTRL", waits: [sitting, look] }),
      forage("OSPREY", { waits: [look] }),
      forage("SKIFFS"),
    ]),
  );
  assert.equal(
    lede(forages),
    "3 things wait on you, from KESTR2, KESTRL and OSPREY. Every cell has its purpose written. KESTR2 has furthest to go, 3 of 6 steps done.",
  );
  const [one] = foragesOf(told([forage("OSPREY", { waits: [look] })])).forages;
  assert.ok(one);
  assert.ok(lede([one]).startsWith("One thing waits on you, from OSPREY. "), lede([one]));
});

it("a find matches what a cell's quest waits on", () => {
  const [kestrel] = foragesOf(told([forage("KESTRL", { waits: [sitting] })])).forages;
  assert.ok(kestrel);
  assert.ok(holds(kestrel, "watch the kestrel"));
});

it("a cell split from another grows under it when nothing here started it", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", { lineage: { split_into: ["TERNAA", "SKIFFS"] } }),
      forage("TERNAA", { lineage: { split_from: "KESTRL" } }),
      // One its starter and its ancestor both name grows under its starter.
      forage("SKIFFS", { lineage: { started_by: "OSPREY", split_from: "KESTRL" } }),
      forage("OSPREY"),
    ]),
  );
  assert.deepEqual(
    forages.map((each) => [each.cell, each.depth]),
    [
      ["KESTRL", 0],
      ["TERNAA", 1],
      ["OSPREY", 0],
      ["SKIFFS", 1],
    ],
  );
  assert.deepEqual(forages[0]?.splitInto, ["SKIFFS", "TERNAA"]);
  assert.equal(forages[1]?.splitFrom, "KESTRL");
});

it("an ancestor's split takes in each cell here naming it, so both ends agree", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", { lineage: { split_into: ["OSPREY"] } }),
      forage("TERNAA", { lineage: { split_from: "KESTRL" } }),
    ]),
  );
  assert.deepEqual(forages[0]?.splitInto, ["OSPREY", "TERNAA"]);
});

it("a cell its ancestor lists splits from it, though it names none itself", () => {
  const { forages, unread } = foragesOf(
    told([
      forage("KESTRL", { lineage: { split_into: ["TERNAA"] } }),
      // The node sends null for a lineage field a cell lacks, which is no fault.
      forage("TERNAA", { lineage: { split_from: null, split_into: null } }),
    ]),
  );
  assert.deepEqual(
    forages.map((each) => [each.cell, each.depth, each.splitFrom]),
    [
      ["KESTRL", 0, undefined],
      ["TERNAA", 1, "KESTRL"],
    ],
  );
  assert.deepEqual(unread, []);
});

it("a split cell's own word stands where its ancestor's list disagrees", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", { lineage: { split_into: ["TERNAA", "SKIFFS"] } }),
      forage("OSPREY"),
      forage("TERNAA", { lineage: { split_from: "OSPREY" } }),
      // One naming a cell not here still drops out of the list naming it.
      forage("SKIFFS", { lineage: { split_from: "FARAWAY" } }),
    ]),
  );
  const cell = (address: string) => forages.find((each) => each.cell === address);
  assert.deepEqual(cell("KESTRL")?.splitInto, []);
  assert.deepEqual(cell("OSPREY")?.splitInto, ["TERNAA"]);
  assert.equal(cell("TERNAA")?.splitFrom, "OSPREY");
  assert.equal(cell("SKIFFS")?.splitFrom, "FARAWAY");
});

it("a cell two ancestors list, naming neither, splits from the first alone", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", { lineage: { split_into: ["TERNAA"] } }),
      forage("OSPREY", { lineage: { split_into: ["TERNAA"] } }),
      forage("TERNAA"),
    ]),
  );
  const cell = (address: string) => forages.find((each) => each.cell === address);
  assert.equal(cell("TERNAA")?.splitFrom, "KESTRL");
  assert.deepEqual(cell("KESTRL")?.splitInto, ["TERNAA"]);
  assert.deepEqual(cell("OSPREY")?.splitInto, []);
});

it("reads what it can of a split, and says what it couldn't", () => {
  const { forages, unread } = foragesOf(
    told([
      forage("KESTRL", { lineage: { split_into: ["TERNAA", 7, "", "KESTRL"] } }),
      forage("OSPREY", { lineage: { split_from: 7, split_into: "TERNAA" } }),
      forage("TERNAA", { lineage: { split_from: "TERNAA" } }),
    ]),
  );
  const cell = (address: string) => forages.find((each) => each.cell === address);
  assert.deepEqual(cell("KESTRL")?.splitInto, ["TERNAA"]);
  assert.equal(cell("OSPREY")?.splitFrom, undefined);
  assert.deepEqual(cell("OSPREY")?.splitInto, []);
  // A cell naming itself is read from its ancestor's side instead.
  assert.equal(cell("TERNAA")?.splitFrom, "KESTRL");
  assert.deepEqual(unread, [
    "3 of KESTRL's split_into named no other cell",
    "OSPREY's split_from named no other cell",
    "OSPREY's split_into listed no cells",
    "TERNAA's split_from named no other cell",
  ]);
});

it("the cell a cell grew from is its starter, else its ancestor, only when it's here", () => {
  const here = new Set(["KESTRL", "OSPREY"]);
  const of = (startedBy?: string, splitFrom?: string) =>
    grewFrom({ cell: "TERNAA", startedBy, splitFrom }, here);
  assert.equal(of("OSPREY", "KESTRL"), "OSPREY");
  assert.equal(of("FARAWAY", "KESTRL"), "KESTRL");
  assert.equal(of(undefined, "FARAWAY"), undefined);
  // A cell naming itself its starter still grows from the cell it split from.
  assert.equal(
    grewFrom({ cell: "OSPREY", startedBy: "OSPREY", splitFrom: "KESTRL" }, here),
    "KESTRL",
  );
  assert.equal(
    grewFrom({ cell: "KESTRL", startedBy: "KESTRL", splitFrom: undefined }, here),
    undefined,
  );
});

it("reads the needs a cell serves and relies on, passing over one naming no other cell", () => {
  const { forages, unread } = foragesOf(
    told([
      forage("KESTRL", {
        serves: [
          { cell: "OSPREY", need: "a hover review", since: "2026-11-18T13:05:00Z" },
          { need: "lost" },
          { cell: "", need: "nobody" },
          { cell: "KESTRL", need: "itself" },
        ],
        relies: [{ cell: "TERNAA" }],
      }),
      forage("OSPREY", { serves: { cell: "KESTRL" }, relies: null }),
    ]),
  );
  const [kestrel] = forages;
  assert.deepEqual(kestrel?.serves, [
    { cell: "OSPREY", need: "a hover review", since: "2026-11-18T13:05:00Z" },
  ]);
  assert.deepEqual(kestrel?.relies, [{ cell: "TERNAA", need: "", since: undefined }]);
  assert.deepEqual(forages[1]?.serves, []);
  assert.deepEqual(unread, [
    "3 of KESTRL's serves named no other cell",
    "OSPREY's serves listed no needs",
  ]);
});

it("a forage the node sends no joins for has none, and splits from nothing", () => {
  const [kestrel] = foragesOf(told([forage("KESTRL")])).forages;
  assert.equal(kestrel?.splitFrom, undefined);
  assert.deepEqual(kestrel?.splitInto, []);
  assert.deepEqual(kestrel?.serves, []);
  assert.deepEqual(kestrel?.relies, []);
});

it("joins each pair a need ties once, however both ends keep it, and only cells here", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", { serves: [{ cell: "OSPREY", need: "a review" }, { cell: "FARAWAY" }] }),
      forage("OSPREY", { relies: [{ cell: "KESTRL", need: "a review" }, { cell: "OSPREY" }] }),
      forage("TERNAA", { relies: [{ cell: "KESTRL", need: "a plan" }] }),
    ]),
  );
  assert.deepEqual(joined(forages), [
    { from: "KESTRL", to: "OSPREY" },
    { from: "TERNAA", to: "KESTRL" },
  ]);
});

it("joins a pair once even when its two ends word the need differently, or keep two", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", {
        serves: [
          { cell: "OSPREY", need: "a review" },
          { cell: "OSPREY", need: "a second look" },
        ],
      }),
      forage("OSPREY", {
        relies: [{ cell: "KESTRL", need: "review my hover" }],
        // A need the other way round is still the same pair on the floor.
        serves: [{ cell: "KESTRL" }],
      }),
    ]),
  );
  assert.deepEqual(joined(forages), [{ from: "KESTRL", to: "OSPREY" }]);
});

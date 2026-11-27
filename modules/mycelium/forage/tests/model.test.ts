// What the forage page makes of the mycelium node's forage records, told as
// the node tells them: one forage:<cell> record for each cell of this
// machine that works on a quest.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  foragesOf,
  grewClaimed,
  grewFrom,
  headline,
  holds,
  keeps,
  lede,
  standingLine,
  startedFrom,
  steps,
} from "../model.ts";
import { forage, plan, told } from "./records.ts";

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

it("a start is booked only when the node says its book holds it", () => {
  const read = (from?: unknown) =>
    foragesOf(
      told([forage("KESTRL", { lineage: { started_by: "OSPREY", started_by_from: from } })]),
    );
  const booked = (from?: unknown) => read(from).forages[0]?.booked;
  assert.equal(booked("book"), true);
  assert.equal(booked("presence"), false);
  // A node from before the field, or one sending something else, is taken at the cell's word.
  assert.equal(booked(), false);
  assert.equal(booked(null), false);
  assert.equal(booked(true), false);
  // A source neither word is said, so a misspelt node shows, and so is none sent at all.
  assert.deepEqual(read("presence").unread, []);
  const odd = ["KESTRL's started_by_from said neither book nor presence"];
  assert.deepEqual(read("Book").unread, odd);
  assert.deepEqual(read(true).unread, odd);
  const none = ["where any start came from, as the node sends none yet, so each reads as claimed"];
  assert.deepEqual(read(null).unread, none);
  assert.deepEqual(read().unread, none);
});

it("says a node-wide fault in where starts came from once", () => {
  const lineage = (from?: unknown) => ({
    lineage: { started_by: "OSPREY", started_by_from: from },
  });
  const unread = (...records: ReturnType<typeof forage>[]) => foragesOf(told(records)).unread;
  assert.deepEqual(unread(forage("KESTRL", lineage("Book")), forage("TERNAA", lineage("Book"))), [
    "2 cells' started_by_from said neither book nor presence (KESTRL and TERNAA)",
  ]);
  // Once the node sends sources, a starter with none is said by its cell.
  assert.deepEqual(unread(forage("KESTRL", lineage("book")), forage("TERNAA", lineage())), [
    "where TERNAA's start came from",
  ]);
  assert.deepEqual(
    unread(
      forage("KESTRL", lineage("book")),
      forage("TERNAA", lineage()),
      forage("SKIFFS", lineage()),
    ),
    ["where 2 cells' starts came from (SKIFFS and TERNAA)"],
  );
  // By address, seven named whole, and past seven six named and the rest counted.
  const unsourced = (letters: string) =>
    unread(
      forage("KESTRL", lineage("book")),
      ...[...letters].map((letter) => forage(letter.repeat(6), lineage())),
    );
  const six = "AAAAAA, BBBBBB, CCCCCC, DDDDDD, EEEEEE, FFFFFF";
  assert.deepEqual(unsourced("GFEDCBA"), [`where 7 cells' starts came from (${six} and GGGGGG)`]);
  assert.deepEqual(unsourced("IHGFEDCBA"), [`where 9 cells' starts came from (${six} and 3 more)`]);
  // A misspelt source is still a source sent, so a starter with none beside it is said by cell.
  assert.deepEqual(unread(forage("KESTRL", lineage("Book")), forage("TERNAA", lineage())), [
    "KESTRL's started_by_from said neither book nor presence",
    "where TERNAA's start came from",
  ]);
  // A node sending no source at all is said once, however many cells it starts.
  assert.deepEqual(unread(forage("KESTRL", lineage()), forage("TERNAA", lineage())), [
    "where any start came from, as the node sends none yet, so each reads as claimed",
  ]);
  // One that names no starter needs no source.
  assert.deepEqual(unread(forage("KESTRL", lineage("book")), forage("TERNAA")), []);
  assert.deepEqual(unread(forage("KESTRL")), []);
});

it("how a cell grew is only claimed unless it grew from a start the book holds", () => {
  const here = new Set(["KESTRL", "OSPREY"]);
  const claimed = (startedBy: string | undefined, booked: boolean) =>
    grewClaimed({ cell: "TERNAA", startedBy, splitFrom: "KESTRL", booked }, here);
  assert.equal(claimed("OSPREY", true), false);
  assert.equal(claimed("OSPREY", false), true);
  // Its booked starter isn't here, so it grows from the cell it split from, only claimed.
  assert.equal(claimed("FARAWAY", true), true);
  // With no starter it grows from the cell it split from, booked or not.
  assert.equal(claimed(undefined, true), true);
  // No caller asks this of a cell that grew from none here, but should one, it fails closed.
  assert.equal(
    grewClaimed({ cell: "TERNAA", startedBy: undefined, splitFrom: undefined, booked: true }, here),
    true,
  );
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
  const started = (from: typeof forages, cell: string) =>
    startedFrom(from, cell).map((each) => each.cell);
  assert.deepEqual(started([...forages].reverse(), "KESTRL"), ["SKIFFS", "TERNAA"]);
  assert.deepEqual(started(forages, "SKIFFS"), ["OSPREY"]);
  assert.deepEqual(started(forages, "TERNAA"), []);
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

it("the headline says how many cells are on quests, and how many have no purpose", () => {
  const { forages } = foragesOf(told([forage("KESTRL"), forage("OSPREY", { purpose: "" })]));
  assert.deepEqual(headline(forages), {
    cells: 2,
    without: 1,
    lead: "2 cells on quests,",
    turn: "1 without purpose.",
  });
  assert.equal(headline(forages.slice(0, 1)).lead, "One cell on a quest,");
  assert.equal(headline(forages.slice(0, 1)).turn, "with its purpose.");
  assert.equal(headline(forages.slice(1)).lead, "One cell on a quest,");
  assert.equal(headline(forages.slice(1)).turn, "without purpose.");
  const hollow = foragesOf(
    told([forage("KESTRL", { purpose: "" }), forage("OSPREY", { purpose: "" })]),
  ).forages;
  assert.deepEqual(headline(hollow), {
    cells: 2,
    without: 2,
    lead: "2 cells on quests,",
    turn: "all without purpose.",
  });
  assert.deepEqual(headline([]), {
    cells: 0,
    without: 0,
    lead: "No cells",
    turn: "on a quest.",
  });
});

/** A broken standing as the line says it, and what it couldn't read after it. */
const BROKEN_SAID = /^Not live since \d\d:\d\d: the node is down\. This is how it stood then\./u;
const UNREAD_SAID = / Couldn't read: forage:NONAME named no cell; a wait had no id\.$/u;

it("the standing line says how the page stands, then what it couldn't read", () => {
  assert.equal(standingLine(undefined, []), "Reaching the mycelium node.");
  const broken = { state: "broken", since: 0, reason: "the node is down" } as const;
  const line = standingLine(broken, ["forage:NONAME named no cell", "a wait had no id"]);
  assert.match(line, BROKEN_SAID);
  assert.match(line, UNREAD_SAID);
});

it("the headline sets a count's thousands apart", () => {
  const cells = Array.from({ length: 3000 }, (_, at) =>
    forage(`C${String(at).padStart(5, "0")}`, at < 1200 ? { purpose: "" } : {}),
  );
  const { forages } = foragesOf(told(cells));
  const said = headline(forages);
  assert.equal(`${said.lead} ${said.turn}`, "3,000 cells on quests, 1,200 without purpose.");
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
      // A cell not standing here grows under neither, so both keep it.
      forage("OSPREY", { lineage: { split_into: ["TERNAA", "FARAWAY"] } }),
      forage("SKIFFS", { lineage: { split_into: ["FARAWAY"] } }),
      forage("TERNAA"),
    ]),
  );
  const cell = (address: string) => forages.find((each) => each.cell === address);
  assert.equal(cell("TERNAA")?.splitFrom, "KESTRL");
  assert.deepEqual(cell("KESTRL")?.splitInto, ["TERNAA"]);
  assert.deepEqual(cell("OSPREY")?.splitInto, ["FARAWAY"]);
  assert.deepEqual(cell("SKIFFS")?.splitInto, ["FARAWAY"]);
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

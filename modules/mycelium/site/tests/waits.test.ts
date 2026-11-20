// What waits on a cell and what holds it has standing, as the drawer says
// them. The words are written out whole, so an edit to any of them shows
// here. Times are in the reader's own zone, with no offset.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  ageOf,
  askerOf,
  heldFirst,
  heldWords,
  holdingFirst,
  holdTarget,
  holdWords,
  needsFirst,
  nextNeeding,
  owedWords,
  type Seen,
  sharedWords,
  unansweredWords,
} from "../waits.ts";
import type { Cell, Network } from "../words.ts";

const AT = "2026-11-19T10:00:00";

/** A cell on kestrel, quiet with nothing waiting, but for what `more` says. */
const cell = (address: string, more: Partial<Cell> = {}): Cell => ({
  address,
  short: address.toLowerCase(),
  standing: "quiet",
  owes: [],
  holds: [],
  machine: "kestrel",
  ...more,
});

const weir: Network = {
  key: "orchestration.weir",
  name: "Weir",
  within: null,
  children: [],
  cells: [],
};

/** A reading holding two cells, one drawn network and one shared repository. */
const seen: Seen = {
  byAddress: new Map([
    ["TERN01", cell("TERN01")],
    ["GULL02", cell("GULL02", { short: "" })],
  ]),
  hubs: new Map([[weir.key, weir]]),
  shared: [
    { key: "repository.backwater", short: "backwater", shape: "repository" },
    { key: "project.locks", short: "locks", shape: "project" },
    { key: "group.nets", short: "nets", shape: "group" },
    { key: "other.bay", short: "bay", shape: "harbour" },
    { key: "other.eddy", short: "eddy", shape: "constructor" },
  ],
};

it("names a shared network by its shape, or its key where none is held", () => {
  assert.deepEqual(
    [
      sharedWords("repository.backwater", seen),
      sharedWords("project.locks", seen),
      sharedWords("group.nets", seen),
      sharedWords("other.bay", seen),
      sharedWords("other.eddy", seen),
      sharedWords("repository.tide", seen),
      sharedWords("repository.backwater", null),
    ],
    [
      "the backwater repository",
      "the locks quest",
      "the nets group",
      "the bay harbour",
      "the eddy constructor",
      "repository.tide",
      "repository.backwater",
    ],
    "a quest should be a project's word, a shape the model never passes its own, " +
      "an inherited name too, and an unheld key itself",
  );
});

it("says where a hold went as the page names each thing", () => {
  const said = [
    "@tern01",
    "@GULL02",
    "@heron03",
    "#machine",
    "#orchestration/weir",
    "#repository/backwater",
    "#repository/tide",
  ].map((to) => holdTarget(to, seen));
  assert.deepEqual(
    said,
    [
      "tern01",
      "GULL02",
      "HERON03",
      "every cell on its machine",
      "Weir",
      "the backwater repository",
      "repository.tide",
    ],
    "a peer should go by its short name or its address, a network by its name, else its key",
  );
  assert.equal(holdTarget("@tern01", null), "TERN01", "with no reading a peer is its address");
});

it("says a hold's reach, since when, and until when at most", () => {
  const said = [
    holdWords({ to: ["@tern01", "#machine"], at: "2026-11-19T09:30:00" }, seen, AT),
    holdWords({ to: [], at: "2026-11-19T09:30:00", until: "2026-11-19T10:30:00" }, seen, AT),
    holdWords({ to: ["#orchestration/weir"], at: "2026-11-19T09:30:00", until: "" }, seen, AT),
  ];
  assert.deepEqual(
    said,
    [
      "holding tern01 and every cell on its machine since 09:30",
      "holding its peers since 09:30, until 10:30 at most",
      "holding Weir since 09:30",
    ],
    "a hold to no one should hold its peers, and an empty until say no end",
  );
});

it("says whether a question went unanswered or unheard by how the page knows", () => {
  assert.deepEqual(
    [unansweredWords(true), unansweredWords(false)],
    ["with no answer yet", "with no word since"],
    "the node's judgement should say no answer, and the page's guess no word",
  );
});

it("names who asked, and how long the oldest question waited", () => {
  const owes = [
    { from: "TERN01", at: "2026-11-19T09:45:00" },
    { from: "GULL02", at: "2026-11-19T09:50:00" },
  ];
  assert.deepEqual(
    [
      askerOf({ from: "TERN01", at: AT }, seen),
      askerOf({ from: "GULL02", at: AT }, seen),
      askerOf({ from: "HERON03", at: AT }, seen),
      ageOf(owes, AT),
      ageOf([{ from: "TERN01", at: "2026-11-19T10:05:00" }], AT),
      ageOf([], AT),
    ],
    ["tern01", "GULL02", "HERON03", 15 * 60_000, 0, 0],
    "an asker should go by its short name or address, and an age be the first's, never below none",
  );
});

it("says one question waiting, and what holds a cell up", () => {
  const one = [{ from: "TERN01", at: "2026-11-19T09:59:30" }];
  const two = [
    { from: "GULL02", at: "2026-11-19T09:45:00" },
    { from: "TERN01", at: "2026-11-19T09:50:00" },
  ];
  const said = [
    owedWords({ from: "TERN01", at: "2026-11-18T21:15:00" }, seen, AT),
    heldWords(cell("A", { owes: one }), seen, AT, { judged: true }),
    heldWords(cell("A", { owes: two }), seen, AT, { judged: false }),
  ];
  assert.deepEqual(
    said,
    [
      "tern01's question waiting since yesterday at 21:15",
      "a question waiting; it came from tern01 under a minute ago, with no answer yet",
      "2 questions waiting; the oldest came from GULL02 15 min ago, with no word since",
    ],
    "the count, the oldest's asker and age, and how the page knows should each be said",
  );
  assert.equal(
    heldWords(cell("A", { owes: one }), seen, AT, { judged: true, named: (n) => n.slice(0, 3) }),
    "a question waiting; it came from ter under a minute ago, with no answer yet",
    "the asker's name should be fitted to the row as the drawer asks",
  );
});

it("lists the cells held up and holding, the oldest first", () => {
  const cells = [
    // Each cell's own are oldest first, as the model leaves them, so a cell
    // goes by its first, not its latest.
    cell("A", { owes: [{ from: "B", at: "2026-11-19T09:45:00" }] }),
    cell("B"),
    cell("C", {
      owes: [
        { from: "B", at: "2026-11-19T09:40:00" },
        { from: "D", at: "2026-11-19T09:58:00" },
      ],
    }),
    cell("D", { holds: [{ to: [], at: "2026-11-19T09:50:00" }] }),
    cell("E", {
      holds: [
        { to: [], at: "2026-11-19T09:35:00" },
        { to: [], at: "2026-11-19T09:59:00" },
      ],
    }),
  ];
  assert.deepEqual(
    [heldFirst(cells).map((s) => s.address), holdingFirst(cells).map((s) => s.address)],
    [
      ["C", "A"],
      ["E", "D"],
    ],
    "only cells with a question or a hold should be listed, the oldest first",
  );
});

/** Five cells: two stuck and two asking, interleaved, and one quiet. */
const standing = [
  cell("S1", { standing: "stuck" }),
  cell("A1", { standing: "asks", wants: "a yes" }),
  cell("Q"),
  cell("S2", { standing: "stuck" }),
  cell("A2", { standing: "asks" }),
];

it("lists every cell asking the reader, then every one stuck, each in its order", () => {
  assert.deepEqual(
    needsFirst(standing).map((s) => s.address),
    ["A1", "A2", "S1", "S2"],
    "a cell asking should come before one stuck, and a quiet one not at all",
  );
  assert.deepEqual(needsFirst([cell("Q")]), [], "nothing should be listed when none needs him");
});

it("steps to the next cell that needs the reader, round to the first", () => {
  assert.deepEqual(
    [null, "A1", "A2", "S1", "S2", "Q", "gone"].map((from) => nextNeeding(standing, from)),
    ["A1", "A2", "S1", "S2", "A1", "A1", "A1"],
    "each step should go to the next in Now's order, and from none or a quiet cell to the first",
  );
  assert.equal(nextNeeding([cell("Q")], null), null, "no step should be taken when none needs him");
});

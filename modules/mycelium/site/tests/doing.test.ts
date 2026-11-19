// What things on the map are doing, as the drawer says it. The words are
// written out whole, so an edit to any of them shows here. Times are in the
// reader's own zone, with no offset.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  asksOrFinds,
  cellDoing,
  fedWords,
  groupDoing,
  lastTurnOf,
  sensesDoing,
  talkWords,
} from "../doing.ts";
import type { Cell } from "../words.ts";

const AT = "2026-11-19T10:00:00";

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

it("says what a receptor fed a cell by its kind", () => {
  assert.deepEqual(
    [fedWords("review"), fedWords("summary"), fedWords("weather"), fedWords("toString")],
    [
      "a review asked of you",
      "a call's summary",
      "something from outside",
      "something from outside",
    ],
    "a kind the page doesn't know, or an object's own key, should be something from outside",
  );
});

it("finds the latest turn a set of cells ended", () => {
  const cells = [
    cell("A", { usage: { last_turn: "2026-11-19T09:10:00" } }),
    cell("B", { usage: { last_turn: "2026-11-19T09:40:00" } }),
    cell("C"),
    cell("D", { usage: { last_turn: "" } }),
  ];
  assert.equal(lastTurnOf(cells), "2026-11-19T09:40:00", "the latest turn should be B's");
  assert.equal(lastTurnOf([cell("A")]), undefined, "no turn said should be none");
});

it("says a set of cells by its moving ones, two named and the rest counted", () => {
  const said = groupDoing(
    [
      cell("A", { standing: "moving", doing: "Reading the plan. " }),
      cell("B", { standing: "moving" }),
      cell("C", { standing: "moving", doing: "Building" }),
      cell("D", { standing: "moving", doing: "Testing" }),
      cell("E", { standing: "asks" }),
    ],
    AT,
  );
  assert.equal(
    said,
    "4 cells moving. A: Reading the plan; B: in a turn; and 2 more.",
    "each doing should lose its full stop, and a silent one be in a turn",
  );
  assert.equal(
    groupDoing([cell("A", { standing: "moving", doing: "Building" })], AT),
    "1 cell moving. A: Building.",
    "one moving cell should be named with nothing more",
  );
});

it("says a set with nothing moving by what waits, or its last turn", () => {
  const said = [
    groupDoing(
      [cell("A", { standing: "asks" }), cell("B", { usage: { last_turn: "2026-11-19T09:55:00" } })],
      AT,
    ),
    groupDoing([cell("A", { usage: { last_turn: "2026-11-19T09:55:00" } })], AT),
    groupDoing([cell("A")], AT),
    groupDoing([], AT),
  ];
  assert.deepEqual(
    said,
    [
      "Nothing moving; 1 cell waiting on you.",
      "Nothing moving; the last turn ended 5 min ago.",
      "Nothing moving.",
      "Nothing moving.",
    ],
    "what waits on the person should come before the last turn",
  );
});

it("says each receptor in full, and how many are reading", () => {
  const said = [
    sensesDoing([cell("GH", { standing: "moving", doing: "Reading a review." }), cell("CALLS")]),
    sensesDoing([cell("GH")]),
  ];
  assert.deepEqual(
    said,
    [
      "1 receptor reading. GH: Reading a review; CALLS: listening.",
      "Nothing reading. GH: listening.",
    ],
    "a receptor saying nothing should be listening",
  );
});

it("says what one cell is doing by how it stands", () => {
  const said = [
    cellDoing(cell("A", { standing: "moving", doing: "Building." }), AT),
    cellDoing(cell("A", { standing: "moving" }), AT),
    cellDoing(cell("A", { standing: "asks", wants: "a yes" }), AT),
    cellDoing(cell("A", { standing: "asks" }), AT),
    cellDoing(cell("A", { standing: "stuck" }), AT),
    cellDoing(cell("A", { standing: "dormant" }), AT),
    cellDoing(cell("A", { usage: { last_turn: "2026-11-19T08:30:00" } }), AT),
    cellDoing(cell("A"), AT),
  ];
  assert.deepEqual(
    said,
    [
      "Building.",
      "In a turn now",
      "Asks you: a yes",
      "Waiting on you",
      "Stuck: paused, waiting to be resumed",
      "Parked, its place kept",
      "Quiet, waiting for its next word; its last turn ended at 08:30",
      "Quiet, waiting for its next word",
    ],
    "a moving cell should keep its doing as it says it",
  );
});

it("counts a question, finding, hold or pivot as asking or finding something", () => {
  const intents = ["question", "finding", "hold", "pivot", "fyi", "constructor"];
  assert.deepEqual(
    [...intents.map((intent) => asksOrFinds({ intent })), asksOrFinds({})],
    [true, true, true, true, false, false, false],
    "an fyi, an object's own key or no intent should be for context only",
  );
});

it("says how much of the talk asked or found something", () => {
  const talk = [
    { intent: "question" },
    { intent: "finding" },
    { intent: "hold" },
    { intent: "pivot" },
    { intent: "fyi" },
    {},
  ];
  const said = [
    talkWords(talk, 15, false),
    talkWords(talk.slice(4, 5), 15, true),
    talkWords(talk.slice(0, 1), 15, true),
    talkWords([], 15, false),
  ];
  assert.deepEqual(
    said,
    [
      "Of the latest 6 messages: 4 asked or found something, and 2 were for context only.",
      "Of the latest 15 messages, 1 touches it: " +
        "0 asked or found something, and 1 was for context only.",
      "Of the latest 15 messages, 1 touches it: " +
        "1 asked or found something, and 0 were for context only.",
      "",
    ],
    "no talk should say nothing",
  );
});

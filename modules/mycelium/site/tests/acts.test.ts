// What a cell's drawer offers, first to last, and what it asks, said whole:
// Go to It leads, since a cell that needs Joel is one he must reach, and a
// question is never cut to its last line.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { ACT_WORDS, askedOf, cellActs, goLink } from "../acts.ts";

const session = { address: "TERN02", host: "kestrel", receptor: false, foraged: true };

it("leads a session's drawer with Go to It, to its window on its machine", () => {
  const acts = cellActs(session);
  assert.deepEqual(
    acts.map((each) => each.act),
    ["go", "talk", "sprout", "forage"],
    "Go to It should come first, then talking, new work and what it is for",
  );
  assert.deepEqual(
    acts[0],
    { act: "go", words: "Go to It", href: "mycelium:cell/TERN02@kestrel/go" },
    "Go to It should go to the cell's window, naming the machine it runs on",
  );
  assert.equal(
    acts[1]?.act === "talk" ? acts[1].href : null,
    "mycelium:cell/TERN02@kestrel",
    "talking should open the cell's talk, not go to its window",
  );
});

it("goes to a cell whose machine has no name a link may carry by its address alone", () => {
  assert.equal(
    goLink("TERN02", null),
    "mycelium:cell/TERN02/go",
    "no machine should mean the address alone",
  );
  assert.equal(
    goLink("TERN02", "not a machine"),
    "mycelium:cell/TERN02/go",
    "a misshaped machine should be left out",
  );
});

it("offers no way to a cell whose address no link may carry", () => {
  assert.equal(
    goLink("tern02", "kestrel"),
    null,
    "a lower-case address isn't one the launcher reaches",
  );
  assert.deepEqual(
    cellActs({ ...session, address: "not-an-address" }).map((each) => each.act),
    ["forage"],
    "only what it is for should be offered",
  );
});

it("offers a receptor only what it is for, since there is no window to go to", () => {
  assert.deepEqual(
    // An address a session's link would carry, so only `receptor` withholds
    // Go to It and talking.
    cellActs({ ...session, receptor: true }).map((each) => each.act),
    ["forage"],
    "a receptor should offer neither Go to It nor talking",
  );
  assert.deepEqual(
    cellActs({ ...session, foraged: false }).map((each) => each.words),
    [ACT_WORDS.go, ACT_WORDS.talk, ACT_WORDS.sprout],
    "a cell with no forage should offer no link to one",
  );
});

it("says what a cell asks whole, its paragraphs, lines and indentation kept", () => {
  const asked =
    "\n  The signing key is ready to deploy, and the house reports sign with it.\r\n\r\n" +
    "Three things are left:\r1. The Mac's copy job.\n  2. The signing pause: yes, no or partly?  \n\n\n";
  assert.deepEqual(
    askedOf({ wants: "2. The signing pause: yes, no or partly?", asked }),
    [
      ["  The signing key is ready to deploy, and the house reports sign with it."],
      [
        "Three things are left:",
        "1. The Mac's copy job.",
        "  2. The signing pause: yes, no or partly?",
      ],
    ],
    "the whole question should be said, never only its last line, its old Mac line ends read as breaks",
  );
});

it("reads a whole text or short line of another kind as none", () => {
  assert.deepEqual(
    askedOf({ wants: "Which thread?", asked: 7 as unknown as string }),
    [["Which thread?"]],
    "a whole text that isn't words should fall back to the short line",
  );
  assert.deepEqual(
    askedOf({ wants: ["Which"] as unknown as string }),
    [],
    "a short line that isn't words should say nothing rather than throw",
  );
});

it("falls back to the short line where the session says nothing longer", () => {
  assert.deepEqual(
    askedOf({ wants: "Which thread: the waxed or the plain?" }),
    [["Which thread: the waxed or the plain?"]],
    "a session that says only its short line should still be heard",
  );
  assert.deepEqual(
    askedOf({ wants: "Which thread?", asked: " \n " }),
    [["Which thread?"]],
    "a blank whole text should read as none",
  );
  assert.deepEqual(askedOf({ wants: null }), [], "a cell asking nothing should say nothing");
});

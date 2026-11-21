// What the page lists first as worth a look, read from the machines' and
// the parts' sides as the daemon writes them (modules/system/src/parts/):
// the order, whose each is, where each leads, and the list drawn, without a
// browser.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { looksDrawn } from "../draw.ts";
import { LOOKS_LISTED, looksOf } from "../looks.ts";
import type { Part, System, Thing } from "../model.ts";
import { tiesOf } from "../ties.ts";

const deeper = { command: "net", site: "network/", launcher: "Network" };

function machine(name: string, health: Thing["health"], side: string[] = []): Thing {
  return { name, health, says: "", side, address: undefined, ties: tiesOf({}) };
}

function part(name: string, health: Part["health"], side: string[], says = ""): Part {
  const site = { ...deeper, site: `${name}/` };
  return { part: name, health, says, side, holds: undefined, things: [], untold: 0, deeper: site };
}

function system(things: Thing[], others: Part[] = [], untold: string[] = []): System {
  const machines: Part = { ...part("machines", "watch", []), things };
  return {
    machine: "kestrel",
    readAt: "2026-11-21T06:20:00Z",
    health: "watch",
    parts: [machines, ...others],
    unread: [],
    untold,
  };
}

it("lists the worst first, across the machines and the parts, the daemon's order within", () => {
  const read = system(
    [
      machine("kestrel", "watch", ["booted 8 hours ago after a firmware error"]),
      machine("osprey", "bad", ["out of reach", "nearly full disk, 3 GB free"]),
      machine("heron", "good"),
    ],
    [part("network", "watch", ["the attic's radio is busy"]), part("code", "good", ["unseen"])],
  );
  const listed = looksOf(read, 8).looks.map((look) => [look.health, look.whose, look.words]);
  assert.deepEqual(
    listed,
    [
      ["bad", "osprey", "Out of reach"],
      ["bad", "osprey", "Nearly full disk, 3 GB free"],
      ["watch", "kestrel", "Booted 8 hours ago after a firmware error"],
      ["watch", "Network", "The attic's radio is busy"],
    ],
    "a bad look leads, and a good part lists nothing",
  );
});

it("leads a machine drawn here to its section, and one past them to the map", () => {
  const read = system([
    machine("kestrel", "watch", ["on battery at 12%"]),
    machine("tern & gull", "watch", ["on battery at 9%"]),
  ]);
  const [here, far] = looksOf(read, 1).looks;
  assert.equal(here?.href, "#machine-1", "a machine drawn whole leads to its own section");
  assert.equal(here?.machine, true, "a machine's name is marked as one");
  assert.equal(
    far?.href,
    "../mycelium/#machine=tern%20%26%20gull",
    "one past those leads to the map",
  );
  const network = looksOf(system([], [part("network", "bad", ["the modem is down"])]), 8).looks[0];
  assert.equal(network?.href, "../network/", "a part's look leads to its own site");
  assert.equal(network?.machine, false, "a part's name is the page's, to translate");
});

it("keys each row by whose it is and its own words, apart from any other", () => {
  const rowsOf = (side: string[]): string[] =>
    looksOf(
      system([machine("kestrel", "watch", side)], [part("code", "watch", ["one"])]),
      8,
    ).looks.map((look) => look.row);
  assert.deepEqual(
    rowsOf(["one", "two", "one"]),
    [
      "look:machine:kestrel:0:one",
      "look:machine:kestrel:0:two",
      "look:machine:kestrel:1:one",
      "look:part:code:0:one",
    ],
    "each row keyed apart, a repeat by how many came before it",
  );
  assert.deepEqual(
    rowsOf(["two"]),
    ["look:machine:kestrel:0:two", "look:part:code:0:one"],
    "a look keeps its key when one before it clears",
  );
  const keys = rowsOf(["x#1", "x", "x", "1:x"]);
  assert.equal(new Set(keys).size, keys.length, "words that look like a count key apart");
  const colons = looksOf(
    system([machine("a", "watch", ["0:x"]), machine("a:0", "watch", ["x"])]),
    8,
  ).looks.map((look) => look.row);
  assert.equal(new Set(colons).size, 2, "a name holding a colon keys apart from another's");
});

it("says a part that wasn't read as a look of its own, first, as the daemon ranks it", () => {
  const unread = part("code", "unread", [], "repos timed out.");
  const bad = part("network", "bad", ["the modem is down"]);
  const read = system([], [bad, unread, part("mycelium", "watch", ["a session is stuck"])]);
  assert.deepEqual(
    looksOf(read, 8).looks.map((look) => [look.health, look.whose, look.words]),
    [
      ["unread", "Code", "Couldn't be read: repos timed out."],
      ["bad", "Network", "The modem is down"],
      ["watch", "Mycelium", "A session is stuck"],
    ],
    "an unread part ranks ahead of a bad one listed before it",
  );
  const machines: System = {
    ...read,
    parts: [{ ...part("machines", "unread", [], "the mycelium couldn't be reached") }],
  };
  assert.deepEqual(
    looksOf(machines, 8).looks.map((look) => [look.whose, look.words, look.href]),
    [["Machines", "Couldn't be read: the mycelium couldn't be reached", "../machines/"]],
    "machines unread are one look, not one per machine",
  );
});

it("lists eight at most, however many there are, and counts the rest", () => {
  const many = Array.from({ length: 1000 }, (_, at) =>
    machine(`m${at}`, "watch", ["code wants a look"]),
  );
  const listed = looksOf(system(many, [part("network", "bad", ["the modem is down"])]), 8);
  assert.equal(listed.looks.length, LOOKS_LISTED, "the list stays a glance");
  assert.equal(listed.more, 1001 - LOOKS_LISTED, "the rest are counted");
  assert.equal(listed.looks[0]?.words, "The modem is down", "ranked before it is cut");
  assert.equal(listed.looks[1]?.whose, "m0", "in the daemon's order");
});

it("says nothing is worth a look only once every part has told", () => {
  const quiet = system([machine("kestrel", "good")]);
  assert.deepEqual(looksOf(quiet, 8), { looks: [], more: 0, settled: true });
  assert.ok(looksDrawn(looksOf(quiet, 8)).includes("Nothing is worth a look."), "said so");
  const waiting = system([machine("kestrel", "good")], [], ["network"]);
  assert.ok(
    looksDrawn(looksOf(waiting, 8)).includes("What is worth a look isn't known yet."),
    "a part still to come may hold a look",
  );
  const coming = system([machine("kestrel", "good")]);
  const named: System = {
    ...coming,
    parts: coming.parts.map((each) => ({ ...each, untold: 1 })),
  };
  assert.equal(looksOf(named, 8).settled, false, "a machine still to come may hold a look");
  const garbled: System = { ...quiet, unread: ["machine:wren has no name"] };
  assert.ok(
    looksDrawn(looksOf(garbled, 8)).includes("What is worth a look isn't known yet."),
    "a record not read may hold a look, and isn't a wait",
  );
});

it("names a part called like an Object key as itself", () => {
  const odd = system(
    [],
    [part("constructor", "watch", ["one look"]), part("toString", "unread", [])],
  );
  assert.deepEqual(
    looksOf(odd, 8).looks.map((look) => look.whose),
    ["toString", "constructor"],
  );
});

it("draws each look as a row of its own, every word escaped", () => {
  const read = system([machine("<b>kestrel</b>", "bad", ['"out" of reach'])]);
  const drawn = looksDrawn(looksOf(read, 8));
  const holds = (words: string, why: string): void => assert.ok(drawn.includes(words), why);
  holds(
    'data-row="look:machine:%3Cb%3Ekestrel%3C%2Fb%3E:0:&quot;out&quot; of reach"',
    "the row keyed",
  );
  holds('<span class="whose" translate="no">&lt;b&gt;kestrel', "the name kept, untranslated");
  holds('<span class="words">&quot;out&quot; of reach</span>', "the words escaped");
  holds('<span class="sr"> (bad)</span>', "its health said to a reader");
  assert.ok(!drawn.includes("<b>"), "no record's markup reaches the page");
  const odd = part('<i>net</i>" x="', "watch", ["one look"]);
  const partDrawn = looksDrawn(looksOf(system([], [odd]), 8));
  const partHolds = (words: string, why: string): void => assert.ok(partDrawn.includes(words), why);
  partHolds('href="../&lt;i&gt;net&lt;/i&gt;&quot; x=&quot;/"', "a part's address escaped");
  partHolds('<span class="whose">&lt;i&gt;net', "a part's name escaped");
  assert.ok(!partDrawn.includes("<i>"), "no part's markup reaches the page");
  const many = Array.from({ length: 10 }, (_, at) => machine(`m${at}`, "watch", ["a look"]));
  const more = looksDrawn(looksOf(system(many), 8));
  // The rest may be machines the page doesn't draw, so they aren't said to be below.
  assert.ok(more.includes("And 2 more.</p>"), "the rest counted, and no place claimed");
});

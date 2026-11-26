// What the page lists first as worth a look, read from the machines' and
// the parts' sides as the daemon writes them (modules/system/src/parts/):
// the order, whose each is, where each leads, and the list drawn, without a
// browser.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { looksDrawn } from "../draw.ts";
import { keptOf, LOOKS_LISTED, looksOf } from "../looks.ts";
import type { Part, System, Thing } from "../model.ts";
import { tiesOf } from "../ties.ts";

const deeper = { command: "net", site: "network/", launcher: "Network" };
/** An act drawn as a row of the cursor's, which it must never be. */
const ACT_AS_ROW = /<a class="launch"[^>]*data-row/u;

function machine(name: string, health: Thing["health"], side: string[] = []): Thing {
  return { name, health, says: "", side, looks: undefined, address: undefined, ties: tiesOf({}) };
}

function part(name: string, health: Part["health"], side: string[], says = ""): Part {
  const site = { ...deeper, site: `${name}/` };
  const read = { holds: undefined, things: [], named: [], untold: 0, deeper: site };
  return { part: name, health, says, side, ...read };
}

function system(things: Thing[], others: Part[] = [], untold: string[] = []): System {
  const named = things.map((thing) => thing.name);
  const machines: Part = { ...part("machines", "watch", []), things, named };
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
    [part("network", "watch", ["the attic's radio is busy"]), part("code", "good", [])],
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

it("lists no kept answer, and says the machines' where the page says how it was read", () => {
  const note = "as read at 11:54; just now mycelium machines gave no answer in 20 s";
  const said = `Machines ${note}.`;
  const machinesOf = (things: Thing[], side: string[], health: Part["health"] = "watch"): Part => {
    const named = things.map((thing) => thing.name);
    return { ...part("machines", health, side), things, named };
  };
  const listed = (...parts: Part[]): string[][] =>
    looksOf({ ...system([]), parts }, 8).looks.map((look) => [look.health, look.whose, look.words]);
  const kept = (...parts: Part[]): string => keptOf({ ...system([]), parts });
  const heron = machinesOf([machine("heron", "good")], [note], "good");
  assert.deepEqual(listed(heron), [], "machines read from a kept answer list nothing");
  assert.equal(kept(heron), said, "the page's note says they were");
  assert.equal(kept(machinesOf([machine("heron", "good")], [])), "", "a fresh read says nothing");
  const unread = machinesOf([], [note], "unread");
  assert.equal(kept(unread), "", "one not read at all is a look, not a note");
  const osprey = machine("osprey", "bad", ["out of reach"]);
  const kestrel = machine("kestrel", "watch", ["on battery at 9%"]);
  const both = ["osprey out of reach", "kestrel on battery at 9%", note];
  assert.deepEqual(
    listed(machinesOf([osprey, kestrel], both, "bad")),
    [
      ["bad", "osprey", "Out of reach"],
      ["watch", "kestrel", "On battery at 9%"],
    ],
    "what a machine says is listed once, as the machine's",
  );
  assert.equal(
    kept(machinesOf([osprey, kestrel], both, "bad")),
    said,
    "the note says only the part's own words, not its machines'",
  );
  const cleared = [machine("kestrel", "good"), machine("osprey", "good")];
  const stale = ["kestrel booted 3 hours ago", "osprey says its code wants a look"];
  assert.deepEqual(
    listed(machinesOf(cleared, stale)),
    [],
    "a machine's look the part still names is gone once the machine's record clears",
  );
  const many = [machine("m0", "watch", ["code wants a look"])];
  const counted = ["m0 code wants a look", "and 4 more worth a look"];
  assert.deepEqual(
    listed(machinesOf(many, counted)),
    [["watch", "m0", "Code wants a look"]],
    "how many more the part counts is left to the machines' own looks",
  );
  assert.equal(
    kept(machinesOf(many, [counted[0] ?? "", note, counted[1] ?? ""])),
    said,
    "the note leaves out how many more, which the daemon counts after it",
  );
  assert.equal(
    kept(machinesOf([], ["wren out of reach", note])),
    said,
    "the note is the part's kept answer alone, not a machine it names that the page doesn't",
  );
  const coming: Part = { ...machinesOf([], ["wren out of reach"]), named: ["wren"], untold: 1 };
  assert.deepEqual(listed(coming), [], "a machine named before its record waits for it");
  assert.equal(kept(coming), "", "nor is a machine named before its record a note");
  const net = "as read at 11:56; just now net --json gave no answer in 20 s";
  assert.deepEqual(
    listed(machinesOf([], []), part("network", "good", [net])),
    [],
    "a good part read from a kept answer lists nothing, its own section saying so",
  );
  assert.deepEqual(
    listed(machinesOf([], []), part("network", "watch", ["the hall point is weak", net])),
    [["watch", "Network", "The hall point is weak, as read at 11:56"]],
    "a part read from a kept answer lists its looks, saying when, never its note",
  );
  assert.deepEqual(
    listed(
      machinesOf([], []),
      part("network", "watch", ["the hall point is weak", net, "and 2 more worth a look"]),
    ),
    [
      ["watch", "Network", "The hall point is weak, as read at 11:56"],
      ["watch", "Network", "And 2 more worth a look"],
    ],
    "how many more it counts is said with no time of its own",
  );
  const rows = (side: string[]): (string | undefined)[] =>
    looksOf({ ...system([]), parts: [part("network", "watch", side)] }, 8).looks.map(
      (look) => look.row,
    );
  assert.deepEqual(
    rows(["the hall point is weak", net]),
    rows(["the hall point is weak"]),
    "a look keeps its row whether or not it stands on a kept answer",
  );
  assert.deepEqual(
    listed(machinesOf([], [note], "good"), part("network", "bad", ["down"])),
    [["bad", "Network", "Down"]],
    "a kept note never stands in the list beside a look",
  );
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

/** A machine whose record lists its looks, each with why, and gives its launcher link. */
function judged(
  name: string,
  health: Thing["health"],
  looks: Thing["looks"],
  reached = true,
): Thing {
  const link = `mycelium:machine/${encodeURIComponent(name)}?from=system`;
  const side = (looks ?? []).map((look) => look.side).slice(0, 3);
  const ties = tiesOf({ reached, acts: { link } });
  return { ...machine(name, health, side), looks, ties };
}

it("says why each machine's look matters, with its one act, the machine's launcher row", () => {
  const why = (said: string): { side: string; matters: string } => ({
    side: said,
    matters: `Why ${said} matters.`,
  });
  const kestrel = judged("kestrel", "bad", [
    why("nearly full disk, 3 GB free"),
    why("on battery at 9%"),
    why("code wants a look"),
    why("no quest backup in 3 h"),
  ]);
  const listed = looksOf(system([kestrel], [part("network", "bad", ["the modem is down"])]), 8);
  const link = "mycelium:machine/kestrel?from=system";
  assert.deepEqual(
    listed.looks.map((look) => [look.whose, look.words, look.matters, look.act]),
    [
      ["kestrel", "Nearly full disk, 3 GB free", "Why nearly full disk, 3 GB free matters.", link],
      ["kestrel", "On battery at 9%", "Why on battery at 9% matters.", link],
      ["kestrel", "Code wants a look", "Why code wants a look matters.", link],
      ["kestrel", "No quest backup in 3 h", "Why no quest backup in 3 h matters.", link],
      ["Network", "The modem is down", "", undefined],
    ],
    "every look the daemon judged, past the side's three; a part's leads to its site alone",
  );
  const aged = { ...judged("kestrel", "watch", undefined), side: ["on battery at 9%"] };
  const old = looksOf(system([aged]), 8).looks[0];
  assert.deepEqual(
    [old?.words, old?.matters, old?.act],
    ["On battery at 9%", "", link],
    "a daemon too old to list its looks leaves the side to say them, with no why but its act",
  );
  const torn = { ...judged("kestrel", "watch", []), side: ["on battery at 9%"] };
  assert.deepEqual(
    looksOf(system([torn]), 8).looks.map((look) => look.words),
    ["On battery at 9%"],
    "an empty list beside a side that names a look leaves the side to say it",
  );
});

it("gives the one look for a machine out of reach its why and act, and several neither", () => {
  const away = (name: string): Thing =>
    judged(
      name,
      "bad",
      [{ side: "out of reach", matters: "What runs there can't be read from here." }],
      false,
    );
  const [alone] = looksOf(system([away("osprey")]), 8).looks;
  assert.deepEqual(
    [alone?.words, alone?.matters, alone?.act],
    [
      "Out of reach",
      "What runs there can't be read from here.",
      "mycelium:machine/osprey?from=system",
    ],
  );
  const [many] = looksOf(system([away("osprey"), away("tern")]), 8).looks;
  assert.deepEqual(
    [many?.words, many?.matters, many?.act],
    ["Out of reach: osprey and tern", "", undefined],
    "the daemon's why speaks of one machine, and no launcher row is theirs together",
  );
});

it("draws a look's why and act under its words, escaped, and no line where it has neither", () => {
  const kestrel = judged("<b>kestrel</b>", "bad", [
    { side: "nearly full disk", matters: 'Builds <i>fail</i> "there".' },
  ]);
  const drawn = looksDrawn(looksOf(system([kestrel]), 8));
  const holds = (words: string, why: string): void => assert.ok(drawn.includes(words), why);
  holds(
    '<p class="why"><span>Builds &lt;i&gt;fail&lt;/i&gt; &quot;there&quot;.</span>',
    "the why escaped",
  );
  holds(
    '<a class="launch" href="mycelium:machine/%3Cb%3Ekestrel%3C%2Fb%3E?from=system">',
    "the act opens the machine's launcher row",
  );
  holds(
    '<span>Open <span translate="no">&lt;b&gt;kestrel&lt;/b&gt;</span> in the Launcher</span></a>',
    "the act names its machine, untranslated, in one run of words a flex box keeps spaced",
  );
  assert.ok(
    !(drawn.includes("<b>") || drawn.includes("<i>")),
    "no record's markup reaches the page",
  );
  assert.ok(!ACT_AS_ROW.test(drawn), "the act is no cursor row, so j and k step from look to look");
  const bare = looksDrawn(looksOf(system([], [part("network", "bad", ["the modem is down"])]), 8));
  assert.ok(!bare.includes('class="why"'), "a look with no why and no act draws no line for them");
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

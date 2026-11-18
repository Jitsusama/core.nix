// What the system page makes of the system daemon's records, told as the
// daemon tells them: the summary, a part record each, a record per thing.
// Then what it draws of them in the system's words, and that its own files
// keep to those words.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { escaped, legend, partsOf } from "../draw.ts";
import { allTold, systemOf } from "../model.ts";

/** The three watches, told `these` whole, each to its kind's. */
function told(these: readonly { readonly key: string; readonly kind: string }[]) {
  const summary = new Held();
  const part = new Held();
  const machine = new Held();
  summary.took({ records: these.filter((record) => record.kind === "summary") });
  part.took({ records: these.filter((record) => record.kind === "part") });
  machine.took({ records: these.filter((record) => record.kind === "machine") });
  return { summary, part, machine };
}

const deeper = { command: "mycelium machines", site: "mycelium/", launcher: "Agents" };

const records = [
  {
    key: "summary",
    kind: "summary",
    machine: "kestrel",
    read_at: "2026-11-16T05:41:00Z",
    health: "watch",
    parts: ["machines", "code"],
  },
  {
    key: "part:machines",
    kind: "part",
    part: "machines",
    health: "watch",
    says: "osprey has a filling disk",
    side: ["osprey has a filling disk"],
    holds: "2 machines",
    things: ["machine:kestrel", "machine:osprey"],
    deeper,
  },
  {
    key: "part:code",
    kind: "part",
    part: "code",
    health: "good",
    says: "18 repositories",
    side: [],
    holds: "18 repositories",
    deeper,
  },
  { key: "machine:kestrel", kind: "machine", name: "kestrel", health: "good", says: "is fine" },
  {
    key: "machine:osprey",
    kind: "machine",
    name: "osprey",
    health: "watch",
    says: "has a filling disk",
    address: "mycelium/#machine=osprey",
  },
];

it("nothing is drawn until the summary is told", () => {
  const { summary, part, machine } = told(records.filter((record) => record.kind !== "summary"));
  assert.equal(systemOf(summary, part, machine), undefined);
});

it("the parts come in the summary's order, each with its things", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.deepEqual(
    system.parts.map((each) => each.part),
    ["machines", "code"],
  );
  assert.deepEqual(system.untold, [], "every part told leaves none to come");
  assert.deepEqual(
    system.parts[0]?.things.map((thing) => [thing.name, thing.health, thing.address]),
    [
      ["kestrel", "good", undefined],
      ["osprey", "watch", "mycelium/#machine=osprey"],
    ],
  );
  assert.deepEqual(system.unread, []);
});

it("a part not told yet is said, a thing not told yet left out, and the rest drawn", () => {
  const { summary, part, machine } = told(
    records.filter((record) => record.key !== "machine:osprey" && record.key !== "part:code"),
  );
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.deepEqual(
    system.parts.map((each) => [each.part, each.things.map((thing) => thing.name)]),
    [["machines", ["kestrel"]]],
  );
  assert.deepEqual(system.unread, ["part:code wasn't told"]);
  assert.deepEqual(system.untold, ["code"], "a part not told yet is named as still to come");
});

it("the daemon's not read is read as unread", () => {
  const { summary, part, machine } = told(
    records.map((record) =>
      record.key === "part:code" ? { ...record, health: "not read" } : record,
    ),
  );
  assert.equal(systemOf(summary, part, machine)?.parts[1]?.health, "unread");
});

it("a health the page doesn't know is read as not read", () => {
  const { summary, part, machine } = told(
    records.map((record) => (record.key === "part:code" ? { ...record, health: "odd" } : record)),
  );
  assert.equal(systemOf(summary, part, machine)?.parts[1]?.health, "unread");
});

it("a part's side and what it holds are read apart", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.deepEqual(
    system?.parts.map((each) => [each.side, each.holds]),
    [
      [["osprey has a filling disk"], "2 machines"],
      [[], "18 repositories"],
    ],
  );
});

/** What `markup` reads as, its tags gone and its spaces run together. */
function read(markup: string): string {
  return markup
    .replace(/<[^>]*>/gu, " ")
    .replace(/\s+/gu, " ")
    .trim();
}

/**
 * The weather words the page once wore, which the glossary keeps for real
 * weather, and the same without cloud, which two marks are drawn as.
 */
const WEATHER =
  /\b(?:weather|forecasts?|clear|clouds?|cloudy|storms?|stormy|fog|foggy|sky|skies)\b/iu;
const WEATHER_BUT_CLOUD =
  /\b(?:weather|forecasts?|clear|cloudy|storms?|stormy|fog|foggy|sky|skies)\b/iu;

/** The attributes a person or a preview reads: descriptions, labels, titles. */
const READ_ATTRIBUTE = /\b(?:content|aria-label|title|alt)="(?<said>[^"]*)"/gu;

/** What `markup` says to a person: its text, and the attributes read aloud. */
function said(markup: string): string {
  const attributes = [...markup.matchAll(READ_ATTRIBUTE)].map((each) => each.groups?.["said"]);
  return [read(markup), ...attributes].join(" ");
}

/** A legend item: its mark's health, then its word. */
const KEY =
  /<li class="key"><span class="mark (?<health>\w+)" aria-hidden="true"><\/span><span class="word">(?<word>[^<]*)<\/span><\/li>/gu;

/** A tie's heading, in the order drawn. */
const TIE = /<section class="tie"><h3>(?<name>[^<]*)<\/h3>/gu;

/** A row the keys step through, by its key. */
const ROW = /data-row="(?<key>[^"]*)"/gu;
const HIDDEN_BRIDGE = /<figure class="bridge" id="bridge" aria-hidden="true">/u;

/** A time, as the page says one. */
const when = (at: string): string => `at ${at}`;

/**
 * Where index.html leaves room for the legend: a list of its own, which
 * says its role outright, since Safari drops it from an unbulleted list.
 */
const LEGEND_LIST = /<ul(?=[^>]*\bid="legend")(?=[^>]*\brole="list")[^>]*><\/ul>/u;

it("the legend gives each health its own item: its own mark, then its word", () => {
  const keys = [...legend().matchAll(KEY)].map((key) => [
    key.groups?.["health"],
    key.groups?.["word"],
  ]);
  assert.deepEqual(keys, [
    ["good", "Good"],
    ["watch", "Watch"],
    ["bad", "Bad"],
    ["unread", "Not read"],
  ]);
  const shapes = new Set(keys.map(([health]) => health));
  assert.equal(shapes.size, 4, "each health is drawn in a mark of its own");
});

/** The system `records` hold, with the machines' records changed by `change`. */
function systemWith(change: (record: (typeof records)[number]) => object = (record) => record) {
  const { summary, part, machine } = told(records.map(change) as typeof records);
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  return system;
}

it("each machine has a section of its own, with its four ties in order", () => {
  const drawn = partsOf(systemWith(), when);
  assert.ok(drawn.includes('<section class="machine good" id="machine-1"'));
  assert.ok(drawn.includes('<section class="machine watch" id="machine-2"'));
  const [first] = drawn.split('id="machine-2"');
  const ties = [...(first ?? "").matchAll(TIE)].map((each) => each.groups?.["name"]);
  assert.deepEqual(ties, ["Cells", "Network", "Code", "Itself"]);
});

it("steps through each machine's rows, then the parts across them", () => {
  const keys = [...partsOf(systemWith(), when).matchAll(ROW)].map((each) => each.groups?.["key"]);
  assert.deepEqual(keys, [
    "machine:kestrel",
    "cells:kestrel",
    "machine:osprey",
    "cells:osprey",
    "part:code",
  ]);
});

it("goes to a machine's place on the network, and offers the launcher only with an act", () => {
  const system = systemWith((record) =>
    record.key === "machine:osprey"
      ? {
          ...record,
          ties: {
            network: [{ device: "mac:a8 b", by: "wired", point: "Den" }],
            code: { repositories: 3, checkouts: 4 },
            acts: {
              link: "mycelium:machine/osprey?from=system",
              commands: ["mycelium update osprey"],
            },
          },
        }
      : record,
  );
  const drawn = partsOf(system, when);
  assert.ok(drawn.includes('href="../network/#device=mac%3Aa8%20b"'));
  assert.ok(drawn.includes('data-row="code:osprey" href="../code/"'));
  assert.ok(drawn.includes('href="mycelium:machine/osprey?from=system">Open in the Launcher<'));
  assert.ok(drawn.includes('data-command="mycelium update osprey"'));
  assert.equal(drawn.split("Open in the Launcher").length - 1, 1, "kestrel has no act");
});

it("offers the launcher only by its own scheme", () => {
  const system = systemWith((record) =>
    record.key === "machine:osprey"
      ? { ...record, ties: { acts: { link: "javascript:alert(1)", commands: [] } } }
      : record,
  );
  assert.ok(!partsOf(system, when).includes("Open in the Launcher"));
});

it("draws eight machines whole, and sends the rest to the map", () => {
  const system = systemWith();
  const [machines] = system.parts;
  const [one] = machines?.things ?? [];
  assert.ok(machines && one);
  const many = Array.from({ length: 11 }, (_, at) => ({ ...one, name: `m${at}` }));
  const drawn = partsOf({ ...system, parts: [{ ...machines, things: many }] }, when);
  assert.equal(drawn.split('<section class="machine ').length - 1, 8);
  assert.ok(drawn.includes('href="../mycelium/">And 3 more machines, on the map<'));
});

// The daemon's word for each health, then what a screen reader hears.
for (const [daemonSays, word] of [
  ["good", "good"],
  ["watch", "watch"],
  ["bad", "bad"],
  ["not read", "not read"],
] as const) {
  it(`a screen reader hears a part and a machine told ${daemonSays} as "${word}"`, () => {
    const system = systemWith((record) =>
      record.key === "part:code" || record.key === "machine:osprey"
        ? { ...record, health: daemonSays }
        : record,
    );
    const drawn = partsOf(system, when);
    assert.ok(drawn.includes(`Code<span class="sr"> (${word})</span>`), "the part's heading");
    assert.ok(
      drawn.includes(`osprey</span><span class="sr"> (${word})</span>`),
      "the machine's heading",
    );
  });
}

it("the page and its words for a model wear no weather words, nor in what is read aloud", () => {
  for (const [file, words] of [
    ["index.html", WEATHER],
    ["llms.txt", WEATHER],
    ["index.md", WEATHER_BUT_CLOUD],
  ] as const) {
    const text = readFileSync(new URL(`../${file}`, import.meta.url), "utf8");
    assert.doesNotMatch(said(text), words, file);
  }
});

it("the legend is a list in every browser, so a screen reader hears its words apart", () => {
  const page = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(page, LEGEND_LIST);
});

it("keeps the whole bridge from a screen reader, since the sections under it say it all", () => {
  const page = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(page, HIDDEN_BRIDGE);
});

it("draws once every watch has told its records, or can't", () => {
  const kinds = ["summary", "part", "machine"];
  const live = new Map([
    ["summary", { state: "live" }],
    ["part", { state: "live" }],
    ["machine", { state: "live" }],
  ]);
  assert.equal(allTold(kinds, new Set(), new Map()), false, "nothing told yet");
  assert.equal(allTold(kinds, new Set(["summary", "part"]), live), false, "live is not told");
  assert.equal(allTold(kinds, new Set(kinds), live), true, "every kind told");
  for (const state of ["broken", "stale"]) {
    const stuck = new Map([...live, ["machine", { state }]]);
    assert.equal(allTold(kinds, new Set(["summary", "part"]), stuck), true, `machine ${state}`);
  }
  const connecting = new Map([...live, ["machine", { state: "connecting" }]]);
  assert.equal(allTold(kinds, new Set(["summary", "part"]), connecting), false, "still connecting");
});

it("a record's words reach the page as text, never as markup", () => {
  assert.equal(escaped(`<b a="x">'&'</b>`), "&lt;b a=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/b&gt;");
});

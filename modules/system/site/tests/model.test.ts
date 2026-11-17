// What the system page makes of the system daemon's records, told as the
// daemon tells them: the summary, a part record each, a record per thing.
// Then what it draws of them in the system's words, and that its own files
// keep to those words.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { escaped, legend, outlookOf, partsOf } from "../draw.ts";
import { headline, lede, looksIn, sentence, systemOf } from "../model.ts";

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

it("a side's closing and N more counts for as many", () => {
  const { summary, part, machine } = told(records);
  const machines = systemOf(summary, part, machine)?.parts[0];
  assert.ok(machines);
  const side = ["a", "b", "and 4 more worth a look"];
  assert.equal(looksIn({ ...machines, side }), 6);
  assert.equal(
    looksIn({ ...machines, side, health: "unread" }),
    0,
    "why it wasn't read is no look",
  );
});

it("the headline counts the parts, what is worth a look and what wasn't read", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.equal(sentence(headline(system)), "Two parts, one worth a look.");
  const [first, second] = system.parts;
  assert.ok(first && second);
  const many = {
    ...system,
    parts: [
      { ...first, side: ["a", "b", "and 10 more worth a look"] },
      { ...second, health: "unread" as const },
    ],
  };
  assert.equal(sentence(headline(many)), "Two parts, 12 worth a look and one not read.");
  const good = { ...system, parts: [{ ...first, health: "good" as const, side: [] }] };
  assert.equal(sentence(headline(good)), "One part, all good.");
});

it("the lede names the worst part's first look, then why a part wasn't read", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.equal(lede(system), "Machines: osprey has a filling disk.", "a name keeps its case");
  const [first, second] = system.parts;
  assert.ok(first && second);
  const worse = {
    ...system,
    parts: [first, { ...second, health: "bad" as const, side: ["the build is failing"] }],
  };
  assert.equal(lede(worse), "Code: the build is failing.", "bad comes before worth a look");
  const stopped = {
    ...system,
    parts: [
      first,
      { ...second, health: "bad" as const, side: ["the radio is busy, 20 hours ago."] },
    ],
  };
  assert.equal(
    lede(stopped),
    "Code: the radio is busy, 20 hours ago.",
    "an owner's words that end their own sentence get no second stop",
  );
  const asked = {
    ...system,
    parts: [first, { ...second, health: "bad" as const, side: ["is the modem on?"] }],
  };
  assert.equal(lede(asked), "Code: is the modem on?", "a question keeps its own mark");
  const unread = {
    ...system,
    parts: [
      { ...first, health: "good" as const, side: [] },
      { ...second, health: "unread" as const, says: "repos timed out" },
    ],
  };
  assert.equal(lede(unread), "Code couldn't be read: repos timed out.");
  const unreadStopped = {
    ...unread,
    parts: [unread.parts[0], { ...second, health: "unread" as const, says: "repos timed out." }],
  };
  assert.equal(
    lede(unreadStopped),
    "Code couldn't be read: repos timed out.",
    "why a part wasn't read gets one stop too",
  );
  assert.equal(
    lede({ ...system, parts: [{ ...first, health: "good" as const, side: [] }] }),
    undefined,
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
  /<li class="key"><svg class="sky (?<health>\w+) [^"]*"[\s\S]*?<\/svg><span class="word">(?<word>[^<]*)<\/span><\/li>/gu;

/** A part on the outlook: its name, then what it says. */
const ON_THE_OUTLOOK =
  /aria-label="(?<heard>[^"]*)">[\s\S]*?<span class="place">(?<place>[^<]*)<\/span><span class="forecast">(?<says>[^<]*)<\/span><\/a>/gu;

/** Where index.html leaves room for the legend: a list of its own. */
const LEGEND_LIST = /<ul class="legend" id="legend"[^>]*><\/ul>/u;

/** A drawn mark's shape: what is inside its svg. */
const SHAPE = /<svg [^>]*>(?<shape>[\s\S]*?)<\/svg>/gu;

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
  const shapes = new Set([...legend().matchAll(SHAPE)].map((each) => each.groups?.["shape"]));
  assert.equal(shapes.size, 4, "each health is drawn in a shape of its own");
});

it("each part on the outlook says how many are worth a look, or else its health's word", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  const [first, second] = system.parts;
  assert.ok(first && second);
  const four = {
    ...system,
    parts: [
      { ...first, health: "good" as const, side: [] },
      { ...second, health: "unread" as const, side: [] },
      { ...second, health: "watch" as const, side: [] },
      { ...second, health: "watch" as const, side: ["a"] },
      { ...second, health: "bad" as const, side: ["a", "b"] },
    ],
  };
  const spoke = [...outlookOf(four).matchAll(ON_THE_OUTLOOK)].map((each) => [
    each.groups?.["place"],
    each.groups?.["says"],
    each.groups?.["heard"],
  ]);
  // What is shown, then what a screen reader hears: with looks, the health
  // too, as the link's name, so watch and bad don't sound alike while the
  // text stays what is shown.
  assert.deepEqual(spoke, [
    ["Machines", "good", "Machines good"],
    ["Code", "not read", "Code not read"],
    ["Code", "watch", "Code watch"],
    ["Code", "1 worth a look", "Code 1 worth a look (watch)"],
    ["Code", "2 worth a look", "Code 2 worth a look (bad)"],
  ]);
});

// The daemon's word for each health, then what a screen reader hears.
for (const [daemonSays, word] of [
  ["good", "good"],
  ["watch", "watch"],
  ["bad", "bad"],
  ["not read", "not read"],
] as const) {
  it(`a screen reader hears a part and a machine told ${daemonSays} as "${word}"`, () => {
    const these = records.map((record) =>
      record.key === "part:machines" || record.key === "machine:osprey"
        ? { ...record, health: daemonSays }
        : record,
    );
    const { summary, part, machine } = told(these);
    const system = systemOf(summary, part, machine);
    assert.ok(system);
    const drawn = partsOf(system);
    assert.ok(drawn.includes(`Machines<span class="sr"> (${word})</span>`), "the part's heading");
    assert.ok(
      drawn.includes(`osprey</span><span class="sr"> (${word})</span>`),
      "the machine's row",
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

it("the legend is a list, so a screen reader hears its words apart", () => {
  const page = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(page, LEGEND_LIST);
});

it("a record's words reach the page as text, never as markup", () => {
  assert.equal(escaped(`<b a="x">'&'</b>`), "&lt;b a=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/b&gt;");
});

// What the system page makes of the system daemon's records, told as the daemon tells them: the
// summary, a part record each, a record per thing. Then what it draws of them in the system's
// words, and that its own files keep to those words.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { escaped, headlineOf, legend, partsOf, placesOf } from "../draw.ts";
import { looksOf as listedOf } from "../looks.ts";
import { machineHeadline } from "../machine.ts";
import { allTold, systemOf, thingsHeld } from "../model.ts";

/** The four watches told `these` whole, the machines' and builders' as one, as the page reads. */
function told(these: readonly { readonly key: string; readonly kind: string }[]) {
  const heldOf = (kind: string) => {
    const held = new Held();
    held.took({ records: these.filter((record) => record.kind === kind) });
    return held;
  };
  const machine = thingsHeld(heldOf("machine"), heldOf("builder"));
  return { summary: heldOf("summary"), part: heldOf("part"), machine };
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
    says: "osprey says its code wants a look",
    side: ["osprey says its code wants a look"],
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
    says: "says its code wants a look",
    address: "system/#machine=osprey",
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
      ["osprey", "watch", "system/#machine=osprey"],
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
      [["osprey says its code wants a look"], "2 machines"],
      [[], "18 repositories"],
    ],
  );
});

it("a machine's looks are read with their whys, and none from a record listing none", () => {
  const named = { key: "machine:osprey", kind: "machine", name: "osprey", health: "bad" };
  const reach = { side: "out of reach", matters: "What can't be read from here.", health: "bad" };
  const side = ["out of reach", "code failing"];
  const osprey = { ...named, side, looks: [reach, { side: "code failing" }] };
  const ospreyAs = (record: Record<string, unknown>) => {
    const others = records.filter((each) => each.key !== "machine:osprey");
    const { summary, part, machine } = told([...others, record]);
    return systemOf(summary, part, machine);
  };
  const looksOf = (record: Record<string, unknown>) =>
    ospreyAs(record)?.parts[0]?.things.map((thing) => thing.looks);
  const looks = [reach, { side: "code failing", matters: "" }];
  const noSide = ["machine:osprey's side, so the page goes by its looks alone"];
  for (const [record, listed, why, unread] of [
    [osprey, looks, "a list whose look without a why is kept with none", []],
    [{ ...osprey, looks: undefined }, undefined, "an old daemon's record, listing none", []],
    [{ ...osprey, health: "good", looks: [], side: [] }, [], "a fine machine's", []],
    [{ ...osprey, side: "out of reach" }, looks, "an unreadable side beside a list", noSide],
    [{ ...osprey, looks: [], side: [7] }, [], "an unreadable side beside none", noSide],
  ] as const) {
    assert.deepEqual(looksOf(record)?.[1], listed, `${why} is read from its list`);
    assert.deepEqual(ospreyAs(record)?.unread, unread, `${why} says only what it can't read`);
  }
  const notRead = ["machine:osprey's looks, so the page goes by its side alone, with no whys"];
  for (const [record, why] of [
    [{ ...osprey, looks: [...osprey.looks, { matters: "A why." }] }, "a look with no side"],
    [{ ...osprey, looks: [...osprey.looks, { side: "" }] }, "a look with an empty side"],
    [{ ...osprey, looks: [...osprey.looks, "a bare string"] }, "a look that's a bare string"],
    [{ ...osprey, looks: "a list" }, "a looks field that isn't a list"],
    [{ ...osprey, looks: [], side: ["on battery at 9%"] }, "an empty list beside a side's look"],
    [{ ...osprey, side: [] }, "a list beside a side naming none"],
  ] as const) {
    assert.deepEqual(looksOf(record), [undefined, undefined], `${why} leaves it to the side`);
    assert.deepEqual(ospreyAs(record)?.unread, notRead, `${why} is said as not read`);
  }
});

it("a look's own health is read only from a word the daemon says a health in", () => {
  const ospreyLooking = (health: unknown) => {
    const look = { side: "code failing", matters: "Builds there fail.", health };
    const named = { key: "machine:osprey", kind: "machine", name: "osprey", health: "watch" };
    const others = records.filter((each) => each.key !== named.key);
    const osprey = { ...named, side: [look.side], looks: [look] };
    const { summary, part, machine } = told([...others, osprey]);
    const system = systemOf(summary, part, machine);
    assert.ok(system, "the records whole drew no system");
    const own = system.parts[0]?.things.find((thing) => thing.name === "osprey")?.looks?.[0];
    const listed = listedOf(system, 8).looks.find((each) => each.whose === "osprey");
    return { own, listed };
  };
  for (const [word, health] of [
    ["good", "good"],
    ["watch", "watch"],
    ["bad", "bad"],
    ["not read", "unread"],
  ] as const) {
    const { own, listed } = ospreyLooking(word);
    assert.equal(own?.health, health, `a look said ${word} was read as ${own?.health}`);
    assert.equal(listed?.health, health, `a look said ${word} was listed as ${listed?.health}`);
  }
  // Osprey itself is watch, so a look with no health of its own is listed as watch.
  for (const [word, why] of [
    ["odd", "a word the page doesn't know"],
    ["unread", "the page's own word, which the daemon never says"],
    ["", "an empty word"],
    [7, "a number"],
    [null, "null"],
  ] as const) {
    const { own, listed } = ospreyLooking(word);
    assert.ok(own !== undefined, `a look whose health is ${why} wasn't read at all`);
    assert.ok(!Object.hasOwn(own, "health"), `${why} gave a look the health ${own.health}`);
    assert.equal(listed?.health, "watch", `${why} listed a look as ${listed?.health}, not watch`);
  }
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
const PLACE = /data-place="(?<place>[^"]*)"/gu;
const HIDDEN_BRIDGE = /<figure class="bridge" id="bridge" aria-hidden="true">/u;

/** A time, as the page says one. */
const when = (at: string): string => `at ${at}`;

/**
 * Where index.html leaves room for the legend: a list of its own, which
 * says its role outright, since Safari drops it from an unbulleted list.
 */
/** A section's opening tag, cut off its `<section `, saying whose it is. */
const OPENS_WITH_ITS_PART = /^[^>]*\bdata-part="[\w-]+"/u;

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

it("turns the headline in its worst health's colour, and names each machine in the places", () => {
  assert.equal(
    headlineOf({ machines: "Two machines", stand: "both worth a look", health: "watch" }),
    'Two machines, <em class="watch">both worth a look.</em>',
  );
  const places = placesOf(systemWith());
  const targets = [...places.matchAll(PLACE)].map((each) => each.groups?.["place"]);
  assert.deepEqual(targets, ["machine-1", "machine-2", "across"]);
  assert.ok(places.includes("kestrel") && places.includes("osprey"));
});

it("says why there are no machines to draw when their part couldn't be read", () => {
  const system = systemWith((record) =>
    record.key === "part:machines"
      ? { ...record, health: "not read", says: "the mycelium couldn't be reached", things: [] }
      : record,
  );
  const drawn = partsOf(system, when);
  assert.ok(
    drawn.includes('<p class="bare health unread">'),
    "the unread part drew no paragraph of its own",
  );
  assert.ok(drawn.includes(`The mycelium couldn${escaped("'")}t be reached`));
  assert.ok(!drawn.includes('class="machine '), "a machine was drawn from an unread part");
});

it("counts the machines a part names whose records haven't come", () => {
  const system = systemWith();
  const { summary, part, machine } = told(
    records.filter((record) => record.key !== "machine:osprey"),
  );
  const partial = systemOf(summary, part, machine);
  assert.equal(system.parts[0]?.untold, 0);
  assert.equal(partial?.parts[0]?.things.length, 1);
  assert.equal(partial?.parts[0]?.untold, 1);
  assert.ok(partial);
  assert.equal(machineHeadline(partial).machines, "Two machines");
});

it("says the machines are awaited, not the part's own words, while none has come", () => {
  const system = systemWith();
  const { summary, part, machine } = told(records.filter((record) => record.kind !== "machine"));
  const awaited = systemOf(summary, part, machine);
  assert.ok(awaited);
  const drawn = partsOf(awaited, when);
  assert.ok(
    drawn.includes('<p class="bare health unread">'),
    "the awaited machines drew no line of their own",
  );
  assert.ok(drawn.includes("Two machines named, none read yet."));
  assert.ok(
    !drawn.includes("Osprey says its code wants a look"),
    "the part's words were said as read",
  );
  assert.ok(partsOf(system, when).includes('class="machine '));
});

it("says why a good part read from its owner's last answer is that answer", () => {
  const note = "as read at 11:54; just now repos --json gave no answer in 20 s";
  const kept = partsOf(
    systemWith((record) =>
      record.key === "part:code"
        ? { ...record, says: `18 repositories, ${note}`, side: [note] }
        : record,
    ),
    when,
  );
  assert.ok(
    kept.includes(
      `<p class="said">18 repositories</p><p class="aside">As read at 11:54; just now repos --json`,
    ),
    "a good part says what it holds, with the kept note aside",
  );
  assert.ok(
    partsOf(systemWith(), when).includes(
      `<p class="said">18 repositories</p><p class="aside">Nothing in it is worth a look.`,
    ),
    "a good part read from its own answer has nothing worth a look",
  );
});

it("draws no command a shell would read more into", () => {
  const system = systemWith((record) =>
    record.key === "part:code"
      ? { ...record, deeper: { ...deeper, command: "system code | sh" } }
      : record,
  );
  assert.equal(system.parts[1]?.deeper.command, "");
  assert.ok(!partsOf(system, when).includes("system code | sh"));
});

it("puts every command y copies inside a section that says whose it is", () => {
  const drawn = partsOf(systemWith(), when);
  const sections = drawn.split("<section ").slice(1);
  assert.ok(sections.some((section) => section.includes('class="copy"')));
  for (const section of sections.filter((one) => one.includes('class="copy"'))) {
    assert.match(section, OPENS_WITH_ITS_PART, `${section.slice(0, 60)} says no part`);
  }
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

it("is a machine's home, and sends its name and its cells on to the map", () => {
  const system = systemWith((record) =>
    record.key === "machine:kestrel" ? { ...record, name: "kestrel two" } : record,
  );
  const drawn = partsOf(system, when);
  const [kestrel, osprey] = drawn.split('<section class="machine ').slice(1);
  assert.ok(kestrel && osprey, "both machines weren't drawn");
  assert.ok(
    osprey.startsWith(
      'watch" id="machine-2" data-part="machine-2" data-home="system/#machine=osprey"',
    ),
    "osprey's section didn't say this page is its home",
  );
  assert.ok(!kestrel.includes("data-home"), "a machine the sites home nowhere claimed a home");
  const map = /href="(?<href>[^"]*)"/gu;
  const [name, cells] = [...osprey.matchAll(map)].map((each) => each.groups?.["href"]);
  assert.equal(name, "../mycelium/#machine=osprey", "osprey's name didn't lead to the map");
  assert.equal(cells, "../mycelium/#machine=osprey", "osprey's cells didn't lead to the map");
  assert.ok(
    kestrel.includes('href="../mycelium/#machine=kestrel%20two"'),
    "a name wasn't encoded on its way to the map",
  );
  assert.ok(!drawn.includes('href="../system/'), "a machine's link led back to this page");
});

it("draws no home for a machine the sites home elsewhere", () => {
  const system = systemWith((record) =>
    record.key === "machine:osprey" ? { ...record, address: "mycelium/#machine=osprey" } : record,
  );
  assert.ok(
    !partsOf(system, when).includes("data-home"),
    "a machine homed on the map was claimed here",
  );
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

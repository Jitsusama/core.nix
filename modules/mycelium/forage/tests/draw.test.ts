// What the forage page draws of a cell: which words it marks as its quests'
// own (a purpose and a document's title are written by the quest and shown as
// written, so the page's checks read them as the quest's, never as the page's
// own prose), and what its panel says of the cells it started.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { clockTime } from "../../../local-sites/kit/status.ts";
import { floorOf, forageRow, panelOf } from "../draw.ts";
import type { Forage } from "../model.ts";

/** A forage of kestrel's on `quest`, with `more` over its fields. */
function forage(more: Partial<Forage> = {}): Forage {
  return {
    cell: "KESTRL",
    machine: "kestrel",
    quest: "QEST-20261116-KESTRL",
    purpose: "Kestrel learns to hover over ~/.fields.",
    status: "active",
    documents: [
      {
        id: "PLAN-20261116-HOVERS",
        kind: "plan",
        title: "Hover Over the Field",
        stage: "build",
        ticked: 1,
        open: 1,
      },
    ],
    waits: [],
    startedBy: undefined,
    booked: false,
    parent: undefined,
    splitFrom: undefined,
    splitInto: [],
    serves: [],
    relies: [],
    depth: 0,
    ...more,
  };
}

/** The text of every element in `markup` marked as said by `quest`. */
function saidBy(markup: string, quest: string): readonly string[] {
  const marked = new RegExp(`<(\\w+)[^>]* data-said-by="${quest}"[^>]*>([^<]*)</\\1>`, "gu");
  return [...markup.matchAll(marked)].map((each) => each[2] ?? "");
}

it("marks a written purpose as its quest's words, in the row and the panel", () => {
  const written = forage();
  const words = ["Kestrel learns to hover over ~/.fields."];
  assert.deepEqual(saidBy(forageRow(written), "QEST-20261116-KESTRL"), words);
  const panel = panelOf(written, new Set(["KESTRL"]), []);
  assert.deepEqual(saidBy(panel, "QEST-20261116-KESTRL"), [...words, "Hover Over the Field"]);
});

it("leaves the page's own words unmarked when no purpose is written", () => {
  const unwritten = forage({ purpose: "" });
  assert.ok(!forageRow(unwritten).includes("data-said-by"), "the page's own words were marked");
  assert.deepEqual(saidBy(panelOf(unwritten, new Set(), []), "QEST-20261116-KESTRL"), [
    "Hover Over the Field",
  ]);
});

it("leaves a title unmarked when the page shows a document's id for want of one", () => {
  const document = { id: "PLAN-20261116-HOVERS", kind: "plan", title: "PLAN-20261116-HOVERS" };
  const untitled = forage({ documents: [{ ...document, stage: "build", ticked: 0, open: 0 }] });
  assert.deepEqual(saidBy(panelOf(untitled, new Set(), []), "QEST-20261116-KESTRL"), [
    "Kestrel learns to hover over ~/.fields.",
  ]);
});

/** What the panel says beside "Started". */
const STARTED = /<dt>Started<\/dt><dd><span>(.*?)<\/span><\/dd>/u;

/** The panel's waits section, its markup within. */
const WAITS_SECTION = /<section aria-labelledby="waits-title"[^>]*>([\s\S]*?)<\/section>/u;

/** Each run of space, folded to one. */
const SPACES = /\s+/gu;

/** What a row says of its quest's waits. */
const ROW_WAITS = /<span class="waits">([^<]*)<\/span>/u;

/**
 * What the panel says of `forage` beside "Started", given the cells it
 * started, each of whose starts the node's book holds unless `booked` says not.
 */
function startedLine(
  open: Forage,
  started: readonly string[],
  booked: (cell: string) => boolean = () => true,
): string {
  const starts = started.map((cell) => ({ cell, booked: booked(cell) }));
  const panel = panelOf(open, new Set([open.cell, ...started]), starts);
  return STARTED.exec(panel)?.[1] ?? "no Started line";
}

/** Any tag, to read a line's words without its markup. */
const TAG = /<[^>]+>/gu;

/** Each cell a line links to, by the address its press moves the cursor to. */
const LINK = /<a href="#cell=([^"]+)" data-cell="\1"><code>\1<\/code><\/a>/gu;

/** What a line says, and the cells it links to. */
function read(line: string): { readonly says: string; readonly links: readonly string[] } {
  return {
    says: line.replace(TAG, ""),
    links: [...line.matchAll(LINK)].map((each) => each[1] ?? ""),
  };
}

it("says a cell that started none started none yet", () => {
  assert.deepEqual(read(startedLine(forage(), [])), { says: "None here yet", links: [] });
});

it("links each cell a cell started to that cell's row", () => {
  assert.deepEqual(read(startedLine(forage(), ["OSPREY"])), {
    says: "One cell: OSPREY",
    links: ["OSPREY"],
  });
  const three = ["OSPREY", "SKIFFS", "TERNAA"];
  assert.deepEqual(read(startedLine(forage(), three)), {
    says: "3 cells: OSPREY, SKIFFS and TERNAA",
    links: three,
  });
});

/** A comma tied to the address before it, which the panel never breaks. */
const TIED = /<span class="tie"><a [^>]*><code>[^<]*<\/code><\/a>( \(as claimed\))?,<\/span>/gu;

/** `count` cells' addresses: AAAAAA, BBBBBB and on. */
function cells(count: number): readonly string[] {
  return [..."ABCDEFGHI"].slice(0, count).map((letter) => letter.repeat(6));
}

it("names up to seven cells a cell started, then names six and counts the rest", () => {
  assert.deepEqual(read(startedLine(forage(), cells(7))), {
    says: "7 cells: AAAAAA, BBBBBB, CCCCCC, DDDDDD, EEEEEE, FFFFFF and GGGGGG",
    links: cells(7),
  });
  assert.deepEqual(read(startedLine(forage(), cells(8))), {
    says: "8 cells: AAAAAA, BBBBBB, CCCCCC, DDDDDD, EEEEEE, FFFFFF and 2 more",
    links: cells(6),
  });
});

it("says the cells a cell started are only claimed unless the node's book holds each", () => {
  const three = ["OSPREY", "SKIFFS", "TERNAA"];
  assert.equal(
    read(startedLine(forage(), three, () => false)).says,
    "3 cells: OSPREY, SKIFFS and TERNAA, as claimed",
  );
  // Some claimed: each in brackets where it's named, and the count by how many of it are.
  const says = (started: readonly string[], claimed: readonly string[]) =>
    read(startedLine(forage(), started, (cell) => !claimed.includes(cell))).says;
  assert.equal(
    says(three, ["OSPREY", "TERNAA"]),
    "3 cells: OSPREY (as claimed), SKIFFS and TERNAA (as claimed)",
  );
  const six = "AAAAAA, BBBBBB, CCCCCC, DDDDDD, EEEEEE, FFFFFF";
  assert.equal(says(cells(8), ["HHHHHH"]), `8 cells: ${six} and 2 more (some as claimed)`);
  assert.equal(says(cells(8), ["GGGGGG", "HHHHHH"]), `8 cells: ${six} and 2 more (as claimed)`);
  assert.equal(says(cells(8), cells(8)), `8 cells: ${six} and 2 more, as claimed`);
  // Seven are all named, so a claimed seventh is said once, by its name.
  assert.equal(says(cells(7), ["GGGGGG"]), `7 cells: ${six} and GGGGGG (as claimed)`);
});

it("escapes an address the node gives before linking it", () => {
  const line = startedLine(forage(), ['<b a="1">&']);
  assert.ok(!line.includes("<b"), line);
  assert.ok(line.includes("#cell=%3Cb%20a%3D%221%22%3E%26"), line);
  assert.ok(line.includes('data-cell="&lt;b a=&quot;1&quot;&gt;&amp;"'), line);
});

it("ties each comma to the address before it, so no line starts with one", () => {
  // A comma follows each name but the two around "and"; eight name six.
  for (const [count, ties] of [
    [3, 1],
    [7, 5],
    [8, 5],
  ] as const) {
    // Every other start claimed, so a claimed name's comma is tied too.
    const line = startedLine(forage(), cells(count), (cell) => cell.charCodeAt(0) % 2 === 0);
    assert.ok(!line.replace(TIED, "").includes(","), `a comma stands loose in ${line}`);
    assert.equal(line.match(TIED)?.length, ties, line);
  }
});

/** A wait of kestrel's quest, with `more` over its fields. */
function wait(more: Partial<Forage["waits"][number]> = {}): Forage["waits"][number] {
  return {
    id: "field-sitting",
    title: "Watch the kestrel hover",
    why: "Only someone at the field can see it.",
    minutes: 30,
    when: "Wednesday or later",
    ready: false,
    ...more,
  };
}

/** What the panel's waits section says, without its markup, or none without one. */
function waitsSaid(panel: string): string | undefined {
  const section = WAITS_SECTION.exec(panel);
  return section?.[1]?.replace(TAG, " ").replace(SPACES, " ").trim();
}

it("says in the panel what a cell's quest waits on you for, first, in its own words", () => {
  const waiting = forage({
    waits: [
      wait(),
      wait({ id: "look", title: "Look", why: "", minutes: 1, when: undefined, ready: true }),
    ],
  });
  const panel = panelOf(waiting, new Set(["KESTRL"]), []);
  assert.equal(
    waitsSaid(panel),
    "What It Waits On You For Watch the kestrel hover Only someone at the field can see it. About 30 minutes · Wednesday or later · Not ready yet Look About a minute · Ready now",
  );
  assert.ok(
    panel.indexOf("waits-title") < panel.indexOf("docs-title"),
    "its waits come before its documents",
  );
  assert.deepEqual(saidBy(panel, "QEST-20261116-KESTRL"), [
    "Kestrel learns to hover over ~/.fields.",
    "Watch the kestrel hover",
    "Only someone at the field can see it.",
    "Wednesday or later",
    "Look",
    "Hover Over the Field",
  ]);
  assert.equal(waitsSaid(panelOf(forage(), new Set(), [])), undefined);
});

it("escapes what a wait says before drawing it", () => {
  const panel = panelOf(
    forage({
      quest: 'QEST-"><s>',
      waits: [wait({ title: '<b a="1">&', why: "<i>", when: "<u>" })],
    }),
    new Set(),
    [],
  );
  for (const tag of ["<b ", "<i>", "<u>", "<s>"]) {
    assert.ok(!panel.includes(tag), `${tag} in ${panel}`);
  }
  assert.ok(panel.includes("&lt;b a=&quot;1&quot;&gt;&amp;"), panel);
  assert.ok(panel.includes("&lt;u&gt;"), panel);
});

it("says how long a wait takes only in whole minutes, and only when it takes some", () => {
  const takes = (minutes: number | undefined): string | undefined =>
    waitsSaid(
      panelOf(forage({ waits: [wait({ minutes, when: undefined })] }), new Set(), []),
    )?.split("can see it. ")[1];
  assert.equal(takes(30), "About 30 minutes · Not ready yet");
  assert.equal(takes(2.4), "About 2 minutes · Not ready yet");
  assert.equal(takes(1), "About a minute · Not ready yet");
  for (const none of [0, -5, 0.2, undefined]) {
    assert.equal(takes(none), "Not ready yet", String(none));
  }
});

it("says in a cell's row how many things its quest waits on you for", () => {
  const said = (waits: number): string | undefined =>
    ROW_WAITS.exec(
      forageRow(
        forage({ waits: Array.from({ length: waits }, (_, at) => wait({ id: `w${at}` })) }),
      ),
    )?.[1];
  assert.equal(said(0), undefined);
  assert.equal(said(1), "Waits on you for one thing");
  assert.equal(said(2), "Waits on you for 2 things");
});

it("says the words are a quest's when the record names none", () => {
  assert.deepEqual(saidBy(forageRow(forage({ quest: "" })), "its quest"), [
    "Kestrel learns to hover over ~/.fields.",
  ]);
});

it("draws no joins on the floor and says none in the panel when the node sends none", () => {
  const panel = panelOf(forage(), new Set(["KESTRL"]), []);
  for (const label of ["Split from", "Split into", "Serves", "Relies on"]) {
    assert.ok(!panel.includes(`<dt>${label}</dt>`), label);
  }
  assert.ok(!floorOf([forage()], null, "kestrel").includes('class="join"'));
});

/** The panel's line for the cell TERNAA split from, linked to its row. */
const SPLIT_FROM =
  /<dt>Split from<\/dt><dd><a href="#cell=TERNAA" data-cell="TERNAA">.*?<\/a>, as claimed<\/dd>/u;

/** The panel's line for the two cells a cell split into, one here and one away. */
const SPLIT_INTO =
  /<dt>Split into<\/dt><dd><span>2 cells: <a href="#cell=TERNAA"[^>]*><code>TERNAA<\/code><\/a> and <span><code>FARAWAY<\/code>, not standing here<\/span>, as claimed<\/span>/u;

/** The panel's first need a cell serves, linked to the row of OSPREY, which asked it. */
const SERVES = /<dt>Serves<\/dt><dd class="needs"><span class="need"><a href="#cell=OSPREY"/u;

/** A need relied on of a cell not standing here, said with what it was for. */
const AWAY_FOR = /<code>FARAWAY<\/code>, not standing here<\/span>, for /u;

/** The floor's arc from the first cell on it to the second, at 30 apart. */
const JOIN_FROM_FIRST_TO_SECOND = /<path class="join" d="M39 84Q54 \d+ 69 84"\/>/u;

it("says a need taken today by the clock alone", () => {
  const now = Date.parse("2026-11-18T15:00:00");
  const since = new Date(now - 60_000).toISOString();
  const panel = panelOf(
    forage({ serves: [{ cell: "OSPREY", need: "", since }] }),
    new Set(["KESTRL"]),
    [],
    now,
  );
  assert.ok(panel.includes(`">${clockTime(Date.parse(since))}</time>`), panel);
});

it("escapes an address not standing here, and names six needs before counting the rest", () => {
  const away = "<b>FAR</b>";
  const serves = Array.from({ length: 9 }, (_, at) => ({
    cell: at === 0 ? away : `CELL0${at}`,
    need: "",
    since: undefined,
  }));
  const panel = panelOf(forage({ serves }), new Set(["KESTRL"]), []);
  assert.ok(panel.includes("<code>&lt;b&gt;FAR&lt;/b&gt;</code>, not standing here"));
  assert.ok(!panel.includes(away));
  assert.equal(panel.match(/class="need"/gu)?.length, 7);
  assert.ok(panel.includes('<span class="need">and 3 more</span>'));
});

it("says the cell a cell split from and the cells it split into, linked when here", () => {
  const here = new Set(["KESTRL", "TERNAA"]);
  const split = panelOf(
    forage({ splitFrom: "TERNAA", splitInto: ["TERNAA", "FARAWAY"] }),
    here,
    [],
  );
  assert.match(split, SPLIT_FROM);
  assert.match(split, SPLIT_INTO);
});

it("says each need a cell serves and relies on, in the asking cell's words, escaped", () => {
  const here = new Set(["KESTRL", "OSPREY"]);
  const panel = panelOf(
    forage({
      serves: [{ cell: "OSPREY", need: "a <b>review</b>", since: "2025-11-16T13:05:00Z" }],
      relies: [{ cell: "FARAWAY", need: "a plan", since: "not a time" }],
    }),
    here,
    [],
  );
  assert.match(panel, SERVES);
  assert.deepEqual(saidBy(panel, "OSPREY"), ["a &lt;b&gt;review&lt;/b&gt;"]);
  // Taken on another day, so the day is said beside the clock.
  const at = Date.parse("2025-11-16T13:05:00Z");
  const day = `${new Date(at).getDate()} Nov, ${clockTime(at)}`;
  assert.ok(panel.includes(`, since <time datetime="2025-11-16T13:05:00.000Z">${day}</time>`));
  assert.ok(saidBy(panel, "QEST-20261116-KESTRL").includes("a plan"));
  assert.match(panel, AWAY_FOR);
  assert.ok(!panel.includes("not a time"));
});

it("draws one dotted arc for each pair a need joins on the floor, and a branch for a split", () => {
  const floor = floorOf(
    [
      forage({ cell: "KESTRL", serves: [{ cell: "OSPREY", need: "", since: undefined }] }),
      forage({ cell: "OSPREY", relies: [{ cell: "KESTRL", need: "", since: undefined }] }),
      forage({ cell: "TERNAA", splitFrom: "KESTRL", depth: 1 }),
    ],
    null,
    "kestrel",
  );
  assert.equal(floor.match(/class="join"/gu)?.length, 1);
  // It runs from the cell relied on, first on the floor, to the one relying, next.
  assert.match(floor, JOIN_FROM_FIRST_TO_SECOND);
  // The main line, then the split's branch, dashed since a split is only claimed.
  assert.equal(floor.match(/class="hypha"/gu)?.length, 1);
  assert.equal(floor.match(/class="hypha claimed"/gu)?.length, 1);
});

it("draws a start the node's book holds solid, and one only claimed dashed", () => {
  const floor = (startedBy: string, booked: boolean) =>
    floorOf(
      [
        forage({ cell: "KESTRL" }),
        forage({ cell: "TERNAA", startedBy, booked, splitFrom: "KESTRL", depth: 1 }),
      ],
      null,
      "kestrel",
    );
  assert.equal(floor("KESTRL", true).match(/class="hypha"/gu)?.length, 2);
  assert.ok(!floor("KESTRL", true).includes("claimed"));
  assert.equal(floor("KESTRL", false).match(/class="hypha claimed"/gu)?.length, 1);
  // Its booked starter isn't drawn, so its branch is the split, which is only claimed.
  assert.equal(floor("FARAWAY", true).match(/class="hypha claimed"/gu)?.length, 1);
});

/** The panel's Started by, told as only claimed. */
const STARTED_CLAIMED = /<dt>Started by<\/dt><dd>.*?<\/a>, as claimed<\/dd>/u;

/** The panel's Started by, told as only claimed, of a starter not drawn here. */
const STARTED_AWAY_CLAIMED =
  /<dt>Started by<\/dt><dd><span><code>FARAWAY<\/code>, not standing here<\/span>, as claimed<\/dd>/u;

/** The panel's Started by, told plainly since the node's book holds it. */
const STARTED_BOOKED = /<dt>Started by<\/dt><dd><a [^>]*><code>KESTRL<\/code><\/a><\/dd>/u;

it("says who started a cell as only claimed unless the node's book holds it", () => {
  const here = new Set(["KESTRL"]);
  const said = (booked: boolean) =>
    panelOf(forage({ cell: "TERNAA", startedBy: "KESTRL", booked }), here, []);
  assert.match(said(false), STARTED_CLAIMED);
  const away = (booked: boolean) =>
    panelOf(forage({ cell: "TERNAA", startedBy: "FARAWAY", booked }), here, []);
  assert.match(away(false), STARTED_AWAY_CLAIMED);
  // The book holds a start whatever machine its starter stands on.
  assert.ok(!away(true).includes("as claimed"));
  assert.match(said(true), STARTED_BOOKED);
});

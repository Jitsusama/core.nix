// Which words the forage page marks as its quests' own: a purpose and a
// document's title are written by the quest and shown as written, so the
// page's checks read them as the quest's, never as the page's own prose.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { forageRow, panelOf } from "../draw.ts";
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
    startedBy: undefined,
    parent: undefined,
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

/** What the panel says of `forage` beside "Started", given the cells it started. */
function startedLine(open: Forage, started: readonly string[]): string {
  const panel = panelOf(open, new Set([open.cell, ...started]), started);
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
  assert.deepEqual(read(startedLine(forage(), [])), { says: "None yet", links: [] });
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

it("counts the cells a cell started past the six it names", () => {
  const nine = [..."ABCDEFGHI"].map((letter) => letter.repeat(6));
  assert.deepEqual(read(startedLine(forage(), nine)), {
    says: "9 cells: AAAAAA, BBBBBB, CCCCCC, DDDDDD, EEEEEE, FFFFFF and 3 more, each set in under it in the list",
    links: nine.slice(0, 6),
  });
});

it("ties each comma to the address before it, so no line starts with one", () => {
  const line = startedLine(forage(), ["OSPREY", "SKIFFS", "TERNAA"]);
  assert.ok(!line.includes("</a>, "), `a comma stands loose in ${line}`);
  assert.equal(line.match(/<span class="tie">[^,]*,<\/span>/gu)?.length, 1, line);
});

it("says the words are a quest's when the record names none", () => {
  assert.deepEqual(saidBy(forageRow(forage({ quest: "" })), "its quest"), [
    "Kestrel learns to hover over ~/.fields.",
  ]);
});

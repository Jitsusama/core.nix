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
  const panel = panelOf(written, new Set(["KESTRL"]));
  assert.deepEqual(saidBy(panel, "QEST-20261116-KESTRL"), [...words, "Hover Over the Field"]);
});

it("leaves the page's own words unmarked when no purpose is written", () => {
  const unwritten = forage({ purpose: "" });
  assert.ok(!forageRow(unwritten).includes("data-said-by"), "the page's own words were marked");
  assert.deepEqual(saidBy(panelOf(unwritten, new Set()), "QEST-20261116-KESTRL"), [
    "Hover Over the Field",
  ]);
});

it("leaves a title unmarked when the page shows a document's id for want of one", () => {
  const document = { id: "PLAN-20261116-HOVERS", kind: "plan", title: "PLAN-20261116-HOVERS" };
  const untitled = forage({ documents: [{ ...document, stage: "build", ticked: 0, open: 0 }] });
  assert.deepEqual(saidBy(panelOf(untitled, new Set()), "QEST-20261116-KESTRL"), [
    "Kestrel learns to hover over ~/.fields.",
  ]);
});

it("says the words are a quest's when the record names none", () => {
  assert.deepEqual(saidBy(forageRow(forage({ quest: "" })), "its quest"), [
    "Kestrel learns to hover over ~/.fields.",
  ]);
});

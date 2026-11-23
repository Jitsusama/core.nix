// Which cells the forage page's list shows, and where its cursor rests on
// them as finds, views and records change them.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { floorFor, floorOf, forageRow } from "../draw.ts";
import {
  cursorAfter,
  type Forage,
  foragesOf,
  headline,
  inView,
  lede,
  shownBy,
  unwritten,
} from "../model.ts";
import { forage, told } from "./records.ts";

// Three cells, OSPREY with no purpose written, as the page's own records hold them.
const VIEWED = foragesOf(
  told([forage("KESTRL"), forage("OSPREY", { purpose: "" }), forage("SKIFFS")]),
).forages;

/** The cells the list shows of `forages` for the find `sought` in its view, by address. */
const cellsOf = (forages: readonly Forage[], sought: string, hollowOnly: boolean): string[] =>
  shownBy(forages, sought, hollowOnly).map((each) => each.cell);

it("the list shows what the find holds in its view, every cell or those without purpose", () => {
  assert.deepEqual(cellsOf(VIEWED, "", false), ["KESTRL", "OSPREY", "SKIFFS"]);
  assert.deepEqual(cellsOf(VIEWED, "", true), ["OSPREY"]);
  assert.deepEqual(cellsOf(VIEWED, "sk", false), ["SKIFFS"]);
  assert.deepEqual(cellsOf(VIEWED, "skiffs", true), []);
  const [kestrel, osprey] = VIEWED;
  assert.ok(kestrel && osprey);
  assert.equal(inView(kestrel, false), true);
  assert.equal(inView(osprey, false), true);
  assert.equal(inView(kestrel, true), false);
  assert.equal(inView(osprey, true), true);
});

it("a purpose showing nothing is unwritten to the view, headline, lede, row and floor", () => {
  // OSPREY's purpose is spaces and breaks, and SKIFFS's marks that take no room.
  const blank = foragesOf(
    told([
      forage("KESTRL"),
      forage("OSPREY", { purpose: " \n\t" }),
      forage("SKIFFS", { purpose: "\u200b\u2060\u00ad" }),
    ]),
  ).forages;
  const unwrittenCells = blank.filter(unwritten).map((each) => each.cell);
  assert.deepEqual(unwrittenCells, ["OSPREY", "SKIFFS"]);
  assert.deepEqual(cellsOf(blank, "", true), unwrittenCells);
  assert.equal(headline(blank).without, unwrittenCells.length);
  assert.ok(lede(blank).startsWith("OSPREY and SKIFFS have no purpose"), lede(blank));
  const floor = floorOf(floorFor(blank, null), null, "kestrel");
  for (const each of blank.filter(unwritten)) {
    const row = forageRow(each);
    assert.ok(row.includes("No purpose written yet."), row);
    assert.ok(!row.includes("data-said-by"), row);
    assert.ok(floor.includes(`<rect class="hollow" data-cell="${each.cell}"`), floor);
  }
});

it("the cursor stays while the list shows it, else goes to its first row", () => {
  assert.equal(cursorAfter(VIEWED, "", false, "SKIFFS"), "SKIFFS");
  assert.equal(cursorAfter(VIEWED, "sk", false, "SKIFFS"), "SKIFFS");
  // A find that hides the cursor's cell moves it to the find's first match.
  assert.equal(cursorAfter(VIEWED, "sk", false, "KESTRL"), "SKIFFS");
  // The view of cells without purpose moves it to the cell it shows.
  assert.equal(cursorAfter(VIEWED, "", true, "KESTRL"), "OSPREY");
  // No cursor rests on the list's first row.
  assert.equal(cursorAfter(VIEWED, "", false, null), "KESTRL");
});

it("a find matching nothing keeps the cursor while its cell lasts and the view takes it in", () => {
  assert.equal(cursorAfter(VIEWED, "zzq", false, "KESTRL"), "KESTRL");
  // In the view of cells without purpose too, so ending the find there finds it again.
  assert.equal(cursorAfter(VIEWED, "zzq", true, "OSPREY"), "OSPREY");
  // A cell that ends meanwhile takes the cursor with it.
  const ended = VIEWED.filter((each) => each.cell !== "KESTRL");
  assert.equal(cursorAfter(ended, "zzq", false, "KESTRL"), null);
  // So does every cell ending, find or not.
  assert.equal(cursorAfter([], "", false, "KESTRL"), null);
  assert.equal(cursorAfter([], "zzq", false, "KESTRL"), null);
  // With no cursor, an empty list keeps none.
  assert.equal(cursorAfter(VIEWED, "zzq", false, null), null);
});

it("a cell the view of cells without purpose no longer takes in takes the cursor", () => {
  // A record writes OSPREY's purpose while a find matching nothing hides it.
  const written = foragesOf(told([forage("KESTRL"), forage("OSPREY"), forage("SKIFFS")])).forages;
  assert.equal(cursorAfter(written, "zzq", true, "OSPREY"), null);
  // And while the find matches it, or there's no find, with nothing else in the view.
  assert.equal(cursorAfter(written, "osp", true, "OSPREY"), null);
  assert.equal(cursorAfter(written, "", true, "OSPREY"), null);
  // Opening that view while a find matching nothing holds a cell with purpose drops it too.
  assert.equal(cursorAfter(VIEWED, "zzq", true, "KESTRL"), null);
});

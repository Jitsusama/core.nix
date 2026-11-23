// Which cells the forage page's list shows, and where its cursor rests on
// them as finds, views and records change them.

import assert from "node:assert/strict";
import { test as it } from "node:test";
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

/** Where the cursor on `cursor` rests once the list shows what `sought` finds in its view. */
const restsOn = (
  forages: readonly Forage[],
  cursor: string | null,
  sought: string,
  hollowOnly: boolean,
): string | null => cursorAfter(forages, sought, hollowOnly, cursor);

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

it("the view of cells without purpose takes in the cells the headline and lede count", () => {
  const unwrittenCells = VIEWED.filter(unwritten).map((each) => each.cell);
  assert.deepEqual(unwrittenCells, ["OSPREY"]);
  assert.deepEqual(cellsOf(VIEWED, "", true), unwrittenCells);
  assert.equal(headline(VIEWED).without, unwrittenCells.length);
  assert.ok(lede(VIEWED).startsWith("OSPREY has no purpose written yet."), lede(VIEWED));
});

it("the cursor stays while the list shows it, else goes to its first row", () => {
  assert.equal(restsOn(VIEWED, "SKIFFS", "", false), "SKIFFS");
  assert.equal(restsOn(VIEWED, "SKIFFS", "sk", false), "SKIFFS");
  // A find that hides the cursor's cell moves it to the find's first match.
  assert.equal(restsOn(VIEWED, "KESTRL", "sk", false), "SKIFFS");
  // The view of cells without purpose moves it to the cell it shows.
  assert.equal(restsOn(VIEWED, "KESTRL", "", true), "OSPREY");
  // No cursor rests on the list's first row.
  assert.equal(restsOn(VIEWED, null, "", false), "KESTRL");
});

it("a find matching nothing keeps the cursor while its cell lasts and the view takes it in", () => {
  assert.equal(restsOn(VIEWED, "KESTRL", "zzq", false), "KESTRL");
  // In the view of cells without purpose too, so ending the find there finds it again.
  assert.equal(restsOn(VIEWED, "OSPREY", "zzq", true), "OSPREY");
  // A cell that ends meanwhile takes the cursor with it.
  const ended = VIEWED.filter((each) => each.cell !== "KESTRL");
  assert.equal(restsOn(ended, "KESTRL", "zzq", false), null);
  // So does every cell ending, find or not.
  assert.equal(restsOn([], "KESTRL", "", false), null);
  assert.equal(restsOn([], "KESTRL", "zzq", false), null);
  // With no cursor, an empty list keeps none.
  assert.equal(restsOn(VIEWED, null, "zzq", false), null);
});

it("a cell the view of cells without purpose no longer takes in takes the cursor", () => {
  // A record writes OSPREY's purpose while a find matching nothing hides it.
  const written = foragesOf(told([forage("KESTRL"), forage("OSPREY"), forage("SKIFFS")])).forages;
  assert.equal(restsOn(written, "OSPREY", "zzq", true), null);
  // And while the find matches it, or there's no find, with nothing else in the view.
  assert.equal(restsOn(written, "OSPREY", "osp", true), null);
  assert.equal(restsOn(written, "OSPREY", "", true), null);
  // Opening that view while a find matching nothing holds a cell with purpose drops it too.
  assert.equal(restsOn(VIEWED, "KESTRL", "zzq", true), null);
});

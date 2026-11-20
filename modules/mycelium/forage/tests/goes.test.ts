// Where the open cell's panel goes: to the map on Return, and to the cell's
// Talk in the launcher by the link that says the machine it stands on.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { panelOf } from "../draw.ts";
import { type Forage, foragesOf } from "../model.ts";
import { forage, told } from "./records.ts";

/** Each link in a panel to talk to a cell, by where it goes. */
const TALK = /<a href="(mycelium:cell\/[^"]*)">/gu;

/** The one cell `record` is read as, as the page reads it. */
function readAs(record: { readonly key: string }): Forage {
  const [read] = foragesOf(told([record])).forages;
  assert.ok(read !== undefined, "the record was read as no cell");
  return read;
}

/** The links to talk to a cell the panel offers, by where each goes. */
function talks(panel: string): readonly string[] {
  return [...panel.matchAll(TALK)].map((each) => each[1] ?? "");
}

it("the panel talks to its cell by the link that says the machine it stands on", () => {
  const panel = panelOf(readAs(forage("KESTRL")), new Set(), []);
  assert.deepEqual(talks(panel), ["mycelium:cell/KESTRL@kestrel"], panel);
  assert.ok(panel.includes("on the map, or <a href="), panel);
  assert.ok(panel.includes(">talk to it in the Launcher</a>.</p>"), panel);
  const placeless = panelOf(readAs(forage("KESTRL", { machine: "" })), new Set(), []);
  assert.deepEqual(talks(placeless), ["mycelium:cell/KESTRL"], placeless);
});

it("a cell the Launcher can't reach is opened on the map and offered no talk", () => {
  const panel = panelOf(readAs(forage("not-an-address")), new Set(), []);
  assert.deepEqual(talks(panel), [], panel);
  assert.ok(panel.includes("<code>not-an-address</code> on the map.</p>"), panel);
});

// How the page leads a machine to the map when the daemon shows it by its
// name made plain: the map knows each machine by the node's own name, which
// a record carries as called where the two differ, so every link there goes
// by that one while the page still shows the plain one.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { partsOf } from "../draw.ts";
import { looksOf } from "../looks.ts";
import { type System, systemOf } from "../model.ts";

const summary = {
  key: "summary",
  kind: "summary",
  machine: "kestrel",
  read_at: "2026-11-28T04:00:00Z",
  health: "bad",
  parts: ["machines"],
};

const machines = {
  key: "part:machines",
  kind: "part",
  part: "machines",
  health: "bad",
  says: "",
  side: [],
  holds: "3 machines",
  things: ["machine:gull", "machine:kestrel", "machine:kite"],
  deeper: { command: "mycelium machines", site: "mycelium/", launcher: "Agents" },
};

/** A machine's record, as the daemon tells one, in reach or not as `reached` says. */
function machine(name: string, health: string, side: string[], reached = true) {
  return { key: `machine:${name}`, kind: "machine", name, health, side, ties: { reached } };
}

const gull = { ...machine("gull", "bad", ["out of reach"], false), called: "gull\u200b" };
const kestrel = machine("kestrel", "watch", ["on battery at 12%"]);
const kite = { ...machine("kite", "watch", ["on battery at 9%"]), called: "kite\u200b" };

/** The system these records hold. */
function systemOfThese(): System {
  const told = (these: readonly object[]) => {
    const held = new Held();
    held.took({ records: these });
    return held;
  };
  const system = systemOf(told([summary]), told([machines]), told([gull, kestrel, kite]));
  assert.ok(system, "the summary is told");
  return system;
}

/** Every link `markup` holds, in the order it holds them. */
function hrefsOf(markup: string): readonly string[] {
  return [...markup.matchAll(/href="(?<href>[^"]*)"/gu)].map((each) => each.groups?.["href"] ?? "");
}

it("leads a machine past those drawn, shown by a plain name, to the map by the node's", () => {
  const kites = looksOf(systemOfThese(), 1).looks.filter((look) => look.whose === "kite");
  assert.deepEqual(
    kites.map((look) => look.href),
    ["../mycelium/#machine=kite%E2%80%8B"],
    "the look is said by the plain name but led to the map by the node's",
  );
});

it("leads a machine drawn, shown by a plain name, to the map by the node's own", () => {
  const drawn = partsOf(systemOfThese(), (at) => `at ${at}`);
  // Kite is the last drawn whole, so its section runs on to those out of reach.
  const [machinesDrawn, rest] = drawn.split('<section class="away ');
  const [, kiteDrawn] = (machinesDrawn ?? "").split('<section class="machine ').slice(1);
  assert.ok(
    kiteDrawn?.includes('<span translate="no">kite</span>'),
    "kite stopped showing plainly",
  );
  assert.deepEqual(
    hrefsOf(kiteDrawn ?? "").filter((href) => href.startsWith("../mycelium/")),
    ["../mycelium/#machine=kite%E2%80%8B", "../mycelium/#machine=kite%E2%80%8B"],
    "kite's name and its cells weren't led to the map by the node's name",
  );
  const [away] = (rest ?? "").split("</section>");
  assert.ok(away?.includes('<span translate="no">gull</span>'), "gull stopped showing plainly");
  assert.deepEqual(
    hrefsOf(away ?? ""),
    ["../mycelium/#machine=gull%E2%80%8B"],
    "a machine out of reach wasn't led to the map by the node's name",
  );
});

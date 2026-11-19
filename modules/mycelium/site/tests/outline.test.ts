// The network as a tree: what hangs from what and in which order, the top
// with machines shown and without, the rows an open set of branches shows,
// and where each of the tree's keys goes from a row, edges included. The
// orders and steps are written out whole rather than read from outline.ts.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { EVERY_TREE_ACT } from "../keys.ts";
import {
  EVERY_TREE_STEP,
  hasKids,
  kidsOf,
  openedTo,
  type Row,
  rootsOf,
  rowsOf,
  rowWords,
  upOf,
  walked,
} from "../outline.ts";
import type { Cell, Network, Reading } from "../words.ts";

/** A cell, quiet with nothing waiting, but for what `more` says. */
const cell = (address: string, more: Partial<Cell> = {}): Cell => ({
  address,
  short: address,
  standing: "quiet",
  owes: [],
  holds: [],
  machine: "kestrel",
  ...more,
});

// Kestrel carries two networks, Harbour (with a branch, Nets) and Cistern;
// a cell works alone; osprey carries nothing.
const tide = cell("TIDE", { short: "Tide", standing: "quiet" });
const gale = cell("GALE", { short: "Gale", standing: "asks" });
const eddy = cell("EDDY", { short: "Eddy", standing: "quiet", usage: { cost_today: 2 } });
const reed = cell("REED", { short: "Reed", standing: "stuck" });
const lone = cell("LONE", { short: "Lone" });
const nets: Network = {
  key: "quest.nets",
  name: "Nets",
  within: "quest.harbour",
  children: [],
  cells: [reed],
};
const harbour: Network = {
  key: "quest.harbour",
  name: "Harbour",
  within: null,
  machine: "kestrel",
  children: [nets],
  cells: [tide, gale, eddy],
};
const cistern: Network = {
  key: "quest.cistern",
  name: "Cistern",
  within: null,
  machine: "kestrel",
  children: [],
  cells: [],
};
const alone: Network = {
  key: "#loose",
  name: "Working alone",
  within: null,
  children: [],
  cells: [lone],
};
const homes: [Cell, Network][] = [
  [tide, harbour],
  [gale, harbour],
  [eddy, harbour],
  [reed, nets],
];
const cells = [tide, gale, eddy, reed, lone];
const now: Reading = {
  hubs: new Map([harbour, nets, cistern, alone].map((h) => [h.key, h])),
  byAddress: new Map([
    ...homes.map(([s, hub]): [string, Cell] => [s.address, { ...s, hub }]),
    ["LONE", { ...lone, hub: null, loose: alone }],
  ]),
  machines: [
    { name: "kestrel", state: "ready" },
    { name: "osprey", state: "ready" },
  ],
  cells,
};
const hanging = {
  kids: (key: string) => kidsOf(now, key),
  any: (key: string) => hasKids(now, key),
};

/** A row's mark: - open, + closed, . nothing to open. */
const mark = (open: boolean | null): string => {
  if (open === null) {
    return ".";
  }
  return open ? "-" : "+";
};

/** Each row's mark, key and place, indented two spaces a level. */
const drawn = (shown: readonly Row[]): string[] =>
  shown.flatMap((r) => [
    `${"  ".repeat(r.level - 1)}${mark(r.open)} ${r.key} ${r.at}/${r.of}`,
    ...drawn(r.kids),
  ]);

it("lists networks by name, and a network's branches before its cells, worst first", () => {
  assert.deepEqual(
    kidsOf(now, "m:kestrel"),
    ["h:quest.cistern", "h:quest.harbour"],
    "networks by name, no branch among them",
  );
  assert.deepEqual(
    kidsOf(now, "h:quest.harbour"),
    ["h:quest.nets", "c:GALE", "c:EDDY", "c:TIDE"],
    "the branch first, then the cell that asks, then the quiet by name",
  );
  assert.deepEqual(kidsOf(now, "m:osprey"), [], "a machine carrying nothing");
  assert.deepEqual(kidsOf(now, "c:TIDE"), [], "nothing hangs from a cell");
  assert.deepEqual(kidsOf(now, "h:gone"), [], "nor from a network gone");
});

it("knows what has anything under it", () => {
  assert.equal(hasKids(now, "h:quest.harbour"), true, "a network with cells");
  assert.equal(hasKids(now, "h:quest.cistern"), false, "a network with none");
  assert.equal(hasKids(now, "m:kestrel"), true, "a machine carrying networks");
  assert.equal(hasKids(now, "m:osprey"), false, "a machine carrying none");
  assert.equal(hasKids(now, "c:TIDE"), false, "a cell");
  assert.equal(hasKids(now, "h:gone"), false, "a network gone");
});

it("says what each thing hangs from, reaching machines only while they're shown", () => {
  assert.equal(upOf(now, "c:REED", true), "h:quest.nets", "a cell its network");
  assert.equal(upOf(now, "c:LONE", true), "h:#loose", "a cell working alone where it works alone");
  assert.equal(upOf(now, "h:quest.nets", false), "h:quest.harbour", "a branch its network");
  assert.equal(
    upOf(now, "h:quest.harbour", true),
    "m:kestrel",
    "a network its machine while shown",
  );
  assert.equal(upOf(now, "h:quest.harbour", false), null, "nothing while machines are hidden");
  assert.equal(upOf(now, "h:#loose", true), null, "cells working alone are on no machine");
  assert.equal(upOf(now, "m:kestrel", true), null, "a machine is the top");
  assert.equal(upOf(now, "c:GONE", true), null, "a cell gone hangs from nothing");
  assert.equal(upOf(now, "h:gone", true), null, "a network gone hangs from nothing");
  assert.deepEqual(
    openedTo("c:REED", (key) => upOf(now, key, true)),
    ["h:quest.nets", "h:quest.harbour", "m:kestrel"],
    "everything above, nearest first",
  );
  assert.deepEqual(
    openedTo("m:kestrel", (key) => upOf(now, key, true)),
    [],
    "nothing above the top",
  );
});

it("tops the tree with the machines while shown, else every network that is no branch", () => {
  assert.deepEqual(rootsOf(now, true), ["m:kestrel", "m:osprey"], "the machines");
  assert.deepEqual(
    rootsOf(now, false),
    ["h:quest.harbour", "h:quest.cistern", "h:#loose"],
    "networks as the reading holds them",
  );
  assert.deepEqual(
    rootsOf({ ...now, machines: [] }, true),
    ["h:quest.harbour", "h:quest.cistern", "h:#loose"],
    "networks where no machine is known",
  );
});

it("shows only the rows of open branches, each with its level and place", () => {
  assert.deepEqual(
    drawn(rowsOf(rootsOf(now, true), new Set(["m:kestrel"]), hanging)),
    ["- m:kestrel 1/2", "  . h:quest.cistern 1/2", "  + h:quest.harbour 2/2", ". m:osprey 2/2"],
    "the machine open, its networks closed",
  );
  assert.deepEqual(
    drawn(
      rowsOf(
        rootsOf(now, false),
        new Set(["h:quest.harbour", "h:quest.nets", "m:kestrel"]),
        hanging,
      ),
    ),
    [
      "- h:quest.harbour 1/3",
      "  - h:quest.nets 1/4",
      "    . c:REED 1/1",
      "  . c:GALE 2/4",
      "  . c:EDDY 3/4",
      "  . c:TIDE 4/4",
      ". h:quest.cistern 2/3",
      "+ h:#loose 3/3",
    ],
    "a branch open inside its network; a closed one shows nothing under it",
  );
  assert.deepEqual(
    drawn(rowsOf(["h:quest.cistern"], new Set(["h:quest.cistern"]), hanging)),
    [". h:quest.cistern 1/1"],
    "opening what has nothing under it opens nothing",
  );
});

it("shows each row's name and the words under it, a machine's name set apart", () => {
  assert.deepEqual(
    rowWords(now, "c:GALE"),
    { name: "Gale", fact: "needs you", mono: false },
    "a cell",
  );
  assert.deepEqual(
    rowWords(now, "h:quest.harbour"),
    {
      name: "Harbour",
      fact: "3 cells: 1 needs you, 2 quiet, $2.00 today with its branches",
      mono: false,
    },
    "a network",
  );
  assert.deepEqual(
    rowWords(now, "m:kestrel"),
    { name: "kestrel", fact: "ready, 5 cells, $2.00 today", mono: true },
    "a machine",
  );
  assert.deepEqual(
    rowWords(now, "c:GONE"),
    { name: "GONE", fact: "", mono: false },
    "a cell gone by its address",
  );
  assert.deepEqual(
    rowWords(now, "h:gone"),
    { name: "gone", fact: "", mono: false },
    "a network gone by its key",
  );
});

// Kestrel open with Harbour open in it and Nets closed: the rows read
// kestrel, Cistern, Harbour, Nets, Gale, Eddy, Tide, then osprey.
const rows = rowsOf(rootsOf(now, true), new Set(["m:kestrel", "h:quest.harbour"]), hanging);

it("walks down and up the rows as they read, stopping at either end", () => {
  assert.deepEqual(
    walked("next", rows, "h:quest.harbour"),
    { step: "focus", key: "h:quest.nets" },
    "into what it shows",
  );
  assert.deepEqual(
    walked("next", rows, "c:TIDE"),
    { step: "focus", key: "m:osprey" },
    "out past a branch's last row",
  );
  assert.deepEqual(
    walked("prev", rows, "m:osprey"),
    { step: "focus", key: "c:TIDE" },
    "back up into a branch",
  );
  assert.deepEqual(
    walked("prev", rows, "h:quest.cistern"),
    { step: "focus", key: "m:kestrel" },
    "up to what holds it",
  );
  assert.equal(walked("next", rows, "m:osprey"), null, "nothing past the last row");
  assert.equal(walked("prev", rows, "m:kestrel"), null, "nothing before the first");
  assert.deepEqual(
    walked("first", rows, "c:EDDY"),
    { step: "focus", key: "m:kestrel" },
    "the first row",
  );
  assert.deepEqual(
    walked("last", rows, "c:EDDY"),
    { step: "focus", key: "m:osprey" },
    "the last row",
  );
});

it("opens or steps into a branch going in, and closes or steps out of one going out", () => {
  assert.deepEqual(
    walked("in", rows, "h:quest.nets"),
    { step: "expand", key: "h:quest.nets" },
    "a closed branch opens",
  );
  assert.deepEqual(
    walked("in", rows, "h:quest.harbour"),
    { step: "focus", key: "h:quest.nets" },
    "an open one steps in",
  );
  assert.equal(walked("in", rows, "c:GALE"), null, "nothing to go into in a cell");
  assert.equal(
    walked("in", rows, "h:quest.cistern"),
    null,
    "nor in a network with nothing under it",
  );
  assert.deepEqual(
    walked("out", rows, "h:quest.harbour"),
    { step: "collapse", key: "h:quest.harbour" },
    "an open one closes",
  );
  assert.deepEqual(
    walked("out", rows, "c:GALE"),
    { step: "focus", key: "h:quest.harbour" },
    "a cell steps out to its network",
  );
  assert.deepEqual(
    walked("out", rows, "h:quest.nets"),
    { step: "focus", key: "h:quest.harbour" },
    "a closed branch steps out",
  );
  assert.equal(walked("out", rows, "m:osprey"), null, "nothing above the top");
});

it("opens what it's on, and goes nowhere from a row the tree no longer shows", () => {
  assert.deepEqual(
    walked("open", rows, "c:GALE"),
    { step: "open", key: "c:GALE" },
    "Return opens the cell",
  );
  assert.deepEqual(
    walked("open", rows, "m:osprey"),
    { step: "open", key: "m:osprey" },
    "and a machine",
  );
  assert.equal(walked("next", rows, "c:REED"), null, "a row under a closed branch isn't shown");
});

it("answers every act a tree key can ask with a step the drawing takes", () => {
  const steps = new Set<string>(EVERY_TREE_STEP);
  for (const act of EVERY_TREE_ACT) {
    for (const r of ["m:kestrel", "h:quest.harbour", "h:quest.nets", "c:GALE", "m:osprey"]) {
      const step = walked(act, rows, r);
      assert.ok(
        step === null || steps.has(step.step),
        `${act} from ${r} came to ${JSON.stringify(step)}`,
      );
    }
  }
});

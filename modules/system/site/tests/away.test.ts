// How the page reads when many machines are out of reach at once, as on
// 23 November 2026, when six cloud machines already gone still read as
// unreachable beside eight that carried the work, and the page led with
// them: their dead piers on the bridge, their empty sections first, a look
// each. Those out of reach are named together, after the rest, and the
// page leads with the few that need Joel.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { landingOf } from "../address.ts";
import { bridgeOf } from "../bridge.ts";
import { awayOf, looksDrawn, MACHINES_DRAWN, partsOf, placesOf, reachedOf } from "../draw.ts";
import { looksOf } from "../looks.ts";
import { machineHeadline } from "../machine.ts";
import type { Part, System, Thing } from "../model.ts";
import { tiesOf } from "../ties.ts";

const deeper = { command: "mycelium machines", site: "mycelium/", launcher: "Machines" };

/** A machine the node reaches, standing as `health` says, worth a look for `side`. */
function reached(name: string, health: Thing["health"], side: string[] = []): Thing {
  const ties = tiesOf({ reached: true });
  const kind = "machine";
  return { kind, name, health, says: "", side, looks: undefined, address: undefined, ties };
}

/** A machine out of reach, last heard at `heard`, as the daemon says one (machines.rs). */
function away(name: string, heard?: string): Thing {
  const ties = tiesOf({ reached: false, ...(heard === undefined ? {} : { heard_at: heard }) });
  return {
    kind: "machine",
    name,
    health: "bad",
    says: "is out of reach",
    side: ["out of reach"],
    address: undefined,
    ties,
  };
}

/** The six gone, by name, as the daemon lists them: the worst first, then by name. */
const GONE = ["cloud-1", "cloud-2", "cloud-3", "cloud-4", "cloud-5", "cloud-6"];

/** The fourteen, worst first as the machines part lists them. */
const FOURTEEN: readonly Thing[] = [
  ...GONE.map((name) => away(name, "2026-11-23T19:02:00Z")),
  reached("plover", "bad", ["nearly full disk, 87 GB free"]),
  ...["heron", "ibis", "kestrel", "osprey", "petrel", "tern", "wren"].map((name) =>
    reached(name, "good"),
  ),
];

function system(things: readonly Thing[]): System {
  const machines: Part = {
    part: "machines",
    health: "bad",
    says: "",
    side: [],
    holds: undefined,
    things,
    named: things.map((thing) => thing.name),
    untold: 0,
    machinesUntold: 0,
    deeper,
  };
  return {
    machine: "plover",
    readAt: "2026-11-23T22:45:00Z",
    health: "bad",
    parts: [machines],
    unread: [],
    untold: [],
  };
}

const when = (at: string): string => `at ${at.slice(11, 16)}`;

it("the headline counts those out of reach apart from the one in trouble", () => {
  assert.deepEqual(machineHeadline(system(FOURTEEN)), {
    machines: "14 machines",
    stand: "one in trouble and six out of reach",
    health: "bad",
  });
  assert.deepEqual(machineHeadline(system(GONE.map((name) => away(name)))), {
    machines: "Six machines",
    stand: "all out of reach",
    health: "bad",
  });
});

it("only machines in reach are drawn whole, those out of reach named together after them", () => {
  const drawn = partsOf(system(FOURTEEN), when);
  const sections = [...drawn.matchAll(/<section class="machine [a-z]+" id="(machine-\d+)"/gu)];
  assert.equal(sections.length, MACHINES_DRAWN, "eight in reach, each a section");
  const machines = [...drawn.matchAll(/<section class="machine [\s\S]*?<\/section>/gu)].map(
    (each) => each[0],
  );
  for (const name of GONE) {
    assert.ok(
      machines.every((section) => !section.includes(`>${name}<`)),
      `${name} has no section of its own`,
    );
  }
  const first = drawn.indexOf('id="machine-1"');
  assert.ok(drawn.slice(first, first + 600).includes("plover"), "the one in trouble leads");
  const awaySection = drawn.slice(drawn.indexOf('id="away"'));
  assert.ok(awaySection.includes("Bad: six machines the mycelium can&#39;t reach"), awaySection);
  for (const name of GONE) {
    assert.ok(awaySection.includes(`data-row="machine:${name}"`), `${name} is a row there`);
  }
  assert.equal(
    [...awaySection.matchAll(/class="mark /gu)].length,
    1,
    "one mark for all of them, not a wall of them",
  );
  assert.ok(awaySection.includes("Last heard at 19:02"), "when each was last heard");
  assert.ok(
    drawn.indexOf('id="away"') < drawn.indexOf('id="across"'),
    "after the machines, before the parts across them",
  );
});

it("one never heard says so, and many past what the list names are counted", () => {
  const unheard = partsOf(system([reached("ibis", "good"), away("cloud-1")]), when);
  assert.ok(unheard.includes("Not heard here yet"), unheard);
  const fleet = Array.from({ length: 30 }, (_, at) => away(`cloud-${at + 1}`));
  const drawn = partsOf(system(fleet), when);
  assert.equal([...drawn.matchAll(/<li><a class="row" data-row="machine:/gu)].length, 24);
  assert.ok(drawn.includes("And 6 more, on the map"), drawn);
  assert.ok(!drawn.includes('data-row="machines:more"'), "none counted among those in reach");
  const nine = [...Array.from({ length: 9 }, (_, at) => reached(`m-${at}`, "good")), away("z")];
  assert.ok(partsOf(system(nine), when).includes("And 1 more machine, on the map"));
});

it("cells the mycelium still holds on a machine out of reach are said on its line", () => {
  const left = tiesOf({
    reached: false,
    heard_at: "2026-11-23T19:02:00Z",
    cells: { busy: 2, idle: 1, ended: 4 },
  });
  const drawn = partsOf(system([{ ...away("cloud-1"), ties: left }]), when);
  assert.ok(drawn.includes("Last heard at 19:02, 3 cells still on it"), drawn);
  const row = drawn.slice(drawn.indexOf('data-row="machine:cloud-1"'));
  assert.ok(!row.slice(0, row.indexOf("</li>")).includes("(bad)"), "said once, above");
});

it("the bridge stands a pier only for a machine in reach, and says the rest", () => {
  const read = system(FOURTEEN);
  const drawn = bridgeOf(reachedOf(read), awayOf(read).length);
  for (const name of GONE) {
    assert.ok(!drawn.includes(`>${name}<`), `${name} stands no pier`);
  }
  assert.ok(drawn.includes(">plover<"), "the first in reach leads the bridge");
  assert.ok(drawn.includes("Six machines out of reach, named below."), drawn);
});

it("those out of reach are one look, so the few that need Joel lead", () => {
  const list = looksOf(system(FOURTEEN), MACHINES_DRAWN);
  assert.deepEqual(
    list.looks.map((look) => [look.whose, look.words, look.href]),
    [
      ["plover", "Nearly full disk, 87 GB free", "#machine-1"],
      ["Six machines", "Out of reach: cloud-1, cloud-2, cloud-3 and 3 more", "#away"],
    ],
  );
  // However the names sort, one in reach leads those out of reach in the same health.
  const renamed = [...GONE.map((name) => away(`z-${name}`)), reached("aardvark", "bad", ["hot"])];
  assert.equal(looksOf(system(renamed), MACHINES_DRAWN).looks[0]?.whose, "aardvark");
  assert.equal(list.more, 0);
  const one = looksOf(system([reached("ibis", "good"), away("cloud-1")]), MACHINES_DRAWN);
  assert.deepEqual(
    one.looks.map((look) => [look.whose, look.words, look.href]),
    [["cloud-1", "Out of reach", "#away"]],
  );
  const two = looksOf(system([away("cloud-1"), away("cloud-2")]), MACHINES_DRAWN);
  assert.equal(two.looks[0]?.words, "Out of reach: cloud-1 and cloud-2");
  assert.ok(looksDrawn(list).includes('data-row="look:away"'));
});

it("anything else one out of reach says stays a look of its own", () => {
  const stale = { ...away("cloud-1"), side: ["out of reach", "nearly full disk, 3 GB free"] };
  const list = looksOf(system([stale, away("cloud-2")]), MACHINES_DRAWN);
  assert.deepEqual(
    list.looks.map((look) => [look.whose, look.words]),
    [
      ["Two machines", "Out of reach: cloud-1 and cloud-2"],
      ["cloud-1", "Nearly full disk, 3 GB free"],
    ],
  );
});

it("the places go to the machines in reach, then those out of reach, then across them", () => {
  const places = [...placesOf(system(FOURTEEN)).matchAll(/data-place="(?<place>[^"]*)"/gu)].map(
    (each) => each.groups?.["place"],
  );
  assert.deepEqual(places, [
    ...Array.from({ length: MACHINES_DRAWN }, (_, at) => `machine-${at + 1}`),
    "away",
    "across",
  ]);
  assert.ok(!placesOf(system([reached("ibis", "good")])).includes("away"), "none, no place");
});

it("an address for one out of reach lands on its row there", () => {
  const read = system(FOURTEEN);
  const names = (things: readonly Thing[]): string[] => things.map((thing) => thing.name);
  assert.deepEqual(landingOf(names(reachedOf(read)), "cloud-3", "in", names(awayOf(read))), {
    row: "machine:cloud-3",
  });
  const fleet = Array.from({ length: 30 }, (_, at) => `cloud-${at + 1}`);
  assert.deepEqual(landingOf([], "cloud-30", "in", fleet), {
    row: "away:more",
    note: "cloud-30 is out of reach, past the first 24 named here; the map has it.",
  });
});

it("a machine from a daemon that doesn't say whether it's reached is drawn as one in reach", () => {
  assert.equal(tiesOf({}).reached, true);
  assert.equal(
    tiesOf({ reached: false, heard_at: "2026-11-23T19:02:00Z" }).heardAt,
    "2026-11-23T19:02:00Z",
  );
});

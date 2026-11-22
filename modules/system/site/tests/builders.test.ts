// The builders the machines send builds to, as the page reads and draws
// them: records of their own kind on a watch of their own, things of the
// machines part after the machines, never counted as machines, said across
// them, and a look at one led to the map.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { escaped, partsOf } from "../draw.ts";
import { looksOf } from "../looks.ts";
import { machineHeadline } from "../machine.ts";
import { type System, systemOf, thingsHeld } from "../model.ts";

interface Told {
  readonly key: string;
  readonly kind: string;
}

/** The system `these` hold, each record told on its kind's own watch. */
function systemOfThese(these: readonly Told[]): System {
  const [summary, part, machines, builders] = ["summary", "part", "machine", "builder"].map(
    (kind) => {
      const held = new Held();
      held.took({ records: these.filter((record) => record.kind === kind) });
      return held;
    },
  );
  assert.ok(summary && part && machines && builders);
  const system = systemOf(summary, part, thingsHeld(machines, builders));
  assert.ok(system, "the summary is told");
  return system;
}

const summary = {
  key: "summary",
  kind: "summary",
  machine: "kestrel",
  read_at: "2026-11-22T09:00:00Z",
  health: "watch",
  parts: ["machines"],
};

/** The machines part, naming `things` and standing at `health`. */
function machinesPart(things: readonly string[], health = "good", side: string[] = []) {
  return {
    key: "part:machines",
    kind: "part",
    part: "machines",
    health,
    says: "1 machine ready",
    side,
    holds: "1 machine",
    things,
    deeper: { command: "mycelium machines", site: "mycelium/", launcher: "Agents" },
  };
}

const kestrel = { key: "machine:kestrel", kind: "machine", name: "kestrel", health: "good" };

/** A builder's record, as the daemon tells one. */
function builder(name: string, health: string, says: string, side: string[] = []) {
  return { key: `builder:${name}`, kind: "builder", name, health, says, side, ties: null };
}

const gull = builder("gull", "watch", "isn't answering (refused)", ["not answering"]);
const osprey = builder("osprey", "good", "answers and takes 16 builds at once");

/** What `markup` reads as, its tags gone and its spaces run together. */
function read(markup: string): string {
  return markup
    .replace(/<[^>]*>/gu, " ")
    .replace(/\s+/gu, " ")
    .trim();
}

/** The builders' section of what `system` draws, from its opening tag's end, or none. */
function buildersDrawn(system: System): string | undefined {
  const [, section] = partsOf(system, (at) => `at ${at}`).split('data-part="part-builders"');
  return section?.split("</section>")[0];
}

it("reads a builder from its own watch, as a thing of its own kind after the machines", () => {
  const system = systemOfThese([
    summary,
    machinesPart(["machine:kestrel", "builder:gull", "builder:osprey"], "watch"),
    kestrel,
    gull,
    osprey,
  ]);
  const [machines] = system.parts;
  assert.deepEqual(
    machines?.things.map((thing) => [thing.kind, thing.name, thing.health]),
    [
      ["machine", "kestrel", "good"],
      ["builder", "gull", "watch"],
      ["builder", "osprey", "good"],
    ],
    "each thing is found on its kind's watch, and keeps its kind",
  );
  assert.equal(machines?.untold, 0, "every thing named was told");
});

it("counts no builder among the machines, told or still to come", () => {
  const system = systemOfThese([
    summary,
    machinesPart(["machine:kestrel", "builder:gull", "builder:osprey"], "watch"),
    kestrel,
    gull,
  ]);
  const [machines] = system.parts;
  assert.equal(machines?.untold, 1, "osprey hasn't come");
  assert.equal(machines?.machinesUntold, 0, "a builder still to come is no machine to come");
  assert.equal(machineHeadline(system).machines, "One machine", "the headline counts machines");
  assert.equal(
    partsOf(system, (at) => `at ${at}`).split('<section class="machine ').length - 1,
    1,
    "a builder was drawn as a machine",
  );
});

it("says the builders across the machines: the worst one's line, then the rest", () => {
  const drawn = buildersDrawn(
    systemOfThese([
      summary,
      machinesPart(["machine:kestrel", "builder:gull", "builder:osprey"], "watch"),
      kestrel,
      gull,
      osprey,
    ]),
  );
  assert.ok(drawn, "no builders were drawn");
  assert.equal(
    read(drawn.slice(drawn.indexOf(">") + 1)),
    escaped(
      [
        "Builders (watch)",
        "gull (watch) isn't answering (refused)",
        "osprey (good) answers and takes 16 builds at once",
        "See them on the map",
      ].join(" "),
    ),
  );
  assert.ok(drawn.includes('data-row="builders" data-health="watch" href="../mycelium/"'));
});

it("counts the builders while none is worth a look, and leads to one by its name", () => {
  const drawn = buildersDrawn(
    systemOfThese([
      summary,
      machinesPart(["machine:kestrel", "builder:osprey two"]),
      kestrel,
      builder("osprey two", "good", "answers and takes 16 builds at once"),
    ]),
  );
  assert.ok(drawn, "no builders were drawn");
  assert.ok(drawn.includes('<p class="said">One builder</p>'), "a good builder wasn't counted");
  assert.ok(
    drawn.includes('href="../mycelium/#builder=osprey%20two">See it on the map<'),
    "one builder wasn't led to by its name",
  );
});

it("draws no builders, nor a word of them, where the node names none", () => {
  const system = systemOfThese([summary, machinesPart(["machine:kestrel"]), kestrel]);
  assert.equal(buildersDrawn(system), undefined);
  assert.ok(!partsOf(system, (at) => `at ${at}`).includes("Not shown yet"), "a placeholder");
});

it("leads a builder's look to it on the map, whatever its place among the things", () => {
  const system = systemOfThese([
    summary,
    machinesPart(["machine:kestrel", "builder:gull"], "watch", ["gull not answering"]),
    kestrel,
    gull,
  ]);
  const { looks, more } = looksOf(system, 8);
  assert.deepEqual(
    looks.map((look) => [look.whose, look.health, look.words, look.href, look.row]),
    [
      [
        "gull",
        "watch",
        "Not answering",
        "../mycelium/#builder=gull",
        "look:builder:gull:0:not answering",
      ],
    ],
    "the part's side, which names gull, is gull's own look",
  );
  assert.equal(more, 0);
});

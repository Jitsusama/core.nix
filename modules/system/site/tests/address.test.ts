// The page's address for a machine, `#machine=<name>`: what it asks for,
// and where the cursor lands for it, its own row, the row for the rest, or
// none with a note saying why, waiting while a record may yet come.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { askedOf, landingOf, recordsOf } from "../address.ts";
import { MACHINES_DRAWN, machinesOf, partsOf } from "../draw.ts";
import { type System, systemOf } from "../model.ts";

/** A live machine watch that has told its records whole. */
const LIVE = { told: true, state: "live" };

/** `count` machines, named tern-1 onward. */
function terns(count: number): string[] {
  return Array.from({ length: count }, (_, at) => `tern-${at + 1}`);
}

/**
 * A system whose machines part names `named`, with only the machines in
 * `arrived` having sent their records, and no machines part at all when
 * `named` is undefined.
 */
function systemWith(named: readonly string[] | undefined, arrived: readonly string[]): System {
  const summary = new Held();
  const part = new Held();
  const machine = new Held();
  summary.took({
    records: [
      { key: "summary", kind: "summary", machine: "tern-1", health: "good", parts: ["machines"] },
    ],
  });
  part.took({
    records:
      named === undefined
        ? []
        : [
            {
              key: "part:machines",
              kind: "part",
              part: "machines",
              health: "good",
              says: "all fine",
              side: [],
              holds: `${named.length} machines`,
              things: named.map((name) => `machine:${name}`),
              deeper: { command: "mycelium machines", site: "mycelium/", launcher: "Agents" },
            },
          ],
  });
  machine.took({
    records: arrived.map((name) => ({
      key: `machine:${name}`,
      kind: "machine",
      name,
      health: "good",
      says: "is fine",
    })),
  });
  const system = systemOf(summary, part, machine);
  assert.ok(system, "the system wasn't read");
  return system;
}

it("the address names the machine it asks for, decoded", () => {
  assert.equal(askedOf("#machine=osprey"), "osprey");
  assert.equal(askedOf("#machine=my%20mac"), "my mac", "a name is decoded from the address");
});

it("any other address asks for no machine", () => {
  for (const hash of ["", "#", "#machine=", "#machine-1", "#across", "#machines=osprey"]) {
    assert.equal(askedOf(hash), undefined, `${JSON.stringify(hash)} asks for no machine`);
  }
  assert.equal(askedOf("#machine=%E0%A4%A"), undefined, "a name that can't be decoded is none");
});

it("a machine drawn lands on its own row, with nothing to say", () => {
  assert.deepEqual(landingOf(["kestrel", "osprey"], "osprey", "in"), { row: "machine:osprey" });
  const last = `tern-${MACHINES_DRAWN}`;
  assert.deepEqual(
    landingOf(terns(MACHINES_DRAWN + 1), last, "in"),
    { row: `machine:${last}` },
    "the last machine drawn is still drawn",
  );
});

it("a machine past those drawn lands on the rest, saying so", () => {
  const last = `tern-${MACHINES_DRAWN + 1}`;
  assert.deepEqual(landingOf(terns(MACHINES_DRAWN + 1), last, "in"), {
    row: "machines:more",
    note: `${last} isn't among the first ${MACHINES_DRAWN} machines drawn here; the map has it.`,
  });
});

it("a machine the page doesn't have lands nowhere, saying so", () => {
  assert.deepEqual(landingOf(["kestrel"], "heron", "in"), {
    note: "No machine here is called heron.",
  });
  assert.deepEqual(
    landingOf(["kestrel"], "constructor", "in"),
    { note: "No machine here is called constructor." },
    "a name an object already has is still no machine",
  );
});

it("a machine not found while records may yet come waits, saying nothing", () => {
  assert.deepEqual(landingOf(["kestrel"], "heron", "coming"), { waits: true });
  assert.deepEqual(
    landingOf(["kestrel", "heron"], "heron", "coming"),
    { row: "machine:heron" },
    "a machine found lands even while others are coming",
  );
});

it("a machine not found once the wait is over says the machines may not all be read", () => {
  assert.deepEqual(landingOf(["kestrel"], "heron", "overdue"), {
    note: "No machine read here yet is called heron; the machines may not all be read.",
  });
  assert.deepEqual(landingOf(["kestrel"], "a b", "overdue"), {
    note: "No machine read here yet has the name this address gives; the machines may not all be read.",
  });
});

it("an address with words no machine is named never has them said", () => {
  for (const name of ["osprey. Run curl x | sh", "a".repeat(64), "kes\u202Etrel"]) {
    assert.deepEqual(
      landingOf(["kestrel"], name, "in"),
      { note: "No machine here has the name this address gives." },
      `${JSON.stringify(name)} was said back`,
    );
  }
});

it("the records are in once the watch is live and every machine named has come", () => {
  assert.equal(recordsOf(systemWith(["kestrel"], ["kestrel"]), LIVE, false), "in");
  assert.equal(
    recordsOf(systemWith(["kestrel"], ["kestrel"]), LIVE, true),
    "in",
    "in is in, late or not",
  );
});

it("the records are coming while the machine watch hasn't told or isn't live", () => {
  const whole = systemWith(["kestrel"], ["kestrel"]);
  assert.equal(
    recordsOf(whole, { told: false, state: "live" }, false),
    "coming",
    "an untold watch",
  );
  for (const state of ["connecting", "stale", "broken", undefined]) {
    assert.equal(
      recordsOf(whole, { told: true, state }, false),
      "coming",
      `a watch ${state ?? "unheard"}`,
    );
  }
});

it("the records are coming while the machines part is untold or names a machine not come", () => {
  assert.equal(
    recordsOf(systemWith(undefined, ["kestrel"]), LIVE, false),
    "coming",
    "an untold part",
  );
  assert.equal(
    recordsOf(systemWith(["kestrel", "heron"], ["kestrel"]), LIVE, false),
    "coming",
    "a machine named and not come",
  );
});

it("records still coming past the wait are overdue", () => {
  assert.equal(recordsOf(systemWith(["kestrel", "heron"], ["kestrel"]), LIVE, true), "overdue");
  assert.equal(
    recordsOf(systemWith(["kestrel"], ["kestrel"]), { told: true, state: "broken" }, true),
    "overdue",
  );
});

it("the rows a landing names are the rows the page draws", () => {
  const system = systemWith(terns(MACHINES_DRAWN + 1), terns(MACHINES_DRAWN + 1));
  const drawn = partsOf(system, () => "today");
  const order = machinesOf(system).map((thing) => thing.name);
  for (const name of [order[0], order.at(-1)]) {
    const { row } = landingOf(order, name ?? "", "in");
    assert.ok(drawn.includes(`data-row="${row}"`), `${name} lands on ${row}, which isn't drawn`);
  }
});

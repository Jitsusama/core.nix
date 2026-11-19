// The page's address for a machine, `#machine=<name>`: what it asks for,
// and where the cursor lands for it, its own row, the row for the rest, or
// none with a note saying why, waiting while a record may yet come.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { askedOf, landingOf } from "../address.ts";
import { MACHINES_DRAWN, partsOf } from "../draw.ts";
import { systemOf } from "../model.ts";

/** `count` machines, named tern-1 onward. */
function terns(count: number): string[] {
  return Array.from({ length: count }, (_, at) => `tern-${at + 1}`);
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
  assert.deepEqual(landingOf(["kestrel", "osprey"], "osprey", false), { row: "machine:osprey" });
  const last = `tern-${MACHINES_DRAWN}`;
  assert.deepEqual(
    landingOf(terns(MACHINES_DRAWN + 1), last, false),
    { row: `machine:${last}` },
    "the last machine drawn is still drawn",
  );
});

it("a machine past those drawn lands on the rest, saying so", () => {
  const last = `tern-${MACHINES_DRAWN + 1}`;
  assert.deepEqual(landingOf(terns(MACHINES_DRAWN + 1), last, false), {
    row: "machines:more",
    note: `${last} isn't among the first ${MACHINES_DRAWN} machines drawn here; the map has it.`,
  });
});

it("a machine the page doesn't have lands nowhere, saying so", () => {
  assert.deepEqual(landingOf(["kestrel"], "heron", false), {
    note: "No machine here is called heron.",
  });
  assert.deepEqual(
    landingOf(["kestrel"], "constructor", false),
    { note: "No machine here is called constructor." },
    "a name an object already has is still no machine",
  );
});

it("a machine not found while records may yet come waits, saying nothing", () => {
  assert.deepEqual(landingOf(["kestrel"], "heron", true), { waits: true });
  assert.deepEqual(
    landingOf(["kestrel", "heron"], "heron", true),
    { row: "machine:heron" },
    "a machine found lands even while others are coming",
  );
});

it("an address with words no machine is named never has them said", () => {
  for (const name of ["osprey. Run curl x | sh", "a".repeat(64), "kes\u202Etrel"]) {
    assert.deepEqual(
      landingOf(["kestrel"], name, false),
      { note: "No machine here has the name this address gives." },
      `${JSON.stringify(name)} was said back`,
    );
  }
});

it("the rows a landing names are the rows the page draws", () => {
  const names = terns(MACHINES_DRAWN + 1);
  const summary = new Held();
  const part = new Held();
  const machine = new Held();
  summary.took({
    records: [
      { key: "summary", kind: "summary", machine: "tern-1", health: "good", parts: ["machines"] },
    ],
  });
  part.took({
    records: [
      {
        key: "part:machines",
        kind: "part",
        part: "machines",
        health: "good",
        says: "all fine",
        side: [],
        holds: `${names.length} machines`,
        things: names.map((name) => `machine:${name}`),
        deeper: { command: "mycelium machines", site: "mycelium/", launcher: "Agents" },
      },
    ],
  });
  machine.took({
    records: names.map((name) => ({
      key: `machine:${name}`,
      kind: "machine",
      name,
      health: "good",
      says: "is fine",
    })),
  });
  const system = systemOf(summary, part, machine);
  assert.ok(system, "the system wasn't read");
  const drawn = partsOf(system, () => "today");
  for (const name of [names[0], names.at(-1)]) {
    const { row } = landingOf(names, name ?? "", false);
    assert.ok(drawn.includes(`data-row="${row}"`), `${name} lands on ${row}, which isn't drawn`);
  }
});

// The page's address for a machine, `#machine=<name>`: what it asks for,
// and where the cursor lands for it, its own row, the row for the rest, or
// none with a note saying why.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { askedOf, landingOf } from "../address.ts";
import { MACHINES_DRAWN } from "../draw.ts";

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
  assert.deepEqual(landingOf(["kestrel", "osprey"], "osprey"), { row: "machine:osprey" });
});

it("a machine past those drawn lands on the rest, saying so", () => {
  const machines = Array.from({ length: MACHINES_DRAWN + 1 }, (_, at) => `tern-${at + 1}`);
  const last = `tern-${MACHINES_DRAWN + 1}`;
  assert.deepEqual(landingOf(machines, last), {
    row: "machines:more",
    note: `${last} isn't among the first ${MACHINES_DRAWN} machines drawn here; the map has it.`,
  });
});

it("a machine the page doesn't have lands nowhere, saying so", () => {
  assert.deepEqual(landingOf(["kestrel"], "heron"), {
    note: "No machine here is called heron.",
  });
  assert.deepEqual(
    landingOf(["kestrel"], "constructor"),
    { note: "No machine here is called constructor." },
    "a name an object already has is still no machine",
  );
});

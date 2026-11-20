// What the map makes of the builders the node says this machine sends its
// builds to: records of their own kind, never cells.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { nowOf } from "../model.ts";

const AT = Date.parse("2026-11-16T07:30:00Z");

/** The watches, each told its kind's records of `these` whole. */
function told(these: readonly { readonly key: string; readonly kind: string }[]) {
  const heard = {
    network: new Held(),
    cell: new Held(),
    machine: new Held(),
    message: new Held(),
    builder: new Held(),
  };
  for (const [kind, held] of Object.entries(heard)) {
    held.took({ records: these.filter((record) => record.kind === kind) });
  }
  return heard;
}

/** A builder, as the node keys and tells it. */
function builder(name: string, state: string, builds: object) {
  return { key: `builder:${name}`, kind: "builder", heard: AT, name, state, ...builds };
}

/** A session's cell, as the node keys and tells it. */
const osprey = {
  key: "cell:OSPREY",
  kind: "cell",
  heard: AT,
  name: "OSPREY",
  type: "session",
  title: "the map",
  machine: "skiff",
  standing: "moving",
  networks: ["orchestration.kestrel"],
};

it("the builders are read from their own records, by name, and never drawn as cells", () => {
  const lent = {
    systems: ["x86_64-linux"],
    jobs: 16,
    lines: ["lent"],
    lent: true,
    said: "passing",
  };
  const standing = { systems: ["x86_64-linux", "aarch64-linux"], jobs: 4, lines: ["standing"] };
  const now = nowOf(
    told([builder("tern", "running", lent), builder("heron", "answering", standing), osprey]),
    AT,
  );
  assert.deepEqual(
    now.builders,
    [
      {
        name: "heron",
        state: "answering",
        systems: ["x86_64-linux", "aarch64-linux"],
        jobs: 4,
        lent: false,
        said: null,
      },
      {
        name: "tern",
        state: "running",
        systems: ["x86_64-linux"],
        jobs: 16,
        lent: true,
        said: "passing",
      },
    ],
    "each builder, by name, with what it builds flattened",
  );
  assert.deepEqual(
    now.cells.map((cell) => cell.name),
    ["OSPREY"],
    "a builder is no cell the map draws",
  );
});

it("a builder that says nothing reads as unheard, and one with no name is none", () => {
  const { key, kind, heard, name } = builder("gull", "", {});
  const now = nowOf(told([{ key, kind, heard, name }, builder("", "answering", {})]), AT);
  assert.deepEqual(
    now.builders,
    [{ name: "gull", state: "unknown", systems: [], jobs: 0, lent: false, said: null }],
    "a builder whose builds the node left out builds nothing known",
  );
});

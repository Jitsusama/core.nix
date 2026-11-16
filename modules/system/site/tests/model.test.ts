// What the system page makes of the system daemon's records, told as the
// daemon tells them: the summary, a part record each, a record per thing.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { escaped } from "../draw.ts";
import { judged, systemOf } from "../model.ts";

/** The three watches, told `these` whole, each to its kind's. */
function told(these: readonly { readonly key: string; readonly kind: string }[]) {
  const summary = new Held();
  const part = new Held();
  const machine = new Held();
  summary.took({ records: these.filter((record) => record.kind === "summary") });
  part.took({ records: these.filter((record) => record.kind === "part") });
  machine.took({ records: these.filter((record) => record.kind === "machine") });
  return { summary, part, machine };
}

const deeper = { command: "mycelium machines", site: "mycelium/", launcher: "Agents" };

const records = [
  {
    key: "summary",
    kind: "summary",
    machine: "kestrel",
    read_at: "2026-11-16T05:41:00Z",
    health: "watch",
    parts: ["machines", "code"],
  },
  {
    key: "part:machines",
    kind: "part",
    part: "machines",
    health: "watch",
    says: "osprey has a filling disk",
    things: ["machine:kestrel", "machine:osprey"],
    deeper,
  },
  { key: "part:code", kind: "part", part: "code", health: "good", says: "18 repositories", deeper },
  { key: "machine:kestrel", kind: "machine", name: "kestrel", health: "good", says: "is fine" },
  {
    key: "machine:osprey",
    kind: "machine",
    name: "osprey",
    health: "watch",
    says: "has a filling disk",
    address: "mycelium/#machine=osprey",
  },
];

it("nothing is drawn until the summary is told", () => {
  const { summary, part, machine } = told(records.filter((record) => record.kind !== "summary"));
  assert.equal(systemOf(summary, part, machine), undefined);
});

it("the parts come in the summary's order, each with its things", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.deepEqual(
    system.parts.map((each) => each.part),
    ["machines", "code"],
  );
  assert.deepEqual(
    system.parts[0]?.things.map((thing) => [thing.name, thing.health, thing.address]),
    [
      ["kestrel", "good", undefined],
      ["osprey", "watch", "mycelium/#machine=osprey"],
    ],
  );
  assert.deepEqual(system.unread, []);
});

it("a part or thing not told yet is said, and the rest drawn", () => {
  const { summary, part, machine } = told(
    records.filter((record) => record.key !== "machine:osprey" && record.key !== "part:code"),
  );
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.equal(system.parts.length, 1);
  assert.deepEqual(system.unread, ["machine:osprey wasn't told", "part:code wasn't told"]);
});

it("a health the page doesn't know is read as not read", () => {
  const { summary, part, machine } = told(
    records.map((record) => (record.key === "part:code" ? { ...record, health: "odd" } : record)),
  );
  assert.equal(systemOf(summary, part, machine)?.parts[1]?.health, "unread");
});

it("the whole is said worst first", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.equal(judged(system), "Machines is worth a look.");
  const worse = {
    ...system,
    parts: system.parts.map((each) => ({ ...each, health: "bad" as const })),
  };
  assert.equal(judged(worse), "Machines and Code are bad.");
  const good = {
    ...system,
    parts: system.parts.map((each) => ({ ...each, health: "good" as const })),
  };
  assert.equal(judged(good), "Every part is good.");
});

it("a record's words reach the page as text, never as markup", () => {
  assert.equal(escaped(`<b a="x">'&'</b>`), "&lt;b a=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/b&gt;");
});

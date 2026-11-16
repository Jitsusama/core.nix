// What the system page makes of the system daemon's records, told as the
// daemon tells them: the summary, a part record each, a record per thing.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { escaped } from "../draw.ts";
import { headline, lede, looksIn, sentence, systemOf } from "../model.ts";

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
    side: ["osprey has a filling disk"],
    holds: "2 machines",
    things: ["machine:kestrel", "machine:osprey"],
    deeper,
  },
  {
    key: "part:code",
    kind: "part",
    part: "code",
    health: "good",
    says: "18 repositories",
    side: [],
    holds: "18 repositories",
    deeper,
  },
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
  assert.deepEqual(system.untold, [], "every part told leaves none to come");
  assert.deepEqual(
    system.parts[0]?.things.map((thing) => [thing.name, thing.health, thing.address]),
    [
      ["kestrel", "good", undefined],
      ["osprey", "watch", "mycelium/#machine=osprey"],
    ],
  );
  assert.deepEqual(system.unread, []);
});

it("a part not told yet is said, a thing not told yet left out, and the rest drawn", () => {
  const { summary, part, machine } = told(
    records.filter((record) => record.key !== "machine:osprey" && record.key !== "part:code"),
  );
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.deepEqual(
    system.parts.map((each) => [each.part, each.things.map((thing) => thing.name)]),
    [["machines", ["kestrel"]]],
  );
  assert.deepEqual(system.unread, ["part:code wasn't told"]);
  assert.deepEqual(system.untold, ["code"], "a part not told yet is named as still to come");
});

it("the daemon's not read is read as unread", () => {
  const { summary, part, machine } = told(
    records.map((record) =>
      record.key === "part:code" ? { ...record, health: "not read" } : record,
    ),
  );
  assert.equal(systemOf(summary, part, machine)?.parts[1]?.health, "unread");
});

it("a health the page doesn't know is read as not read", () => {
  const { summary, part, machine } = told(
    records.map((record) => (record.key === "part:code" ? { ...record, health: "odd" } : record)),
  );
  assert.equal(systemOf(summary, part, machine)?.parts[1]?.health, "unread");
});

it("a part's side and what it holds are read apart", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.deepEqual(
    system?.parts.map((each) => [each.side, each.holds]),
    [
      [["osprey has a filling disk"], "2 machines"],
      [[], "18 repositories"],
    ],
  );
});

it("a side's closing and N more counts for as many", () => {
  const { summary, part, machine } = told(records);
  const machines = systemOf(summary, part, machine)?.parts[0];
  assert.ok(machines);
  const side = ["a", "b", "and 4 more worth a look"];
  assert.equal(looksIn({ ...machines, side }), 6);
  assert.equal(
    looksIn({ ...machines, side, health: "unread" }),
    0,
    "why it wasn't read is no look",
  );
});

it("the headline counts the parts, what is worth a look and what wasn't read", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.equal(sentence(headline(system)), "Two parts, one worth a look.");
  const [first, second] = system.parts;
  assert.ok(first && second);
  const many = {
    ...system,
    parts: [
      { ...first, side: ["a", "b", "and 10 more worth a look"] },
      { ...second, health: "unread" as const },
    ],
  };
  assert.equal(sentence(headline(many)), "Two parts, 12 worth a look and one not read.");
  const good = { ...system, parts: [{ ...first, health: "good" as const, side: [] }] };
  assert.equal(sentence(headline(good)), "One part, all clear.");
});

it("the lede names the worst part's first look, then why a part wasn't read", () => {
  const { summary, part, machine } = told(records);
  const system = systemOf(summary, part, machine);
  assert.ok(system);
  assert.equal(lede(system), "Machines: osprey has a filling disk.", "a name keeps its case");
  const [first, second] = system.parts;
  assert.ok(first && second);
  const worse = {
    ...system,
    parts: [first, { ...second, health: "bad" as const, side: ["the build is failing"] }],
  };
  assert.equal(lede(worse), "Code: the build is failing.", "bad comes before worth a look");
  const stopped = {
    ...system,
    parts: [
      first,
      { ...second, health: "bad" as const, side: ["the radio is busy, 20 hours ago."] },
    ],
  };
  assert.equal(
    lede(stopped),
    "Code: the radio is busy, 20 hours ago.",
    "an owner's words that end their own sentence get no second stop",
  );
  const unread = {
    ...system,
    parts: [
      { ...first, health: "good" as const, side: [] },
      { ...second, health: "unread" as const, says: "repos timed out" },
    ],
  };
  assert.equal(lede(unread), "Code couldn't be read: repos timed out.");
  assert.equal(
    lede({ ...system, parts: [{ ...first, health: "good" as const, side: [] }] }),
    undefined,
  );
});

it("a record's words reach the page as text, never as markup", () => {
  assert.equal(escaped(`<b a="x">'&'</b>`), "&lt;b a=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/b&gt;");
});

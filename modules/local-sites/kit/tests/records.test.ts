// What Held makes of a watch's answers, and what the guards make of fields,
// read as a page reads them.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held, isRecord, number, object, text, texts } from "../records.ts";

it("a watch's records whole are held by key", () => {
  const held = new Held();
  const moved = held.took({ records: [{ key: "cell:A", title: "A" }, { key: "cell:B" }] });
  assert.deepEqual(moved, { changed: ["cell:A", "cell:B"], gone: [], skipped: 0 });
  assert.equal(held.size, 2);
  assert.equal(held.get("cell:A")?.title, "A");
});

it("a record told twice is one record, as last told", () => {
  const held = new Held();
  held.took({ records: [{ key: "cell:A", machine: "kestrel" }] });
  held.took({ changed: [{ key: "cell:A", machine: "osprey" }] });
  held.took({ changed: [{ key: "cell:A", machine: "osprey" }] });
  assert.equal(held.size, 1);
  assert.equal(held.get("cell:A")?.machine, "osprey");
});

it("records whole again stand for every record, those unnamed gone", () => {
  const held = new Held();
  held.took({ records: [{ key: "cell:A" }, { key: "cell:B" }] });
  const moved = held.took({ records: [{ key: "cell:B" }] });
  assert.deepEqual(moved.gone, ["cell:A"]);
  assert.deepEqual(
    held.all().map((record) => record.key),
    ["cell:B"],
  );
});

it("a change mends its fields, and a field told null goes", () => {
  const held = new Held();
  held.took({ records: [{ key: "cell:A", title: "A", doing: "editing" }] });
  held.took({ changed: [{ key: "cell:A", doing: null, standing: "quiet" }] });
  assert.deepEqual(held.get("cell:A"), { key: "cell:A", title: "A", standing: "quiet" });
});

it("a deep field is mended one level in, and any other replaced", () => {
  const held = new Held(["presence"]);
  held.took({
    records: [{ key: "s:a", presence: { cwd: "/a", busy: true }, usage: { turns: 1, cost: 2 } }],
  });
  held.took({ changed: [{ key: "s:a", presence: { busy: false }, usage: { turns: 3 } }] });
  assert.deepEqual(held.get("s:a"), {
    key: "s:a",
    presence: { cwd: "/a", busy: false },
    usage: { turns: 3 },
  });
});

it("the keys gone go, and a key never held is not said gone", () => {
  const held = new Held();
  held.took({ records: [{ key: "cell:A" }] });
  const moved = held.took({ gone: ["cell:A", "cell:never"] });
  assert.deepEqual(moved.gone, ["cell:A"]);
  assert.equal(held.size, 0);
});

it("what has no key is passed over and counted", () => {
  const held = new Held();
  const moved = held.took({ records: [{ key: "cell:A" }, { title: "keyless" }, 7] });
  assert.equal(moved.skipped, 2);
  assert.equal(held.size, 1);
});

it("a guard answers a field only when it has the shape asked", () => {
  const record = { key: "machine:kestrel", name: "kestrel", load: 0.5, roles: ["a"], disk: {} };
  assert.ok(isRecord(record));
  assert.equal(text(record, "name"), "kestrel");
  assert.equal(text(record, "load"), undefined);
  assert.equal(number(record, "load"), 0.5);
  assert.equal(number(record, "name"), undefined);
  assert.deepEqual(texts(record, "roles"), ["a"]);
  assert.equal(texts({ key: "k", roles: ["a", 1] }, "roles"), undefined);
  assert.deepEqual(object(record, "disk"), {});
  assert.equal(object(record, "roles"), undefined);
  assert.equal(isRecord({ title: "keyless" }), false);
});

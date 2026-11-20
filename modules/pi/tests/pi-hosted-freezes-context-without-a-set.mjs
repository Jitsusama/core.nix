// pi as the host runs it freezes each context entry it reads, everything in
// it, without putting a single one of their objects in a set. Unpatched,
// pi-durable's freezeJson adds every object of every entry to one weak set
// for the process's life, and the scheduler keeps each conversation's range,
// so a host's set held millions of live keys. V8 rehashes a weak table in
// place on the thread: on 19 November one add held a host's thread 95 s,
// past its watchdog, ending every conversation on it.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
// pi-durable sits beside the coding agent in an installed tree, or in the
// workspace's packages where the tree is pi's own source.
const contextModule = [
  `${root}/node_modules/@earendil-works/pi-durable/dist/harness/context.js`,
  `${root}/packages/durable/dist/harness/context.js`,
].find((path) => existsSync(path));
assert.ok(contextModule, `no pi-durable context module under ${root}`);

// Every object a weak set is given, counted before pi-durable loads, so a
// set made at its import is seen too.
const given = [];
const add = WeakSet.prototype.add;
WeakSet.prototype.add = function (value) {
  given.push(value);
  return add.call(this, value);
};
const { advanceRange } = await import(pathToFileURL(contextModule).href);

// A range that reached its conversation's newest entry, so the entries a
// commit appends extend it, as the scheduler's kept range does after a turn.
const range = {
  bounds: { head: undefined, tail: 0 },
  entries: [],
  view: { head: undefined, entries: [], contributions: [], messages: [] },
  edited: new Set(),
  settled: [],
  open: [],
  baseline: undefined,
  current: true,
};
const meta = { tool: { arguments: { path: "a", lines: [1, 2, 3] } } };
const message = { role: "user", content: [{ type: "text", text: "hi", meta }], timestamp: 0 };
const entry = { id: 1, model: [message] };
const ours = [entry, entry.model, message, message.content, message.content[0], meta, meta.tool];
ours.push(meta.tool.arguments, meta.tool.arguments.lines);

const advanced = advanceRange(range, [entry]);

assert.ok(advanced, "the range wasn't extended by the appended entry");
assert.equal(advanced.entries.at(-1), entry, "the appended entry isn't the range's newest");
for (const object of ours) {
  assert.ok(
    Object.isFrozen(object),
    `an object in the entry wasn't frozen: ${JSON.stringify(object)}`,
  );
}
const kept = given.filter((value) => ours.includes(value));
assert.equal(kept.length, 0, `${kept.length} of the entry's objects went into a weak set`);

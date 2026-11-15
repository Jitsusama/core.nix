// pi as the host runs it builds a turn's boundary context, the whole branch
// copied and projected, only when a handler reads it, and again after each
// handler, which may have changed what it holds. Entries a handler returns
// are built at once, to check them. Unpatched, it is built before the
// handlers and again after each, whether any reads it or not: ten builds a
// round for nine observers, the main thread held tens of milliseconds each
// on a long session while keys wait.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
// The coding agent sits beside its libraries in an installed tree, or is the
// tree itself where the package was installed from its own workspace.
const runnerModule = [
  `${root}/node_modules/@earendil-works/pi-coding-agent/dist/core/extensions/runner.js`,
  `${root}/packages/coding-agent/dist/core/extensions/runner.js`,
  `${root}/dist/core/extensions/runner.js`,
].find((path) => existsSync(path));
assert.ok(runnerModule, `no extension runner under ${root}`);
const { ExtensionRunner } = await import(pathToFileURL(runnerModule).href);

// Handlers are plain functions, since pi awaits whatever one returns.
// A runner of one extension holding `handlers` for turn_end, standing in for
// the parts emitBoundary reaches, and a context builder that counts its
// builds and refuses entries marked bad, as pi refuses ones that don't fit.
const boundary = async (handlers) => {
  const builds = [];
  const errors = [];
  const runner = Object.create(ExtensionRunner.prototype);
  Object.assign(runner, {
    extensions: [{ path: "observer", handlers: new Map([["turn_end", handlers]]) }],
    createContext: () => ({}),
    emitError: (error) => errors.push(error.error),
  });
  const build = (entries) => {
    builds.push(entries.map((entry) => entry.id));
    if (entries.some((entry) => entry.bad)) {
      throw new Error("does not fit");
    }
    return { contextEntries: entries, builtAs: builds.length };
  };
  const result = await runner.emitBoundary({ type: "turn_end", turnIndex: 0 }, build);
  return { builds, errors, result };
};

// Observers that never read the context build none, the result's included.
{
  const observer = () => undefined;
  const { builds, result } = await boundary([observer, observer, observer]);
  assert.deepEqual(builds, [], "observers that never read the context still built it");
  assert.equal(result.valid, true);
  assert.deepEqual(result.entries, []);
}

// A handler reading the context twice builds it once; the next reader builds
// it afresh, since the handler before may have changed what it holds.
{
  const seen = [];
  const reader = (event) => {
    seen.push(event.context.builtAs, event.context.builtAs);
  };
  const { builds } = await boundary([reader, () => undefined, reader]);
  assert.deepEqual(seen, [1, 1, 2, 2], "a read didn't build once a handler, afresh for the next");
  assert.deepEqual(builds, [[], []]);
}

// Entries a handler returns are built at once to check them, and the next
// handler reads that build, not another.
{
  let read;
  const { builds, result } = await boundary([
    () => ({ entries: [{ id: "note" }] }),
    (event) => {
      read = event.context;
    },
  ]);
  assert.deepEqual(builds, [["note"]], "returned entries weren't built once, at once");
  assert.deepEqual(read.contextEntries, [{ id: "note" }]);
  assert.deepEqual(result.entries, [{ id: "note" }]);
}

// Entries that don't fit are said and dropped, as before, and a later reader
// sees the context of the last entries that did.
{
  let read;
  const { errors, result } = await boundary([
    () => ({ entries: [{ id: "good" }] }),
    () => ({ entries: [{ id: "bad", bad: true }], continue: true }),
    (event) => {
      read = event.context;
    },
  ]);
  assert.deepEqual(errors, ["Invalid boundary entries: does not fit"]);
  assert.equal(result.valid, false, "entries that don't fit passed");
  assert.deepEqual(result.entries, []);
  assert.equal(result.continue, false);
  assert.deepEqual(read.contextEntries, [{ id: "good" }], "a reader saw entries that don't fit");
  assert.deepEqual(result.context.contextEntries, [{ id: "good" }]);
}

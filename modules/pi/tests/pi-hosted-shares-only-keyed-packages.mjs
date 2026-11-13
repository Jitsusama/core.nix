// pi as the host runs it shares the extensions of a package that declares
// its per-session state keyed across every folder, and no other's. A
// package without the declaration, an extension in no package, and one
// whose declaration isn't `true` keep a copy for each folder, so a pairing
// one keeps in a module, as neovim.pi keeps nvim's, made in one folder's
// conversation never reaches another folder's. A shared factory outlives
// its conversations, a reload empties it, and a load begun before a reload
// caches nothing after it. Without the patch, a keyed
// package is evaluated in every folder; sharing every package, as a cache
// with no declaration would, lets the pairing cross.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import v8 from "node:v8";
import { runInNewContext } from "node:vm";

const root = resolve(process.argv[2]);
// The coding agent sits beside its libraries in an installed tree, or is the
// tree itself where the package was installed from its own workspace.
const loaderModule = [
  `${root}/node_modules/@earendil-works/pi-coding-agent/dist/core/extensions/loader.js`,
  `${root}/packages/coding-agent/dist/core/extensions/loader.js`,
  `${root}/dist/core/extensions/loader.js`,
].find((path) => existsSync(path));
assert.ok(loaderModule, `no extension loader under ${root}`);
const { clearExtensionCache, loadExtensionsCached } = await import(
  pathToFileURL(loaderModule).href
);

const scratch = mkdtempSync(join(tmpdir(), "pi-hosted-keyed-"));
const write = (path, text) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
};
const manifest = (pi) => JSON.stringify({ name: "probe", pi });

// A probe counts each evaluation of its module under its name, and keeps a
// pairing in module scope, as neovim.pi keeps nvim's: one command pairs it,
// another reads it back, a third names the evaluation it came from.
const probe = (name) =>
  [
    "globalThis.evaluations ??= {};",
    `globalThis.evaluations[${JSON.stringify(name)}] = (globalThis.evaluations[${JSON.stringify(name)}] ?? 0) + 1;`,
    `const evaluation = globalThis.evaluations[${JSON.stringify(name)}];`,
    "let paired;",
    "export default function (pi) {",
    `  pi.registerCommand(${JSON.stringify(`${name}-pair`)}, { description: "", handler: async (to) => { paired = to; } });`,
    `  pi.registerCommand(${JSON.stringify(`${name}-paired`)}, { description: "", handler: async () => paired });`,
    `  pi.registerCommand(${JSON.stringify(`${name}-evaluation`)}, { description: "", handler: async () => evaluation });`,
    "}",
    "",
  ].join("\n");

// The declared package, whose extension sits in a folder with a package.json
// of its own and no `pi` field, as two of the harness's do: the manifest is
// the one above it.
const keyedPackage = (where, name) => {
  write(
    join(where, "package.json"),
    manifest({ extensions: ["./extensions"], sessionStateKeyed: true }),
  );
  write(join(where, "extensions", "probe", "package.json"), JSON.stringify({ type: "module" }));
  write(join(where, "extensions", "probe", "index.ts"), probe(name));
  return join(where, "extensions", "probe", "index.ts");
};
const keyed = keyedPackage(join(scratch, "keyed"), "keyed");
write(join(scratch, "unkeyed", "package.json"), manifest({ extensions: ["./index.ts"] }));
write(join(scratch, "unkeyed", "index.ts"), probe("unkeyed"));
const unkeyed = join(scratch, "unkeyed", "index.ts");
write(join(scratch, "loose", "index.ts"), probe("loose"));
const loose = join(scratch, "loose", "index.ts");
write(join(scratch, "half", "package.json"), manifest({ sessionStateKeyed: "yes" }));
write(join(scratch, "half", "index.ts"), probe("half"));
const half = join(scratch, "half", "index.ts");

const folder = (name) => {
  const where = join(scratch, "folders", name);
  mkdirSync(where, { recursive: true });
  return where;
};

// What the live conversations hold: the extensions each loaded. Letting them
// go is all a host does when one ends or parks.
const live = [];
// A conversation opening in a folder, which can run its probes' commands.
const open = async (name, entries = [keyed, unkeyed, loose, half]) => {
  const loaded = await loadExtensionsCached(entries, folder(name));
  assert.deepEqual(loaded.errors, [], `the probes didn't load in ${name}`);
  live.push(loaded);
  const run = (command, ...args) => {
    const owner = loaded.extensions.find((one) => one.commands.has(command));
    assert.ok(owner, `no probe in ${name} has ${command}`);
    return owner.commands.get(command).handler(...args);
  };
  return {
    evaluation: (probeName) => run(`${probeName}-evaluation`),
    pair: (probeName, to) => run(`${probeName}-pair`, to),
    paired: (probeName) => run(`${probeName}-paired`),
  };
};
const evaluations = () => ({ ...globalThis.evaluations });
const afresh = () => {
  clearExtensionCache();
  live.length = 0;
  globalThis.evaluations = {};
};
v8.setFlagsFromString("--expose-gc");
const gc = runInNewContext("gc");
const collect = async () => {
  await new Promise((done) => setImmediate(done));
  gc();
  await new Promise((done) => setImmediate(done));
};

// Two folders: the declared package is evaluated once for both, and every
// other probe once in each.
// In a scope of its own, so nothing but `live` holds these conversations.
afresh();
await (async () => {
  const a = await open("a");
  const b = await open("b");
  assert.deepEqual(
    evaluations(),
    { keyed: 1, unkeyed: 2, loose: 2, half: 2 },
    "only the package declaring its state keyed was shared across two folders",
  );
  assert.equal(await b.evaluation("keyed"), await a.evaluation("keyed"));

  // The red case: nvim paired in a's conversation stays unpaired in b's, for
  // every probe the declaration doesn't cover.
  for (const name of ["unkeyed", "loose", "half"]) {
    await a.pair(name, "nvim in a");
    assert.equal(await a.paired(name), "nvim in a", `${name} lost its own pairing`);
    assert.equal(await b.paired(name), undefined, `${name}'s pairing in a reached b`);
  }
})();

// Once every conversation has ended, the declared package's factory stays,
// so a conversation waking anywhere evaluates nothing of it, while the
// others load afresh.
live.length = 0;
await collect();
await open("c");
await open("a");
assert.deepEqual(
  evaluations(),
  { keyed: 1, unkeyed: 4, loose: 4, half: 4 },
  "a keyed package was evaluated again after every conversation ended, or another was kept",
);

// A reload empties the shared factories too.
clearExtensionCache();
await open("a");
assert.equal(evaluations().keyed, 2, "a reload reused a shared factory");

// A load begun before a reload caches nothing after it.
afresh();
const inFlight = open("a");
clearExtensionCache();
await inFlight;
await open("b");
assert.equal(evaluations().keyed, 2, "a load from before a reload cached a shared factory");

// Shared factories are keyed by the file a link points at: once it moves, the
// next open loads the new copy, whichever folder it opens in.
afresh();
const linked = join(scratch, "links", "keyed");
mkdirSync(dirname(linked), { recursive: true });
const first = join(scratch, "keyed-first");
cpSync(join(scratch, "keyed"), first, { recursive: true });
symlinkSync(first, linked);
const throughLink = join(linked, "extensions", "probe", "index.ts");
await open("a", [throughLink]);
await open("b", [throughLink]);
assert.equal(evaluations().keyed, 1, "two folders loading through one link didn't share");
rmSync(linked);
keyedPackage(join(scratch, "keyed-second"), "keyed-second");
symlinkSync(join(scratch, "keyed-second"), linked);
const moved = await open("a", [throughLink]);
assert.equal(await moved.evaluation("keyed-second"), 1, "a moved link kept its old copy");
await open("b", [throughLink]);
assert.equal(evaluations()["keyed-second"], 1, "the new copy wasn't shared");

rmSync(scratch, { recursive: true, force: true });

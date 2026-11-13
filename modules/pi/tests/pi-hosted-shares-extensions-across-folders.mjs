// pi as the host runs it keeps one factory for each extension file, whichever
// folder loads it, so a conversation opening or waking in any folder reuses
// the modules another conversation's extensions still hold rather than
// evaluating them again, and a folder's own extension, another file, stays
// its own. Unpatched, pi keeps one folder's and empties them whenever a load
// comes from another, so A, B, A evaluates the modules three times.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { cpSync, existsSync, mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
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

const scratch = mkdtempSync(join(tmpdir(), "pi-hosted-shared-"));
// An extension whose helper counts each evaluation of its module graph and
// whose command names the one its factory came from, so a load shows which
// evaluation it reused.
const extensionIn = (where) => {
  mkdirSync(where, { recursive: true });
  writeFileSync(
    join(where, "helper.ts"),
    "globalThis.evaluations = (globalThis.evaluations ?? 0) + 1;\n" +
      "export const evaluation = globalThis.evaluations;\n",
  );
  writeFileSync(
    join(where, "index.ts"),
    [
      'import { evaluation } from "./helper.js";',
      "export default function (pi) {",
      '  pi.registerCommand("evaluation-" + evaluation, { description: "", handler: async () => {} });',
      "}",
      "",
    ].join("\n"),
  );
  return join(where, "index.ts");
};
// The one every folder loads, found through a link as an installed package's
// are, so moving the link is what an install moving on does.
const installs = join(scratch, "installs");
extensionIn(join(installs, "v1"));
const current = join(scratch, "current");
symlinkSync(join(installs, "v1"), current);
const entry = join(current, "index.ts");
const folder = (name) => {
  const where = join(scratch, "folders", name);
  mkdirSync(where, { recursive: true });
  return where;
};

// What the live conversations hold: the extensions each loaded. One that
// ends or parks lets them go, which is all a host does to free them.
const live = [];
// The evaluation of each extension a conversation opening in a folder gets
// its factory from. It stays open unless told it ends at once.
const load = async (name, { ends = false, also = [] } = {}) => {
  const loaded = await loadExtensionsCached([entry, ...also], folder(name));
  assert.deepEqual(loaded.errors, [], `the extensions didn't load in ${name}`);
  if (!ends) {
    live.push(loaded);
  }
  return loaded.extensions.map((one) => Number([...one.commands.keys()][0].replace("evaluation-", "")));
};
const open = async (name, options) => (await load(name, options))[0];
const afresh = () => {
  clearExtensionCache();
  live.length = 0;
  globalThis.evaluations = 0;
};
// Collects what nothing holds, once the turns that last touched it are over.
v8.setFlagsFromString("--expose-gc");
const gc = runInNewContext("gc");
const collect = async () => {
  await new Promise((done) => setImmediate(done));
  gc();
  await new Promise((done) => setImmediate(done));
};

// Folders share one factory for one file, in any order.
afresh();
assert.deepEqual(
  [await open("a"), await open("b"), await open("a"), await open("c")],
  [1, 1, 1, 1],
  "a folder evaluated an extension another folder had loaded",
);
assert.equal(globalThis.evaluations, 1, "A, B, A, C evaluated the modules more than once");

// A folder's own extension is another file, so it gets its own evaluation and
// no other folder reuses it, while the shared one stays shared.
afresh();
await open("a");
const own = extensionIn(join(folder("own"), ".pi", "extensions", "own"));
assert.deepEqual(await load("own", { also: [own] }), [1, 2], "a folder's own extension was shared");
assert.deepEqual(await load("own", { also: [own] }), [1, 2], "a folder's own extension was evaluated again");
assert.equal(await open("b"), 1);

// A reload empties the cache, so it loads what's on disk now.
afresh();
await open("a");
clearExtensionCache();
assert.equal(await open("b"), 2, "a reload reused the modules");

// A load in flight when a reload comes caches nothing into the new generation.
afresh();
const inFlight = open("a");
clearExtensionCache();
assert.equal(await inFlight, 1);
assert.equal(await open("b"), 2, "a load begun before a reload cached into the one after it");

// Once the link moves on, a new conversation loads the copy it points at now,
// while one already open keeps the copy it loaded.
afresh();
assert.equal(await open("a"), 1);
cpSync(join(installs, "v1"), join(installs, "v2"), { recursive: true });
rmSync(current);
symlinkSync(join(installs, "v2"), current);
assert.equal(await open("b"), 2, "a conversation after the link moved got the old copy");
assert.equal(await open("a"), 2, "the moved link's copy wasn't shared");

// The factory stays while any conversation holds its extensions, goes once
// the last one ends, and the next open loads afresh.
afresh();
await open("a");
await open("b", { ends: true });
await collect();
assert.equal(await open("c"), 1, "the modules went while a conversation held them");
live.length = 0;
await collect();
assert.equal(await open("d"), 2, "the modules stayed once the last conversation ended");

rmSync(scratch, { recursive: true, force: true });

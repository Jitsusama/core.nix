// pi as the host runs it keeps the extensions it loaded for each folder, so
// a conversation opening or waking in a folder that already loaded them
// reuses that folder's modules rather than evaluating them again, and two
// folders never share a module. Unpatched, pi keeps one folder's and empties
// them whenever a load comes from another, so A, B, A evaluates A's twice.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

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

const scratch = mkdtempSync(join(tmpdir(), "pi-hosted-folders-"));
// One extension every folder loads, as a user's own are, whose helper counts
// each evaluation of its module graph and whose command names the one its
// factory came from, so a load shows which evaluation it reused.
const extension = join(scratch, "extension");
mkdirSync(extension);
writeFileSync(
  join(extension, "helper.ts"),
  "globalThis.evaluations = (globalThis.evaluations ?? 0) + 1;\n" +
    "export const evaluation = globalThis.evaluations;\n",
);
writeFileSync(
  join(extension, "index.ts"),
  [
    'import { evaluation } from "./helper.js";',
    "export default function (pi) {",
    '  pi.registerCommand("evaluation-" + evaluation, { description: "", handler: async () => {} });',
    "}",
    "",
  ].join("\n"),
);
const entry = join(extension, "index.ts");
const folder = (name) => {
  const where = join(scratch, "folders", name);
  mkdirSync(where, { recursive: true });
  return where;
};

// The evaluation a conversation opening in a folder gets its factory from.
const open = async (name) => {
  const loaded = await loadExtensionsCached([entry], folder(name));
  assert.deepEqual(loaded.errors, [], `the extension didn't load in ${name}`);
  const [command] = loaded.extensions.flatMap((one) => [...one.commands.keys()]);
  return Number(command.replace("evaluation-", ""));
};
const afresh = () => {
  clearExtensionCache();
  globalThis.evaluations = 0;
};

// Two in one folder share, a second folder gets its own, and the first
// folder's come back when a conversation opens there again.
afresh();
assert.deepEqual(
  [await open("a"), await open("a"), await open("b"), await open("a")],
  [1, 1, 2, 1],
  "a folder's extensions weren't kept once another folder loaded",
);
assert.equal(globalThis.evaluations, 2, "A, A, B, A evaluated the modules more than twice");

// Loads in two folders at once each keep what they loaded, as a host opening
// two conversations together does.
afresh();
const together = await Promise.all([open("a"), open("b")]);
assert.deepEqual(together.toSorted(), [1, 2], "two folders loading at once shared a module");
assert.deepEqual(
  [await open("a"), await open("b")],
  together,
  "a folder that loaded beside another kept nothing",
);
assert.equal(globalThis.evaluations, 2, "loading together lost a folder's modules");

// A reload empties every folder's, so it loads what's on disk now.
afresh();
await open("a");
await open("b");
clearExtensionCache();
assert.equal(await open("a"), 3, "a reload reused a folder's modules");

// The sixteen folders loaded most recently are kept: a seventeenth drops the
// one loaded longest ago, and a folder opened again counts as recent.
afresh();
const sixteen = Array.from({ length: 16 }, (_, at) => `f${at}`);
// One after another, since the order they load in is what's held.
await sixteen.reduce((before, name) => before.then(() => open(name)), Promise.resolve());
assert.equal(await open("f0"), 1, "one of sixteen folders lost its modules");
await open("f16");
assert.equal(globalThis.evaluations, 17);
assert.equal(await open("f0"), 1, "the folder opened most recently was dropped");
assert.equal(await open("f1"), 18, "past sixteen folders none was dropped");

rmSync(scratch, { recursive: true, force: true });

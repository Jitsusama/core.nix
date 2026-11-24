// pi as the host runs it evaluates each module of a package that declares
// its per-session state keyed once, whichever of the package's extensions
// imports it, rather than once for each. Every other package's extensions
// keep a copy each. A reload, or a link that has moved on, lets go of what
// a package's loads kept, so the next load reads its files as they are now,
// and a load begun before a reload hands nothing it read before to a load
// after. Without the patch, a keyed package's library is evaluated once for
// each of its extensions; sharing every package's modules lets one
// extension's module state reach another's.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";
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

// Resolved, since Node's module cache holds each file by its real path.
const scratch = realpathSync(mkdtempSync(join(tmpdir(), "pi-hosted-keyed-modules-")));
const write = (path, text) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
};
const manifest = (pi) => JSON.stringify({ name: "probe", type: "module", pi });

// A library counts each evaluation of it under its package's name, and says
// which version of its file it was evaluated from.
const library = (name, version) =>
  [
    "globalThis.evaluations ??= {};",
    `globalThis.evaluations[${JSON.stringify(name)}] = (globalThis.evaluations[${JSON.stringify(name)}] ?? 0) + 1;`,
    `export const version = ${JSON.stringify(version)};`,
    "",
  ].join("\n");
// An extension that imports its package's library, and says which version
// it was given, as `<extension>-version`.
const extension = (command) =>
  [
    'import { version } from "../../lib/shared.ts";',
    "export default function (pi) {",
    `  pi.registerCommand(${JSON.stringify(`${command}-version`)}, { description: "", handler: async () => version });`,
    "}",
    "",
  ].join("\n");
// A package of two extensions sharing one library, keyed or not.
const aPackage = (where, name, pi) => {
  write(join(where, "package.json"), manifest(pi));
  write(join(where, "lib", "shared.ts"), library(name, "first"));
  for (const one of ["one", "two"]) {
    write(join(where, "extensions", one, "index.ts"), extension(`${name}-${one}`));
  }
  return ["one", "two"].map((one) => join(where, "extensions", one, "index.ts"));
};

const keyedAt = join(scratch, "keyed");
const keyed = aPackage(keyedAt, "keyed", { extensions: ["./extensions"], sessionStateKeyed: true });
const unkeyed = aPackage(join(scratch, "unkeyed"), "unkeyed", { extensions: ["./extensions"] });

const folder = (name) => {
  const where = join(scratch, "folders", name);
  mkdirSync(where, { recursive: true });
  return where;
};
// A conversation opening in a folder, which can run its extensions' commands.
const open = async (name, entries) => {
  const loaded = await loadExtensionsCached(entries, folder(name));
  assert.deepEqual(loaded.errors, [], `the probes didn't load in ${name}`);
  return (command) => {
    const owner = loaded.extensions.find((one) => one.commands.has(command));
    assert.ok(owner, `no probe in ${name} has ${command}`);
    return owner.commands.get(command).handler();
  };
};
const evaluations = () => ({ ...globalThis.evaluations });
const afresh = () => {
  clearExtensionCache();
  globalThis.evaluations = {};
};
// The files Node's module cache holds under `where`.
const cached = (where) =>
  Object.keys(createRequire(import.meta.url).cache).filter((file) => file.startsWith(where + sep));

// A keyed package's two extensions are given one evaluation of its library;
// an unkeyed package's are given one each, so module state stays each
// extension's own.
afresh();
await open("a", [...keyed, ...unkeyed]);
assert.deepEqual(
  evaluations(),
  { keyed: 1, unkeyed: 2 },
  "a keyed package's library wasn't shared by its extensions, or an unkeyed one's was",
);

// A reload lets go of the library, so the next load reads its file as it is.
write(join(keyedAt, "lib", "shared.ts"), library("keyed", "second"));
afresh();
const reloaded = await open("a", keyed);
assert.equal(await reloaded("keyed-one-version"), "second", "a reload kept the library as it was");
assert.equal(await reloaded("keyed-two-version"), "second", "a reload kept the library as it was");
assert.equal(evaluations().keyed, 1, "the reloaded library wasn't shared");

// A load begun before a reload hands nothing it read before the reload to a
// load after it. Its extension reads `slow` before the reload and finishes it
// after, then waits in `last`; a load in that wait reads both files as they
// are after the reload.
afresh();
const racedAt = join(scratch, "raced");
write(
  join(racedAt, "package.json"),
  manifest({ extensions: ["./extensions"], sessionStateKeyed: true }),
);
// A module that says it was reached and waits for the test to let it go on;
// a load reaching it once the gate is put away goes straight through.
const gated = (name, version) =>
  [
    `globalThis.${name}Reached?.();`,
    `await globalThis.${name}Gate;`,
    `export const ${name} = ${JSON.stringify(version)};`,
    "",
  ].join("\n");
const racer = (version) => {
  write(join(racedAt, "lib", "slow.ts"), gated("slow", version));
  write(join(racedAt, "lib", "last.ts"), gated("last", version));
};
const gate = (name) => {
  const opened = Promise.withResolvers();
  const reached = Promise.withResolvers();
  globalThis[`${name}Gate`] = opened.promise;
  globalThis[`${name}Reached`] = reached.resolve;
  const putAway = () => {
    globalThis[`${name}Gate`] = undefined;
    globalThis[`${name}Reached`] = undefined;
  };
  return { open: opened.resolve, reached: reached.promise, putAway };
};
racer("before");
write(
  join(racedAt, "extensions", "race", "index.ts"),
  [
    'import { slow } from "../../lib/slow.ts";',
    'import { last } from "../../lib/last.ts";',
    "export default function (pi) {",
    '  pi.registerCommand("race-version", { description: "", handler: async () => slow + " " + last });',
    "}",
    "",
  ].join("\n"),
);
const race = [join(racedAt, "extensions", "race", "index.ts")];
const slowGate = gate("slow");
const lastGate = gate("last");
const early = open("early", race);
await slowGate.reached;
slowGate.putAway();
clearExtensionCache();
racer("after");
slowGate.open();
await lastGate.reached;
lastGate.putAway();
const during = await open("during", race);
assert.equal(
  await during("race-version"),
  "after after",
  "a load after a reload was given a module a load from before it read",
);
lastGate.open();
assert.equal(
  await (await early)("race-version"),
  "before after",
  "the early load didn't finish a module it read before the reload",
);

// A load begun before a reload, of a package no load after it resolves
// into, leaves nothing of it in the cache once it ends, not even a file it
// first read after the reload, which no later reload would let go of.
afresh();
const parkedAt = join(scratch, "parked");
write(
  join(parkedAt, "package.json"),
  manifest({ extensions: ["./extensions"], sessionStateKeyed: true }),
);
write(join(parkedAt, "lib", "slow.ts"), gated("slow", "parked"));
write(join(parkedAt, "lib", "after.ts"), 'export const after = "parked";\n');
write(
  join(parkedAt, "extensions", "parked", "index.ts"),
  [
    'import { slow } from "../../lib/slow.ts";',
    'import { after } from "../../lib/after.ts";',
    "// Used, since jiti's compile drops an import nothing uses.",
    "export default function () { return slow + after; }",
    "",
  ].join("\n"),
);
const parkedGate = gate("slow");
const parked = open("parked", [join(parkedAt, "extensions", "parked", "index.ts")]);
await parkedGate.reached;
parkedGate.putAway();
clearExtensionCache();
parkedGate.open();
await parked;
assert.deepEqual(cached(parkedAt), [], "a load from before a reload left its package in the cache");

// A link that moves on lets go of the copy it pointed at, once no extension
// resolves into it.
afresh();
const linked = join(scratch, "links", "keyed");
const firstCopy = join(scratch, "keyed-first");
aPackage(firstCopy, "keyed-first", { extensions: ["./extensions"], sessionStateKeyed: true });
mkdirSync(dirname(linked), { recursive: true });
symlinkSync(firstCopy, linked);
const throughLink = ["one", "two"].map((one) => join(linked, "extensions", one, "index.ts"));
await open("a", throughLink);
assert.ok(cached(firstCopy).length > 0, "the linked copy's modules weren't in the cache");
rmSync(linked);
const secondCopy = join(scratch, "keyed-second");
aPackage(secondCopy, "keyed-second", { extensions: ["./extensions"], sessionStateKeyed: true });
symlinkSync(secondCopy, linked);
const moved = await open("b", throughLink);
assert.equal(await moved("keyed-second-one-version"), "first", "a moved link kept its old copy");
assert.deepEqual(cached(firstCopy), [], "a copy no link points at stayed in the cache");
assert.equal(evaluations()["keyed-second"], 1, "the new copy's library wasn't shared");

rmSync(scratch, { recursive: true, force: true });

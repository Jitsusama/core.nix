// The hosted pi the keyed-modules test drives, not a test itself: pi's
// extension loader from the tree its test is run on (node TEST PI_ROOT), a
// scratch folder to write packages into, libraries that count each
// evaluation of them, gates that hold a load part-way through, and the
// packages and links the test loads.
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
/** pi's own reload, and its load of extensions into a folder. */
export const { clearExtensionCache, loadExtensionsCached } = await import(
  pathToFileURL(loaderModule).href
);

/** Where the test writes its packages, resolved, since Node's module cache
 * holds each file by its real path. */
export const scratch = realpathSync(mkdtempSync(join(tmpdir(), "pi-hosted-keyed-modules-")));
/** Writes `text` to `path`, making its folder. */
export const write = (path, text) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
};
/** A package's manifest, declaring `pi`. */
export const manifest = (pi) => JSON.stringify({ name: "probe", type: "module", pi });
/** What a package declaring its per-session state keyed says. */
export const keyedManifest = { extensions: ["./extensions"], sessionStateKeyed: true };

const nodeCache = createRequire(import.meta.url).cache;
/** The files Node's module cache holds under `where`. */
export const cached = (where) =>
  Object.keys(nodeCache).filter((file) => file.startsWith(where + sep));
// A file required here is seen in the cache, so its emptiness in the test
// means something.
const seen = join(scratch, "seen.cjs");
write(seen, "module.exports = 1;\n");
createRequire(import.meta.url)(seen);
assert.deepEqual(cached(scratch), [seen], "the cache check can't see what Node's cache holds");
delete nodeCache[seen];

/** Source that counts each evaluation of the module it opens under `name`. */
export const counted = (name) =>
  [
    "globalThis.evaluations ??= {};",
    `globalThis.evaluations[${JSON.stringify(name)}] = (globalThis.evaluations[${JSON.stringify(name)}] ?? 0) + 1;`,
  ].join("\n");
/** A library counting each evaluation of it under `name`, which says which
 * version of its file it was evaluated from. */
export const library = (name, version) =>
  [counted(name), `export const version = ${JSON.stringify(version)};`, ""].join("\n");

/** A conversation's folder, named `name`. */
export const folder = (name) => {
  const where = join(scratch, "folders", name);
  mkdirSync(where, { recursive: true });
  return where;
};
/** A conversation opening in a folder, which can run its extensions'
 * commands. */
export const open = async (name, entries) => {
  const loaded = await loadExtensionsCached(entries, folder(name));
  assert.deepEqual(loaded.errors, [], `the probes didn't load in ${name}`);
  return (command) => {
    const owner = loaded.extensions.find((one) => one.commands.has(command));
    assert.ok(owner, `no probe in ${name} has ${command}`);
    return owner.commands.get(command).handler();
  };
};
/** How many times each counted library has been evaluated. */
export const evaluations = () => ({ ...globalThis.evaluations });
/** Reloads, and starts the counts again. */
export const afresh = () => {
  clearExtensionCache();
  globalThis.evaluations = {};
};
/** A module that says it was reached and waits for the test to let it go
 * on; a load reaching it once the gate is put away goes straight through. */
export const gated = (name, version) =>
  [
    `globalThis.${name}Reached?.();`,
    `await globalThis.${name}Gate;`,
    `export const ${name} = ${JSON.stringify(version)};`,
    "",
  ].join("\n");
/** The gate a module `gated` under `name` waits at. */
export const gate = (name) => {
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
/** Settles as `promise` does, or fails saying `message` once `ms` have
 * passed, so a wait that never ends fails the check rather than hanging it. */
export const within = (promise, message, ms = 10_000) => {
  let timer;
  const late = new Promise((_, fail) => {
    timer = setTimeout(() => fail(new assert.AssertionError({ message })), ms);
  });
  return Promise.race([promise, late]).finally(() => clearTimeout(timer));
};
/** Whether `promise` is still pending once `ms` have passed. */
export const pendingAfter = (promise, ms) =>
  Promise.race([
    promise.then(
      () => false,
      () => false,
    ),
    new Promise((go) => setTimeout(go, ms, true)),
  ]);

/** Points the symlink `link` at `copy`, in place of where it pointed. */
export const pointAt = (link, copy) => {
  mkdirSync(dirname(link), { recursive: true });
  rmSync(link, { force: true });
  symlinkSync(copy, link);
};
// An extension that imports its package's library and the one outside
// every package. Its `<command>-version` says which versions it was given,
// and `<command>-late` imports a library of its package that no extension
// imports before it runs.
const extension = (command) =>
  [
    'import { version } from "../../lib/shared.ts";',
    'import { version as outside } from "../../../outside/helper.ts";',
    "export default function (pi) {",
    `  pi.registerCommand(${JSON.stringify(`${command}-version`)}, { description: "", handler: async () => version + " " + outside });`,
    `  pi.registerCommand(${JSON.stringify(`${command}-late`)}, { description: "", handler: async () => (await import("../../lib/late.ts")).version });`,
    "}",
    "",
  ].join("\n");
/** A package at `where` declaring `pi`, of two extensions sharing one
 * library counted under `name`, beside the scratch folder's `outside`;
 * its extensions' entries. */
export const aPackage = (where, name, pi, version = "first") => {
  write(join(where, "package.json"), manifest(pi));
  write(join(where, "lib", "shared.ts"), library(name, version));
  write(join(where, "lib", "late.ts"), library(`${name}-late`, version));
  for (const one of ["one", "two"]) {
    write(join(where, "extensions", one, "index.ts"), extension(`${name}-${one}`));
  }
  return ["one", "two"].map((one) => join(where, "extensions", one, "index.ts"));
};
/** A keyed package at `where` whose `hangs` extension waits at the gate
 * `hang` when `hangs`, and whose `behind` extension's library counts under
 * `behind`, then waits at the gate `release` should a test set one; or,
 * not `hangs`, one that waits at neither, declaring `pi`. Its entries. */
export const hungPackage = (where, hangs, pi = keyedManifest) => {
  write(join(where, "package.json"), manifest(pi));
  write(
    join(where, "lib", "hangs.ts"),
    hangs ? gated("hang", "hung") : 'export const hang = "moved";\n',
  );
  write(join(where, "lib", "release.ts"), gated("release", "released"));
  write(
    join(where, "lib", "shared.ts"),
    hangs ? library("behind", "behind") : library("behind-moved", "moved"),
  );
  write(
    join(where, "extensions", "hangs", "index.ts"),
    [
      'import { version } from "../../lib/shared.ts";',
      'import { hang } from "../../lib/hangs.ts";',
      "// Used, since jiti's compile drops an import nothing uses.",
      "export default function () { return [version, hang]; }",
      "",
    ].join("\n"),
  );
  write(
    join(where, "extensions", "behind", "index.ts"),
    [
      'import { version } from "../../lib/shared.ts";',
      hangs ? 'import { release } from "../../lib/release.ts";' : "const release = 0;",
      "export default function (pi) {",
      '  pi.registerCommand("behind-version", { description: "", handler: async () => [version, release][0] });',
      "}",
      "",
    ].join("\n"),
  );
  return ["hangs", "behind"].map((one) => join(where, "extensions", one, "index.ts"));
};
/** Hangs a load of `hangs`, queues one of `behind`, then lets go as `what`
 * does and expects the queued load to end within `ms` of the hung load
 * reaching its hang, evaluating afresh what the hung load had finished.
 * The default ends before a turn held 10 seconds is let go of, so only
 * `letGo` can end the wait. */
export const heldUpUntil = async (what, [hangs, behind], letGo, ms = 8000) => {
  const hangGate = gate("hang");
  const hanging = open(`${what}-hanging`, [hangs]);
  await hangGate.reached;
  const deadline = Date.now() + ms;
  const waiting = open(`${what}-waiting`, [behind]);
  assert.equal(
    await pendingAfter(waiting, 200),
    true,
    `a load didn't wait behind one that never ends, before ${what}`,
  );
  await letGo();
  const released = await within(
    waiting,
    `${what} didn't let go of a load waiting behind one that never ends`,
    Math.max(1, deadline - Date.now()),
  );
  assert.equal(await released("behind-version"), "behind", `a load ${what} let go of failed`);
  assert.equal(
    evaluations().behind,
    2,
    `a load ${what} let go of was given the modules it let go of`,
  );
  hangGate.putAway();
  hangGate.open();
  await hanging;
};

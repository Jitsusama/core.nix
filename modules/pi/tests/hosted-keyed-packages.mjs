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
/** Called before a load that may hold its package's turn is begun: waits
 * as `within` does, but only until 8 seconds from that call, so a wait that
 * only the 10-second bound on a held turn would end fails instead. */
export const beforeTheBound = () => {
  const deadline = Date.now() + 8000;
  return (promise, message) => within(promise, message, Math.max(1, deadline - Date.now()));
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
 * not `hangs`, one that waits at neither, declaring `pi`. Its `early`
 * extension imports only that library. Its entries. */
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
  const early = 'import { version } from "../../lib/shared.ts";\nexport default () => version;\n';
  write(join(where, "extensions", "early", "index.ts"), early);
  return ["hangs", "behind", "early"].map((one) => join(where, "extensions", one, "index.ts"));
};
/** A keyed package at `where` whose `slow` extension imports a library
 * that waits at the gate `name` and says it is `version`, which the
 * extension's `<name>-version` command answers; evaluating it counts under
 * `<name>-slow`. Its `still` extension imports nothing. Its folder. */
export const gatedPackage = (where, name, version) => {
  write(join(where, "package.json"), manifest(keyedManifest));
  write(join(where, "lib", "slow.ts"), gated(name, version));
  write(
    join(where, "extensions", "slow", "index.ts"),
    [
      `import { ${name} } from "../../lib/slow.ts";`,
      counted(`${name}-slow`),
      "export default function (pi) {",
      `  pi.registerCommand("${name}-version", { description: "", handler: async () => ${name} });`,
      "}",
      "",
    ].join("\n"),
  );
  write(join(where, "extensions", "still", "index.ts"), "export default function () {}\n");
  return where;
};
/** A keyed package at `where` whose library counts under `stalled`, with
 * the extensions `early`, `hangs`, `behind`, `meanwhile`, `after`, `holds`
 * and `waits`, each counting under `stalled-<name>`; `hangs` waits at the gate
 * `hang`, `behind` at the gate `lag` and `holds` at the gate `hold`. Its
 * entries by name, and its counts, the library's first. */
export const stalledPackage = (where) => {
  write(join(where, "package.json"), manifest(keyedManifest));
  write(join(where, "lib", "shared.ts"), library("stalled", "stalled"));
  write(join(where, "lib", "hangs.ts"), gated("hang", "hung"));
  write(join(where, "lib", "lags.ts"), gated("lag", "lagged"));
  write(join(where, "lib", "holds.ts"), gated("hold", "held"));
  const names = ["early", "hangs", "behind", "meanwhile", "after", "holds", "waits"];
  const entries = {};
  for (const one of names) {
    entries[one] = join(where, "extensions", one, "index.ts");
    const gatedBy = { hangs: "hangs", behind: "lags", holds: "holds" }[one];
    write(
      entries[one],
      [
        'import { version } from "../../lib/shared.ts";',
        gatedBy ? `import * as gated from "../../lib/${gatedBy}.ts";` : "const gated = 0;",
        counted(`stalled-${one}`),
        "// Used, since jiti's compile drops an import nothing uses.",
        "export default function () { return [version, gated]; }",
        "",
      ].join("\n"),
    );
  }
  const counts = () =>
    ["stalled", ...names.map((one) => `stalled-${one}`)].map((name) => evaluations()[name] ?? 0);
  return { entries, counts };
};
/** Has `holder`, which waits at the gate `hold`, take its package's turn,
 * then expects `kept` to open meanwhile without waiting, as a factory kept
 * for it does, and `waiting`, which must take the turn, to wait. */
export const opensWhileHeld = async (what, holder, kept, waiting) => {
  const holdGate = gate("hold");
  const holding = open(`${what}-holding`, [holder]);
  await holdGate.reached;
  const reopened = open(`${what}-reopened`, [kept]);
  const waited = open(`${what}-waited`, [waiting]);
  assert.equal(await pendingAfter(reopened, 200), false, `${what}: a kept factory waited a turn`);
  assert.equal(await pendingAfter(waited, 200), true, `${what}: a load didn't wait its turn`);
  holdGate.putAway();
  holdGate.open();
  const all = Promise.all([holding, reopened, waited]);
  await within(all, `${what}: a load holding its turn never ended`);
};
/** A library counting under `name` that throws part-way through. */
const breaks = (name) =>
  [
    counted(name),
    'export const before = "before";',
    'throw new Error("breaks part-way");',
    "",
  ].join("\n");
/** A keyed package at `where` whose `late`, `again` and `caught`
 * extensions each import, as they run, a library counting under `breaks`
 * that throws part-way through, with `breaks-<extension>`, and require one
 * counting under `breaks-required`, with `requires-<extension>`, answering
 * with the error's message; whose `caught` extension imports the first as
 * it loads and catches what it throws; and whose `loads` extension imports
 * the first as it loads. Its entries by name. */
export const breakingPackage = (where) => {
  write(join(where, "package.json"), manifest(keyedManifest));
  write(join(where, "lib", "breaks.ts"), breaks("breaks"));
  write(join(where, "lib", "required.ts"), breaks("breaks-required"));
  const entries = {};
  for (const one of ["late", "again", "caught"]) {
    entries[one] = join(where, "extensions", one, "index.ts");
    const catches = 'try { await import("../../lib/breaks.ts"); } catch { /* Left unfinished. */ }';
    write(
      entries[one],
      [
        one === "caught" ? catches : "",
        "export default function (pi) {",
        `  pi.registerCommand("breaks-${one}", { description: "", handler: () => import("../../lib/breaks.ts").then(() => "loaded", (error) => error.message) });`,
        `  pi.registerCommand("requires-${one}", { description: "", handler: async () => { try { require("../../lib/required.ts"); return "loaded"; } catch (error) { return error.message; } } });`,
        "}",
        "",
      ].join("\n"),
    );
  }
  entries.loads = join(where, "extensions", "loads", "index.ts");
  write(
    entries.loads,
    'import { before } from "../../lib/breaks.ts";\nexport default function () { return before; }\n',
  );
  return entries;
};
/** A keyed package at `where` whose one extension's `natives` imports and
 * requires, as it runs, what jiti doesn't evaluate itself: a builtin by
 * either name, a data URL, JSON and a module it leaves to Node, written as
 * ESM, which counts its evaluations, answering what each gave. Its entry. */
export const nativesPackage = (where) => {
  write(join(where, "package.json"), manifest(keyedManifest));
  write(join(where, "lib", "data.json"), '{ "version": "json" }\n');
  const typescript = join(where, "node_modules", "typescript");
  write(join(typescript, "package.json"), '{ "name": "typescript", "main": "index.js" }\n');
  write(
    join(typescript, "index.js"),
    "export const native = (globalThis.nativeEvaluations = (globalThis.nativeEvaluations ?? 0) + 1);\n",
  );
  const entry = join(where, "extensions", "one", "index.ts");
  write(
    entry,
    [
      "export default function (pi) {",
      '  pi.registerCommand("natives", { description: "", handler: async () => [',
      '    (await import("node:path")).sep, (await import("path")).sep, (await import("path/posix")).sep,',
      '    (await import("data:text/javascript,export default 1")).default,',
      '    (await import("../../lib/data.json")).version, require("node:path").sep, require("path").sep,',
      '    require("../../lib/data.json").version, (await import("typescript")).native,',
      '    require("typescript").native,',
      "  ] });",
      "}",
      "",
    ].join("\n"),
  );
  return entry;
};
/** A keyed package at `where` whose `loads` extension imports a library
 * waiting at the gate `first`, then one counting under `raced` that waits
 * at the gate `raced`, answering what the second said as its factory ran
 * with `raced-loads`; and whose `late` extension imports the second as it
 * runs, answering what it says at once with `raced-late`. Its two entries. */
export const racingPackage = (where) => {
  write(join(where, "package.json"), manifest(keyedManifest));
  write(join(where, "lib", "first.ts"), gated("first", "first"));
  write(join(where, "lib", "raced.ts"), `${counted("raced")}\n${gated("raced", "whole")}`);
  const loads = join(where, "extensions", "loads", "index.ts");
  write(
    loads,
    [
      'import { first } from "../../lib/first.ts";',
      'import { raced } from "../../lib/raced.ts";',
      "export default function (pi) {",
      "  const seen = first && raced;",
      '  pi.registerCommand("raced-loads", { description: "", handler: async () => seen });',
      "}",
      "",
    ].join("\n"),
  );
  const late = join(where, "extensions", "late", "index.ts");
  write(
    late,
    [
      "export default function (pi) {",
      '  pi.registerCommand("raced-late", { description: "", handler: async () => (await import("../../lib/raced.ts")).raced });',
      "}",
      "",
    ].join("\n"),
  );
  return { loads, late };
};
/** A keyed package at `where` whose one extension imports a library that
 * imports one counting under `parented`, then one waiting at the gate
 * `parented`. The first library sets `globalThis.parentedLater`, which
 * imports, as the extension runs, a third library that imports the second
 * and answers what that says, and `globalThis.parentedWaits`, which imports
 * the one waiting and answers what it says. Its entry. */
export const parentedPackage = (where) => {
  write(join(where, "package.json"), manifest(keyedManifest));
  write(join(where, "lib", "counted.ts"), library("parented", "parented"));
  write(
    join(where, "lib", "early.ts"),
    [
      'import { version } from "./counted.ts";',
      'globalThis.parentedLater = async () => (await import("./uses.ts")).used;',
      'globalThis.parentedWaits = async () => (await import("./waits.ts")).parented;',
      "export const early = version;",
      "",
    ].join("\n"),
  );
  write(
    join(where, "lib", "uses.ts"),
    'import { version } from "./counted.ts";\nexport const used = version;\n',
  );
  write(join(where, "lib", "waits.ts"), gated("parented", "waited"));
  const entry = join(where, "extensions", "one", "index.ts");
  write(
    entry,
    [
      'import { early } from "../../lib/early.ts";',
      'import { parented } from "../../lib/waits.ts";',
      "export default function () { return [early, parented]; }",
      "",
    ].join("\n"),
  );
  return entry;
};
/** A keyed package at `where` whose `late` extension's `mixed-late`
 * imports, as it runs, a library counting under `mixed-late` that imports
 * one counting under `mixed-deeper`, and whose `fails` extension imports a
 * library that waits at the gate `wait` and one that throws. Its two
 * entries. */
export const mixedPackage = (where) => {
  write(join(where, "package.json"), manifest(keyedManifest));
  write(join(where, "lib", "deeper.ts"), library("mixed-deeper", "deeper"));
  write(
    join(where, "lib", "late.ts"),
    `import { version as deeper } from "./deeper.ts";\n${library("mixed-late", "late")}export const deep = deeper;\n`,
  );
  write(join(where, "lib", "waits.ts"), gated("wait", "waited"));
  write(
    join(where, "lib", "throws.ts"),
    'throw new Error("never loads");\nexport const never = 1;\n',
  );
  const runsLate = join(where, "extensions", "late", "index.ts");
  write(
    runsLate,
    [
      "export default function (pi) {",
      '  pi.registerCommand("mixed-late", { description: "", handler: async () => (await import("../../lib/late.ts")).version });',
      "}",
      "",
    ].join("\n"),
  );
  const fails = join(where, "extensions", "fails", "index.ts");
  write(
    fails,
    [
      'import { wait } from "../../lib/waits.ts";',
      'import { never } from "../../lib/throws.ts";',
      "export default function () { return [wait, never]; }",
      "",
    ].join("\n"),
  );
  return { runsLate, fails };
};
/** Loads `early`, hangs a load of `hangs`, queues one of `behind`, then
 * lets go as `what` does and expects the queued load to end before the
 * bound on a held turn could let it go on, evaluating afresh what the
 * package had finished, so only `letGo` can end the wait. */
export const heldUpUntil = async (what, [hangs, behind, early], letGo) => {
  await within(open(`${what}-early`, [early]), `a load before ${what} never ended`);
  const hangGate = gate("hang");
  const inTime = beforeTheBound();
  const hanging = open(`${what}-hanging`, [hangs]);
  await hangGate.reached;
  const waiting = open(`${what}-waiting`, [behind]);
  assert.equal(
    await pendingAfter(waiting, 200),
    true,
    `a load didn't wait behind one that never ends, before ${what}`,
  );
  await letGo();
  const released = await inTime(
    waiting,
    `${what} didn't let go of a load waiting behind one that never ends`,
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

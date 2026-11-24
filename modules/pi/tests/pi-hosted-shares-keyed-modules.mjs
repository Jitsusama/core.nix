// pi as the host runs it evaluates each module that a package declaring its
// per-session state keyed reaches once, whichever of the package's
// extensions imports it, while every other package's extensions keep a copy
// each. Node's module cache holds none of it. A reload, or a link that has
// moved on, lets go of what a package's loads kept, so the next load reads
// its files as they are now, and a load begun before a reload hands nothing
// to a load after. A package's loads take turns, so none is given a module
// another is still evaluating, a reload lets go of loads waiting behind one
// that never ends, and a failed load's retry evaluates again every module it
// added. Without the patch, a keyed package's library is
// evaluated once for each of its extensions; sharing every package's modules
// lets one extension's module state reach another's.
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
const keyedManifest = { extensions: ["./extensions"], sessionStateKeyed: true };

// The files Node's module cache holds under `where`.
const nodeCache = createRequire(import.meta.url).cache;
const cached = (where) => Object.keys(nodeCache).filter((file) => file.startsWith(where + sep));
// A file required here is seen in the cache, so its emptiness below means
// something.
const seen = join(scratch, "seen.cjs");
write(seen, "module.exports = 1;\n");
createRequire(import.meta.url)(seen);
assert.deepEqual(cached(scratch), [seen], "the cache check can't see what Node's cache holds");
delete nodeCache[seen];

// A library counts each evaluation of it under its name, and says which
// version of its file it was evaluated from.
const counted = (name) =>
  [
    "globalThis.evaluations ??= {};",
    `globalThis.evaluations[${JSON.stringify(name)}] = (globalThis.evaluations[${JSON.stringify(name)}] ?? 0) + 1;`,
  ].join("\n");
const library = (name, version) =>
  [counted(name), `export const version = ${JSON.stringify(version)};`, ""].join("\n");
// A library outside every package, which extensions of both kinds import.
const outsideAt = join(scratch, "outside", "helper.ts");
write(outsideAt, library("outside", "first"));
// An extension that imports its package's library and the outside one. Its
// `<command>-version` says which versions it was given, and
// `<command>-late` imports a library of its package that no extension
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
// A package of two extensions sharing one library, keyed or not.
const aPackage = (where, name, pi, version = "first") => {
  write(join(where, "package.json"), manifest(pi));
  write(join(where, "lib", "shared.ts"), library(name, version));
  write(join(where, "lib", "late.ts"), library(`${name}-late`, version));
  for (const one of ["one", "two"]) {
    write(join(where, "extensions", one, "index.ts"), extension(`${name}-${one}`));
  }
  return ["one", "two"].map((one) => join(where, "extensions", one, "index.ts"));
};

const keyedAt = join(scratch, "keyed");
const keyed = aPackage(keyedAt, "keyed", keyedManifest);
const other = aPackage(join(scratch, "other"), "other", keyedManifest);
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
// A module that says it was reached and waits for the test to let it go on;
// a load reaching it once the gate is put away goes straight through.
const gated = (name, version) =>
  [
    `globalThis.${name}Reached?.();`,
    `await globalThis.${name}Gate;`,
    `export const ${name} = ${JSON.stringify(version)};`,
    "",
  ].join("\n");
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
// Settles as `promise` does, or fails saying `message` once `ms` have
// passed, so a wait that never ends fails the check rather than hanging it.
const within = (promise, message, ms = 10_000) => {
  let timer;
  const late = new Promise((_, fail) => {
    timer = setTimeout(() => fail(new assert.AssertionError({ message })), ms);
  });
  return Promise.race([promise, late]).finally(() => clearTimeout(timer));
};
// Whether `promise` is still pending once `ms` have passed.
const pendingAfter = (promise, ms) =>
  Promise.race([
    promise.then(
      () => false,
      () => false,
    ),
    new Promise((go) => setTimeout(go, ms, true)),
  ]);

// A keyed package's two extensions are given one evaluation of each module
// they reach, its own library and the outside one alike, and so are imports
// they make as they run. Another keyed package is given its own, and an
// unkeyed package's extensions one each, so module state stays each
// extension's own, even of a file a keyed load evaluated.
afresh();
const first = await open("a", [...keyed, ...other, ...unkeyed]);
assert.deepEqual(
  evaluations(),
  { keyed: 1, other: 1, unkeyed: 2, outside: 4 },
  "a keyed package's modules weren't shared by its extensions, or were shared beyond them",
);
for (const command of [
  "keyed-one-late",
  "keyed-two-late",
  "unkeyed-one-late",
  "unkeyed-two-late",
]) {
  assert.equal(await first(command), "first", `${command} was given another library`);
}
assert.deepEqual(
  [evaluations()["keyed-late"], evaluations()["unkeyed-late"]],
  [1, 2],
  "imports a keyed package's extensions made as they ran weren't shared, or an unkeyed one's were",
);
assert.deepEqual(cached(scratch), [], "a load put modules in Node's cache");

// A reload lets go of every module a keyed load reached, so the next load
// reads their files as they are.
write(join(keyedAt, "lib", "shared.ts"), library("keyed", "second"));
write(outsideAt, library("outside", "second"));
afresh();
const reloaded = await open("a", keyed);
assert.equal(
  await reloaded("keyed-one-version"),
  "second second",
  "a reload kept a module as it was",
);
assert.equal(
  await reloaded("keyed-two-version"),
  "second second",
  "a reload kept a module as it was",
);
assert.deepEqual(evaluations(), { keyed: 1, outside: 1 }, "the reloaded modules weren't shared");

// A load begun before a reload hands nothing it read before the reload to a
// load after it, while it runs or once it has ended. Its extension reads
// `slow` before the reload and finishes it after, then waits in `last`; a
// load in that wait, and one after the early load ends, read both files as
// they are after the reload.
afresh();
const racedAt = join(scratch, "raced");
write(join(racedAt, "package.json"), manifest(keyedManifest));
const racer = (version) => {
  write(join(racedAt, "lib", "slow.ts"), gated("slow", version));
  write(join(racedAt, "lib", "last.ts"), gated("last", version));
};
racer("before");
const race = ["race", "later"].map((name) => {
  write(
    join(racedAt, "extensions", name, "index.ts"),
    [
      'import { slow } from "../../lib/slow.ts";',
      'import { last } from "../../lib/last.ts";',
      "export default function (pi) {",
      `  pi.registerCommand("${name}-version", { description: "", handler: async () => slow + " " + last });`,
      "}",
      "",
    ].join("\n"),
  );
  return join(racedAt, "extensions", name, "index.ts");
});
const slowGate = gate("slow");
const lastGate = gate("last");
const early = open("early", [race[0]]);
await slowGate.reached;
slowGate.putAway();
clearExtensionCache();
racer("after");
slowGate.open();
await lastGate.reached;
lastGate.putAway();
const during = await within(
  open("during", [race[0]]),
  "a load after a reload waited for one from before it",
);
assert.equal(
  await during("race-version"),
  "after after",
  "a load after a reload was given a module a load from before it read",
);
lastGate.open();
assert.equal(
  await (await within(early, "the early load never ended"))("race-version"),
  "before after",
  "the early load didn't finish a module it read before the reload",
);
const later = await within(open("later", [race[1]]), "a load after the early one never ended");
assert.equal(
  await later("later-version"),
  "after after",
  "a load after the early one ended was given a module it read",
);

// A keyed package's loads take turns: two extensions loading at once, one of
// them twice, wait while the first evaluates their library, are each given
// it once it has finished, and the extension loading twice is evaluated
// once. Each reads the library as its factory runs, so one given it
// part-way through sees nothing.
afresh();
const turnsAt = join(scratch, "turns");
write(join(turnsAt, "package.json"), manifest(keyedManifest));
write(join(turnsAt, "lib", "gated.ts"), gated("turn", "ready"));
const turns = ["one", "two"].map((name) => {
  write(
    join(turnsAt, "extensions", name, "index.ts"),
    [
      'import { turn } from "../../lib/gated.ts";',
      counted(`turns-${name}`),
      "export default function (pi) {",
      "  const seen = turn;",
      `  pi.registerCommand("${name}-seen", { description: "", handler: async () => seen });`,
      "}",
      "",
    ].join("\n"),
  );
  return join(turnsAt, "extensions", name, "index.ts");
});
const turnGate = gate("turn");
const atOnce = [open("x", [turns[0]]), open("y", [turns[1]]), open("z", [turns[0]])];
await turnGate.reached;
assert.deepEqual(
  await Promise.all(atOnce.slice(1).map((load) => pendingAfter(load, 100))),
  [true, true],
  "a load didn't wait its turn",
);
turnGate.putAway();
turnGate.open();
const [x, y, z] = await within(Promise.all(atOnce), "loads taking turns never ended");
assert.deepEqual(
  [await x("one-seen"), await y("two-seen"), await z("one-seen")],
  ["ready", "ready", "ready"],
  "a load was given a module another was still evaluating",
);
assert.equal(
  evaluations()["turns-one"],
  1,
  "an extension loading twice at once was evaluated twice",
);

// A failed load forgets every module it added, so a retry evaluates again
// what it left unfinished and what it finished alike.
afresh();
const flakyAt = join(scratch, "flaky");
write(join(flakyAt, "package.json"), manifest(keyedManifest));
write(join(flakyAt, "lib", "steady.ts"), library("steady", "steady"));
write(
  join(flakyAt, "lib", "flaky.ts"),
  [
    counted("flaky"),
    'if (globalThis.flakyFails) throw new Error("not yet");',
    'export const flaky = "recovered";',
    "",
  ].join("\n"),
);
const flaky = join(flakyAt, "extensions", "flaky", "index.ts");
write(
  flaky,
  [
    'import { version } from "../../lib/steady.ts";',
    'import { flaky } from "../../lib/flaky.ts";',
    "export default function (pi) {",
    '  pi.registerCommand("flaky-version", { description: "", handler: async () => version + " " + flaky });',
    "}",
    "",
  ].join("\n"),
);
globalThis.flakyFails = true;
const failed = await loadExtensionsCached([flaky], folder("failed"));
assert.equal(failed.errors.length, 1, "a load whose library throws didn't fail");
globalThis.flakyFails = false;
const retried = await open("retried", [flaky]);
assert.equal(
  await retried("flaky-version"),
  "steady recovered",
  "a retry was given what a failed load left unfinished",
);
assert.deepEqual(
  evaluations(),
  { steady: 2, flaky: 2 },
  "a retry was given a module a failed load added",
);

// A link that moves on lets go of the copy it pointed at, once no extension
// resolves into it, so moving back reads that copy as it is now.
afresh();
write(outsideAt, library("outside", "link"));
const pointAt = (link, copy) => {
  mkdirSync(dirname(link), { recursive: true });
  rmSync(link, { force: true });
  symlinkSync(copy, link);
};
const linked = join(scratch, "links", "keyed");
const firstCopy = join(scratch, "keyed-first");
aPackage(firstCopy, "keyed-first", keyedManifest);
pointAt(linked, firstCopy);
const throughLink = ["one", "two"].map((one) => join(linked, "extensions", one, "index.ts"));
const before = await open("a", throughLink);
assert.equal(await before("keyed-first-one-version"), "first link", "the link's copy didn't load");
const secondCopy = join(scratch, "keyed-second");
aPackage(secondCopy, "keyed-second", keyedManifest, "second");
pointAt(linked, secondCopy);
const moved = await open("b", throughLink);
assert.equal(
  await moved("keyed-second-one-version"),
  "second link",
  "a moved link kept its old copy",
);
assert.equal(evaluations()["keyed-second"], 1, "the new copy's library wasn't shared");
write(join(firstCopy, "lib", "shared.ts"), library("keyed-first", "third"));
pointAt(linked, firstCopy);
const back = await open("c", throughLink);
assert.equal(
  await back("keyed-first-one-version"),
  "third link",
  "a copy no link pointed at was kept",
);

// A link that moves on keeps the copy it pointed at while another of the
// package's extensions still resolves into it, so that extension's modules
// stay one graph.
afresh();
const held = join(scratch, "links", "held");
const heldFirst = join(scratch, "held-first");
aPackage(heldFirst, "held-first", keyedManifest);
aPackage(join(scratch, "held-second"), "held-second", keyedManifest);
pointAt(held, heldFirst);
const throughHeld = ["one", "two"].map((one) => join(held, "extensions", one, "index.ts"));
await open("held-a", throughHeld);
pointAt(held, join(scratch, "held-second"));
await open("held-b", [throughHeld[0]]);
await open("held-c", [join(heldFirst, "extensions", "one", "index.ts")]);
assert.equal(
  evaluations()["held-first"],
  1,
  "a copy another extension still resolved into was let go",
);

// A load that never ends holds up the package's loads behind it until its
// modules are let go, by a reload or by a link moving off its copy, and
// those then load as an unkeyed package's do.
const hungPackage = (where, hangs) => {
  write(join(where, "package.json"), manifest(keyedManifest));
  write(
    join(where, "lib", "hangs.ts"),
    hangs ? gated("hang", "hung") : 'export const hang = "moved";\n',
  );
  write(join(where, "lib", "shared.ts"), library("behind", "behind"));
  write(
    join(where, "extensions", "hangs", "index.ts"),
    [
      'import { hang } from "../../lib/hangs.ts";',
      "// Used, since jiti's compile drops an import nothing uses.",
      "export default function () { return hang; }",
      "",
    ].join("\n"),
  );
  write(
    join(where, "extensions", "behind", "index.ts"),
    [
      'import { version } from "../../lib/shared.ts";',
      "export default function (pi) {",
      '  pi.registerCommand("behind-version", { description: "", handler: async () => version });',
      "}",
      "",
    ].join("\n"),
  );
  return ["hangs", "behind"].map((one) => join(where, "extensions", one, "index.ts"));
};
// Hangs a load of `hangs`, queues one of `behind`, then lets go as `what`
// does and expects the queued load to end.
const heldUpUntil = async (what, [hangs, behind], letGo) => {
  const hangGate = gate("hang");
  const hanging = open(`${what}-hanging`, [hangs]);
  await hangGate.reached;
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
  );
  assert.equal(await released("behind-version"), "behind", `a load ${what} let go of failed`);
  hangGate.putAway();
  hangGate.open();
  await hanging;
};
afresh();
await heldUpUntil("reload", hungPackage(join(scratch, "hung"), true), () => clearExtensionCache());
afresh();
const stuck = join(scratch, "links", "hung");
hungPackage(join(scratch, "hung-first"), true);
hungPackage(join(scratch, "hung-second"), false);
pointAt(stuck, join(scratch, "hung-first"));
const throughStuck = ["hangs", "behind"].map((one) => join(stuck, "extensions", one, "index.ts"));
await heldUpUntil("link", throughStuck, async () => {
  pointAt(stuck, join(scratch, "hung-second"));
  await within(open("link-moved", throughStuck), "a load through a moved link never ended");
});

assert.deepEqual(cached(scratch), [], "a load put modules in Node's cache");
rmSync(scratch, { recursive: true, force: true });

// pi as the host runs it evaluates each module that a package declaring its
// per-session state keyed reaches once, whichever of the package's
// extensions imports it, while every other package's extensions keep a copy
// each. Node's module cache holds none of it. A reload, or a link that has
// moved on, lets go of what a package's loads kept, so the next load reads
// its files as they are now, and a load begun before a reload hands nothing
// to a load after. A package's loads take turns, so none is given a module
// another is still evaluating; a reload, a moved link or a turn held too long
// lets go of loads waiting behind one that never ends, and none is kept
// for the copy the link moved to or once its modules are let go. A failed
// load's retry evaluates again every module it was first to import, however
// deep, but none another extension was first to import. Without the patch,
// a keyed package's library is evaluated once for each of its extensions;
// sharing every package's modules lets one extension's module state reach
// another's.
// Run as: node THIS PI_ROOT, with the helper beside it.
import assert from "node:assert/strict";
import { rmSync } from "node:fs";
import { join } from "node:path";
import {
  aPackage,
  afresh,
  cached,
  clearExtensionCache,
  counted,
  evaluations,
  folder,
  gate,
  gated,
  heldUpUntil,
  hungPackage,
  keyedManifest,
  library,
  loadExtensionsCached,
  manifest,
  open,
  pendingAfter,
  pointAt,
  scratch,
  within,
  write,
} from "./hosted-keyed-packages.mjs";
// A library outside every package, which extensions of both kinds import.
const outsideAt = join(scratch, "outside", "helper.ts");
write(outsideAt, library("outside", "first"));

const keyedAt = join(scratch, "keyed");
const keyed = aPackage(keyedAt, "keyed", keyedManifest);
const other = aPackage(join(scratch, "other"), "other", keyedManifest);
const unkeyed = aPackage(join(scratch, "unkeyed"), "unkeyed", { extensions: ["./extensions"] });

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

// A failed load forgets every module it was first to import, through any
// module it imported, so a retry evaluates again what it left unfinished
// and what it finished alike.
afresh();
const flakyAt = join(scratch, "flaky");
write(join(flakyAt, "package.json"), manifest(keyedManifest));
write(join(flakyAt, "lib", "deep.ts"), library("deep", "deep"));
write(
  join(flakyAt, "lib", "steady.ts"),
  `import { version as deep } from "./deep.ts";\n${library("steady", "steady")}export const deeper = deep;\n`,
);
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
  { steady: 2, flaky: 2, deep: 2 },
  "a retry was given a module a failed load added",
);

// A failed load forgets only what it was first to import: a module another
// of the package's extensions imports as it runs, while the failed load
// waits, stays, and so does one that module imports, so each is evaluated
// once.
afresh();
const mixedAt = join(scratch, "mixed");
write(join(mixedAt, "package.json"), manifest(keyedManifest));
write(join(mixedAt, "lib", "deeper.ts"), library("mixed-deeper", "deeper"));
write(
  join(mixedAt, "lib", "late.ts"),
  `import { version as deeper } from "./deeper.ts";\n${library("mixed-late", "late")}export const deep = deeper;\n`,
);
write(join(mixedAt, "lib", "waits.ts"), gated("wait", "waited"));
write(
  join(mixedAt, "lib", "throws.ts"),
  'throw new Error("never loads");\nexport const never = 1;\n',
);
const runsLate = join(mixedAt, "extensions", "late", "index.ts");
write(
  runsLate,
  [
    "export default function (pi) {",
    '  pi.registerCommand("mixed-late", { description: "", handler: async () => (await import("../../lib/late.ts")).version });',
    "}",
    "",
  ].join("\n"),
);
const fails = join(mixedAt, "extensions", "fails", "index.ts");
write(
  fails,
  [
    'import { wait } from "../../lib/waits.ts";',
    'import { never } from "../../lib/throws.ts";',
    "export default function () { return [wait, never]; }",
    "",
  ].join("\n"),
);
const mixed = await open("mixed", [runsLate]);
const waitGate = gate("wait");
const failing = loadExtensionsCached([fails], folder("mixed-failing"));
await waitGate.reached;
assert.equal(await mixed("mixed-late"), "late", "an import beside a waiting load failed");
waitGate.putAway();
waitGate.open();
assert.equal((await failing).errors.length, 1, "a load whose library throws didn't fail");
assert.equal(await mixed("mixed-late"), "late", "an import after a failed load failed");
assert.deepEqual(
  evaluations(),
  { "mixed-late": 1, "mixed-deeper": 1 },
  "a failed load forgot a module another extension imported meanwhile",
);

// A link that moves on lets go of the copy it pointed at, once no extension
// resolves into it still, though one of the link's extensions last loaded
// from it, so moving back reads that copy as it is now.
afresh();
write(outsideAt, library("outside", "link"));
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
const moved = await open("b", [throughLink[0]]);
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
const heldOne = join(heldFirst, "extensions", "one", "index.ts");
await open("held-a", throughHeld);
await open("held-direct", [heldOne]);
pointAt(held, join(scratch, "held-second"));
await open("held-b", [throughHeld[0]]);
await open("held-c", [heldOne]);
assert.equal(
  evaluations()["held-first"],
  1,
  "a copy another extension still resolved into was let go",
);

// A load that never ends holds up the package's loads behind it until its
// modules are let go, by a reload, by a link moving off its copy to a keyed
// copy or an unkeyed one, or by its turn being held too long, and those
// then load as an unkeyed package's do, evaluating afresh what the load
// that never ends had finished. One a link let go of, ending after the
// moved link's own load, isn't kept for the copy the link moved to.
afresh();
await heldUpUntil("reload", hungPackage(join(scratch, "hung"), true), () => clearExtensionCache());
afresh();
const stuck = join(scratch, "links", "hung");
hungPackage(join(scratch, "hung-first"), true);
hungPackage(join(scratch, "hung-second"), false);
pointAt(stuck, join(scratch, "hung-first"));
const throughStuck = ["hangs", "behind"].map((one) => join(stuck, "extensions", one, "index.ts"));
const releaseGate = gate("release");
await heldUpUntil("link", throughStuck, async () => {
  pointAt(stuck, join(scratch, "hung-second"));
  await within(open("link-moved", throughStuck), "a load through a moved link never ended");
  releaseGate.putAway();
  releaseGate.open();
});
const afterMove = await open("link-after", throughStuck);
assert.equal(
  await afterMove("behind-version"),
  "moved",
  "a load a moved link let go of was kept for the copy it moved to",
);
afresh();
const toUnkeyed = join(scratch, "links", "unkeyed");
hungPackage(join(scratch, "unkeyed-first"), true);
hungPackage(join(scratch, "unkeyed-second"), false, { extensions: ["./extensions"] });
pointAt(toUnkeyed, join(scratch, "unkeyed-first"));
const throughUnkeyed = ["hangs", "behind"].map((one) =>
  join(toUnkeyed, "extensions", one, "index.ts"),
);
await heldUpUntil("a link to an unkeyed copy", throughUnkeyed, async () => {
  pointAt(toUnkeyed, join(scratch, "unkeyed-second"));
  await within(open("unkeyed-moved", throughUnkeyed), "a load through a moved link never ended");
});
afresh();
const stalled = hungPackage(join(scratch, "stalled"), true);
await heldUpUntil("a turn held too long", stalled, () => undefined, 30_000);
await open("stalled-after", [stalled[1]]);
await open("stalled-again", [stalled[1]]);
assert.equal(evaluations().behind, 3, "loads after a held turn didn't share new modules");

// A load in flight as its link moves to another copy, while the copy it
// began in is still held, isn't kept for the copy the link moved to.
afresh();
const flight = join(scratch, "links", "flight");
const [flightFirst, flightSecond] = ["first", "second"].map((version) => {
  const where = join(scratch, `flight-${version}`);
  write(join(where, "package.json"), manifest(keyedManifest));
  write(join(where, "lib", "slow.ts"), gated("flight", version));
  write(
    join(where, "extensions", "slow", "index.ts"),
    [
      'import { flight } from "../../lib/slow.ts";',
      "export default function (pi) {",
      '  pi.registerCommand("flight-version", { description: "", handler: async () => flight });',
      "}",
      "",
    ].join("\n"),
  );
  write(join(where, "extensions", "still", "index.ts"), "export default function () {}\n");
  return where;
});
await open("flight-still", [join(flightFirst, "extensions", "still", "index.ts")]);
pointAt(flight, flightFirst);
const flightGate = gate("flight");
const inFlight = open("flight-a", [join(flight, "extensions", "slow", "index.ts")]);
await flightGate.reached;
pointAt(flight, flightSecond);
flightGate.putAway();
flightGate.open();
await within(inFlight, "a load in flight as its link moved never ended");
const landed = await open("flight-b", [join(flight, "extensions", "slow", "index.ts")]);
assert.equal(
  await landed("flight-version"),
  "second",
  "a load in flight as its link moved was kept for the copy it moved to",
);

// A load in flight as its link moves away and back, its copy's modules let
// go of meanwhile, isn't kept, so the next load shares the copy's new ones.
afresh();
const aba = join(scratch, "links", "aba");
const [abaFirst, abaSecond] = ["first", "second"].map((version) => {
  const where = join(scratch, `aba-${version}`);
  write(join(where, "package.json"), manifest(keyedManifest));
  write(join(where, "lib", "slow.ts"), gated("aba", version));
  write(
    join(where, "extensions", "slow", "index.ts"),
    [
      'import { aba } from "../../lib/slow.ts";',
      counted("aba-slow"),
      "export default function () { return aba; }",
      "",
    ].join("\n"),
  );
  write(join(where, "extensions", "other", "index.ts"), "export default function () {}\n");
  return where;
});
pointAt(aba, abaFirst);
const [abaSlow, abaOther] = ["slow", "other"].map((one) =>
  join(aba, "extensions", one, "index.ts"),
);
await open("aba-other", [abaOther]);
const abaGate = gate("aba");
const abaInFlight = open("aba-a", [abaSlow]);
await abaGate.reached;
pointAt(aba, abaSecond);
await open("aba-b", [abaOther]);
pointAt(aba, abaFirst);
await open("aba-c", [abaOther]);
abaGate.putAway();
abaGate.open();
await within(abaInFlight, "a load in flight as its link moved away and back never ended");
await open("aba-d", [abaSlow]);
assert.equal(evaluations()["aba-slow"], 2, "a load through modules let go of was kept");

assert.deepEqual(cached(scratch), [], "a load put modules in Node's cache");
rmSync(scratch, { recursive: true, force: true });

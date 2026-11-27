// pi as the host runs it shares a keyed package's modules only once they
// have finished evaluating. A failed load's retry evaluates again every
// module the failed load evaluated, however deep, but none another
// extension finished meanwhile, and the retry's modules are shared after.
// An import made as an extension runs, after its load, is given what the
// package finished, and otherwise evaluates what it needs in an evaluation
// of its own: one that throws leaves nothing for a load or a sibling's
// import, asynchronous or synchronous, after it; one racing a load is never
// given a module the load is still evaluating, nor the load one of its;
// one made while its own load is still under way shares what that load
// finished and goes with it should it fail, as does what a failed load's
// modules import after it, however deep; and two at once from one load
// share one evaluation. Of two evaluations at once that each finish a copy
// of one module, only the first's is shared, so no later load or late
// import is given both. The first keyed-modules test covers the rest.
// Run as: node THIS PI_ROOT, with the helper beside it.
import assert from "node:assert/strict";
import { rmSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import {
  afresh,
  breakingPackage,
  cached,
  counted,
  evaluatedBy,
  evaluations,
  folder,
  gate,
  gated,
  library,
  loadExtensionsCached,
  manifest,
  keyedManifest,
  mixedPackage,
  nativesPackage,
  open,
  parentedPackage,
  pendingAfter,
  QUEUED_MS,
  racingPackage,
  SETTLES_MS,
  scratch,
  within,
  write,
} from "./hosted-keyed-packages.mjs";

// A failed load leaves nothing it evaluated, through any module it
// imported, so a retry evaluates again what it left unfinished and what it
// finished alike, and another extension after the retry shares the
// retry's.
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
const steadyToo = join(flakyAt, "extensions", "steady", "index.ts");
write(
  steadyToo,
  'import { version } from "../../lib/steady.ts";\nexport default function () { return version; }\n',
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
  "a retry was given a module a failed load evaluated",
);
await open("steady", [steadyToo]);
assert.equal(evaluations().steady, 2, "a load after a retry didn't share what the retry finished");

// A failed load leaves only what it evaluated: a module another of the
// package's extensions imports as it runs, while the failed load waits,
// stays, and so does one that module imports, so each is evaluated once.
afresh();
const { runsLate, fails } = mixedPackage(join(scratch, "mixed"));
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
  "a failed load let go of a module another extension imported meanwhile",
);

// A module an extension imports or requires as it runs that throws stays
// unfinished, as does one its load caught the throw of, so its own import
// or require again, a sibling's, and a load after it, evaluate it afresh
// and fail as it does, rather than be given what the module defined before
// it threw.
afresh();
const breaking = breakingPackage(join(scratch, "breaking"));
const late = await open("breaking-late", [breaking.late, breaking.again, breaking.caught]);
for (const [command, what] of [
  ["breaks-late", "a late import that throws"],
  ["breaks-late", "a late import again after one that threw"],
  ["breaks-again", "a sibling's late import after one that threw"],
  ["requires-late", "a late require that throws"],
  ["requires-late", "a late require again after one that threw"],
  ["requires-again", "a sibling's late require after one that threw"],
  ["breaks-caught", "a late import after a load that caught its throw"],
]) {
  assert.equal(await late(command), "breaks part-way", `${what} loaded`);
}
const afterBreak = await loadExtensionsCached([breaking.loads], folder("breaking-loads"));
assert.equal(afterBreak.errors.length, 1, "a load was given a module left unfinished");
assert.deepEqual(
  [evaluations().breaks, evaluations()["breaks-required"]],
  [6, 3],
  "an import, require or load after an unfinished module didn't evaluate it",
);

// An import made as an extension runs while a load is still evaluating the
// module it imports evaluates its own and is given it once finished, not
// the load's part-way through.
afresh();
const racing = racingPackage(join(scratch, "racing"));
const racingLate = await open("racing-late", [racing.late]);
const racedGate = gate("raced");
const racingLoad = open("racing-loads", [racing.loads]);
await racedGate.reached;
const lateRace = racingLate("raced-late");
const lateWaited = await pendingAfter(lateRace, QUEUED_MS);
assert.equal(lateWaited, true, "a late import was given a module a load was still evaluating");
const racedTwice = await evaluatedBy("raced", 2, SETTLES_MS);
assert.equal(racedTwice, 2, "a late import waited on the load, not evaluating its own");
racedGate.putAway();
racedGate.open();
assert.equal(await within(lateRace, "a late import beside a load never ended"), "whole");
const raced = await within(racingLoad, "a load beside a late import never ended");
assert.equal(await raced("raced-loads"), "whole", "a load raced by a late import saw nothing");
assert.equal(evaluations().raced, 2, "a late import racing a load didn't evaluate its own");

// Nor is a load given a module that an import made as an extension runs is
// still evaluating: it evaluates its own.
afresh();
const reversed = racingPackage(join(scratch, "reversed"));
const reversedLate = await open("reversed-late", [reversed.late]);
const firstGate = gate("first");
const racedAgain = gate("raced");
const reversedLoad = open("reversed-loads", [reversed.loads]);
await firstGate.reached;
const lateFirst = reversedLate("raced-late");
await racedAgain.reached;
firstGate.putAway();
firstGate.open();
const reversedSaid = "a load was given a module a late import was still evaluating";
assert.equal(await pendingAfter(reversedLoad, QUEUED_MS), true, reversedSaid);
// Both wait at the shut gate, so only its own count says the load didn't wait.
assert.equal(await evaluatedBy("raced", 2, SETTLES_MS), 2, reversedSaid);
racedAgain.putAway();
racedAgain.open();
const reversedLoaded = await within(reversedLoad, "a load beside a late import never ended");
assert.equal(
  await reversedLoaded("raced-loads"),
  "whole",
  "a load beside a late import saw nothing",
);
assert.equal(await within(lateFirst, "a late import beside a load never ended"), "whole");
assert.equal(evaluations().raced, 2, "a load and a late import racing shared a module");

// An import made by a module of a load still under way shares what that
// load has finished, so a library both reach is evaluated once, and is
// given part-way, as jiti gives it, one the load is still evaluating.
afresh();
const parented = parentedPackage(join(scratch, "parented"));
const parentGate = gate("parented");
const parentLoad = open("parented", [parented]);
await parentGate.reached;
assert.equal(
  await within(globalThis.parentedLater(), "an import beside its own load never ended"),
  "parented",
);
assert.equal(
  evaluations().parented,
  1,
  "an import made during its own load evaluated again what the load had finished",
);
const waits = globalThis.parentedWaits();
assert.equal(
  await pendingAfter(waits, SETTLES_MS),
  false,
  "an import made during its own load evaluated again what the load was evaluating",
);
assert.equal(
  await waits,
  undefined,
  "an import made during its own load wasn't given part-way what the load was evaluating",
);
parentGate.putAway();
parentGate.open();
await within(parentLoad, "a load whose module imported beside it never ended");

// What an import made during a load evaluates goes with that load: should
// the load then fail, a retry evaluates it again beside the rest, so no
// module the retry shares was made from one the failed load left. So too
// what a failed load's modules import once it has ended, however deep,
// which no import after the retry joins while it is still evaluating.
afresh();
const doomedAt = join(scratch, "doomed");
write(join(doomedAt, "package.json"), manifest(keyedManifest));
write(join(doomedAt, "lib", "counted.ts"), library("doomed", "doomed"));
write(
  join(doomedAt, "lib", "early.ts"),
  [
    'import { version } from "./counted.ts";',
    'globalThis.doomedLater = async () => (await import("./uses.ts")).used;',
    'globalThis.doomedAfter = async () => (await import("./after.ts")).after;',
    'globalThis.doomedLingers = async () => (await import("./lingers.ts")).lingered;',
    "export const early = version;",
    "",
  ].join("\n"),
);
write(
  join(doomedAt, "lib", "uses.ts"),
  `import { version } from "./counted.ts";\n${counted("doomed-uses")}\nexport const used = version;\n`,
);
write(
  join(doomedAt, "lib", "after.ts"),
  [
    'import { version } from "./counted.ts";',
    counted("doomed-after"),
    'globalThis.doomedDeeper = async () => (await import("./deeper.ts")).deeper;',
    "export const after = version;",
    "",
  ].join("\n"),
);
write(
  join(doomedAt, "lib", "deeper.ts"),
  `import { version } from "./counted.ts";\n${counted("doomed-deeper")}\nexport const deeper = version;\n`,
);
write(
  join(doomedAt, "lib", "lingers.ts"),
  `import { version } from "./counted.ts";\n${counted("doomed-lingers")}\n${gated("lingers", "lingered")}export const lingered = version;\n`,
);
write(join(doomedAt, "lib", "waits.ts"), gated("doomed", "waited"));
write(
  join(doomedAt, "lib", "throws.ts"),
  'if (globalThis.doomedFails) throw new Error("not yet");\nexport const thrown = "recovered";\n',
);
const doomed = join(doomedAt, "extensions", "one", "index.ts");
write(
  doomed,
  [
    'import { early } from "../../lib/early.ts";',
    'import { doomed as waited } from "../../lib/waits.ts";',
    'import { thrown } from "../../lib/throws.ts";',
    "export default function () { return [early, waited, thrown]; }",
    "",
  ].join("\n"),
);
globalThis.doomedFails = true;
const doomedGate = gate("doomed");
const doomedLoad = loadExtensionsCached([doomed], folder("doomed"));
await doomedGate.reached;
assert.equal(
  await within(globalThis.doomedLater(), "an import during a load that will fail never ended"),
  "doomed",
);
doomedGate.putAway();
doomedGate.open();
assert.equal((await doomedLoad).errors.length, 1, "a load whose library throws didn't fail");
assert.equal(
  await within(globalThis.doomedAfter(), "an import after a failed load never ended"),
  "doomed",
);
assert.equal(
  await within(globalThis.doomedDeeper(), "an import below one after a failed load never ended"),
  "doomed",
);
const failedLingers = globalThis.doomedLingers;
const lingersGate = gate("lingers");
const lingering = failedLingers();
await lingersGate.reached;
lingersGate.putAway();
globalThis.doomedFails = false;
await open("doomed-retried", [doomed]);
assert.equal(
  await within(globalThis.doomedLater(), "an import after a retry never ended"),
  "doomed",
);
assert.equal(
  await pendingAfter(globalThis.doomedLingers(), SETTLES_MS),
  false,
  "an import after a retry joined one a failed load's module began",
);
lingersGate.open();
await within(lingering, "an import a failed load's module began never ended");
assert.equal(
  await within(globalThis.doomedAfter(), "an import after a retry never ended"),
  "doomed",
);
assert.equal(
  await within(globalThis.doomedDeeper(), "an import below one after a retry never ended"),
  "doomed",
);
assert.deepEqual(
  evaluations(),
  { doomed: 2, "doomed-uses": 2, "doomed-after": 2, "doomed-deeper": 2, "doomed-lingers": 2 },
  "a retry was given a module an import made during or after the failed load evaluated",
);

// Two imports of one module at once, made as an extension runs after its
// load, share one evaluation of it.
afresh();
const twiceAt = join(scratch, "twice");
write(join(twiceAt, "package.json"), manifest(keyedManifest));
write(join(twiceAt, "lib", "twice.ts"), library("twice", "twice"));
write(
  join(twiceAt, "extensions", "one", "index.ts"),
  [
    "export default function (pi) {",
    '  pi.registerCommand("twice", { description: "", handler: async () => {',
    '    const [one, other] = await Promise.all([import("../../lib/twice.ts"), import("../../lib/twice.ts")]);',
    "    return one === other;",
    "  } });",
    "}",
    "",
  ].join("\n"),
);
const twice = await open("twice", [join(twiceAt, "extensions", "one", "index.ts")]);
assert.equal(
  await within(twice("twice"), "two imports at once never ended"),
  true,
  "two imports at once were given two modules",
);
assert.equal(evaluations().twice, 1, "two imports at once evaluated their module twice");

// Two evaluations at once may each finish a copy of one module. The first
// to end joins the package's; the second joins none of what it finished,
// since some of it was evaluated over its own copy, and neither do imports
// its extension makes as it runs, nor do they join another graph's. So a
// later load is given one copy of each module, and an extension's late
// imports stay on its own copy. Here `runs` imports the counted library
// `x` as it runs while `loads` evaluates its own `x` and `y`, which
// imports `x`, and `runs`'s ends first.
afresh();
const overlapAt = join(scratch, "overlap");
write(join(overlapAt, "package.json"), manifest(keyedManifest));
write(
  join(overlapAt, "lib", "x.ts"),
  [
    counted("overlap-x"),
    'export const copy = globalThis.evaluations["overlap-x"];',
    gated("overlap", "x"),
  ].join("\n"),
);
write(
  join(overlapAt, "lib", "y.ts"),
  'import { copy } from "./x.ts";\nexport const seen = copy;\n',
);
write(
  join(overlapAt, "lib", "z.ts"),
  'import { seen as through } from "./y.ts";\nexport const seen = through;\n',
);
write(
  join(overlapAt, "lib", "w.ts"),
  `import { copy } from "./x.ts";\n${gated("overlapW", "w")}export const seen = copy;\n`,
);
const overlapCommand = (name, body) =>
  `  pi.registerCommand("overlap-${name}", { description: "", handler: async () => ${body} });`;
const lateSeen = (lib) => `(await import("../../lib/${lib}.ts")).seen`;
const overlapEntry = (name, lines) => {
  const entry = join(overlapAt, "extensions", name, "index.ts");
  write(entry, [...lines, ""].join("\n"));
  return entry;
};
const runs = overlapEntry("runs", [
  "export default function (pi) {",
  overlapCommand("x", '(await import("../../lib/x.ts")).copy'),
  overlapCommand("runs-w", lateSeen("w")),
  "}",
]);
const loads = overlapEntry("loads", [
  'import { seen } from "../../lib/y.ts";',
  "export default function (pi) {",
  overlapCommand("mine", "seen"),
  overlapCommand("z", lateSeen("z")),
  overlapCommand("loads-w", lateSeen("w")),
  "}",
]);
const after = overlapEntry("after", [
  'import { copy } from "../../lib/x.ts";',
  'import { seen } from "../../lib/y.ts";',
  'import { seen as through } from "../../lib/z.ts";',
  "export default function (pi) {",
  overlapCommand("after", "[copy, seen, through]"),
  "}",
]);
const running = await open("overlap-runs", [runs]);
const aloneGate = gate("overlap");
const firstCopy = running("overlap-x");
await within(aloneGate.reached, "a late import never reached its library");
const secondGate = gate("overlap");
const loading = open("overlap-loads", [loads]);
await within(secondGate.reached, "a load beside a late import never reached its library");
aloneGate.open();
assert.equal(await within(firstCopy, "the late import never ended"), 1, "the late import's copy");
secondGate.putAway();
secondGate.open();
const loaded = await within(loading, "the load beside a late import never ended");
assert.equal(
  await loaded("overlap-mine"),
  2,
  "the load beside a late import wasn't given its own copy",
);
assert.equal(
  await within(loaded("overlap-z"), "a late import over the load's copy never ended"),
  2,
  "a late import of the load's extension left its own copy",
);
const wGate = gate("overlapW");
const runsW = running("overlap-runs-w");
await within(wGate.reached, "a late import never reached its gated library");
const loadsW = loaded("overlap-loads-w");
assert.equal(await pendingAfter(loadsW, QUEUED_MS), true, "a late import didn't wait at the gate");
wGate.putAway();
wGate.open();
assert.deepEqual(
  await within(Promise.all([runsW, loadsW]), "two late imports at once never ended"),
  [1, 2],
  "a late import joined one evaluated over another copy of its library",
);
const later = await open("overlap-after", [after]);
assert.deepEqual(
  await later("overlap-after"),
  [1, 1, 1],
  "a later load was given two copies of one module",
);
assert.equal(evaluations()["overlap-x"], 2, "a later load evaluated the finished library again");

// An import or require made as an extension runs of what jiti doesn't
// evaluate itself, a builtin, a data URL, JSON or a module it leaves to
// Node, is jiti's own, which hands the last two to Node's require: so a
// module it leaves to Node that Node already holds, as it holds pi's own
// jiti and typescript, isn't evaluated again.
afresh();
const nativesAt = join(scratch, "natives");
const nativesEntry = nativesPackage(nativesAt);
await import(pathToFileURL(join(nativesAt, "node_modules", "typescript", "index.js")).href);
const natives = await open("natives", [nativesEntry]);
assert.deepEqual(
  await within(natives("natives"), "a late import of a builtin, data URL or JSON never ended"),
  ["/", "/", "/", 1, "json", "/", "/", "json", 1, 1],
  "a late import of a builtin, data URL, JSON or native module wasn't jiti's own",
);
const handedToNode = [
  join(nativesAt, "lib", "data.json"),
  join(nativesAt, "node_modules", "typescript", "index.js"),
];
assert.deepEqual(
  cached(scratch).sort((one, other) => one.localeCompare(other)),
  handedToNode,
  "a late import of JSON or a native module wasn't Node's, or put another module in its cache",
);
for (const file of handedToNode) {
  delete createRequire(import.meta.url).cache[file];
}

assert.deepEqual(cached(scratch), [], "a load put modules in Node's cache");
rmSync(scratch, { recursive: true, force: true });

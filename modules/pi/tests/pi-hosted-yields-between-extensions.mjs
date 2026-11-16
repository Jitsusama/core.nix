// pi as the host runs it gives the event loop a turn before it imports each
// extension and before it runs each factory. A host runs every conversation
// on one thread, and a load awaited only microtasks, so one conversation's
// whole load held the thread while the others waited. Each step now has a
// turn of the loop before it, and the extensions still load in order.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { after, test as it } from "node:test";
import { pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
// The coding agent sits beside its libraries in an installed tree, or is the
// tree itself where the package was installed from its own workspace.
const agentFile = (path) => {
  const found = [
    `${root}/node_modules/@earendil-works/pi-coding-agent/dist/${path}`,
    `${root}/packages/coding-agent/dist/${path}`,
    `${root}/dist/${path}`,
  ].find((candidate) => existsSync(candidate));
  assert.ok(found, `no coding agent ${path} under ${root}`);
  return found;
};
const { clearExtensionCache, loadExtensionsCached } = await import(
  pathToFileURL(agentFile("core/extensions/loader.js")).href
);

const scratch = mkdtempSync(join(tmpdir(), "pi-hosted-yields-"));
after(() => rmSync(scratch, { recursive: true, force: true }));
const write = (path, text) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
};

// An extension whose module notes its import and whose factory notes its
// run, on the one log the loop's turns go on too.
const extension = (name) => {
  const where = join(scratch, name);
  write(join(where, "package.json"), JSON.stringify({ name, type: "module" }));
  write(
    join(where, "index.ts"),
    [
      `globalThis.steps.push(${JSON.stringify(`${name} import`)});`,
      "export default function (pi) {",
      `  globalThis.steps.push(${JSON.stringify(`${name} factory`)});`,
      `  pi.registerCommand(${JSON.stringify(name)}, { description: "", handler: async () => {} });`,
      "}",
      "",
    ].join("\n"),
  );
  return join(where, "index.ts");
};
const names = ["first", "second", "third"];
const extensions = names.map(extension);

// Log a turn each time the loop comes round to its immediates, until the
// load ends, folding turns in a row into one.
const loadWatchingTurns = async () => {
  globalThis.steps = [];
  let watching = true;
  const turn = () => {
    if (!watching) return;
    if (globalThis.steps.at(-1) !== "turn") globalThis.steps.push("turn");
    setImmediate(turn);
  };
  setImmediate(turn);
  const folder = join(scratch, "folder");
  mkdirSync(folder, { recursive: true });
  try {
    return { loaded: await loadExtensionsCached(extensions, folder), steps: globalThis.steps };
  } finally {
    watching = false;
  }
};

it("gives the loop a turn before each extension's import and each factory", async () => {
  clearExtensionCache();
  const { loaded, steps } = await loadWatchingTurns();
  assert.deepEqual(loaded.errors, [], "the extensions didn't load");
  const between = names.flatMap((name) => ["turn", `${name} import`, "turn", `${name} factory`]);
  assert.deepEqual(
    steps.slice(0, between.length),
    between,
    "a step of the load ran without the loop coming round first",
  );
});

it("still loads every extension, in the order it was given", async () => {
  clearExtensionCache();
  const { loaded } = await loadWatchingTurns();
  assert.deepEqual(
    loaded.extensions.map((one) => [...one.commands.keys()]),
    names.map((name) => [name]),
  );
});

// pi as the host runs it loads extensions without sorting its alias map for
// every module it evaluates. jiti makes a nested loader for each module and
// sorts the alias map it is given unless that map is marked as sorted; it
// marks only the sorted copy it keeps, so pi's own map was sorted again for
// each of the hundreds of modules a host's restart evaluates. The map is
// sorted at most once, the aliases still resolve as jiti's own order
// resolves them (the longest specifier first), and each load still
// evaluates an extension's modules afresh, so one folder's module state
// never reaches another's.
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
const aiModels = [
  `${root}/node_modules/@earendil-works/pi-ai/dist/models.js`,
  `${root}/packages/ai/dist/models.js`,
].find((candidate) => existsSync(candidate));
assert.ok(aiModels, `no pi-ai models under ${root}`);
const { clearExtensionCache, loadExtensionsCached } = await import(
  pathToFileURL(agentFile("core/extensions/loader.js")).href
);

const scratch = mkdtempSync(join(tmpdir(), "pi-hosted-loads-"));
after(() => rmSync(scratch, { recursive: true, force: true }));
const write = (path, text) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
};

// An extension of a few modules, as a package's are: its entry imports a
// helper, which imports another, and the entry reaches pi-ai through both a
// longer and a shorter aliased specifier. Each module counts its
// evaluations under the extension's name.
const extension = (name) => {
  const where = join(scratch, name);
  write(join(where, "package.json"), JSON.stringify({ name, type: "module" }));
  const counted = (module) =>
    [
      "globalThis.evaluations ??= {};",
      `globalThis.evaluations[${JSON.stringify(`${name}/${module}`)}] = (globalThis.evaluations[${JSON.stringify(`${name}/${module}`)}] ?? 0) + 1;`,
    ].join("\n");
  write(join(where, "deeper.ts"), `${counted("deeper")}\nexport const depth = 2;\n`);
  write(
    join(where, "helper.ts"),
    `${counted("helper")}\nimport { depth } from "./deeper.ts";\nexport const helped = depth + 1;\n`,
  );
  write(
    join(where, "index.ts"),
    [
      counted("index"),
      'import * as models from "@earendil-works/pi-ai/models";',
      'import * as ai from "@earendil-works/pi-ai";',
      'import { helped } from "./helper.ts";',
      "export default function (pi) {",
      `  pi.registerCommand(${JSON.stringify(`${name}-reach`)}, { description: "", handler: async () => ({ helped, models: Object.keys(models).sort((a, b) => a.localeCompare(b)), ai: Object.keys(ai).sort((a, b) => a.localeCompare(b)) }) });`,
      "}",
      "",
    ].join("\n"),
  );
  return join(where, "index.ts");
};
const extensions = [extension("first"), extension("second")];

const folder = (name) => {
  const where = join(scratch, "folders", name);
  mkdirSync(where, { recursive: true });
  return where;
};

// Each sort of the alias map builds it again with Object.fromEntries, keyed
// by pi's own specifiers: count those while extensions load.
const sortsWhile = async (work) => {
  const { fromEntries } = Object;
  let sorts = 0;
  Object.fromEntries = (entries) => {
    const built = fromEntries(entries);
    if (Object.hasOwn(built, "@earendil-works/pi-coding-agent")) {
      sorts += 1;
    }
    return built;
  };
  try {
    return { result: await work(), sorts };
  } finally {
    Object.fromEntries = fromEntries;
  }
};

const open = async (name) => {
  const loaded = await loadExtensionsCached(extensions, folder(name));
  assert.deepEqual(loaded.errors, [], `the extensions didn't load in ${name}`);
  return loaded;
};
const reach = (loaded, name) =>
  loaded.extensions
    .find((one) => one.commands.has(`${name}-reach`))
    .commands.get(`${name}-reach`)
    .handler();

it("sorts the alias map at most once, however many modules load", async () => {
  clearExtensionCache();
  globalThis.evaluations = {};
  const { sorts } = await sortsWhile(async () => {
    await open("a");
    await open("b");
  });
  assert.ok(
    Object.keys(globalThis.evaluations).length >= 6,
    `fewer modules evaluated than the probes hold: ${JSON.stringify(globalThis.evaluations)}`,
  );
  assert.ok(sorts <= 1, `the alias map was sorted ${sorts} times for two folders' loads`);
});

it("resolves the aliases as jiti's own order does, the longest specifier first", async () => {
  clearExtensionCache();
  const loaded = await open("c");
  const reached = await reach(loaded, "first");
  const byName = (a, b) => a.localeCompare(b);
  const models = Object.keys(await import(pathToFileURL(aiModels).href)).sort(byName);
  const view = Object.keys(
    await import(pathToFileURL(agentFile("core/extensions/pi-ai-extension-view.js")).href),
  ).sort(byName);
  assert.equal(reached.helped, 3);
  assert.deepEqual(reached.models, models, "pi-ai/models reached something else");
  assert.deepEqual(reached.ai, view, "pi-ai reached something other than the extension view");
});

it("evaluates an extension's modules afresh for each folder and after a reload", async () => {
  clearExtensionCache();
  globalThis.evaluations = {};
  await open("d");
  await open("e");
  clearExtensionCache();
  await open("d");
  assert.deepEqual(
    globalThis.evaluations,
    {
      "first/index": 3,
      "first/helper": 3,
      "first/deeper": 3,
      "second/index": 3,
      "second/helper": 3,
      "second/deeper": 3,
    },
    "a module was shared between folders' loads or kept over a reload",
  );
});

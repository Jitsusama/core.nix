// pi as the host runs it loads from its bundle's host entry as one module
// graph: the coding agent and its libraries as the bundle hands them to
// extensions, and the durable runtime and chord built into the same chunks,
// so pi-ai, pi-tui and pi-agent-core each keep their state once: one UUIDv7
// sequence, one set of session cleanups, one kitty flag. The host's own
// imports of those libraries, and of typebox, resolve to the bundle's host
// modules, each the copy the extensions get. It reads no file of pi's from
// outside the bundle, and holds less heap and spends less CPU than loading
// the same modules file by file.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { registerHooks } from "node:module";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import { test as it } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = realpathSync(resolve(process.argv[2]));
const bundle = join(root, "dist", "bundle");
const entryFile = join(bundle, "host.js");
assert.ok(existsSync(entryFile), `No host entry at ${entryFile}`);

const loaded = [];
registerHooks({
  load(url, context, next) {
    if (url.startsWith("file:")) loaded.push(realpathSync(fileURLToPath(url)));
    return next(url, context);
  },
});
const entry = await import(pathToFileURL(entryFile).href);
const virtual = entry.VIRTUAL_MODULES;
const ai = virtual["@earendil-works/pi-ai"];
const libraries = Object.keys(virtual);
const hostModules = Object.fromEntries(
  await Promise.all(
    libraries.map(async (name) => {
      const file = join(bundle, "host-modules", `${name.replace(/^@earendil-works\//u, "")}.js`);
      assert.ok(existsSync(file), `No host module for ${name} at ${file}`);
      return [name, await import(pathToFileURL(file).href)];
    }),
  ),
);

it("names the host entry's version and hands on the bundle's own coding agent", async () => {
  assert.equal(entry.hostEntryVersion, 1);
  const index = await import(pathToFileURL(join(bundle, "index.js")).href);
  assert.equal(virtual["@earendil-works/pi-coding-agent"].AgentSession, index.AgentSession);
  assert.equal(typeof entry.durable.GenerationTask, "object");
  assert.ok(Array.isArray(entry.builtInExtensions));
  assert.equal(typeof entry.resolveProjectTrusted, "function");
});

it("gives the host, in each host module, the very library the extensions get", () => {
  for (const name of [
    "@earendil-works/pi-tui",
    "@earendil-works/pi-ai",
    "typebox",
    "typebox/compile",
    "typebox/value",
  ])
    assert.ok(libraries.includes(name), `the extensions get ${name}`);
  for (const name of libraries) {
    const handed = virtual[name];
    const hosted = hostModules[name];
    // export * passes over a default export; each is also exported by name.
    const named = Object.keys(handed).filter((key) => key !== "default");
    assert.deepEqual(Object.keys(hosted).sort(), named.sort(), name);
    for (const key of named) assert.equal(hosted[key], handed[key], `${name} ${key}`);
    if ("default" in handed)
      assert.ok(Object.values(hosted).includes(handed.default), `${name}'s default, by name`);
  }
  // One TypeBox, so a schema the host builds is the kind the extensions'
  // checker knows.
  const schema = hostModules.typebox.Type.Object({ a: hostModules.typebox.Type.String() });
  assert.equal(virtual["typebox/value"].Value.Check(schema, { a: "b" }), true);
  // The viewers' terminal tells pi-tui its keys are kitty's, and pi's key
  // reader must hear it.
  const tui = "@earendil-works/pi-tui";
  hostModules[tui].setKittyProtocolActive(true);
  assert.equal(virtual[tui].isKittyProtocolActive(), true);
  hostModules[tui].setKittyProtocolActive(false);
});

it("runs durable's SQLite storage and Node environment and chord's context from the bundle", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pi-hosted-bundle-"));
  writeFileSync(join(dir, "said.txt"), "said\n");
  const env = new entry.durableNodeEnv.NodeExecutionEnv({ cwd: dir });
  const read = await env.readTextFile(join(dir, "said.txt"), entry.chordContext.BACKGROUND_CONTEXT);
  assert.deepEqual(read, { ok: true, value: "said\n" });
  const storage = await entry.durableSqlite.openNodeSqliteStorage(join(dir, "durable.db"));
  await storage.close();
});

it("reads none of pi's files from outside its bundle as it loads", () => {
  const outside = loaded.filter(
    (file) => file.startsWith(root + sep) && !file.startsWith(bundle + sep),
  );
  assert.deepEqual(outside, []);
});

it("declares the state pi-ai, pi-tui and pi-agent-core keep once in all it loads", () => {
  const declared = (pattern) =>
    loaded.reduce(
      (count, file) => count + (readFileSync(file, "utf8").match(pattern) ?? []).length,
      0,
    );
  assert.equal(declared(/\blastOrdinaryTimestamp\d*\s*=\s*-1\b/gu), 1, "pi-ai's UUIDv7 sequence");
  assert.equal(
    declared(/\bsessionResourceCleanups\d*\s*=\s*new Set\b/gu),
    1,
    "pi-ai's session cleanups",
  );
  assert.equal(declared(/\bvar _kittyProtocolActive\d*\s*=/gu), 1, "pi-tui's kitty flag");
  assert.equal(
    declared(/(?:\bvar|\blet|,)\s*defaultStreamFn\d*\s*[;,=]/gu),
    1,
    "pi-agent-core's stream function",
  );
});

it("keeps the order of ids the host and the durable runtime mint in turn", () => {
  const mint = entry.durable.ProviderDoc.definition.initial;
  let previous = "";
  let outOfOrder = 0;
  for (let i = 0; i < 20_000; i++) {
    for (const id of [ai.uuidv7(), mint().sessionId]) {
      if (id <= previous) outOfOrder++;
      previous = id;
    }
  }
  assert.equal(outOfOrder, 0);
});

/** Heap after a full collection and CPU spent, loading pi as the host does, in a fresh process. */
const measure = (how) => {
  const script = `
    import { join } from "node:path";
    import { pathToFileURL } from "node:url";
    const root = ${JSON.stringify(root)};
    const at = (...parts) => import(pathToFileURL(join(root, ...parts)).href);
    const lib = (name, ...parts) => at("node_modules", "@earendil-works", name, "dist", ...parts);
    const before = process.cpuUsage();
    if (${JSON.stringify(how)} === "bundle") await at("dist", "bundle", "host.js");
    else await Promise.all([
      at("dist", "index.js"), at("dist", "extensions", "index.js"), at("dist", "core", "project-trust.js"),
      lib("pi-agent-core", "index.js"), lib("pi-ai", "index.js"), lib("pi-tui", "index.js"),
      lib("pi-durable", "index.js"), lib("pi-durable", "storage", "sqlite", "node.js"),
      lib("pi-durable", "env", "node.js"), lib("chord", "context", "index.js"),
    ]);
    const spent = process.cpuUsage(before);
    globalThis.gc();
    console.log(JSON.stringify({ heap: process.memoryUsage().heapUsed, cpu: spent.user + spent.system }));
  `;
  const run = spawnSync(
    process.execPath,
    ["--no-warnings", "--expose-gc", "--input-type=module", "-e", script],
    { encoding: "utf8", timeout: 120_000 },
  );
  assert.equal(run.status, 0, run.error?.message ?? run.stderr);
  return JSON.parse(run.stdout);
};

const median = (values) => values.sort((a, b) => a - b)[Math.floor(values.length / 2)];

it("holds less heap and spends less CPU than loading pi file by file", () => {
  // Three runs each, in turn, so a busy builder slows both alike.
  const runs = { bundle: [], files: [] };
  for (let round = 0; round < 3; round++)
    for (const how of ["bundle", "files"]) runs[how].push(measure(how));
  const [bundled, unbundled] = ["bundle", "files"].map((how) => ({
    heap: median(runs[how].map((run) => run.heap)),
    cpu: median(runs[how].map((run) => run.cpu)),
  }));
  const mib = 1024 * 1024;
  assert.ok(
    bundled.heap < unbundled.heap - 5 * mib,
    `heap ${bundled.heap / mib} MiB bundled against ${unbundled.heap / mib} MiB file by file`,
  );
  assert.ok(
    bundled.cpu < unbundled.cpu,
    `CPU ${bundled.cpu} µs bundled against ${unbundled.cpu} µs file by file`,
  );
});

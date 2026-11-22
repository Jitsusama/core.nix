// pi as the host runs it loads from its bundle's host entry as one module
// graph: the coding agent and its libraries as the bundle hands them to
// extensions, and the durable runtime and chord built into the same chunks,
// so one copy of pi-ai keeps one UUIDv7 sequence and one set of session
// cleanups. It reads no file of pi's from outside the bundle, and holds
// less heap and spends less CPU than loading the same modules file by file.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { registerHooks } from "node:module";
import { join, resolve } from "node:path";
import { test as it } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
const bundle = join(root, "dist", "bundle");
const entryFile = join(bundle, "host.js");
assert.ok(existsSync(entryFile), `No host entry at ${entryFile}`);

const loaded = [];
registerHooks({
  load(url, context, next) {
    if (url.startsWith("file:")) loaded.push(fileURLToPath(url));
    return next(url, context);
  },
});
const entry = await import(pathToFileURL(entryFile).href);
const virtual = entry.VIRTUAL_MODULES;
const ai = virtual["@earendil-works/pi-ai"];

it("names the host entry's version and hands on the bundle's own coding agent", async () => {
  assert.equal(entry.hostEntryVersion, 1);
  const index = await import(pathToFileURL(join(bundle, "index.js")).href);
  assert.equal(virtual["@earendil-works/pi-coding-agent"].AgentSession, index.AgentSession);
  for (const name of ["durable", "durableSqlite", "durableNodeEnv", "chordContext"])
    assert.equal(typeof entry[name], "object", `${name} is exported`);
  assert.equal(typeof entry.durable.GenerationTask, "object");
  assert.ok(Array.isArray(entry.builtInExtensions));
  assert.equal(typeof entry.resolveProjectTrusted, "function");
});

it("reads none of pi's files from outside its bundle", () => {
  const outside = loaded.filter((file) => file.startsWith(root) && !file.startsWith(bundle));
  assert.deepEqual(outside, []);
});

it("keeps one copy of pi-ai's id sequence and session cleanups", () => {
  const holding = (pattern) =>
    loaded.filter((file) => pattern.test(readFileSync(file, "utf8"))).length;
  assert.equal(holding(/lastOrdinaryTimestamp/u), 1);
  assert.equal(holding(/sessionResourceCleanups\s*=\s*new Set/u), 1);
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

it("runs a cleanup the host's pi-ai registers when a session's cleanups run", () => {
  const ended = [];
  const forget = ai.registerSessionResourceCleanup((session) => ended.push(session));
  ai.cleanupSessionResources("a-session");
  forget();
  assert.deepEqual(ended, ["a-session"]);
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
    { encoding: "utf8" },
  );
  assert.equal(run.status, 0, run.stderr);
  return JSON.parse(run.stdout);
};

it("holds less heap and spends less CPU than loading pi file by file", () => {
  const bundled = measure("bundle");
  const unbundled = measure("files");
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

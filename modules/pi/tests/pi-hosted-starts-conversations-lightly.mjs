// pi as the host runs it: what every conversation's start repeats in the
// host's one process is done once, or without holding the process. An MCP
// server's headers run the command they name without blocking, and the
// connections opened within a short while share its one run, while a run
// that resolved nothing is tried again; a tool found on the path stays
// found, and one missing is looked for again; and a stored model catalogue
// reaches its provider as the store's own copy, not cloned a second time.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { test as it } from "node:test";
import { pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
// The coding agent sits beside its libraries in an installed tree, or is the
// tree itself where the package was installed from its own workspace.
const agentModule = (path) => {
  const found = [
    `${root}/node_modules/@earendil-works/pi-coding-agent/dist/${path}`,
    `${root}/packages/coding-agent/dist/${path}`,
    `${root}/dist/${path}`,
  ].find((candidate) => existsSync(candidate));
  assert.ok(found, `No coding agent ${path} under ${root}`);
  return import(pathToFileURL(found).href);
};
const aiModule = [
  `${root}/node_modules/@earendil-works/pi-ai/dist/models.js`,
  `${root}/packages/ai/dist/models.js`,
].find((candidate) => existsSync(candidate));
assert.ok(aiModule, `No pi-ai models under ${root}`);

const folder = mkdtempSync(join(tmpdir(), "pi-starts-"));
// The tools' own folder is under pi's agent folder, kept empty here so only
// the path is searched.
process.env.PI_CODING_AGENT_DIR = join(folder, "agent");

/** A command for a header that counts its runs in a file and prints a token. */
const counted = (name) => {
  const runs = join(folder, `${name}.runs`);
  writeFileSync(runs, "");
  return {
    command: `!printf x >> ${runs}; printf token-${name}`,
    runs: () => readFileSync(runs, "utf8").length,
  };
};

const NAMES_COMMAND = /Failed to resolve test header from shell command/u;
const resolver = await agentModule("core/resolve-config-value.js");
const mcp = await agentModule("extensions/mcp/runtime.js");

it("runs an MCP server's header command without holding the process, once for a burst", async () => {
  const header = counted("burst");
  let turned = false;
  setImmediate(() => {
    turned = true;
  });
  const opening = Array.from({ length: 5 }, () =>
    resolver.resolveHeadersFreshOrThrow({ authorization: header.command }, "test"),
  );
  assert.equal(turned, false, "the loop turned before the headers were asked for");
  const headers = await Promise.all(opening);
  assert.equal(turned, true, "the loop never turned while the command ran");
  assert.deepEqual(
    headers.map((each) => each.authorization),
    Array.from({ length: 5 }, () => "token-burst"),
  );
  assert.equal(header.runs(), 1, "five connections opened together ran the command more than once");
});

it("runs a header command that resolved nothing again, and names it when it fails", async () => {
  const runs = join(folder, "fails.runs");
  writeFileSync(runs, "");
  const failing = `!printf x >> ${runs}; exit 1`;
  // One after the other, as two connections a while apart would ask.
  await assert.rejects(
    resolver.resolveConfigValueFreshOrThrow(failing, "test header"),
    NAMES_COMMAND,
  );
  await assert.rejects(
    resolver.resolveConfigValueFreshOrThrow(failing, "test header"),
    NAMES_COMMAND,
  );
  assert.equal(
    readFileSync(runs, "utf8").length,
    2,
    "a failed run was kept for the next connection",
  );
});

it("runs a header command afresh once its cache is cleared", async () => {
  const header = counted("cleared");
  await resolver.resolveConfigValueFresh(header.command);
  resolver.clearConfigValueCache();
  await resolver.resolveConfigValueFresh(header.command);
  assert.equal(header.runs(), 2, "a cleared cache still answered from the run before");
});

it("opens an MCP server's transport through a promise, resolving its headers once a burst", async () => {
  const header = counted("transport");
  const entry = {
    name: "counted",
    config: { url: "http://counted.invalid/mcp", headers: { authorization: header.command } },
  };
  const made = mcp.createDefaultTransport(entry, folder, undefined);
  assert.ok(
    made instanceof Promise,
    "a transport was made while the process waited on its headers",
  );
  await Promise.all([made, mcp.createDefaultTransport(entry, folder, undefined)]);
  assert.equal(header.runs(), 1, "two transports opened together ran the header command twice");
});

it("looks a tool up on the path once it's found, and again while it's missing", async () => {
  const { getToolPath } = await agentModule("utils/tools-manager.js");
  const bin = join(folder, "bin");
  const runs = join(folder, "rg.runs");
  writeFileSync(runs, "");
  mkdirSync(bin, { recursive: true });
  // Only this folder is searched, so a tool the machine has isn't found.
  process.env.PATH = bin;

  assert.equal(getToolPath("rg"), null, "a tool not on the path was found");
  writeFileSync(join(bin, "rg"), `#!/bin/sh\nprintf x >> ${runs}\n`);
  chmodSync(join(bin, "rg"), 0o755);
  assert.equal(getToolPath("rg"), "rg", "a tool put on the path after a miss wasn't found");
  assert.equal(getToolPath("rg"), "rg");
  assert.equal(
    readFileSync(runs, "utf8").length,
    1,
    "a tool already found was run again to find it",
  );
});

it("hands a provider the store's own copy of its catalogue, not a second clone", async () => {
  const { createModels } = await import(pathToFileURL(aiModule).href);
  const model = { id: "stored-1", name: "Stored", provider: "stored", api: "openai-completions" };
  const entry = { models: [model], checkedAt: 1 };
  // A store that hands back what it holds, so a clone made past it shows.
  const store = {
    read: async () => entry,
    write: () => Promise.resolve(),
    delete: () => Promise.resolve(),
  };
  const seen = [];
  const models = createModels({ modelsStore: store });
  models.setProvider({
    id: "stored",
    name: "Stored",
    auth: {},
    refreshModels: ({ stored }) => {
      seen.push(stored);
      return Promise.resolve();
    },
  });
  const result = await models.refresh({ allowNetwork: false });
  assert.deepEqual([...result.errors.values()], []);
  assert.equal(seen.length, 1, "the provider's stored catalogue wasn't offered once");
  assert.equal(
    seen[0].models[0],
    model,
    "the stored catalogue was cloned again on its way to the provider",
  );
});

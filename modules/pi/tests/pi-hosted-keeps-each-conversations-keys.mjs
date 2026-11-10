// pi as the host runs it: a model runtime given an auth context reads keys
// and files through it alone, so conversations sharing one process each
// keep their own credentials and none falls back to the process's own; a
// runtime given none reads the process, as ordinary pi does. It reads a
// models file naming its key by variable, so no credential leaves it.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { test as it } from "node:test";
import { pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
// The coding agent sits beside its libraries in an installed tree, or is the
// tree itself where the package was installed from its own workspace.
const runtimeModule = [
  `${root}/node_modules/@earendil-works/pi-coding-agent/dist/core/model-runtime.js`,
  `${root}/packages/coding-agent/dist/core/model-runtime.js`,
  `${root}/dist/core/model-runtime.js`,
].find((path) => existsSync(path));
assert.ok(runtimeModule, `No coding agent model runtime under ${root}`);
const { ModelRuntime } = await import(pathToFileURL(runtimeModule).href);

const PROVIDER = "keyed";
const VARIABLE = "KEYED_TEST_KEY";
const folder = mkdtempSync(join(tmpdir(), "pi-keys-"));
const modelsPath = join(folder, "models.json");
writeFileSync(
  modelsPath,
  JSON.stringify({
    providers: {
      [PROVIDER]: {
        baseUrl: "http://keyed.invalid",
        api: "openai-completions",
        apiKey: `$${VARIABLE}`,
        models: [{ id: "keyed-1", name: "Keyed" }],
      },
    },
  }),
);

/** An auth context over a fixed set of variables, with no files. */
const context = (values) => ({
  env: async (name) => values[name],
  fileExists: async () => false,
});

const runtime = (authContext) =>
  ModelRuntime.create({
    authPath: join(folder, "auth.json"),
    modelsPath,
    refreshOnCreate: false,
    ...(authContext ? { authContext } : {}),
  });

const keyOf = async (models) => (await models.getAuth(PROVIDER))?.auth?.apiKey;

it("marks a runtime that takes an auth context", () => {
  assert.equal(ModelRuntime.hostAuthContextVersion, 1);
});

it("gives each runtime the key its own context holds", async () => {
  const first = await runtime(context({ [VARIABLE]: "first" }));
  const second = await runtime(context({ [VARIABLE]: "second" }));
  assert.equal(await keyOf(first), "first");
  assert.equal(await keyOf(second), "second");
  assert.ok(await first.checkAuth(PROVIDER), "A context's key configures its provider");
});

it("never falls back to the process for a key its context lacks", async () => {
  process.env[VARIABLE] = "the-process";
  try {
    const models = await runtime(context({}));
    assert.equal(await models.checkAuth(PROVIDER), undefined);
    assert.equal(await keyOf(models), undefined);
  } finally {
    delete process.env[VARIABLE];
  }
});

it("reads the process for a runtime given no context, as ordinary pi does", async () => {
  process.env[VARIABLE] = "the-process";
  try {
    assert.equal(await keyOf(await runtime()), "the-process");
  } finally {
    delete process.env[VARIABLE];
  }
});

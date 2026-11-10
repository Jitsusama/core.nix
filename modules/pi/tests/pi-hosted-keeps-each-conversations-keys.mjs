// pi as the host runs it: a model runtime given an auth context reads keys,
// provider headers and model headers through it alone, so conversations
// sharing one process each keep their own credentials and none falls back
// to the process's own; a runtime given none reads the process, as
// ordinary pi does. It reads a models file naming its key and headers by
// variable, so no credential leaves it.
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
// A provider whose key, provider header and model header each name a variable.
const HEADED = "headed";
const HEADED_KEY = "HEADED_TEST_KEY";
const PROVIDER_HEADER = "HEADED_TEST_PROVIDER_HEADER";
const MODEL_HEADER = "HEADED_TEST_VIEWER_KEY";
// An extension's OAuth provider whose header names a variable.
const OAUTHED = "oauthed";
const OAUTH_HEADER = "OAUTHED_TEST_HEADER";
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
      [HEADED]: {
        baseUrl: "http://headed.invalid",
        api: "openai-completions",
        apiKey: `$${HEADED_KEY}`,
        headers: { "X-Provider": `$${PROVIDER_HEADER}` },
        models: [
          {
            id: "headed-1",
            name: "Headed",
            headers: { "X-Viewer": `$${MODEL_HEADER}` },
          },
        ],
      },
    },
  }),
);
// A signed-in OAuth credential that never expires, so no refresh runs.
writeFileSync(
  join(folder, "auth.json"),
  JSON.stringify({
    [OAUTHED]: {
      type: "oauth",
      access: "oauth-access",
      refresh: "oauth-refresh",
      expires: Date.now() + 86_400_000,
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

/** The request headers a runtime gives the headed model. */
const headersOf = async (models) =>
  (await models.getAuth(models.getModel(HEADED, "headed-1")))?.auth?.headers;

/** Every header variable set in the process, run with them, then cleared. */
const withProcessHeaders = async (run) => {
  for (const name of [PROVIDER_HEADER, MODEL_HEADER, OAUTH_HEADER]) {
    process.env[name] = "the-process";
  }
  try {
    return await run();
  } finally {
    for (const name of [PROVIDER_HEADER, MODEL_HEADER, OAUTH_HEADER]) {
      delete process.env[name];
    }
  }
};

/** Register the OAuth provider an extension would, with a header naming a variable. */
const withOAuthProvider = (models) => {
  models.registerProvider(OAUTHED, {
    baseUrl: "http://oauthed.invalid",
    api: "openai-completions",
    headers: { "X-OAuth": `$${OAUTH_HEADER}` },
    oauth: {
      name: "Fake OAuth",
      login: () => Promise.reject(new Error("no login in a test")),
      refreshToken: async (credential) => credential,
      getApiKey: (credential) => credential.access,
    },
    models: [
      {
        id: "oauthed-1",
        name: "OAuthed",
        reasoning: false,
        input: ["text"],
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
        contextWindow: 1000,
        maxTokens: 100,
      },
    ],
  });
  return models;
};

it("marks a runtime that takes an auth context", () => {
  assert.equal(ModelRuntime.hostAuthContextVersion, 2);
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

it("reads provider and model headers through the runtime's own context", async () => {
  const values = (who) => ({
    [HEADED_KEY]: `${who}-key`,
    [PROVIDER_HEADER]: `${who}-provider`,
    [MODEL_HEADER]: `${who}-viewer`,
  });
  const first = await runtime(context(values("first")));
  const second = await runtime(context(values("second")));
  await withProcessHeaders(async () => {
    const firstHeaders = await headersOf(first);
    assert.equal(firstHeaders?.["X-Provider"], "first-provider");
    assert.equal(firstHeaders?.["X-Viewer"], "first-viewer");
    const secondHeaders = await headersOf(second);
    assert.equal(secondHeaders?.["X-Provider"], "second-provider");
    assert.equal(secondHeaders?.["X-Viewer"], "second-viewer");
  });
});

it("never takes a header variable its context lacks from the process", async () => {
  const lacksProvider = await runtime(context({ [HEADED_KEY]: "mine" }));
  const lacksModel = await runtime(context({ [HEADED_KEY]: "mine", [PROVIDER_HEADER]: "mine" }));
  const blank = await runtime(
    context({ [HEADED_KEY]: "mine", [PROVIDER_HEADER]: "", [MODEL_HEADER]: "mine" }),
  );
  await withProcessHeaders(async () => {
    await assert.rejects(headersOf(lacksProvider), new RegExp(PROVIDER_HEADER));
    await assert.rejects(headersOf(lacksModel), new RegExp(MODEL_HEADER));
    await assert.rejects(headersOf(blank), new RegExp(PROVIDER_HEADER));
  });
});

it("reads an OAuth provider's headers through the context alone", async () => {
  const own = withOAuthProvider(await runtime(context({ [OAUTH_HEADER]: "mine" })));
  const lacking = withOAuthProvider(await runtime(context({})));
  await withProcessHeaders(async () => {
    const auth = (await own.getAuth(OAUTHED))?.auth;
    assert.equal(auth?.apiKey, "oauth-access");
    assert.equal(auth?.headers?.["X-OAuth"], "mine");
    await assert.rejects(lacking.getAuth(OAUTHED), new RegExp(OAUTH_HEADER));
  });
});

it("refuses header variables in the fallback for an unconfigured provider", async () => {
  const models = await runtime(context({}));
  await withProcessHeaders(() => {
    assert.throws(
      () => models.getCompatibilityRequestConfig(models.getModel(HEADED, "headed-1")),
      new RegExp(PROVIDER_HEADER),
    );
  });
});

it("reads every header from the process for a runtime given no context", async () => {
  const models = withOAuthProvider(await runtime());
  process.env[HEADED_KEY] = "the-process";
  try {
    await withProcessHeaders(async () => {
      const headers = await headersOf(models);
      assert.equal(headers?.["X-Provider"], "the-process");
      assert.equal(headers?.["X-Viewer"], "the-process");
      assert.equal((await models.getAuth(OAUTHED))?.auth?.headers?.["X-OAuth"], "the-process");
      const fallback = models.getCompatibilityRequestConfig(models.getModel(HEADED, "headed-1"));
      assert.equal(fallback.headers?.["X-Viewer"], "the-process");
    });
  } finally {
    delete process.env[HEADED_KEY];
  }
});

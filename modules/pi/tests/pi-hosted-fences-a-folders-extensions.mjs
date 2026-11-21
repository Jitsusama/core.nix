// pi as the host runs it runs a folder's own extensions' commands only
// through the exec a user extension sets, such as one inside the sandbox, and
// refuses them until one does, so a folder's extension can't run a program
// outside the sandbox that a sandboxed command has set up. A user extension's
// commands run as they always have. Unpatched, every extension's exec runs
// outside, and nothing can set how a folder's run.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
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
const { loadExtensions } = await import(pathToFileURL(loaderModule).href);

const scratch = mkdtempSync(join(tmpdir(), "pi-hosted-fenced-"));
// Each extension hands its pi to the test, and tries a command as it loads,
// so what a factory may run is seen before pi has named any scope.
const extension = (where, name) => {
  mkdirSync(where, { recursive: true });
  const entry = join(where, `${name}.ts`);
  writeFileSync(
    entry,
    [
      "export default function (pi) {",
      `  globalThis.apis.${name} = pi;`,
      `  globalThis.loading.${name} = pi.exec("echo", ["${name}"]).then(`,
      "    (ran) => ran.stdout.trim(),",
      "    (refused) => refused.message,",
      "  );",
      "  try {",
      "    pi.setProjectExec(async () => ({ stdout: '', stderr: '', code: 0, killed: false }));",
      `    globalThis.setWhileLoading.${name} = "set";`,
      "  } catch (refused) {",
      `    globalThis.setWhileLoading.${name} = refused.message;`,
      "  }",
      "}",
      "",
    ].join("\n"),
  );
  return entry;
};
globalThis.apis = {};
globalThis.loading = {};
globalThis.setWhileLoading = {};

const folder = join(scratch, "folder");
const user = extension(join(scratch, "user"), "user");
const project = extension(join(folder, ".pi", "extensions"), "project");
const cli = extension(join(scratch, "cli"), "cli");
const loaded = await loadExtensions([user, project, cli], folder);
assert.deepEqual(loaded.errors, [], "the extensions didn't load");

// While loading, a factory under the folder's own .pi is refused, and one
// elsewhere runs; none may set how a folder's extensions run.
assert.equal(
  await globalThis.loading.user,
  "user",
  "a user extension's factory couldn't run a command",
);
assert.match(
  await globalThis.loading.project,
  /comes from this folder/u,
  "a folder's extension ran a command outside as it loaded",
);
for (const name of ["user", "project", "cli"]) {
  assert.match(
    globalThis.setWhileLoading[name],
    /may not set/u,
    `${name} set how a folder's extensions run while loading`,
  );
}

// pi names each one's scope once all have loaded, as its resource loader does.
const scopes = new Map([
  [user, "user"],
  [project, "project"],
  [cli, "temporary"],
]);
for (const one of loaded.extensions) {
  one.sourceInfo = { ...one.sourceInfo, scope: scopes.get(one.path) };
}
const { user: userApi, project: projectApi, cli: cliApi } = globalThis.apis;

await assert.rejects(
  projectApi.exec("echo", ["outside"]),
  /no user extension has said how its commands run/u,
  "a folder's extension ran a command with no fenced exec set",
);
assert.throws(() => projectApi.setProjectExec(() => Promise.resolve({})), /may not set/u);
assert.throws(() => cliApi.setProjectExec(() => Promise.resolve({})), /may not set/u);

// Once a user extension sets it, a folder's commands go through it, with the
// folder they'd run in, and everyone else's run as before.
const through = [];
userApi.setProjectExec((command, args, cwd, options) => {
  through.push({ command, args, cwd, timeout: options?.timeout });
  return Promise.resolve({ stdout: "fenced\n", stderr: "", code: 0, killed: false });
});
const ran = await projectApi.exec("echo", ["inside"], { timeout: 5000 });
assert.equal(ran.stdout, "fenced\n");
assert.deepEqual(through, [{ command: "echo", args: ["inside"], cwd: folder, timeout: 5000 }]);
assert.equal((await userApi.exec("echo", ["mine"])).stdout.trim(), "mine");
assert.equal((await cliApi.exec("echo", ["named"])).stdout.trim(), "named");
assert.equal(through.length, 1, "a user extension's command went through the folder's exec");

rmSync(scratch, { recursive: true, force: true });

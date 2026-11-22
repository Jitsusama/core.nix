// pi as the host runs it keeps the extensions jiti compiled in the account's
// runtime folder, not in /tmp. A host's child has no TMPDIR, so jiti's
// default was /tmp/jiti, and on 21 November a sweep of /tmp mid-load failed
// a conversation's reload with ENOENT on a compiled file there, dropping nine
// of its tools until the next reload. With no runtime folder it keeps jiti's
// own default, so a pi without one still caches.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
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

const scratch = mkdtempSync(join(tmpdir(), "pi-hosted-jiti-"));
after(() => rmSync(scratch, { recursive: true, force: true }));
const write = (path, text) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
};

// The folders jiti could cache in: the runtime folder, and a TMPDIR of its
// own, so os.tmpdir()/jiti is seen apart from anything else in /tmp.
const runtime = join(scratch, "runtime");
const temporary = join(scratch, "tmp");
mkdirSync(runtime, { recursive: true });
mkdirSync(temporary, { recursive: true });
process.env.TMPDIR = temporary;

const { clearExtensionCache, loadExtensionsCached } = await import(
  pathToFileURL(agentFile("core/extensions/loader.js")).href
);

// A TypeScript extension, which jiti has to compile, with a name of its own
// each time so no cache from an earlier load serves it.
const extension = (name) => {
  const where = join(scratch, name);
  write(join(where, "package.json"), JSON.stringify({ name, type: "module" }));
  write(
    join(where, "index.ts"),
    [
      "export default function (pi: { registerCommand: Function }) {",
      `  const name: string = ${JSON.stringify(name)};`,
      '  pi.registerCommand(name, { description: "", handler: async () => {} });',
      "}",
      "",
    ].join("\n"),
  );
  return join(where, "index.ts");
};

const load = async (name) => {
  clearExtensionCache?.();
  const folder = join(scratch, `folder-${name}`);
  mkdirSync(folder, { recursive: true });
  const loaded = await loadExtensionsCached([extension(name)], folder);
  assert.deepEqual(
    loaded.errors ?? [],
    [],
    `the extension didn't load: ${JSON.stringify(loaded.errors)}`,
  );
  return loaded;
};
// A file jiti compiled, which it names .cjs or .mjs.
const COMPILED = /\.[cm]js$/u;
const compiled = (folder) =>
  existsSync(folder) ? readdirSync(folder).filter((file) => COMPILED.test(file)) : [];

it("caches compiled extensions in the runtime folder, not TMPDIR", async () => {
  process.env.XDG_RUNTIME_DIR = runtime;
  await load("in-runtime");
  assert.ok(
    compiled(join(runtime, "pi-jiti")).some((file) => file.includes("in-runtime")),
    `nothing compiled under ${join(runtime, "pi-jiti")}`,
  );
  assert.deepEqual(compiled(join(temporary, "jiti")), [], "a compiled file went to TMPDIR");
});

it("keeps jiti's own default where there's no runtime folder", async () => {
  delete process.env.XDG_RUNTIME_DIR;
  await load("no-runtime");
  assert.ok(
    compiled(join(temporary, "jiti")).some((file) => file.includes("no-runtime")),
    `nothing compiled under ${join(temporary, "jiti")}`,
  );
});

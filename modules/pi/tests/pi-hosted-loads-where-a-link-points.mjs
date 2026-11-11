// pi as the host runs it loads a linked extension through the file its link
// points at now: once a harness update moves the link and the copy it first
// loaded is gone, a reload loads the new copy and that copy's own helpers.
// Ordinary pi, which never outlives a harness update, imports the link as
// given, and a host's import of it failed with ENOENT on the old copy's helper.
// Run as: node THIS PI_ROOT, the built package's pi-monorepo folder.
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
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
const { clearExtensionCache, loadExtensions } = await import(pathToFileURL(loaderModule).href);

const folder = mkdtempSync(join(tmpdir(), "pi-hosted-link-"));
// Each copy names itself through a helper beside it, as the harness's
// extensions import their own modules, so a load that resolved the helper
// through the gone copy fails rather than passing on the entry alone.
const copy = (name) => {
  const where = join(folder, "copies", name);
  mkdirSync(where, { recursive: true });
  writeFileSync(join(where, "helper.ts"), `export const copy = ${JSON.stringify(name)};\n`);
  writeFileSync(
    join(where, "index.ts"),
    [
      'import { copy } from "./helper.ts";',
      "export default function (pi) {",
      '  pi.registerCommand(`from-${copy}`, { description: copy, handler: async () => {} });',
      "}",
      "",
    ].join("\n"),
  );
  return where;
};
const link = join(folder, "extension");
const entry = join(link, "index.ts");
const commandsOf = (loaded) => loaded.extensions.flatMap((one) => [...one.commands.keys()]);

symlinkSync(copy("first"), link);
const before = await loadExtensions([entry], folder);
assert.deepEqual(before.errors, [], "the first copy didn't load through its link");
assert.deepEqual(commandsOf(before), ["from-first"]);

// A harness update: the link moves to the new copy and the old one goes, as a
// package swap removes the versions no pi runs. A reload starts with an empty
// extension cache.
rmSync(link);
symlinkSync(copy("second"), link);
rmSync(join(folder, "copies", "first"), { recursive: true });
clearExtensionCache();

const after = await loadExtensions([entry], folder);
assert.deepEqual(after.errors, [], "the moved link's copy didn't load once the first was gone");
assert.deepEqual(commandsOf(after), ["from-second"]);
assert.equal(after.extensions[0].path, entry, "the extension is no longer known by its link");

rmSync(folder, { recursive: true, force: true });

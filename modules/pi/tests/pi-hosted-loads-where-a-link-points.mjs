// pi as the host runs it loads a linked extension through the file its link
// points at now: once the link an extension is found through moves, a reload
// loads the new copy with that copy's own helpers, whether the copy it first
// loaded is gone or still there. Unpatched, a reload resolves the helpers
// through the first copy, failing with ENOENT once it's gone and mixing the
// new entry with the old helpers while it stays.
// Run as: node THIS PI_ROOT, the built package's pi-monorepo folder.
import assert from "node:assert/strict";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  renameSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
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

// Each copy names itself through a helper beside it, imported by its built
// name, as linked extensions import their own, so a load that resolved the
// helper through the first copy shows it rather than passing on the entry.
const copy = (folder, name) => {
  const where = join(folder, "copies", name);
  mkdirSync(where, { recursive: true });
  writeFileSync(join(where, "helper.ts"), `export const copy = ${JSON.stringify(name)};\n`);
  writeFileSync(
    join(where, "index.ts"),
    [
      'import { copy } from "./helper.js";',
      "export default function (pi) {",
      '  pi.registerCommand("from-" + copy, { description: copy, handler: async () => {} });',
      "}",
      "",
    ].join("\n"),
  );
  return where;
};
const commandsOf = (loaded) => loaded.extensions.flatMap((one) => [...one.commands.keys()]);

// Loads an extension through its link, moves the link in one rename to a
// second copy, removing the first when asked, and loads it again, as a
// reload does, with an empty extension cache, holding that the second copy
// and its own helper load, still known by the link.
const reloadAfterTheLinkMoves = async ({ firstGoes }) => {
  const folder = mkdtempSync(join(tmpdir(), "pi-hosted-link-"));
  const link = join(folder, "extensions", "linked");
  mkdirSync(join(folder, "extensions"));
  // The entry as pi's discovery names it, through the link.
  const entry = join(link, "index.ts");
  symlinkSync(copy(folder, "first"), link);
  const before = await loadExtensions([entry], folder);
  assert.deepEqual(before.errors, [], "the first copy didn't load through its link");
  assert.deepEqual(commandsOf(before), ["from-first"]);

  symlinkSync(copy(folder, "second"), `${link}.next`);
  renameSync(`${link}.next`, link);
  if (firstGoes) {
    rmSync(join(folder, "copies", "first"), { recursive: true });
  }
  clearExtensionCache();
  const after = await loadExtensions([entry], folder);
  rmSync(folder, { recursive: true, force: true });
  const which = firstGoes ? "once the first copy was gone" : "while the first copy stayed";
  assert.deepEqual(after.errors, [], `the moved link's copy didn't load ${which}`);
  assert.deepEqual(commandsOf(after), ["from-second"], `the reload ran old code ${which}`);
  assert.equal(after.extensions[0].path, entry, "the extension is no longer known by its link");
};

await reloadAfterTheLinkMoves({ firstGoes: true });
await reloadAfterTheLinkMoves({ firstGoes: false });

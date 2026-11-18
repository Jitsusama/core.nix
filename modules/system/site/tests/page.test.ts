// What index.html asks the browser for before site.js runs. A module the page
// reaches only through another module is fetched a round later, once the one
// naming it has loaded, and a page whose script arrives after its first paint
// paints empty and then jumps as the drawing lands. So every module of the
// page's own that site.ts doesn't import itself is preloaded.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";

const SITE = new URL("../", import.meta.url);
// A type-only import is erased, so it fetches nothing.
const OWN_IMPORT = /^import\s(?!type\s)[^;]*?from\s+"\.\/(?<module>[a-z]+)\.ts";/gmu;
const PRELOAD = /<link rel="modulepreload" href="(?<module>[a-z]+)\.js">/gu;

/** The page's own modules a module imports, by name. */
function importsOf(module: string): string[] {
  const source = readFileSync(new URL(`${module}.ts`, SITE), "utf8");
  return [...source.matchAll(OWN_IMPORT)].flatMap((found) => found.groups?.["module"] ?? []);
}

/** Every one of the page's own modules site.ts reaches, at any depth. */
function reached(): Set<string> {
  const seen = new Set<string>();
  const waiting = importsOf("site");
  while (waiting.length > 0) {
    const module = waiting.pop();
    if (module !== undefined && !seen.has(module)) {
      seen.add(module);
      waiting.push(...importsOf(module));
    }
  }
  return seen;
}

it("preloads every module of its own that site.ts reaches only through another", () => {
  const direct = new Set(importsOf("site"));
  const html = readFileSync(new URL("index.html", SITE), "utf8");
  const preloaded = new Set(
    [...html.matchAll(PRELOAD)].flatMap((found) => found.groups?.["module"] ?? []),
  );
  const late = [...reached()]
    .filter((module) => !direct.has(module))
    .sort((one, other) => one.localeCompare(other));
  assert.deepEqual(
    late.filter((module) => !preloaded.has(module)),
    [],
    "a module only another names is fetched a round late",
  );
});

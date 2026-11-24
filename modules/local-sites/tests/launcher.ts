// The launcher's own files as the ask row's checks read them: handed by
// the check under the names below, its scripts run as Qt loads them, so
// the row is held to For You's code rather than to what ask.ts copied.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

/** A launcher script, its functions by their names. */
export type Script = Record<string, (...args: never[]) => unknown>;

/** A line of Qt's own, which no other engine reads, as `.pragma library`. */
const QT_LINE = /^\.(?:pragma|import)\b.*$/gmu;

/** A file the check hands its test, by the name it hands it under. */
export function launcher(name: string): string {
  const path = process.env[name];
  assert.ok(path, `the check handed no ${name}`);
  return readFileSync(path, "utf8");
}

/**
 * A launcher script as Qt loads it: run on its own, its functions what it
 * gives, with `imports`, the scripts it imports, by the names it imports
 * them as.
 */
export function loaded(name: string, imports: Record<string, Script> = {}): Script {
  const script: Script = { ...imports };
  runInNewContext(launcher(name).replace(QT_LINE, ""), script);
  return script;
}

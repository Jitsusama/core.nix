// What a page may answer of a look, read from the table the server's own
// test reads too (offered.json, by answer.rs's `offered`), so the page and
// the server that guards it say alike which presses may be carried.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { answering } from "../ask.ts";

/** One look of the table: what it differs by, what's pressed, and whether it's taken. */
interface Case {
  readonly why: string;
  readonly with: Readonly<Record<string, unknown>>;
  readonly value: string;
  readonly offered: boolean;
}

const table = JSON.parse(readFileSync(new URL("offered.json", import.meta.url), "utf8")) as {
  readonly now: string;
  readonly look: Readonly<Record<string, unknown>>;
  readonly cases: readonly Case[];
};

it("a page answers just the looks the table says the server takes", () => {
  const now = Date.parse(table.now);
  for (const each of table.cases) {
    const look = { ...table.look, ...each.with };
    assert.equal(answering(look, each.value, now) !== null, each.offered, each.why);
  }
});

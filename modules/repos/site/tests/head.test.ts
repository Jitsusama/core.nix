// What the page's foot says of where the grove was read and what couldn't be.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { footOf } from "../head.ts";
import { type Feed, groveOf } from "../model.ts";

const NOW = Date.parse("2026-11-16T15:00:00Z");

/** A feed told only the daemon's reading, with `unread` from GitHub. */
function readingWith(unread: readonly string[]): Feed {
  const reading = {
    key: "reading",
    kind: "reading",
    machine: "kestrel",
    roots: ["/home/ada/src"],
    sources: [{ name: "github", what: "changes", unread }],
  };
  const kept = (records: readonly object[]): Held => {
    const held = new Held();
    held.took({ records });
    return held;
  };
  return {
    reading: kept([reading]),
    repository: kept([]),
    checkout: kept([]),
    tree: kept([]),
    change: kept([]),
    using: kept([]),
  };
}

it("says where the grove was read, and nothing more when everything was", () => {
  const grove = groveOf(readingWith([]), [], NOW);
  assert.equal(footOf(grove, undefined, []), "Read on kestrel.", "the foot said more");
});

it("says what couldn't be read in one sentence, the feeds unheard after the sources", () => {
  const grove = groveOf(readingWith(["github:owl/a: gone", "github:owl/b: gone"]), [], NOW);
  assert.equal(
    footOf(grove, undefined, ["the map"]),
    "Read on kestrel. Couldn't read owl/a and owl/b on github, gone; the map.",
    "the foot didn't join the reasons into one sentence",
  );
});

// What Return on the map's new-work line comes to: a machine that left the
// map let go of and said, words no link can carry refused with why, or the
// link that hands them on, with the words kept to send again.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  CLOUD,
  keptOnStands,
  LONGEST_WORDS,
  leftWords,
  NO_MACHINE,
  NO_WORDS,
  type Returned,
  returned,
  sproutLink,
  unsprouted,
} from "../sprout.ts";

it("lets go of a machine that left before Return, whatever the words", () => {
  for (const typed of ["tidy the rows", ""]) {
    assert.deepEqual(
      returned(typed, "TERN02", "heron", ["kestrel", "osprey"]),
      { kind: "left", on: "heron", said: leftWords("heron") },
      `${JSON.stringify(typed)} for a machine gone should say it left, not why no link`,
    );
  }
  assert.deepEqual(
    returned("tidy", null, "osprey", []),
    { kind: "left", on: "osprey", said: leftWords("osprey") },
    "a machine on no map should have left it",
  );
});

it("says a machine that left before Return, and where the work starts instead", () => {
  assert.equal(
    leftWords("osprey"),
    "osprey has left the map, so the work won't start there; " +
      "Return starts it where new work starts.",
  );
});

it("refuses Return that no link can carry, saying why", () => {
  assert.deepEqual(
    returned(" \t", "TERN02", null, []),
    { kind: "refused", why: NO_WORDS },
    "no words should be refused as no words",
  );
  assert.deepEqual(
    returned("", null, "osprey", ["osprey"]),
    { kind: "refused", why: NO_WORDS },
    "no words on a machine still on the map should be refused as no words",
  );
  assert.deepEqual(
    returned("tidy", null, "os prey", ["os prey"]),
    { kind: "refused", why: NO_MACHINE },
    "a machine on the map whose name no link carries should be refused",
  );
});

it("hands on the link from Return, keeping the words as typed", () => {
  assert.deepEqual(
    returned("  tidy the rows ", null, null, []),
    {
      kind: "link",
      href: "mycelium:sprout?words=tidy%20the%20rows",
      kept: { words: "  tidy the rows ", from: null, on: null },
    },
    "words from no cell on no machine should go alone, kept as typed",
  );
  assert.deepEqual(
    returned("tidy", "TERN02", "osprey", ["osprey"]),
    {
      kind: "link",
      href: "mycelium:sprout?words=tidy&from=TERN02&on=osprey",
      kept: { words: "tidy", from: "TERN02", on: "osprey" },
    },
    "words from a cell on a machine on the map should go from it, on it",
  );
  assert.deepEqual(
    returned("tidy", null, CLOUD, []),
    {
      kind: "link",
      href: "mycelium:sprout?words=tidy&on=cloud",
      kept: { words: "tidy", from: null, on: CLOUD },
    },
    "the cloud with no machines on the map should stand, being no machine",
  );
  const long = "a".repeat(LONGEST_WORDS + 5);
  assert.deepEqual(
    returned(long, null, null, []),
    {
      kind: "link",
      href: `mycelium:sprout?words=${"a".repeat(LONGEST_WORDS)}`,
      kept: { words: long, from: null, on: null },
    },
    "words past the cap should go cut at it, as the link carries them",
  );
});

// What Return should come to, as keptOnStands, sproutLink and unsprouted
// each say it apart.
const wanted = (
  typed: string,
  from: string | null,
  on: string | null,
  machines: readonly string[],
): Returned => {
  if (!keptOnStands(on, machines)) {
    return { kind: "left", on: String(on), said: leftWords(String(on)) };
  }
  const href = sproutLink(typed, from, on);
  if (href === null) {
    return { kind: "refused", why: String(unsprouted(typed, on)) };
  }
  return { kind: "link", href, kept: { words: typed, from, on } };
};

it("returns as keptOnStands and sproutLink say, in every combination", () => {
  for (const typed of ["tidy the rows", "", " \t", "\u200b"]) {
    for (const from of [null, "TERN02", "tern02"]) {
      for (const on of [null, "", "osprey", "heron", CLOUD, "os prey", "-help"]) {
        for (const machines of [[], ["osprey"], ["osprey", "os prey"]]) {
          assert.deepEqual(
            returned(typed, from, on, machines),
            wanted(typed, from, on, machines),
            `${JSON.stringify({ typed, from, on, machines })}: what Return comes to`,
          );
        }
      }
    }
  }
});

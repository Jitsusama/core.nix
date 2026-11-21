// The link the map hands the launcher's Start New Work: the words as one
// line and percent-encoded, the cell it comes from only when that is an
// address, and no link at all without words.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { handedWords, isAddress, LONGEST_WORDS, sproutLink, sproutWords } from "../sprout.ts";

it("reads the typed words as one line", () => {
  assert.equal(
    sproutWords("  tidy\tthe\n rows  "),
    "tidy the rows",
    "spaces, tabs and line breaks should each read as one space, none at the ends",
  );
});

it("encodes a lone half of a UTF-16 pair rather than throwing", () => {
  assert.equal(
    sproutLink("tidy \uD800 rows", null),
    "mycelium:sprout?words=tidy%20%EF%BF%BD%20rows",
    "a lone half should read as the replacement character",
  );
});

it("cuts the words where the line's own field does", () => {
  const long = "a".repeat(LONGEST_WORDS + 100);
  assert.equal(sproutWords(long).length, LONGEST_WORDS, "the words should stop at the cap");
  assert.equal(
    sproutWords(`${"a".repeat(LONGEST_WORDS - 1)} tail`),
    "a".repeat(LONGEST_WORDS - 1),
    "a cut falling on a space should leave no space at the end",
  );
  assert.equal(
    sproutWords(`${"a".repeat(LONGEST_WORDS - 1)}\u{1F331}`),
    `${"a".repeat(LONGEST_WORDS - 1)}\uFFFD`,
    "a cut through a pair of halves should leave the replacement character",
  );
});

it("hands no link without words", () => {
  assert.equal(sproutLink("", null), null, "nothing typed should hand nothing");
  assert.equal(
    sproutLink(" \n\t ", "TERN02"),
    null,
    "only spaces should hand nothing, from a cell too",
  );
});

it("percent-encodes the words, so none of them can add to the link", () => {
  const link = sproutLink("fix #5677 & the rest?from=X y/z ü", null);
  assert.equal(
    link,
    "mycelium:sprout?words=fix%20%235677%20%26%20the%20rest%3Ffrom%3DX%20y%2Fz%20%C3%BC",
    "every letter a link reads as its own should be encoded",
  );
  assert.equal(
    decodeURIComponent(link?.split("words=")[1] ?? ""),
    "fix #5677 & the rest?from=X y/z ü",
    "the launcher should read back the words as typed",
  );
});

it("says the cell it comes from only when that is an address", () => {
  assert.equal(
    sproutLink("tidy the rows", "TERN02"),
    "mycelium:sprout?words=tidy%20the%20rows&from=TERN02",
    "an open cell's address should go with the words",
  );
  assert.equal(isAddress("TERN02"), true, "an address should be one");
  assert.equal(isAddress(null), false, "no cell should be no address");
  for (const from of ["T", "9", "ABCDEFGHIJKL"]) {
    assert.equal(isAddress(from), true, `${from} should be an address, 1 to 12 long`);
  }
  for (const from of ["tern02", "TERN-02", "", "ABCDEFGHIJKLM", "X&words=Y", "calls@kestrel"]) {
    assert.equal(isAddress(from), false, `${JSON.stringify(from)} should be no address`);
    assert.equal(
      sproutLink("tidy the rows", from),
      "mycelium:sprout?words=tidy%20the%20rows",
      `${JSON.stringify(from)} isn't an address, so it should be left out`,
    );
  }
});

it("says where the work went, that nothing starts yet, and how to send it again", () => {
  const again = " If it didn't open, wait a moment, then s brings the words back.";
  assert.equal(
    handedWords(null),
    `Sent to the Launcher's Start New Work; nothing starts until you press Return there.${again}`,
  );
  assert.equal(
    handedWords("Heron"),
    `Sent to the Launcher's Start New Work, from Heron; nothing starts until you press Return there.${again}`,
  );
});

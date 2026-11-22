// The link the map hands the launcher's Start New Work: the words as one
// line and percent-encoded, the cell it comes from only when that is an
// address, and no link at all without words.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
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
  assert.equal(
    sproutLink("\u200B\u202E\u0007 \u2800\uFE0F", null),
    null,
    "only what the launcher leaves out should hand nothing, as it would refuse it",
  );
});

it("leaves out what the launcher leaves out of the words", () => {
  assert.equal(
    sproutWords("ti\u200Bdy \u202Ethe\u0007 \u2800 rows"),
    "tidy the rows",
    "control, format and blank characters should go, and a word of only them with them",
  );
  assert.equal(
    sproutWords("tidy\u0085the\uFEFFse rows"),
    "tidy these rows",
    "U+0085 should part words and U+FEFF go, as plain.rs splits and drops them",
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
  assert.equal(isAddress(null), false, "no cell should be no address");
  for (const from of ["tern02", "TERN-02", "", "ABCDEFGHIJKLM", "X&words=Y", "calls@kestrel"]) {
    assert.equal(
      sproutLink("tidy the rows", from),
      "mycelium:sprout?words=tidy%20the%20rows",
      `${JSON.stringify(from)} isn't an address, so it should be left out`,
    );
  }
});

it("judges an address as the notices daemon and mycelium-link do", () => {
  const cases = readFileSync(new URL("addresses.txt", import.meta.url), "utf8")
    .split("\n")
    .filter((line) => line !== "" && !line.startsWith("#"));
  assert.ok(cases.length > 20, "the shared cases should be read");
  for (const line of cases) {
    assert.ok(!line.includes("\r"), `${JSON.stringify(line)} should hold no carriage return`);
    const [kind, ...rest] = line.split(" ");
    const text = rest
      .join(" ")
      .replace(/\\u\{([0-9A-F]{1,6})\}/gu, (_, code) => String.fromCodePoint(Number.parseInt(code, 16)));
    assert.ok(!text.includes("\\"), `${JSON.stringify(line)} should escape only as \\u{HEX}`);
    assert.ok(kind === "is" || kind === "not", `${JSON.stringify(line)} should say is or not`);
    const is = kind === "is";
    assert.equal(isAddress(text), is, `${JSON.stringify(text)} should ${is ? "" : "not "}be one`);
  }
});

it("says where the work went, that nothing starts yet, and how to send it again", () => {
  const again = " If it didn't open, wait a moment, then s brings the words back.";
  assert.equal(
    handedWords(null),
    `Sent to the Launcher's Start New Work; nothing starts until you press Return there.${again}`,
  );
  const fromHeron = "Sent to the Launcher's Start New Work, from Heron; ";
  assert.equal(
    handedWords("Heron"),
    `${fromHeron}nothing starts until you press Return there.${again}`,
  );
});

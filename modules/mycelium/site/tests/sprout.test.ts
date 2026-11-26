// The link the map hands the launcher's Start New Work: the words as one
// line and percent-encoded, the cell it comes from only when that is an
// address, and no link at all without words.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import {
  CLOUD,
  handedWords,
  isAddress,
  isMachine,
  keptOnStands,
  LONGEST_WORDS,
  NO_MACHINE,
  NO_WORDS,
  onWords,
  sproutLink,
  sproutWords,
  unsprouted,
} from "../sprout.ts";

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

it("says the machine it starts on, and hands nothing for a misshaped one", () => {
  assert.equal(
    sproutLink("tidy the rows", null, "osprey"),
    "mycelium:sprout?words=tidy%20the%20rows&on=osprey",
    "a machine's name should go with the words",
  );
  assert.equal(
    sproutLink("tidy", "TERN02", CLOUD),
    "mycelium:sprout?words=tidy&from=TERN02&on=cloud",
    "the cloud should go as cloud, after the cell",
  );
  assert.equal(
    sproutLink("tidy", null, "v2s-gcp-provider-s99ia2-1795647845"),
    "mycelium:sprout?words=tidy&on=v2s-gcp-provider-s99ia2-1795647845",
    "a cloud machine's own name should go as it is",
  );
  assert.equal(isMachine(null), false, "no machine should be no name");
  assert.equal(isMachine("a".repeat(64)), true, "64 letters should be a name");
  assert.equal(
    sproutLink("tidy", null, ""),
    "mycelium:sprout?words=tidy",
    "an empty machine should name none, as mycelium-link reads it",
  );
  for (const on of ["-help", ".hidden", "os prey", "osprey&words=x", "a".repeat(65)]) {
    assert.equal(
      sproutLink("tidy", null, on),
      null,
      `${JSON.stringify(on)} isn't a machine's name, so work should start nowhere`,
    );
  }
  assert.equal(sproutLink("", null, "osprey"), null, "no words should hand nothing");
  assert.equal(onWords("osprey"), "On osprey", "a machine should be said by its name");
  assert.equal(onWords(CLOUD), "On the cloud", "the cloud should be said as a place");
});

it("says why no link was given: no words, or a machine no link can name", () => {
  assert.equal(unsprouted("  ", null), NO_WORDS, "no words should be said as no words");
  assert.equal(unsprouted("", "os prey"), NO_WORDS, "with no words, those come first");
  assert.equal(unsprouted("tidy", "os prey"), NO_MACHINE, "words on a bad machine name it");
  assert.equal(sproutLink("tidy", null, "os prey"), null, "and that machine gives no link");
});

it("keeps words for a machine only while the map still has it", () => {
  const names = ["kestrel", "osprey"];
  assert.equal(keptOnStands("osprey", names), true, "a machine on the map should stand");
  assert.equal(keptOnStands("heron", names), false, "a machine that left should go");
  assert.equal(keptOnStands(CLOUD, []), true, "the cloud is no machine, and should stand");
  assert.equal(keptOnStands(null, []), true, "words sent on no machine should stand");
});

/** The escape a case may hold: 1 to 6 capital hex digits (README.md, The Address Cases). */
const ESCAPE = /\\u\{([0-9A-F]{1,6})\}/gu;

/** A case's text with each escape made the character it names, failing on any other. */
function caseText(written: string): string {
  const line = JSON.stringify(written);
  assert.ok(!written.replace(ESCAPE, "").includes("\\"), `${line} should escape only as \\u{HEX}`);
  return written.replace(ESCAPE, (_, hex: string) => {
    const code = Number.parseInt(hex, 16);
    assert.ok(
      code <= 0x10_ff_ff && (code < 0xd8_00 || code > 0xdf_ff),
      `${line} should name a character`,
    );
    return String.fromCodePoint(code);
  });
}

it("refuses a case escaped any other way", () => {
  for (const written of [
    "T\\",
    "T\\n",
    "T\\u{a}",
    "T\\u{+A}",
    "T\\u{0000041}",
    "T\\u{}",
    "T\\u{41",
    "T\\u{D800}",
    "T\\u{DFFF}",
    "T\\u{110000}",
    "T\\\\",
  ]) {
    assert.throws(() => caseText(written), `${JSON.stringify(written)} should be refused`);
  }
  assert.equal(caseText("T\\u{5C}\\u{9}"), "T\\\t", "each escape should be read the one way");
  assert.equal(
    caseText("\\u{000041}\\u{10FFFF}\\u{E000}\\u{D7FF}"),
    "A\u{10FFFF}\u{E000}\u{D7FF}",
    "six digits, the top and each side of the surrogates should be read",
  );
  assert.equal(
    caseText("T\\u{5C}u{41}"),
    "T\\u{41}",
    "what an escape gives should never be read again",
  );
});

/**
 * The cases `written` holds, each whether it is an address and its text, failing on a line
 * written any way but one, a carriage return in it included (README.md, The Address Cases).
 */
function casesIn(written: string): [boolean, string][] {
  return written
    .split("\n")
    .filter((line) => line !== "" && !line.startsWith("#"))
    .map((line) => {
      assert.ok(!line.includes("\r"), `${JSON.stringify(line)} should hold no carriage return`);
      const [kind, ...rest] = line.split(" ");
      assert.ok(kind === "is" || kind === "not", `${JSON.stringify(line)} should say is or not`);
      return [kind === "is", caseText(rest.join(" "))];
    });
}

it("refuses a line written any other way", () => {
  for (const written of ["is T\r", "not T\r\n", "is T\n\r", "IS T", "maybe T"]) {
    assert.throws(() => casesIn(written), `${JSON.stringify(written)} should be refused`);
  }
  assert.deepEqual(
    casesIn("# a note\nis T\n\nnot\nnot T "),
    [
      [true, "T"],
      [false, ""],
      [false, "T "],
    ],
    "a note and an empty line should be skipped, and `not` alone should be the empty text",
  );
});

it("judges each shared address case", () => {
  const cases = casesIn(readFileSync(new URL("addresses.txt", import.meta.url), "utf8"));
  assert.ok(cases.length > 20, "the shared cases should be read");
  for (const [is, text] of cases) {
    assert.equal(isAddress(text), is, `${JSON.stringify(text)} should ${is ? "" : "not "}be one`);
  }
});

it("judges each shared machine case, as mycelium-link does", () => {
  const cases = casesIn(readFileSync(new URL("machines.txt", import.meta.url), "utf8"));
  assert.ok(cases.length > 20, "the shared cases should be read");
  for (const [is, text] of cases) {
    assert.equal(isMachine(text), is, `${JSON.stringify(text)} should ${is ? "" : "not "}be one`);
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
  assert.equal(
    handedWords(null, "osprey"),
    `Sent to the Launcher's Start New Work, on osprey; nothing starts until you press Return there.${again}`,
  );
  assert.equal(
    handedWords("Heron", CLOUD),
    `Sent to the Launcher's Start New Work, from Heron, on the cloud; nothing starts until you press Return there.${again}`,
  );
});

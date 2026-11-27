// The link the map hands the launcher's Start New Work: the words as one
// line and percent-encoded, the cell it comes from only when that is an
// address, and no link at all without words.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import {
  type Asked,
  CLOUD,
  drawerOffersOn,
  handedWords,
  isAddress,
  isMachine,
  keptOnStands,
  LONGEST_WORDS,
  NO_MACHINE,
  NO_WORDS,
  offered,
  onWords,
  type Standing,
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
  assert.equal(unsprouted("tidy", "osprey"), null, "words on a machine a link names go");
  for (const typed of ["", " \t", "tidy", "\u200b"]) {
    for (const on of [null, "", "osprey", "cloud", "os prey", "-help"]) {
      assert.equal(
        sproutLink(typed, "DAHTNJ", on) === null,
        unsprouted(typed, on) !== null,
        `${JSON.stringify(typed)} on ${JSON.stringify(on)}: a link only where nothing is said`,
      );
    }
  }
});

it("keeps words for a machine only while the map still has it", () => {
  const names = ["kestrel", "osprey"];
  assert.equal(keptOnStands("osprey", names), true, "a machine on the map should stand");
  assert.equal(keptOnStands("heron", names), false, "a machine that left should go");
  assert.equal(keptOnStands(CLOUD, []), true, "the cloud is no machine, and should stand");
  assert.equal(keptOnStands(null, []), true, "words sent on no machine should stand");
});

it("offers a machine's drawer button on it, but never on a machine named cloud", () => {
  assert.equal(drawerOffersOn("osprey", false), "osprey", "a machine should offer itself");
  assert.equal(
    drawerOffersOn("heron basin", true),
    CLOUD,
    "a basin's stand should offer the cloud",
  );
  assert.equal(
    drawerOffersOn(CLOUD, false),
    null,
    "a machine named cloud should offer nothing, since its link would start any cloud machine",
  );
  assert.equal(
    drawerOffersOn("os prey", false),
    null,
    "a name no link carries should offer nothing",
  );
});

// A map with kestrel and osprey on it and the cell TERN02 gone from it, its drawer open on
// `drawer`: a cell's address, a machine's name, or nothing.
const onMap = (drawer: string | null): Standing => ({
  machines: ["kestrel", "osprey"],
  drawerOn: drawer === "kestrel" || drawer === "osprey" ? drawer : null,
  offersFrom: (address) => address === drawer,
  stands: (address) => address !== "TERN02",
});

// What the s key asks with `drawer` open, with what `rest` says in place of its own.
const askedOn = (drawer: string | null, rest: Partial<Asked> = {}): Asked => ({
  from: drawer === "HERON1" || drawer === "EGRET2" ? drawer : null,
  on: onMap(drawer).drawerOn,
  named: false,
  kept: null,
  typed: "",
  ...rest,
});

const keptWords = { words: "tidy the rows", from: "EGRET2", on: "kestrel" };

it("offers the open drawer's cell or machine when no words are kept", () => {
  assert.deepEqual(
    offered(askedOn("HERON1"), onMap("HERON1")),
    { from: "HERON1", on: null, kept: false },
    "a cell's drawer should offer its cell",
  );
  assert.deepEqual(
    offered(askedOn("osprey"), onMap("osprey")),
    { from: null, on: "osprey", kept: false },
    "a machine's drawer should offer its machine",
  );
  assert.deepEqual(
    offered(askedOn(null), onMap(null)),
    { from: null, on: null, kept: false },
    "the whole map should offer nothing",
  );
  assert.deepEqual(
    offered(askedOn("HERON1", { from: "EGRET2", on: "osprey" }), onMap("HERON1")),
    { from: null, on: null, kept: false },
    "a cell or machine the open drawer doesn't offer should be left out",
  );
  assert.deepEqual(
    offered(askedOn("osprey", { from: "tern02" }), { ...onMap("osprey"), offersFrom: () => true }),
    { from: null, on: "osprey", kept: false },
    "what isn't an address should never be offered, whatever the drawer has",
  );
});

it("offers kept words from their cell and on their machine, whatever drawer is open", () => {
  for (const drawer of [null, "HERON1", "osprey"]) {
    assert.deepEqual(
      offered(askedOn(drawer, { kept: keptWords, typed: keptWords.words }), onMap(drawer)),
      { from: "EGRET2", on: "kestrel", kept: true },
      `with ${JSON.stringify(drawer)} open, the kept words should go where they went`,
    );
  }
  const fromNone = { ...keptWords, from: null };
  assert.deepEqual(
    offered(askedOn("HERON1", { kept: fromNone, typed: fromNone.words }), onMap("HERON1")),
    { from: null, on: "kestrel", kept: true },
    "words that went from no cell should go from none, not from the open one",
  );
  const onNone = { ...keptWords, on: null };
  assert.deepEqual(
    offered(askedOn("osprey", { kept: onNone, typed: onNone.words }), onMap("osprey")),
    { from: "EGRET2", on: null, kept: true },
    "words that went on no machine should go on none, not on the open drawer's",
  );
  const onCloud = { ...keptWords, on: CLOUD };
  assert.deepEqual(
    offered(askedOn(null, { kept: onCloud, typed: onCloud.words }), {
      ...onMap(null),
      machines: [],
    }),
    { from: "EGRET2", on: CLOUD, kept: true },
    "words for the cloud should stand with no machine on the map",
  );
});

it("offers kept words from the open cell once theirs has gone, still on their machine", () => {
  const gone = { ...keptWords, from: "TERN02" };
  assert.deepEqual(
    offered(askedOn("HERON1", { kept: gone, typed: gone.words }), onMap("HERON1")),
    { from: "HERON1", on: "kestrel", kept: true },
    "a gone cell's words should go from the cell open now",
  );
  assert.deepEqual(
    offered(askedOn("osprey", { kept: gone, typed: gone.words }), onMap("osprey")),
    { from: null, on: "kestrel", kept: true },
    "on a machine's drawer they should go from no cell, on the machine they went to",
  );
  const goneOnNone = { ...gone, on: null };
  assert.deepEqual(
    offered(askedOn("osprey", { kept: goneOnNone, typed: goneOnNone.words }), onMap("osprey")),
    { from: null, on: null, kept: true },
    "a gone cell should never send its words on the open drawer's machine, where they never went",
  );
});

it("offers afresh for a drawer's button, new words or a machine that has left", () => {
  assert.deepEqual(
    offered(
      askedOn("osprey", { named: true, kept: keptWords, typed: keptWords.words }),
      onMap("osprey"),
    ),
    { from: null, on: "osprey", kept: false },
    "a drawer's button should offer its own machine over the kept words'",
  );
  assert.deepEqual(
    offered(
      askedOn("HERON1", { named: true, kept: keptWords, typed: keptWords.words }),
      onMap("HERON1"),
    ),
    { from: "HERON1", on: null, kept: false },
    "a drawer's button should offer its own cell over the kept words'",
  );
  for (const drawer of [null, "HERON1", "osprey"]) {
    assert.deepEqual(
      offered(askedOn(drawer, { kept: keptWords, typed: "tidy the rows again" }), onMap(drawer)),
      offered(askedOn(drawer), onMap(drawer)),
      `words typed over the kept ones with ${JSON.stringify(drawer)} open should be new work`,
    );
  }
  const left = { ...keptWords, on: "heron" };
  assert.deepEqual(
    offered(askedOn("HERON1", { kept: left, typed: left.words }), onMap("HERON1")),
    { from: "HERON1", on: null, kept: false },
    "words for a machine that has left should never go there, nor from their cell",
  );
});

it("offers only the kept words' machine or the open drawer's, in every combination", () => {
  for (const drawer of [null, "HERON1", "EGRET2", "osprey", "kestrel"]) {
    for (const named of [false, true]) {
      for (const typed of ["tidy the rows", "other words"]) {
        for (const from of [null, "EGRET2", "TERN02"]) {
          for (const on of [null, "kestrel", "heron", CLOUD]) {
            const kept = { words: "tidy the rows", from, on };
            const asked = askedOn(drawer, { named, kept, typed });
            const map = onMap(drawer);
            const got = offered(asked, map);
            const keeps = !named && typed === kept.words && on !== "heron";
            const said = JSON.stringify({ drawer, named, typed, from, on });
            assert.equal(got.kept, keeps, `${said}: whether the kept words are offered`);
            assert.equal(got.on, keeps ? on : map.drawerOn, `${said}: the machine offered`);
            const fresh = asked.from !== null && map.offersFrom(asked.from) ? asked.from : null;
            const stays = keeps && (from === null || map.stands(from));
            assert.equal(got.from, stays ? from : fresh, `${said}: the cell offered`);
          }
        }
      }
    }
  }
});

it("never offers a name no link carries, nor a cell the open drawer doesn't", () => {
  const unnamed = "kestrel basin";
  const map = { ...onMap(null), machines: [unnamed], drawerOn: unnamed };
  const kept = { ...keptWords, on: unnamed };
  assert.deepEqual(
    offered(askedOn(null, { kept, typed: kept.words }), map),
    { from: "EGRET2", on: null, kept: true },
    "kept words should never go on a name no link carries",
  );
  assert.deepEqual(
    offered(askedOn(null, { on: unnamed }), map),
    { from: null, on: null, kept: false },
    "a drawer should never offer a name no link carries",
  );
  const gone = { ...keptWords, from: "TERN02" };
  assert.deepEqual(
    offered(askedOn("HERON1", { from: "EGRET2", kept: gone, typed: gone.words }), onMap("HERON1")),
    { from: null, on: "kestrel", kept: true },
    "a gone cell's words should never go from a cell the open drawer doesn't offer",
  );
});

/** The escape a case may hold: 1 to 6 capital hex digits (README.md, The Address Cases). */
const ESCAPE = /\\u\{([0-9A-F]{1,6})\}/gu;

/** Plain letters and digits, as a machine case at the length bound's edge is written. */
const PLAIN_LETTERS = /^[a-z0-9]+$/iu;

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
  // The length bound's own edge, in plain letters, so neither copy can move it and pass.
  const plainAt = (at: number, is: boolean) =>
    cases.some(([said, text]) => said === is && text.length === at && PLAIN_LETTERS.test(text));
  assert.ok(plainAt(64, true) && plainAt(65, false), "the cases should hold the edge at 64");
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

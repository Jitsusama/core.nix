// What the murmurs page makes of the mycelium node's murmur records, told
// as the node tells them: one murmur:<writer>:<at> record for each murmur a
// quest's sessions said into its web. Every quest and word here is made up.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { ago, headline, lede, murmursOf, questsOf, trimmed } from "../model.ts";

/** A murmur record by `writer` at `at`, with `more` over its fields. */
function murmur(writer: string, at: number, more: { readonly [field: string]: unknown } = {}) {
  return {
    key: `murmur:${writer}:${at}`,
    kind: "murmur",
    heard: at * 1000,
    writer,
    at,
    text: `${writer} noticed the tide turn at ${at}.`,
    tied_to: [],
    purpose: `Watch the ${writer.toLowerCase()} shore: the tide's hours, read each morning.`,
    settled: null,
    ...more,
  };
}

/** What a watch holds once its first answer has told it `records`. */
function holding(records: readonly { readonly key: string }[]): Held {
  const held = new Held();
  held.took({ id: 2, records });
  return held;
}

it("murmurs come newest first, each named by its quest's title", () => {
  const { murmurs, unread } = murmursOf(
    holding([murmur("OSPREY", 100), murmur("KESTRL", 300), murmur("TERNAA", 200)]),
  );
  assert.deepEqual(
    murmurs.map((each) => [each.writer, each.quest]),
    [
      ["KESTRL", "Watch the kestrl shore"],
      ["TERNAA", "Watch the ternaa shore"],
      ["OSPREY", "Watch the osprey shore"],
    ],
    "murmurs should be newest first, each with its quest's title before the colon",
  );
  assert.deepEqual(unread, [], "every record here is whole, so none should be unread");
});

it("a tie names the other quests by their short ids, never the writer's own", () => {
  const { murmurs } = murmursOf(
    holding([
      murmur("KESTRL", 100, {
        tied_to: ["QEST-20261116-KESTRL", "QEST-20261116-OSPREY", "HERONS"],
      }),
    ]),
  );
  assert.deepEqual(
    murmurs[0]?.ties,
    ["OSPREY", "HERONS"],
    "ties should be short ids, the writer's own quest left out",
  );
});

it("a settled murmur keeps how it settled, and an empty settling is still in the air", () => {
  const { murmurs } = murmursOf(
    holding([
      murmur("KESTRL", 200, { settled: "The tide tables were wrong; fixed." }),
      murmur("OSPREY", 100, { settled: "" }),
    ]),
  );
  assert.deepEqual(
    murmurs.map((each) => each.settled),
    ["The tide tables were wrong; fixed.", null],
    "a settling's words should be kept, and an empty one read as not settled",
  );
});

it("a murmur that names no writer, words or time is said, and the rest still drawn", () => {
  const { murmurs, unread } = murmursOf(
    holding([{ key: "murmur:GULLSS:1", kind: "murmur" }, murmur("KESTRL", 100)]),
  );
  assert.deepEqual(
    murmurs.map((each) => each.writer),
    ["KESTRL"],
    "the whole murmur should still be read",
  );
  assert.deepEqual(
    unread,
    ["murmur:GULLSS:1, which names no writer, words or time"],
    "the broken record should be named",
  );
});

it("ties or a settling not in the node's shape are drawn as none, and said", () => {
  const { murmurs, unread } = murmursOf(
    holding([murmur("KESTRL", 100, { tied_to: "OSPREY", settled: { how: "done" } })]),
  );
  assert.deepEqual(
    murmurs.map((each) => [each.ties, each.settled]),
    [[[], null]],
    "the murmur should still be drawn, untied and in the air",
  );
  assert.deepEqual(
    unread,
    [
      "murmur:KESTRL:100's ties, which aren't a list of names",
      "murmur:KESTRL:100's settling, which isn't words",
    ],
    "both misreadings should be said",
  );
});

it("the headline counts the murmurs and those still in the air", () => {
  const { murmurs } = murmursOf(
    holding([
      murmur("KESTRL", 300),
      murmur("OSPREY", 200),
      murmur("TERNAA", 100, { settled: "Done." }),
    ]),
  );
  const said = headline(murmurs);
  assert.equal(
    `${said.lead} ${said.turn}`,
    "Three murmured, two still in the air.",
    "the sentence should count all three, and the two not settled",
  );
  assert.equal(said.open, 2, "two are still in the air");
});

it("the headline turns when none or all have settled, and when nothing was said", () => {
  const settledAll = murmursOf(holding([murmur("KESTRL", 100, { settled: "Done." })])).murmurs;
  const openAll = murmursOf(holding([murmur("KESTRL", 100), murmur("OSPREY", 50)])).murmurs;
  const says = (one: ReturnType<typeof headline>): string => `${one.lead} ${one.turn}`;
  assert.equal(says(headline(settledAll)), "One murmured, every one come to roost.", "all settled");
  assert.equal(says(headline(openAll)), "Two murmured, all still in the air.", "none settled");
  assert.equal(says(headline([])), "Nothing murmured yet.", "nothing said");
});

it("the lede quotes the newest still in the air, cut on a word", () => {
  const long = `${"gulls wheel over the cistern ".repeat(8)}end`;
  const { murmurs } = murmursOf(
    holding([murmur("KESTRL", 300, { settled: "Done." }), murmur("OSPREY", 200, { text: long })]),
  );
  const line = lede(murmurs);
  assert.ok(
    line.startsWith("Newest in the air, from Watch the osprey shore: gulls wheel"),
    `the lede should quote osprey's murmur, not kestrel's settled one: ${line}`,
  );
  assert.ok(line.endsWith("over the..."), `the quote should end on a whole word: ${line}`);
});

it("a murmur's words are cut on a word, without a stop before the ellipsis", () => {
  assert.equal(trimmed("short", 140), "short", "words that fit should stand whole");
  assert.equal(trimmed("the skiff, the tern", 10), "the skiff...", "the comma should go");
  assert.equal(trimmed("cisterncistern", 7), "cistern...", "a word too long is cut at the most");
  assert.equal(trimmed("🐦🐦🐦🐦", 2), "🐦🐦...", "a character beyond one unit is never split");
});

it("an age is said short, from just now to days", () => {
  assert.deepEqual(
    [ago(100, 130), ago(0, 600), ago(0, 7200), ago(0, 3 * 86_400), ago(500, 100)],
    ["just now", "10 min ago", "2 h ago", "3 days ago", "just now"],
    "ages should read short, and a time ahead of the clock as just now",
  );
});

it("each murmuring quest is named once, in the order of its newest murmur", () => {
  const { murmurs } = murmursOf(
    holding([murmur("KESTRL", 100), murmur("OSPREY", 300), murmur("KESTRL", 200)]),
  );
  assert.deepEqual(
    questsOf(murmurs),
    ["OSPREY", "KESTRL"],
    "osprey's murmur is newest, then kestrel's",
  );
});

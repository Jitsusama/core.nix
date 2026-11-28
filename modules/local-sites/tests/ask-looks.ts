// The looks ask-as-for-you.test.ts reads on both sides, the kit's ask row
// and For You, each rule's cases either side of what it decides, and when
// they came and expire, which the test's moments are taken around. They're
// kept apart from the test so neither passes the 500 lines a file may hold.

/** A day, in milliseconds. */
export const DAY = 86_400_000;

/** When the looks below came, and when those of them that expire do, a day on. */
export const CAME = Date.parse("2026-11-23T14:00:00Z");
export const EXPIRES = CAME + DAY;

/** A look of the second form, as the harness writes version 1. */
const LOOK = {
  v: 1,
  id: "look-1",
  at: "2026-11-23T14:00:00Z",
  kind: "pr",
  target: "https://github.com/Jitsusama/core.nix/pull/6146",
  title: "Answer a Look Where It's Asked",
  why: "The ask row is ready.",
  ask: "Say whether a page may ask.",
  ifNothing: "No page asks.",
  // What follows each: the wanted words, its own, its own over the wanted,
  // and its own though only spaces, which For You keeps over the wanted.
  choices: [
    { value: "keep", label: "Keep It" },
    { value: "show", label: "Always Show It", follows: " Its own words. " },
    { value: "both", label: "Both", follows: "Its own." },
    { value: "blank", label: "Blank", follows: "  " },
  ],
  state: "waiting",
  happened: "The ask row is ready",
  matters: "Pages ask as For You does",
  act: "Read the Diff",
  wants: {
    choices: [
      { label: "Keep It", follows: "A page asks." },
      { label: "Both", follows: "Wanted." },
      { label: "Blank", follows: "Wanted too." },
    ],
    words: true,
  },
};

/**
 * The stops and closing marks a part's end is tried with, none of either,
 * and marks ENDED takes for neither, so a set widened shows as one narrowed.
 */
const TRIED_STOPS = ["", ".", "!", "?", "\u2026", ",", ":", ";"];
const TRIED_CLOSERS = ["", '"', "'", "\u2019", "\u201d", ")", "]", '")', "\u00bb", "*"];

/** Each kind and state a look may say, and one of each For You doesn't know. */
const KINDS = ["pr", "page", "file", "diff", "terminal", "folder"];
const STATES = ["waiting", "opened", "cleared", "resolved", "expired", "quiet"];

/** A target fit for each kind, so each is read as a look of it would be. */
const TARGETS: Record<string, string> = {
  file: "/home/joel/notes.md",
  terminal: "QGB4ON",
};

/** Choices offering each label as its own value. */
function labelled(labels: readonly string[]): readonly object[] {
  return labels.map((label) => ({ value: label, label }));
}

/** Looks to read alike: each rule's cases, either side of what it decides. */
export const LOOKS: Record<string, object> = {
  "a look of the second form": LOOK,
  "one of the first form": { ...LOOK, happened: "", act: "" },
  "one saying only spaces happened": { ...LOOK, happened: "  " },
  // Each way a part ends, or says nothing, that LookForm's ENDED, its trim
  // and its filter of empty parts decide.
  "one ending a quoted question, mattering nothing": {
    ...LOOK,
    happened: 'It asked "why?"',
    matters: "",
  },
  "one ending in a bang and an ellipsis": { ...LOOK, happened: "Done!", matters: "Now\u2026" },
  "one whose stop sits in its brackets": { ...LOOK, happened: "It ended (as planned.)" },
  "one mattering only spaces": { ...LOOK, matters: "  " },
  "one with no word of why it matters": { ...LOOK, matters: undefined },
  "one padded with spaces": { ...LOOK, happened: " Ready! ", matters: " It's said. " },
  // What it asks, which only the first form shows, and what follows doing
  // nothing, which both do: padded, only spaces, or not said.
  "one of the first form asking padded": { ...LOOK, happened: "", act: "", ask: " Merge it? " },
  "one of the first form asking only spaces": { ...LOOK, happened: "", act: "", ask: "  " },
  "one of the first form asking nothing": { ...LOOK, happened: "", act: "", ask: undefined },
  "one saying padded what follows nothing": { ...LOOK, ifNothing: " It waits. " },
  "one saying only spaces follow nothing": { ...LOOK, ifNothing: "  " },
  "one saying nothing of what follows nothing": { ...LOOK, ifNothing: undefined },
  "one with a stop inside each part, none ending it": {
    ...LOOK,
    happened: "Merged v1.2 of the kit",
    matters: "Done. Then more",
  },
  // Each stop ENDED takes or doesn't, then each mark it lets close one or
  // doesn't, none or two, so its anchor, its set of stops and of closers
  // and how many it takes all show.
  ...Object.fromEntries(
    TRIED_STOPS.flatMap((stop) =>
      TRIED_CLOSERS.map((closer) => [
        `one ending ${JSON.stringify(`${stop}${closer}`)}`,
        { ...LOOK, happened: `Done${stop}${closer}`, matters: `It said${stop}${closer}` },
      ]),
    ),
  ),
  ...Object.fromEntries(
    KINDS.map((kind) => [
      `one of kind ${kind}`,
      { ...LOOK, kind, target: TARGETS[kind] ?? LOOK.target },
    ]),
  ),
  // A kind the drawer names no target for links nowhere, even at a web
  // address, so either side's linked kinds widening shows.
  ...Object.fromEntries(
    Object.keys(TARGETS).map((kind) => [`one of kind ${kind} at a web address`, { ...LOOK, kind }]),
  ),
  "one at a plain http address": { ...LOOK, target: "http://example.com/review" },
  ...Object.fromEntries(STATES.map((state) => [`one in state ${state}`, { ...LOOK, state }])),
  "five plain choices": {
    ...LOOK,
    choices: ["one", "two", "three", "four", "five"].map((value) => ({ value, label: value })),
  },
  "one with no time it came": { ...LOOK, at: undefined },
  "one with an empty time it came": { ...LOOK, at: "" },
  "one with an expiry no clock reads": { ...LOOK, expiresAt: "soon" },
  "five choices, posing ones among the first four": {
    ...LOOK,
    choices: [
      { value: "close", label: "Clear It" },
      { value: "keep", label: "Keep It" },
      { value: "merge", label: "clear  it." },
      { value: "ship", label: "Ship\u202e" },
      { value: "fifth", label: "A Fifth" },
    ],
    act: "Ship",
  },
  "choices with no value, no label or neither": {
    ...LOOK,
    choices: [
      { label: "No Value" },
      { value: "bare" },
      null,
      "said",
      { value: "", label: "Empty" },
    ],
  },
  // Each posing rule either side of what it decides, so either copy's
  // WHOLE_CHOICE, LONGEST, word count, CUTS, WORDED, WIDTHS or VARIANTS drifting shows.
  "choices opening with a one-word act, either side of its cut": {
    ...LOOK,
    choices: labelled([
      "Open a PR",
      "Talk to them about it, now",
      `Talk ${"x".repeat(19)}`,
      `Talk ${"x".repeat(20)}`,
    ]),
  },
  "choices an act's words lead before a line break or spaces": {
    ...LOOK,
    choices: labelled([
      "Clear It\nfor good",
      `Clear It${" ".repeat(40)}Away`,
      "Talk\nto them",
      "Open\r\nit",
    ]),
  },
  "choices of five words, a count can't measure, or Qt splits": {
    ...LOOK,
    choices: labelled([
      "Talk to me a bit",
      "Talk\u2024\u2024\u2024",
      "Open \u65e5\u65e5",
      "Talk\u009cX",
    ]),
  },
  // Four to a look, since each side offers only a look's first four, so
  // every label here reaches both copies' poses.
  "choices measured as drawn, wide, stopped or accented": {
    ...LOOK,
    choices: labelled([
      `Talk ${"W".repeat(19)}`,
      `Talk x${"!".repeat(200)}`,
      "Open The Pull Request",
      "Open Jos\u00e9\u2019s PR",
    ]),
  },
  "choices weighed by their capitals": {
    ...LOOK,
    choices: labelled([
      "Answer \u201cyes\u201d",
      "Talk With Mom Tomorrow",
      `Talk ${"X".repeat(13)}`,
      `Talk ${"X".repeat(12)}`,
    ]),
  },
  "choices weighed by their widest letters": {
    ...LOOK,
    choices: labelled([
      `Talk ${"\u0141".repeat(13)}`,
      `Talk ${"m".repeat(10)}`,
      `Talk ${"\u0174".repeat(19)}`,
      `Talk ${"x".repeat(18)}\u{e0001}`,
    ]),
  },
  "choices weighed by ASCII's wide marks and digits": {
    ...LOOK,
    choices: labelled(["#", "&", "=", "0"].map((one) => `Talk ${one.repeat(19)}`)),
  },
  "choices of digits and wide marks short enough to draw whole": {
    ...LOOK,
    choices: labelled(["Open PR 7282", "Talk Sam & Kim", `Talk ${"#".repeat(9)}`]),
  },
  "choices spaced wider than one": {
    ...LOOK,
    choices: labelled([`Talk x\t${"y".repeat(15)}`, `Talk\u2003${"x".repeat(19)}`]),
  },
  "choices hiding nothing before or inside their act": {
    ...LOOK,
    choices: labelled([
      `\u200bTalk ${"W".repeat(19)}`,
      `Ta\u00adlk ${"W".repeat(19)}`,
      `Talk\u200b ${"W".repeat(19)}`,
      `ANSWER ${"X".repeat(11)}`,
    ]),
  },
  "choices opening with a space before their act": {
    ...LOOK,
    choices: labelled([` Talk ${"x".repeat(19)}`, " Talk", "Talk now"]),
  },
  "choices borrowing another script's letters that draw as an act's": {
    ...LOOK,
    choices: labelled(["\u0421lear It", "\u03a4alk", "\uff34alk", "\u0414\u0430"]),
  },
  "choices holding a mark no list names inside or after their act": {
    ...LOOK,
    choices: labelled(["Ta\u0485lk", "Tal\u064bk", "T\u3099alk", "Ta\u0485lk now"]),
  },
  "choices holding letters beyond Latin no list names": {
    ...LOOK,
    choices: labelled([
      "\u03f9lear It",
      "\ua4da\ua4e1\ua4f0\ua4ee\ua4e3 \ua4f2\ua4d4",
      "\u{1d5b3}alk",
      `Talk ${"\uff58".repeat(12)}`,
    ]),
  },
  "choices opening with an act, then Greek or Cyrillic": {
    ...LOOK,
    choices: labelled([
      "Answer \u0414\u0430",
      "Talk to \u041c\u0430\u0440\u0438\u044f",
      "Open \u039d\u03b1\u03b9",
    ]),
  },
  "choices holding a character outside the closed set, or accented": {
    ...LOOK,
    choices: labelled(["Talk ^", "Keep\tIt", "Ta\u013ck", "Open Jos\u00e9\u2019s PR"]),
  },
  "choices holding the marks sessions write": {
    ...LOOK,
    choices: labelled(["Merge into `main`", "Ship at 50%", "Ping @sam", "Not yet\u2026"]),
  },
  "choices whose marks stand for an act's letters": {
    ...LOOK,
    choices: labelled(["Cle@r It", "T@lk", "Ta|k", "Keep feature_flag"]),
  },
  "an act beside a choice held back for its character": {
    ...LOOK,
    act: "Review It",
    choices: labelled(["\u0414\u0430", "Review It\nfor good", "Keep It"]),
  },
  "an act a choice's $ stands for a letter of": {
    ...LOOK,
    act: "Save",
    choices: labelled(["$ave"]),
  },
  "choices whose letters draw as an act's within the closed set": {
    ...LOOK,
    choices: labelled(["Tal\u0138", "E\u00d7plain It", "Ansvver Later", "Dear It"]),
  },
  "choices either side of LONGEST": {
    ...LOOK,
    choices: labelled([`Keep ${"x".repeat(92)}`, `Keep ${"x".repeat(91)}`]),
  },
  "choices whose stop after the act the cut strips": {
    ...LOOK,
    choices: labelled([
      `Talk, ${"W".repeat(17)}`,
      `Talk - ${"W".repeat(16)}`,
      `Talk \u2013 ${"W".repeat(17)}`,
      "Talk, now",
    ]),
  },
  "an act whose stops draw past its room": { ...LOOK, act: `Talk x${"!".repeat(200)}` },
  "choices a digit joins, of four words, or an act's own": {
    ...LOOK,
    choices: labelled(["Clear It2", `Talk9${"x".repeat(25)}`, "Talk to me now", "Open"]),
  },
  "an act a choice opens with": { ...LOOK, act: "Review It", choices: labelled(["Review It Now"]) },
  "an act a one-word choice begins": { ...LOOK, act: "Review It", choices: labelled(["Review"]) },
  "an act opening with a choice's words": { ...LOOK, act: "Keep It Now" },
  "an act Qt splits": { ...LOOK, act: "Review\u009cIt" },
  "an act read as a choice it offers": { ...LOOK, act: "keep it." },
  "an act posing as the launcher's": { ...LOOK, act: "Clear\u200b It" },
  "an act holding a turn": { ...LOOK, act: "Read\u202e" },
  "an act of four words": { ...LOOK, act: "Read the Whole Diff" },
  "an act of five words": { ...LOOK, act: "Read the Whole Diff Now" },
  "an act reading as nothing": { ...LOOK, act: "\u200b." },
  "a numeric came and expiry": { ...LOOK, at: CAME, expiresAt: EXPIRES },
  "an expiry written as a time": { ...LOOK, expiresAt: new Date(EXPIRES).toISOString() },
  "one asking nothing": { ...LOOK, asks: false },
  "one about a page by a script": { ...LOOK, kind: "page", target: "javascript:alert(1)" },
  "one of another version": { ...LOOK, v: 2 },
  "one with no title": { ...LOOK, title: "" },
  "one with no id": { ...LOOK, id: undefined },
  "one with no target": { ...LOOK, target: undefined },
  "one with an empty target": { ...LOOK, target: "" },
};

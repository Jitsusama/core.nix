// The kit's ask row reads a look as the launcher's For You does, held
// against the launcher's own code rather than against what ask.ts says it
// copied, so a rule changed on either side fails here. Looks.js's `look`,
// `quieted` and `asksNothing` and Now.qml's own lookOffers and followsOf
// decide, over looks at each boundary and of every kind and state, which a
// page reads, which wait, what each offers and what follows each, and when
// it stops; LookForm.js's wordsOf, what the second form leads with; Now.qml's
// lookDrawer, which kinds it links, at a web address only, the page's own
// rule; LookForm.js's actOf decides its act, beside the launcher's act
// names as Now.qml's lookActNames and the joined words Words.qml reads name
// them, the last of an id's winning, and the ids local-sites' home module
// guards as Now.qml lists them. A side saying nothing (no lapse, no link)
// agrees only where the other has nothing to say. The launcher's scripts
// run in Node, not in Qt's engine, which tests/for-you.qml holds them in;
// ask-reads-every-character.test.ts sweeps each character on its own. The
// check the-ask-row-reads-as-for-you hands it the launcher's files by the
// names `launcher` reads.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { runInNewContext } from "node:vm";
import { type Ask, asked, LAUNCHER_ACTS, QUIET_AFTER } from "../kit/ask.ts";
import { launcher, loaded, type Script } from "./launcher.ts";

/** A look as Looks.js reads one, as far as this compares it. */
interface Read {
  readonly state: string;
  readonly words: string;
  readonly offers: readonly { value: string; label: string; follows: string }[];
}

/** For You as far as a page's row goes: Looks.js, LookForm.js, lookOffers and the act names. */
interface ForYou {
  readonly looks: Script;
  readonly form: Script;
  readonly offers: (one: Read) => Read["offers"];
  /** Whether a choice's drawer says what follows it, as Now.qml's followsOf decides. */
  readonly shows: (one: Read, choice: Read["offers"][number]) => boolean;
  /** The kinds Now.qml's drawer names a target for, which a page may link. */
  readonly linked: readonly string[];
  readonly acts: readonly string[];
  readonly open: string;
}

/** Now.qml's list of the launcher's acts on a look, by their words' ids. */
const ACT_IDS = /lookActNames:\s*\[([^\]]*)\]/u;
/** Now.qml's lookOffers, its body as written. */
const LOOK_OFFERS = /function lookOffers\(one\) \{\n([\s\S]*?)\n {4}\}/u;
/** What keeps Now.qml's followsOf from saying what follows a choice. */
const FOLLOWS_UNSAID = /const followsOf = choice => (.+?) \? \{\} : \{/u;
/** The kinds Now.qml's lookDrawer names a target for. */
const DRAWN_KINDS = /const target = (\[[^\]]*\])\.includes\(one\.kind\)/u;
const QUOTED = /"([^"]+)"/gu;
/** What For You calls opening a look, which its act is where it names none. */
const OPEN_ID = "verb:on:look:open";
/** A day, in milliseconds. */
const DAY = 86_400_000;

/**
 * The launcher's words, each name by its id, as Words.qml reads the joined
 * words.json: its things in order, the last of an id's the one it names.
 */
function launcherWords(): Map<string, unknown> {
  const { things } = JSON.parse(launcher("WORDS")) as { things?: unknown };
  const names = new Map<string, unknown>();
  for (const thing of Array.isArray(things) ? things : []) {
    const { id, name } = (thing ?? {}) as { id?: unknown; name?: unknown };
    if (typeof id === "string") {
      names.set(id, name);
    }
  }
  return names;
}

/** A word's name, or what says it has none. */
function nameOf(names: Map<string, unknown>, id: string): string {
  const name = names.get(id);
  return typeof name === "string" ? name : `no ${id}`;
}

/** The launcher's acts on a look, by their words' ids, as Now.qml's lookActNames lists them. */
function actIds(): string[] {
  const ids = ACT_IDS.exec(launcher("NOW_QML"))?.[1];
  assert.ok(ids !== undefined, "Now.qml holds no lookActNames list to read");
  return Array.from(ids.matchAll(QUOTED), ([, id]) => id ?? "");
}

/** The launcher's act names on a look, as Now.qml's lookActNames names them. */
function actNames(): string[] {
  const names = launcherWords();
  return actIds().map((id) => nameOf(names, id));
}

/** A piece of Now.qml, as `pattern` finds it. */
function nowSays(pattern: RegExp, what: string): string {
  const found = pattern.exec(launcher("NOW_QML"))?.[1];
  assert.ok(found !== undefined, `Now.qml holds no ${what} to read`);
  return found;
}

/** The launcher's own scripts and words, loaded as For You loads them. */
function forYou(): ForYou {
  const order = loaded("ORDER");
  const form = loaded("LOOK_FORM");
  const looks = loaded("LOOKS", {
    ForYou: loaded("FOR_YOU", { Order: order }),
    LookForm: form,
    Order: order,
  });
  const acts = actNames();
  const offers = runInNewContext(
    `(function lookOffers(one) {${nowSays(LOOK_OFFERS, "lookOffers")}})`,
    { LookForm: form, lookActNames: acts },
  ) as ForYou["offers"];
  const shows = runInNewContext(
    `(function shows(one, choice) { return !(${nowSays(FOLLOWS_UNSAID, "followsOf")}); })`,
    { LookForm: form },
  ) as ForYou["shows"];
  const linked = JSON.parse(nowSays(DRAWN_KINDS, "lookDrawer's kinds")) as string[];
  return { looks, form, offers, shows, linked, acts, open: nameOf(launcherWords(), OPEN_ID) };
}

/** When the looks below came, and when those of them that expire do, a day on. */
const CAME = Date.parse("2026-11-23T14:00:00Z");
const EXPIRES = CAME + DAY;

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

/** Looks to read alike: each rule's cases, either side of what it decides. */
const LOOKS: Record<string, object> = {
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

/** Moments either side of each that ends a wait: its expiry, and its quiet. */
const MOMENTS = [CAME, EXPIRES - 1, EXPIRES, CAME + QUIET_AFTER, CAME + QUIET_AFTER + 1];

/** A choice as both sides offer it: what it sends, what it's named and what follows it. */
const sent = ({ value, label, follows }: Read["offers"][number]): string =>
  `${value}: ${label}: ${follows.trim()}`;

/** A moment past every quiet and expiry the looks above say. */
const LONG_AFTER = CAME + QUIET_AFTER + DAY;

/** The schemes a page links, its own rule, since For You hands any target to xdg-open. */
const LINKS = ["http:", "https:"];

/** Where a page's act should take him: a kind For You's drawer names, at a web address. */
function linkFor(
  { kind, target }: { kind?: unknown; target?: unknown },
  linked: readonly string[],
): string | null {
  if (typeof kind !== "string" || typeof target !== "string" || !linked.includes(kind)) {
    return null;
  }
  const address = URL.canParse(target) ? new URL(target) : null;
  return address !== null && LINKS.includes(address.protocol) ? address.href : null;
}

/** Where the row and For You part, for one look at one moment: none where they agree. */
function parted(ask: Ask | null, read: Read | null, now: number, shell: ForYou): string[] {
  if (ask === null || read === null) {
    return ask === null && read === null ? [] : ["read whole otherwise"];
  }
  const { looks, form, offers, shows, linked, acts, open } = shell;
  const held = looks.quieted(read as never, now as never) as Read;
  const waits = held.state === "waiting" && looks.asksNothing(held as never) === false;
  // For You offers a waiting look's choices under To Read too, where the
  // row, stricter, offers none on a look not asking (ask.ts). It says what
  // follows each where Now.qml's own followsOf does.
  const offered = waits
    ? offers(held).map((one) => sent({ ...one, follows: shows(read, one) ? one.follows : "" }))
    : [];
  const opens = linkFor(read as never, linked);
  const act = (form.actOf(read as never, acts as never) as string) || open;
  const lapsed = (at: number): boolean =>
    (looks.quieted(read as never, at as never) as Read).state !== "waiting";
  // The first form is laid out apart on purpose, its question on its own.
  const second = form.secondForm(read as never) === true;
  const leads = !second || ask.said.join(" ") === read.words;
  // What For You's row asks and its drawer says follows doing nothing,
  // while the row, stricter, asks, each as For You reads it.
  const question = second ? "" : String(looks.askOf(read as never));
  const ifNothing = String(looks.ifNothingOf(held as never));
  return [
    leads ? "" : `leads with ${ask.said.join(" ")}, not ${read.words}`,
    !ask.asks || ask.question === question ? "" : `asks ${ask.question}, not ${question}`,
    !ask.asks || ask.ifNothing === ifNothing
      ? ""
      : `says ${ask.ifNothing} if nothing, not ${ifNothing}`,
    ask.asks === waits ? "" : "waits otherwise",
    ask.choices.map(sent).join(" | ") === offered.join(" | ") ? "" : "offers otherwise",
    ask.opens === opens ? "" : `opens ${ask.opens}, not ${opens}`,
    (ask.act === null) === (opens === null) ? "" : "links otherwise than it acts",
    ask.act === null || ask.act === act ? "" : `acts as ${ask.act}, not ${act}`,
    ask.lapses === null || (!lapsed(ask.lapses - 1) && lapsed(ask.lapses))
      ? ""
      : "lapses otherwise",
    !waits || ask.lapses !== null || !lapsed(LONG_AFTER) ? "" : "never lapses",
  ].filter((one) => one !== "");
}

it("reads, holds waiting, offers, acts and lapses as Looks.js, LookForm.js and lookOffers do", () => {
  const shell = forYou();
  for (const [name, look] of Object.entries(LOOKS)) {
    const text = JSON.stringify(look);
    for (const now of MOMENTS) {
      const read = shell.looks.look(text as never) as Read | null;
      const ask = asked(JSON.parse(text), now);
      assert.deepEqual(parted(ask, read, now, shell), [], `${name}, at ${now}`);
    }
  }
});

it("names the launcher's acts as Now.qml's lookActNames and its words do", () => {
  assert.deepEqual([...LAUNCHER_ACTS], actNames(), "the kit names the launcher's acts otherwise");
  assert.deepEqual(
    JSON.parse(launcher("LOOK_ACTS")),
    actIds(),
    "local-sites' home module guards other acts than Now.qml lists",
  );
});

// The kit's ask row reads a look as the launcher's For You does, held
// against the launcher's own code rather than against what ask.ts says it
// copied, so a rule changed on either side fails here. Looks.js's `look`,
// `quieted` and `asksNothing` and Now.qml's own lookOffers decide, over
// looks at each boundary and of every kind and state, which a page reads,
// which wait, what each offers and what follows each, and when it stops;
// LookForm.js's actOf decides its act, beside the launcher's act names as
// Now.qml's lookActNames and its words name them; and every character,
// within, leading and ending a name, is read and judged as LookForm.js
// reads and judges it. A side saying nothing (no lapse, no act) agrees only
// where the other has nothing to say. The launcher's scripts run in Node,
// not in Qt's engine, which tests/for-you.qml holds them in. The check
// the-ask-row-reads-as-for-you hands it the launcher's files by the names
// `launcher` reads.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { runInNewContext } from "node:vm";
import { type Ask, asked, LAUNCHER_ACTS, poses, QUIET_AFTER, readAs } from "../kit/ask.ts";

/** A launcher script, its functions by their names. */
type Script = Record<string, (...args: never[]) => unknown>;

/** A look as Looks.js reads one, as far as this compares it. */
interface Read {
  readonly state: string;
  readonly offers: readonly { value: string; label: string; follows: string }[];
}

/** For You as far as a page's row goes: Looks.js, LookForm.js, lookOffers and the act names. */
interface ForYou {
  readonly looks: Script;
  readonly form: Script;
  readonly offers: (one: Read) => Read["offers"];
  readonly acts: readonly string[];
  readonly open: string | undefined;
}

/** A line of Qt's own, which no other engine reads, as `.pragma library`. */
const QT_LINE = /^\.(?:pragma|import)\b.*$/gmu;
/** Now.qml's list of the launcher's acts on a look, by their words' ids. */
const ACT_IDS = /lookActNames:\s*\[([^\]]*)\]/u;
/** Now.qml's lookOffers, its body as written. */
const LOOK_OFFERS = /function lookOffers\(one\) \{\n([\s\S]*?)\n {4}\}/u;
const QUOTED = /"([^"]+)"/gu;
/** What For You calls opening a look, which its act is where it names none. */
const OPEN_ID = "verb:on:look:open";
/** The last code point, and those standing for half a pair, which are no character. */
const LAST_POINT = 0x10_ff_ff;
const HALVES = [0xd8_00, 0xdf_ff] as const;
/** How many characters read otherwise are named before the sweep stops. */
const NAMED = 10;
/** A code point's digits as it's written, U+ and hexadecimal. */
const HEX = 16;
/** A day, in milliseconds. */
const DAY = 86_400_000;

/** A file the check hands this, by the name it hands it under. */
function launcher(name: string): string {
  const path = process.env[name];
  assert.ok(path, `the check handed no ${name}`);
  return readFileSync(path, "utf8");
}

/**
 * A launcher script as Qt loads it: run on its own, its functions what it
 * gives, with `imports`, the scripts it imports, by the names it imports
 * them as.
 */
function loaded(name: string, imports: Record<string, Script> = {}): Script {
  const script: Script = { ...imports };
  runInNewContext(launcher(name).replace(QT_LINE, ""), script);
  return script;
}

/** Each word's name by its id, wherever the words file holds it. */
function namesIn(words: unknown, names: Map<string, string>): Map<string, string> {
  if (Array.isArray(words)) {
    for (const one of words) {
      namesIn(one, names);
    }
  } else if (words !== null && typeof words === "object") {
    const { id, name } = words as { id?: unknown; name?: unknown };
    if (typeof id === "string" && typeof name === "string") {
      names.set(id, name);
    }
    for (const one of Object.values(words)) {
      namesIn(one, names);
    }
  }
  return names;
}

/** The launcher's words, each name by its id. */
function launcherWords(): Map<string, string> {
  return namesIn(
    JSON.parse(launcher("ON_WORDS")),
    namesIn(JSON.parse(launcher("AGENT_WORDS")), new Map()),
  );
}

/** The launcher's act names on a look, as Now.qml's lookActNames names them. */
function actNames(): string[] {
  const ids = ACT_IDS.exec(launcher("NOW_QML"))?.[1];
  assert.ok(ids !== undefined, "Now.qml holds no lookActNames list to read");
  const names = launcherWords();
  return Array.from(ids.matchAll(QUOTED), ([, id]) => names.get(id ?? "") ?? `no ${id}`);
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
  const body = LOOK_OFFERS.exec(launcher("NOW_QML"))?.[1];
  assert.ok(body !== undefined, "Now.qml holds no lookOffers to run");
  const acts = actNames();
  const offers = runInNewContext(`(function lookOffers(one) {${body}})`, {
    LookForm: form,
    lookActNames: acts,
  }) as ForYou["offers"];
  return { looks, form, offers, acts, open: launcherWords().get(OPEN_ID) };
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
  ...Object.fromEntries(
    KINDS.map((kind) => [
      `one of kind ${kind}`,
      { ...LOOK, kind, target: TARGETS[kind] ?? LOOK.target },
    ]),
  ),
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
  "an act of five words": { ...LOOK, act: "Read the Whole Diff Now" },
  "a numeric came and expiry": { ...LOOK, at: CAME, expiresAt: EXPIRES },
  "an expiry written as a time": { ...LOOK, expiresAt: new Date(EXPIRES).toISOString() },
  "one asking nothing": { ...LOOK, asks: false },
  "one about a page by a script": { ...LOOK, kind: "page", target: "javascript:alert(1)" },
  "one of another version": { ...LOOK, v: 2 },
  "one with no title": { ...LOOK, title: "" },
  "one with no id": { ...LOOK, id: undefined },
};

/** Moments either side of each that ends a wait: its expiry, and its quiet. */
const MOMENTS = [CAME, EXPIRES - 1, EXPIRES, CAME + QUIET_AFTER, CAME + QUIET_AFTER + 1];

/** A choice as both sides offer it: what it sends, what it's named and what follows it. */
const sent = ({ value, label, follows }: Read["offers"][number]): string =>
  `${value}: ${label}: ${follows.trim()}`;

/** A moment past every quiet and expiry the looks above say. */
const LONG_AFTER = CAME + QUIET_AFTER + DAY;

/** Where the row and For You part, for one look at one moment: none where they agree. */
function parted(ask: Ask | null, read: Read | null, now: number, shell: ForYou): string[] {
  if (ask === null || read === null) {
    return ask === null && read === null ? [] : ["read whole otherwise"];
  }
  const { looks, form, offers, acts, open } = shell;
  const held = looks.quieted(read as never, now as never) as Read;
  const waits = held.state === "waiting" && looks.asksNothing(held as never) === false;
  // For You offers a waiting look's choices under To Read too, where the
  // row, stricter, offers none on a look not asking (ask.ts). It says what
  // follows each only in the second form, as Now.qml's followsOf does.
  const said = form.secondForm(read as never) === true;
  const offered = waits
    ? offers(held).map((one) => sent({ ...one, follows: said ? one.follows : "" }))
    : [];
  const act = (form.actOf(read as never, acts as never) as string) || open;
  const lapsed = (at: number): boolean =>
    (looks.quieted(read as never, at as never) as Read).state !== "waiting";
  return [
    ask.asks === waits ? "" : "waits otherwise",
    ask.choices.map(sent).join(" | ") === offered.join(" | ") ? "" : "offers otherwise",
    (ask.act === null) === (ask.opens === null) ? "" : "links otherwise than it acts",
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

/**
 * Whether `character` reads and is judged alike by both, within a name,
 * leading one and ending one, and against a name it could pass for or by a
 * turn pose as.
 */
function alike(form: Script, character: string): boolean {
  return [`Keep${character} It`, `${character}tI peeK`, `Keep It${character}`].every(
    (name) =>
      readAs(name) === form.readAs(name as never) &&
      poses(name, ["Keep It"]) === form.poses(name as never, ["Keep It"] as never),
  );
}

it("reads and judges every character as LookForm.js does, wherever it stands in a name", () => {
  const form = loaded("LOOK_FORM");
  const otherwise: string[] = [];
  for (const [from, to] of [
    [0, HALVES[0] - 1],
    [HALVES[1] + 1, LAST_POINT],
  ] as const) {
    for (let point = from; point <= to && otherwise.length < NAMED; point += 1) {
      if (!alike(form, String.fromCodePoint(point))) {
        otherwise.push(`U+${point.toString(HEX)}`);
      }
    }
  }
  assert.deepEqual(otherwise, [], "a character read or judged otherwise");
});

it("names the launcher's acts as Now.qml's lookActNames and its words do", () => {
  assert.deepEqual([...LAUNCHER_ACTS], actNames(), "the kit names the launcher's acts otherwise");
});

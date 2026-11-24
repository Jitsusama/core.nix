// An ask answered where it's asked: a look, as the harness writes it in
// looks/<id>.json, read into the parts every page's ask row lays out the
// same way, as the launcher's For You does (the kit inventory's seventh
// rule, and the For You layout of 22 November 2026). What happened and why
// it matters first, in the serif; the question, and what happens if he does
// nothing; up to four choices, each with what follows from it; and the act,
// opening what the look is about. The page draws the row; this says what's
// in it.
//
// It reads a look as the launcher's Looks.js, LookForm.js and Now.qml do,
// so a page shows only the looks For You shows and asks only on those For
// You holds as waiting. A look is read only whole, as Looks.js `whole`
// reads one: version 1, with its id, title, target, kind and state at the
// top. It waits only while its state is "waiting", its expiresAt hasn't
// passed and it came less than three days ago, and one saying `asks: false`
// is only to be read. Only while it waits does it show its question, what
// happens if he does nothing and its choices, as For You's drawer does.
// Its choices are the first form's, each sending the value the session
// offered exactly as written, but for one whose label poses as one of the
// launcher's own acts, which For You hides (Now.qml's lookOffers). Its act
// is For You's "Open" unless the second form names one LookForm.js takes.
// The check the-ask-row-reads-as-for-you runs the launcher's own Looks.js,
// LookForm.js and lookOffers against these over looks at each boundary,
// and every character through how a name is read, in Node rather than in
// Qt's engine, which tests/for-you.qml holds LookForm.js in.
//
// It's stricter than For You in two places, on the side of not asking.
// For You offers a look's choices under To Read too, though it says it
// asks nothing, where a page offers none on a look not asking. And For You
// reads its looks at the hour it last opened or was told them, so it may
// keep one waiting past its expiresAt or its quiet until it next does,
// where a page lapses at the moment. It knows the launcher's six act names
// only as core's own words say them, so on a machine renaming one
// (jitsusama.quickshell.launcher.words) a page would offer a choice, or
// link an act, named as the renamed one, which For You refuses, and would
// fall back to "Open" where For You says the renamed word: local-sites'
// home module refuses to build such a machine.
//
// A page holds a look as its feed last gave it, so while the feed is stale
// or broken the look may have been answered or cleared since: a page offers
// a row's choices only while its feed is live, and asks `answering` again at
// the press rather than trusting a timer set for `lapses` alone, which a
// suspend makes late.
//
// The harness writes version 1 with the second form's fields beside the
// first's (85PKB2, 22 November 2026), so what happened, why it matters,
// the act and what follows each choice are read from the second form
// wherever it says what happened, as LookForm.js reads them, and from the
// first form otherwise, which says nothing of what follows. What a look is
// about is read from the first form's fields alone, which are what For You
// opens.
//
// Two gaps stay. An answer the launcher has sent moves a look to For You's
// History at once, but its file says "waiting" until its session reads the
// answer and writes it resolved, and only the shell knows it was answered.
// In that window a page asks again, so a page drops its row's choices once
// one is pressed, as For You does, and whatever carries a page's answer must
// refuse a second answer to the same look. And the sites share their origin
// with documents sessions write, so an answer can't say it's from Joel:
// whatever carries it must know he pressed it, and only then says so.
//
// Every word here is free text a session wrote, so a page draws each as
// text, never as markup.

import { type Fields, isObject } from "./records.ts";

/** How many of a look's choices are offered, as Looks.js's OFFERED. */
const OFFERED = 4;

/** How long after it came a waiting look goes quiet, three days, as Looks.js's QUIET_AFTER. */
const QUIET_AFTER = 259_200_000;

/** The record's version, as Looks.js's VERSION. */
const VERSION = 1;

/** What a look can be about, as the harness writes them. */
const KINDS = ["pr", "page", "file", "diff", "terminal"] as const;

/** Where a look can stand, as Looks.js's STATES. */
const STATES = ["waiting", "opened", "cleared", "resolved", "expired"];

/** What a look is about. */
type Kind = (typeof KINDS)[number];

/** How a look ends a sentence, as LookForm.js's ENDED reads it. */
const ENDED = /[.!?\u2026]["'\u2019\u201d)\]]*$/u;

/**
 * The act's words where the look names none For You takes: what For You
 * calls opening it (words/on.json's verb:on:look:open).
 */
const OPEN = "Open";

/**
 * The names of the launcher's own acts on a look, which neither its act
 * nor a choice may pass for, as Now.qml's lookActNames: Open, Answer,
 * Clear It, Go To, Explain It and Talk, each as the launcher's words name it.
 */
const LAUNCHER_ACTS: readonly string[] = [
  OPEN,
  "Answer",
  "Clear It",
  "Go To",
  "Explain It",
  "Talk",
];

/** One answer a look offers. */
interface Choice {
  /** What the answer sends, exactly as the session that asked offered it. */
  readonly value: string;
  /** Its name, on the button. */
  readonly label: string;
  /** What follows from it, in a sentence, or "" where the look doesn't say. */
  readonly follows: string;
}

/** What a page's ask row holds for one look. */
interface Ask {
  /** The look's id, which an answer names. */
  readonly id: string;
  /** What it's about, as its kind and its name. */
  readonly kind: Kind;
  readonly title: string;
  /**
   * Where the act takes him, or null where a browser can't go: a file or a
   * session, or anything but a web address, which is shown and never linked,
   * so a look can't hand the page a script to run.
   */
  readonly opens: string | null;
  /** What the look points at, whole, to show as text where `opens` is null. */
  readonly target: string;
  /** What happened and why it matters, the row's first words, as For You's drawer leads. */
  readonly said: readonly string[];
  /** What it asks of him, in a sentence, or "" where it doesn't say. */
  readonly question: string;
  /**
   * What happens if he does nothing, in a sentence, or "" where it doesn't
   * say, which a page fills from `lapses` as For You does.
   */
  readonly ifNothing: string;
  /**
   * When it stops waiting if he does nothing, in milliseconds since the
   * epoch: the first moment it's past its expiresAt or quiet three days
   * after it came, whichever is sooner; null while it doesn't wait or
   * neither is known. A page sets its row asking no more then, since
   * nothing else tells it, and checks again at the press. An expiry may
   * stand further off than a browser's timer waits (2^31 - 1 ms, about 25
   * days, past which it fires at once), so a page waits at most that long
   * and asks `asked` again when its timer fires.
   */
  readonly lapses: number | null;
  /**
   * The act's words, on the link that opens what it's about, or null where
   * `opens` is: the page can't do what the act would name, so it shows the
   * target as text instead.
   */
  readonly act: string | null;
  /** The answers it offers, at most four. */
  readonly choices: readonly Choice[];
  /**
   * Whether, while it waits, it takes an answer in his own words, which
   * only the second form asks for (LookForm.js's asksBack). No page sends
   * one yet: the launcher's shape for them comes with For You's second part.
   */
  readonly takesWords: boolean;
  /** Whether it waits on him, as For You's Waiting holds it, but for the gap above. */
  readonly asks: boolean;
}

/**
 * An answer for the session that asked, as the launcher's steer says one
 * (Looks.js `answer`): a choice's value, as it was offered. It names no
 * one it's from, since any document on the sites' origin could build the
 * same: whatever carries it checks it again against the look itself and
 * knows Joel pressed it, and only then adds who it's from, the session it
 * goes to and the time, as the shell adds them.
 */
interface Answer {
  readonly act: "answer";
  readonly look: string;
  readonly value: string;
}

/** Whether `value` is a string with anything in it, as Looks.js's `named`. */
function named(value: unknown): value is string {
  return typeof value === "string" && value !== "";
}

/** `value`'s words where it's a string, trimmed, else "". */
function written(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

/**
 * When `value` falls, from milliseconds or an ISO time, as ForYou.js's `when`;
 * or null. A time written otherwise is read as each engine reads it, so Qt's
 * may read one a browser reads otherwise; a session writes ISO times.
 */
function when(value: unknown): number | null {
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : null;
  }
  const at = named(value) ? Date.parse(value) : Number.NaN;
  return Number.isNaN(at) ? null : at;
}

function isKind(value: unknown): value is Kind {
  return (KINDS as readonly unknown[]).includes(value);
}

/** Whether For You reads `look` at all, as Looks.js `whole` does. */
function whole(look: Fields): boolean {
  return (
    look["v"] === VERSION &&
    named(look["id"]) &&
    named(look["title"]) &&
    named(look["target"]) &&
    isKind(look["kind"]) &&
    STATES.includes(look["state"] as string)
  );
}

/**
 * The kinds a page links, as Now.qml's drawer names a target for them. For
 * You hands any target but a session's to xdg-open, a file's too, which a
 * browser can't open from a page, and only a web address is ever linked.
 */
const LINKED: readonly Kind[] = ["pr", "page", "diff"];

/**
 * The address a link opens for `target`, as the browser parsed it, where it
 * is a web address; else null.
 */
function onTheWeb(target: string): string | null {
  try {
    const address = new URL(target);
    return address.protocol === "https:" || address.protocol === "http:" ? address.href : null;
  } catch {
    return null;
  }
}

/** The most words an act runs to, as LookForm.js's ACT_WORDS. */
const ACT_WORDS = 4;

/**
 * What a name may hide in to pass for another, as LookForm.js reads it:
 * spaces, and the stops it may end in; what draws as nothing (UNSEEN,
 * UNMARKED and UNDRAWN, the last by code point here, as Qt's engine can't);
 * what draws as a space (FILLERS); and what turns the letters after it
 * around (TURNS), so "\u202etI raelC" draws as Clear It.
 */
const SPACES = /\s+/gu;
const STOPS = /[\s.!?,;:\u2026]+$/u;
const UNSEEN = /[\u200b-\u200f\u202a-\u202e\u2060-\u206f\ufeff\ufff0-\ufff8]/gu;
const UNMARKED = /[\u00ad\u061c\u180e]|\u034f/gu;
const UNDRAWN =
  /\u17b4|\u17b5|[\u180b-\u180d]|\u180f|[\ufe00-\ufe0f]|[\u{1bca0}-\u{1bca3}]|[\u{1d173}-\u{1d17a}]|[\u{e0000}-\u{e0fff}]/gu;
const FILLERS = /[\u115f\u1160\u2800\u3164\uffa0]/gu;
const TURNS = /[\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/u;

/**
 * A name as it reads, to tell two apart, as LookForm.js's readAs: what
 * draws as nothing gone, its spaces one, its closing stops gone, small.
 */
function readAs(name: string): string {
  return name
    .replace(UNDRAWN, "")
    .replace(UNSEEN, "")
    .replace(UNMARKED, "")
    .replace(FILLERS, " ")
    .replace(SPACES, " ")
    .replace(STOPS, "")
    .trim()
    .toLowerCase();
}

/**
 * Whether `name` reads as one of `taken`, as LookForm.js's poses: one
 * holding a turn poses as any, since it can draw as either.
 */
function poses(name: string, taken: readonly string[]): boolean {
  const read = readAs(name);
  return taken.length > 0 && (TURNS.test(name) || taken.some((each) => readAs(each) === read));
}

/**
 * The second form's act in its own words, as LookForm.js's actOf takes it
 * with the launcher's act names: never past four words, holding a turn,
 * or read as one of the launcher's acts or of `choices`, the look's offers
 * before any posing is dropped, as LookForm.js reads them; else "".
 */
function actOf(act: string, choices: readonly Choice[]): string {
  const read = readAs(act);
  return read === "" ||
    read.split(" ").length > ACT_WORDS ||
    TURNS.test(act) ||
    poses(act, LAUNCHER_ACTS) ||
    choices.some((one) => readAs(one.label) === read)
    ? ""
    : act;
}

/** `text` ending as a sentence ends, as LookForm.js ends each part. */
function ended(text: string): string {
  return text === "" || ENDED.test(text) ? text : `${text}.`;
}

/**
 * The look's choices, as Looks.js offers them: the first form's first four
 * sending a value, each value exactly as written, labelled by its value
 * where it has no label, with what follows its own else the first of the
 * second form's choices of that label, two of one label sharing it, as
 * Looks.js finds it, a choice's own kept wherever it says one. What follows
 * is said only in the second form (`said`), as For You's drawer says it. A
 * page never sends a value the session didn't offer. These are what the
 * act is told apart from; `shown` drops those posing as the launcher's.
 */
function choicesOf(look: Fields, wants: Fields, said: boolean): readonly Choice[] {
  const wanted = (Array.isArray(wants["choices"]) ? wants["choices"] : []).filter(isObject);
  const follows = (one: Fields, label: string): string =>
    named(one["follows"])
      ? written(one["follows"])
      : written(wanted.find((each) => each["label"] === label)?.["follows"]);
  return (Array.isArray(look["choices"]) ? look["choices"] : [])
    .filter(isObject)
    .filter((one) => named(one["value"]))
    .slice(0, OFFERED)
    .map((one) => {
      const value = one["value"] as string;
      const label = named(one["label"]) ? one["label"] : value;
      return { value, label, follows: said ? follows(one, label) : "" };
    });
}

/**
 * The choices a page offers, as Now.qml's lookOffers: the first four,
 * less any whose label poses as the launcher's own acts, so none takes a
 * refused one's place.
 */
function shown(offered: readonly Choice[]): readonly Choice[] {
  return offered.filter((one) => !poses(one.label, LAUNCHER_ACTS));
}

/**
 * When `look` first stops waiting if he does nothing: at its expiresAt, or
 * the moment it's more than three days since it came, whichever is sooner.
 */
function lapsing(look: Fields): number | null {
  const came = when(look["at"]);
  const ends = [when(look["expiresAt"]), came === null ? null : came + QUIET_AFTER + 1];
  const known = ends.filter((one) => one !== null);
  return known.length === 0 ? null : Math.min(...known);
}

/**
 * Whether `look` waits on him at `now`, as For You's Waiting holds it: its
 * state "waiting", not past its expiresAt, not quiet three days after it
 * came, and asking something rather than only to be read.
 */
function waits(look: Fields, now: number): boolean {
  const expires = when(look["expiresAt"]);
  const came = when(look["at"]);
  return (
    look["state"] === "waiting" &&
    !(expires !== null && now >= expires) &&
    !(came !== null && now - came > QUIET_AFTER) &&
    look["asks"] !== false
  );
}

/** The act for a look `opens` links to: the second form's own words, else For You's. */
function actFor(look: Fields, choices: readonly Choice[], said: boolean): string {
  return (said ? actOf(written(look["act"]), choices) : "") || OPEN;
}

/**
 * What the ask row shows for `look`, a look's file as parsed, at `now`, in
 * milliseconds since the epoch; or null for one For You doesn't read.
 */
function asked(look: unknown, now: number): Ask | null {
  if (!(isObject(look) && whole(look))) {
    return null;
  }
  const wants = isObject(look["wants"]) ? look["wants"] : {};
  const kind = look["kind"] as Kind;
  const target = look["target"] as string;
  // The second form leads where it says what happened, as LookForm.js's secondForm.
  const happened = written(look["happened"]);
  const said =
    happened === ""
      ? [written(look["why"])].filter((one) => one !== "")
      : [
          [happened, written(look["matters"])]
            .map(ended)
            .filter((one) => one !== "")
            .join(" "),
        ];
  const opens = LINKED.includes(kind) ? onTheWeb(target) : null;
  const asks = waits(look, now);
  const offered = choicesOf(look, wants, happened !== "");
  return {
    id: look["id"] as string,
    kind,
    title: look["title"] as string,
    opens,
    target,
    said,
    question: asks ? written(look["ask"]) : "",
    ifNothing: asks ? written(look["ifNothing"]) : "",
    lapses: asks ? lapsing(look) : null,
    act: opens === null ? null : actFor(look, offered, happened !== ""),
    choices: asks ? shown(offered) : [],
    takesWords: asks && happened !== "" && wants["words"] === true,
    asks,
  };
}

/**
 * The answer that picking `choice`, one of the values `look` offers, sends
 * at `now`, the look read again at the press so a row left open doesn't
 * answer a look that has since expired or gone quiet. Null for a look that
 * no longer asks or a value it doesn't offer. It keeps an honest page from
 * sending what the look didn't ask; it guards nothing, since whatever
 * carries the answer must check it against the look again.
 */
function answering(look: unknown, choice: string, now: number): Answer | null {
  const ask = asked(look, now);
  if (ask === null || !ask.asks || !ask.choices.some((one) => one.value === choice)) {
    return null;
  }
  return { act: "answer", look: ask.id, value: choice };
}

export {
  actOf,
  LAUNCHER_ACTS,
  OPEN,
  poses,
  readAs,
  type Answer,
  type Ask,
  answering,
  asked,
  type Choice,
  type Kind,
  OFFERED,
  QUIET_AFTER,
};

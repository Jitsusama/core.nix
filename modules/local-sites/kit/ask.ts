// An ask answered where it's asked: a look, as the harness writes it in
// looks/<id>.json, read into the parts every page's ask row lays out the
// same way, as the launcher's For You does (the kit inventory's seventh
// rule, and the For You layout of 22 November 2026). What happened and why
// it matters first, in the serif; the question, and what happens if he does
// nothing; up to four choices, each with what follows from it; and the act,
// opening what the look is about. The page draws the row; this says what's
// in it.
//
// It reads a look as the launcher's Looks.js and LookForm.js do, so a page
// shows only the looks For You shows and asks only on those For You holds
// as waiting. A look is read only whole, as Looks.js `whole` reads one:
// version 1, with its id, title, target, kind and state at the top. It
// waits only while its state is "waiting", its expiresAt hasn't passed and
// it came less than three days ago, and one saying `asks: false` is only to
// be read. Its choices are the first form's, each sending the value the
// session offered exactly as written, with what follows from it taken from
// the second form's choice of the same label.
//
// The harness writes version 1 with the second form's fields beside the
// first's (85PKB2, 22 November 2026), so what happened, why it matters,
// the act and the choices' outcomes are read from the second form wherever
// it says what happened, as LookForm.js reads them, and from the first form
// otherwise. What a look is about is read from the first form's fields
// alone, which are what For You opens.
//
// One gap stays: an answer the launcher has sent moves a look to For You's
// History at once, but its file says "waiting" until its session reads the
// answer and writes it resolved, and only the shell knows it was answered.
// In that window a page asks again, so whatever carries a page's answer
// must refuse a second answer to the same look.
//
// Every word here is free text a session wrote, so a page draws each as
// text, never as markup.

/** How many of a look's choices are offered, as Looks.js's OFFERED. */
const OFFERED = 4;

/** How long after it came a waiting look goes quiet, as Looks.js's QUIET_AFTER. */
const QUIET_AFTER = 3 * 24 * 60 * 60 * 1000;

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
 * The act's words for a look of the first form, which names none: what
 * pressing it does to the thing it's about, short and in Title Case, as
 * the second form's act must be.
 */
const ACTS: Readonly<Record<Kind, string>> = {
  pr: "Review It",
  page: "Open the Page",
  file: "Read the File",
  diff: "Read the Diff",
  terminal: "Go to the Session",
};

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
  /** What happens if he does nothing, in a sentence, or "" where it doesn't say. */
  readonly ifNothing: string;
  /**
   * The act's words, on the link that opens what it's about, or null where
   * `opens` is: the page can't do what the act would name, so it shows the
   * target as text instead.
   */
  readonly act: string | null;
  /** The answers it offers, at most four. */
  readonly choices: readonly Choice[];
  /**
   * Whether it takes an answer in his own words. No page sends one yet:
   * the launcher's shape for them comes with For You's second part.
   */
  readonly words: boolean;
  /** Whether it waits on him, as For You's Waiting holds it, but for the gap above. */
  readonly asks: boolean;
}

/**
 * An answer for the session that asked, as the launcher's steer says one
 * (Looks.js `answer`): a choice's value, as it was offered. Whatever
 * carries it there names the session and the time, and checks it again
 * against the look itself, since a page is no guard: any page can say
 * anything.
 */
interface Answer {
  readonly act: "answer";
  readonly look: string;
  readonly value: string;
  readonly from: "joel";
}

/** A record read from a look's file: anything, until it's read. */
type Fields = Readonly<Record<string, unknown>>;

function isRecord(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Whether `value` is a string with anything in it, as Looks.js's `named`. */
function named(value: unknown): value is string {
  return typeof value === "string" && value !== "";
}

/** `value`'s words where it's a string, trimmed, else "". */
function written(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

/** When `value` falls, from milliseconds or an ISO time, as ForYou.js's `when`; or null. */
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

/** Whether `target` is a web address a link may open. */
function onTheWeb(target: string): boolean {
  try {
    const address = new URL(target);
    return address.protocol === "https:" || address.protocol === "http:";
  } catch {
    return false;
  }
}

/** `text` ending as a sentence ends, as LookForm.js ends each part. */
function ended(text: string): string {
  return text === "" || ENDED.test(text) ? text : `${text}.`;
}

/**
 * The look's choices, as Looks.js offers them: the first form's first four
 * sending a value, each value exactly as written, labelled by its value
 * where it has no label, with what follows its own else the second form's
 * choice of that label. A page never sends a value the session didn't offer.
 */
function choicesOf(look: Fields, wants: Fields): readonly Choice[] {
  const wanted = (Array.isArray(wants["choices"]) ? wants["choices"] : []).filter(isRecord);
  const follows = (one: Fields, label: string): string =>
    written(one["follows"]) || written(wanted.find((each) => each["label"] === label)?.["follows"]);
  return (Array.isArray(look["choices"]) ? look["choices"] : [])
    .filter(isRecord)
    .filter((one) => named(one["value"]))
    .slice(0, OFFERED)
    .map((one) => {
      const value = one["value"] as string;
      const label = named(one["label"]) ? one["label"] : value;
      return { value, label, follows: follows(one, label) };
    });
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

/**
 * What the ask row shows for `look`, a look's file as parsed, at `now`, in
 * milliseconds since the epoch; or null for one For You doesn't read.
 */
function asked(look: unknown, now: number): Ask | null {
  if (!(isRecord(look) && whole(look))) {
    return null;
  }
  const wants = isRecord(look["wants"]) ? look["wants"] : {};
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
  const opens = onTheWeb(target) ? target : null;
  return {
    id: look["id"] as string,
    kind,
    title: look["title"] as string,
    opens,
    target,
    said,
    question: written(look["ask"]),
    ifNothing: written(look["ifNothing"]),
    act: opens === null ? null : (happened === "" ? "" : written(look["act"])) || ACTS[kind],
    choices: choicesOf(look, wants),
    words: wants["words"] === true,
    asks: waits(look, now),
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
  return { act: "answer", look: ask.id, value: choice, from: "joel" };
}

export {
  ACTS,
  type Answer,
  type Ask,
  answering,
  asked,
  type Choice,
  type Kind,
  OFFERED,
  QUIET_AFTER,
};

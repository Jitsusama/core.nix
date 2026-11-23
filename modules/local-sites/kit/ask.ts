// An ask answered where it's asked: a look, as the harness writes it in
// looks/<id>.json, read into the parts every page's ask row lays out the
// same way, as the launcher's For You does (the kit inventory's seventh
// rule, and the For You layout of 22 November 2026). What happened and why
// it matters first, in the serif; the question, and what happens if he does
// nothing; up to four choices, each with what follows from it; a line for
// his own words where the look takes them; and the act, in the record's own
// words, opening what the look is about. The page draws the row; this says
// what's in it.
//
// It reads a look as the launcher's Looks.js and LookForm.js do, so the two
// never disagree about what waits on him: a look waits only while its state
// is "waiting", its expiresAt hasn't passed and it came less than three days
// ago, and one saying `asks: false` is only to be read. Its choices are the
// first form's, each sending the value the session offered, with what
// follows from it taken from the second form's choice of the same label.
//
// The harness writes version 1 with the second form's fields beside the
// first's (85PKB2, 22 November 2026) until every reader reads them, so the
// second form's fields are read wherever they're present, with no check
// of the version, and a look kept before them reads from the first form's.
//
// Every word here is free text a session wrote, so a page draws each as
// text, never as markup.

/** How many of a look's choices are offered, as the harness writes at most four. */
const OFFERED = 4;

/** How long after it came a waiting look goes quiet, as Looks.js's QUIET_AFTER. */
const QUIET_AFTER = 3 * 24 * 60 * 60 * 1000;

/** What a look can be about, as the harness writes them. */
const KINDS = ["pr", "page", "file", "diff", "terminal"] as const;

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
  /** What the answer sends, as the session that asked offered it. */
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
  /** Whether it takes an answer in his own words. */
  readonly words: boolean;
  /** Whether it waits on him, as For You's Waiting holds it, and isn't yet answered. */
  readonly asks: boolean;
  /** How he answered, in words for the row ("Keep It", "your own words"), or null. */
  readonly answered: string | null;
}

/**
 * An answer for the session that asked, as the launcher's steer says one
 * (Looks.js `answer`): a choice's value, or his own words. Whatever carries
 * it there names the session and the time, and checks it again against the
 * look itself, since a page is no guard: any page can say anything.
 */
interface Answer {
  readonly act: "answer";
  readonly look: string;
  readonly value?: string;
  readonly words?: string;
  readonly from: "joel";
}

/** A record read from a look's file: anything, until it's read. */
type Fields = Readonly<Record<string, unknown>>;

function isRecord(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** `value` where it's a string with words in it, else "". */
function written(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

/** When `value`, a time as the harness writes one, falls, or null. */
function when(value: unknown): number | null {
  const at = Date.parse(written(value));
  return Number.isNaN(at) ? null : at;
}

function isKind(value: string): value is Kind {
  return (KINDS as readonly string[]).includes(value);
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
 * The look's choices, as Looks.js offers them: the first form's, each
 * naming the value it sends, labelled by its value where it has no label,
 * with what follows its own else the second form's choice of that label.
 * One sending no value is left out, so a page never sends a value the
 * session didn't offer, and no name shows twice: the first keeps it.
 */
function choicesOf(look: Fields, wants: Fields): readonly Choice[] {
  const wanted = (Array.isArray(wants["choices"]) ? wants["choices"] : []).filter(isRecord);
  const said = (label: string): string =>
    written(wanted.find((one) => written(one["label"]) === label)?.["follows"]);
  const seen = new Set<string>();
  const offered: Choice[] = [];
  for (const one of (Array.isArray(look["choices"]) ? look["choices"] : []).filter(isRecord)) {
    const value = written(one["value"]);
    const label = written(one["label"]) || value;
    if (value !== "" && !seen.has(label.toLowerCase())) {
      seen.add(label.toLowerCase());
      offered.push({ value, label, follows: written(one["follows"]) || said(label) });
    }
  }
  return offered.slice(0, OFFERED);
}

/** How the look was answered, in words for the row, or null. */
function answeredOf(look: Fields, choices: readonly Choice[]): string | null {
  const answered = isRecord(look["answered"]) ? look["answered"] : {};
  const picked = written(answered["choice"]) || written(look["answer"]);
  if (picked !== "") {
    return choices.find((one) => one.value === picked || one.label === picked)?.label ?? picked;
  }
  return written(answered["words"]) === "" ? null : "your own words";
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
    written(look["state"]) === "waiting" &&
    !(expires !== null && now >= expires) &&
    !(came !== null && now - came > QUIET_AFTER) &&
    look["asks"] !== false
  );
}

/**
 * What the ask row shows for `look`, a look's file as parsed, at `now`, in
 * milliseconds since the epoch; or null for one it can't show: no id,
 * nothing it's about, or a kind it doesn't know.
 */
function asked(look: unknown, now: number): Ask | null {
  if (!isRecord(look)) {
    return null;
  }
  const about = isRecord(look["about"]) ? look["about"] : {};
  const wants = isRecord(look["wants"]) ? look["wants"] : {};
  const id = written(look["id"]);
  const named = written(about["kind"]);
  const kind = isKind(named) ? named : written(look["kind"]);
  const target = written(about["target"]) || written(look["target"]);
  const title = written(about["title"]) || written(look["title"]);
  if (id === "" || target === "" || title === "" || !isKind(kind)) {
    return null;
  }
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
  const choices = choicesOf(look, wants);
  const answered = answeredOf(look, choices);
  const opens = onTheWeb(target) ? target : null;
  return {
    id,
    kind,
    title,
    opens,
    target,
    said,
    question: written(look["ask"]),
    ifNothing: written(look["ifNothing"]),
    act: opens === null ? null : written(look["act"]) || ACTS[kind],
    choices,
    words: wants["words"] === true,
    asks: answered === null && waits(look, now),
    answered,
  };
}

/**
 * The answer that picking `choice`, one of `ask`'s values, sends; or, given
 * words, the answer in his own words. Null for a look that no longer asks,
 * a choice it doesn't offer, or words where it takes none or they're blank.
 * It keeps an honest page from sending what the look didn't ask; it guards
 * nothing, since whatever carries the answer must check it against the
 * look again.
 */
function answering(ask: Ask, given: { choice: string } | { words: string }): Answer | null {
  if (!ask.asks) {
    return null;
  }
  if ("choice" in given) {
    const choice = ask.choices.find((one) => one.value === given.choice);
    return choice === undefined
      ? null
      : { act: "answer", look: ask.id, value: choice.value, from: "joel" };
  }
  const words = given.words.trim();
  return ask.words && words !== "" ? { act: "answer", look: ask.id, words, from: "joel" } : null;
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

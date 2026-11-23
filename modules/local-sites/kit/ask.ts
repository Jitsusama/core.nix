// An ask answered where it's asked: a look, as the harness writes it in
// looks/<id>.json, read into the parts every page's ask row lays out the
// same way, as the launcher's For You does (the kit inventory's seventh
// rule, and the For You layout of 22 November 2026). What happened and why
// it matters first, in the serif; up to four choices, each with what
// follows from it; a line for his own words where the look takes them; and
// the act, in the record's own words, opening what the look is about. The
// page draws the row and sends the answer; this says what's in it.
//
// The harness writes version 1 with the second form's fields beside the
// first's (85PKB2, 22 November 2026) until every reader reads them, so the
// second form's fields are read wherever they're present, with no check
// of the version, and a look kept before them reads from the first form's.

/** How many of a look's choices are offered, as the harness writes at most four. */
const OFFERED = 4;

/** What a look can be about, as the harness writes them. */
const KINDS = ["pr", "page", "file", "diff", "terminal"] as const;

/** What a look is about. */
type Kind = (typeof KINDS)[number];

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
  /** What the answer sends, as the session that asked reads it. */
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
  /** What the look points at, whole, to show where `opens` is null. */
  readonly target: string;
  /** What happened and why it matters, each a paragraph, the row's first words. */
  readonly said: readonly string[];
  /** The act's words, on what he presses to open what it's about. */
  readonly act: string;
  /** The answers it offers, at most four. */
  readonly choices: readonly Choice[];
  /** Whether it takes an answer in his own words. */
  readonly words: boolean;
  /** Whether it waits on him: open, and asking something not yet answered. */
  readonly asks: boolean;
  /** How he answered, in words for the row ("Keep It", "your own words"), or null. */
  readonly answered: string | null;
}

/** An answer to send to the session that asked, from this page. */
interface Answer {
  readonly id: string;
  readonly choice?: string;
  readonly words?: string;
  readonly where: "site";
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

/**
 * The look's choices: the second form's, each with what follows, carrying
 * the value of the first form's choice of the same label, else the first
 * form's alone. One with no name is left out, and no name shows twice.
 */
function choicesOf(look: Fields, wants: Fields): readonly Choice[] {
  const first = (Array.isArray(look["choices"]) ? look["choices"] : []).filter(isRecord);
  const sent = new Map<string, string>();
  for (const one of first) {
    const value = written(one["value"]);
    if (value !== "") {
      sent.set(written(one["label"]) || value, value);
    }
  }
  const second = (Array.isArray(wants["choices"]) ? wants["choices"] : []).filter(isRecord);
  const offered: Choice[] =
    second.length > 0
      ? second.map((one) => {
          const label = written(one["label"]);
          return { value: sent.get(label) ?? label, label, follows: written(one["follows"]) };
        })
      : [...sent].map(([label, value]) => ({ value, label, follows: "" }));
  const seen = new Set<string>();
  return offered
    .filter((one) => {
      const named = one.label !== "" && !seen.has(one.label.toLowerCase());
      seen.add(one.label.toLowerCase());
      return named;
    })
    .slice(0, OFFERED);
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
 * What the ask row shows for `look`, a look's file as parsed, or null for
 * one it can't show: no id, nothing it's about, or a kind it doesn't know.
 */
function asked(look: unknown): Ask | null {
  if (!isRecord(look)) {
    return null;
  }
  const about = isRecord(look["about"]) ? look["about"] : {};
  const wants = isRecord(look["wants"]) ? look["wants"] : {};
  const id = written(look["id"]);
  const kind = written(about["kind"]) || written(look["kind"]);
  const target = written(about["target"]) || written(look["target"]);
  const title = written(about["title"]) || written(look["title"]);
  if (id === "" || target === "" || title === "" || !isKind(kind)) {
    return null;
  }
  const happened = [written(look["happened"]), written(look["matters"])].filter(
    (one) => one !== "",
  );
  const said =
    happened.length > 0
      ? [happened.join(" ")]
      : [written(look["why"]), written(look["ask"])].filter((one) => one !== "");
  const choices = choicesOf(look, wants);
  const words = wants["words"] === true;
  const answered = answeredOf(look, choices);
  const open = !["resolved", "cleared", "expired"].includes(written(look["state"]));
  return {
    id,
    kind,
    title,
    opens: onTheWeb(target) ? target : null,
    target,
    said,
    act: written(look["act"]) || ACTS[kind],
    choices,
    words,
    asks: open && answered === null && (choices.length > 0 || words),
    answered,
  };
}

/**
 * The answer that picking `choice`, one of `ask`'s, sends; or, given words,
 * the answer in his own words. Null for a choice the look doesn't offer, or
 * words where it takes none or they're blank, so a page can't send what the
 * session never asked.
 */
function answering(ask: Ask, given: { choice: string } | { words: string }): Answer | null {
  if ("choice" in given) {
    const choice = ask.choices.find((one) => one.value === given.choice);
    return choice === undefined ? null : { id: ask.id, choice: choice.value, where: "site" };
  }
  const words = given.words.trim();
  return ask.words && words !== "" ? { id: ask.id, words, where: "site" } : null;
}

export { ACTS, type Answer, type Ask, answering, asked, type Choice, type Kind, OFFERED };

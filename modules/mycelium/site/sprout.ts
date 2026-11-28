// New work Joel starts from the map, said as the link the desktop hands the
// launcher (modules/quickshell/mycelium-link): `mycelium:sprout` opens Start
// New Work with his words filled in, from the cell it is started from
// where one is open, and on the machine it is started on where a machine's
// drawer asked, `cloud` for any of the cloud's. A page reads and never
// acts, so the map can't ask the node to sprout; the launcher's own
// Return, pressed by him, is what starts anything, as it is when he types
// there himself. What follows shapes only
// the links this page writes: any page can write one, so mycelium-link and
// the launcher hold their own bounds on what a link may carry.

import { isNamed } from "../../local-sites/kit/cell.ts";

// A cell's address, as mycelium-link lets one through: 1 to 12 capitals
// and digits.
const ADDRESS = /^[A-Z0-9]{1,12}$/u;
// Any run of spaces, tabs or line breaks, which a line of words reads as
// one: Unicode's White_Space, as Rust's `char::is_whitespace` splits
// plain.rs's words, so U+0085 parts words and U+FEFF is left out below.
const GAPS = /\p{White_Space}+/u;
// What mycelium-link leaves out of words, as its plain.rs does: control and
// format characters, what else draws nothing, and the braille blank.
const UNSEEN = /[\p{Cc}\p{Cf}\p{Default_Ignorable_Code_Point}\u2800]/gu;

/**
 * The most letters new work's line takes: a few paragraphs, ample for a
 * task said in words, and well short of what a link can carry, since every
 * other part of a mycelium link is bounded too.
 */
export const LONGEST_WORDS = 1000;

/** Whether `from` is a cell's address as mycelium-link lets one through. */
export const isAddress = (from: string | null): from is string =>
  from !== null && ADDRESS.test(from);

/** What names any of the cloud's machines, as the Launcher reads it. */
export const CLOUD = "cloud";

/**
 * Whether `on` is a machine's name as mycelium-link lets one through: the
 * kit's one rule for a name a link carries, held to `tests/machines.txt`.
 */
export const isMachine = (on: string | null): on is string => on !== null && isNamed(on);

// The machine new work starts on, said as a place.
const placeWords = (on: string): string => (on === CLOUD ? "the cloud" : on);

/** Where new work starts, as the map's tick says it: on a machine, or the cloud. */
export const onWords = (on: string): string => `On ${placeWords(on)}`;

/**
 * Whether words kept to send again, `on` the machine they went to, may
 * still go there: they went to none, or to the cloud, or to a machine of
 * `names`, the map's own. Words for a machine that has left go with it,
 * since sent anywhere else they'd start work where Joel didn't ask.
 */
export const keptOnStands = (on: string | null, names: readonly string[]): boolean =>
  on === null || on === CLOUD || names.includes(on);

/**
 * The machine a machine's drawer offers new work on: any of the cloud's
 * for a basin's stand, else the machine itself. A machine named `cloud`
 * is offered nothing, since a link naming it would start the work on any
 * of the cloud's machines rather than that one, and nor is one whose name
 * no link carries.
 */
export const drawerOffersOn = (name: string, basin: boolean): string | null =>
  basin ? CLOUD : name !== CLOUD && isMachine(name) ? name : null;

/** Words last handed on, the cell they went from and the machine they went to. */
export interface Kept {
  readonly words: string;
  readonly from: string | null;
  readonly on: string | null;
}

/** What the line is asked to offer as it opens, or as words are typed on it. */
export interface Asked {
  /** The cell asked from: the open cell, or the one a drawer's button names. */
  readonly from: string | null;
  /** The machine asked on: the open drawer's, or the one its button names. */
  readonly on: string | null;
  /** Whether a drawer's button asked, naming its own cell or machine. */
  readonly named: boolean;
  /** The words kept to send again, or null. */
  readonly kept: Kept | null;
  /** The words on the line now. */
  readonly typed: string;
}

/** What the map holds now that bears on what the line offers. */
export interface Standing {
  /** The machines on the map, by name. */
  readonly machines: readonly string[];
  /** The machine the open drawer's Start New Work Here names, or null. */
  readonly drawerOn: string | null;
  /** Whether the open drawer has a Start New Work button for this cell. */
  readonly offersFrom: (address: string) => boolean;
  /** Whether this cell is still on the map. */
  readonly stands: (address: string) => boolean;
}

/** The cell and machine the line offers, and whether they are the kept words'. */
export interface Offer {
  readonly from: string | null;
  readonly on: string | null;
  readonly kept: boolean;
}

/**
 * What the line offers. Kept words come back with the machine they went
 * to, or none, and from the cell they went from while it is still on the
 * map, else from the one open now; a cell that leaves never sends its
 * words on the open drawer's machine, where they never went. A drawer's
 * button, words typed over the kept ones, or kept words for a machine that
 * has left offer afresh: the cell asked from where the open drawer offers
 * it, and the machine asked on where it is the open drawer's own.
 */
export const offered = (asked: Asked, map: Standing): Offer => {
  const { kept } = asked;
  const fresh = (address: string | null) =>
    isAddress(address) && map.offersFrom(address) ? address : null;
  if (
    kept !== null &&
    !asked.named &&
    asked.typed === kept.words &&
    keptOnStands(kept.on, map.machines)
  ) {
    const from = kept.from === null || map.stands(kept.from) ? kept.from : fresh(asked.from);
    return { from, on: isMachine(kept.on) ? kept.on : null, kept: true };
  }
  const on = isMachine(asked.on) && asked.on === map.drawerOn ? asked.on : null;
  return { from: fresh(asked.from), on, kept: false };
};

/**
 * What the map says when `on`, the machine the open line would start work
 * on, left the map before Return. It lets the machine go rather than send
 * there, so the next Return starts the work where new work starts.
 */
export const leftWords = (on: string): string =>
  `${placeWords(on)} has left the map, so the work won't start there; ` +
  "Return starts it where new work starts.";

/**
 * The words as one line, spaces between them, or "" when there are none,
 * with what the launcher leaves out left out here too, so words it would
 * refuse as empty are never said as sent, and cut at LONGEST_WORDS as the
 * line's own field is. A half of a pair of
 * UTF-16 halves left alone, pasted or by the cut, becomes the replacement
 * character, since encoding it as a link would throw.
 */
export const sproutWords = (typed: string): string =>
  typed
    .split(GAPS)
    .map((word) => word.replace(UNSEEN, ""))
    .filter((word) => word !== "")
    .join(" ")
    .slice(0, LONGEST_WORDS)
    .trimEnd()
    .toWellFormed();

/** What the map says when asked to start work with no words. */
export const NO_WORDS = "Say what the new work is first; the Launcher needs its words.";

/** What the map says when the machine it would start work on has no name a link carries. */
export const NO_MACHINE = "That machine's name can't go in a link; untick it to send the words.";

/**
 * Why no link can hand the launcher `typed` on `on`, or null where one can:
 * no words, or a machine whose name no link carries. `sproutLink` gives a
 * link exactly where this is null, so what the map says never strays from
 * why it gave none.
 */
export const unsprouted = (typed: string, on: string | null): string | null => {
  if (sproutWords(typed) === "") {
    return NO_WORDS;
  }
  return on && !isMachine(on) ? NO_MACHINE : null;
};

// The link for words and a machine unsprouted lets through, said once so
// sproutLink and returned can't hand on different links.
const linkFor = (typed: string, from: string | null, on: string | null): string => {
  const link = `mycelium:sprout?words=${encodeURIComponent(sproutWords(typed))}`;
  const fromLink = isAddress(from) ? `${link}&from=${from}` : link;
  return on ? `${fromLink}&on=${on}` : fromLink;
};

/**
 * The link that hands the launcher new work: its words, percent-encoded,
 * the cell it comes from when `from` is an address, and the machine it
 * starts on when `on` is a machine's name, or null when no words were
 * typed, since Start New Work opened empty asks nothing of it. An address
 * that isn't one is left out rather than sent for the launcher to refuse,
 * so the work still starts, from nowhere. An empty machine names none, and
 * a misshaped one gives null too, as mycelium-link refuses one: work
 * started anywhere but where Joel asked is worse than none.
 */
export const sproutLink = (
  typed: string,
  from: string | null,
  on: string | null = null,
): string | null => (unsprouted(typed, on) === null ? linkFor(typed, from, on) : null);

/**
 * What Return on the line comes to: the machine it would start work on
 * left the map, and is let go of and said; no link can be given, and why;
 * or the link that hands the words on, with what to keep to send again.
 */
export type Returned =
  | { readonly kind: "left"; readonly on: string; readonly said: string }
  | { readonly kind: "refused"; readonly why: string }
  | { readonly kind: "link"; readonly href: string; readonly kept: Kept };

/**
 * What Return does with `typed` from `from` on `on`, each already left out
 * where its tick is cleared, while the map holds `machines`. A machine
 * that left comes first, so words never go to one the map no longer has,
 * whatever else is wrong with them; then why no link can be given, so the
 * map says it rather than hand on nothing; else the link, keeping the words
 * as typed, since offered brings them back only while the line reads so.
 */
export const returned = (
  typed: string,
  from: string | null,
  on: string | null,
  machines: readonly string[],
): Returned => {
  if (on !== null && !keptOnStands(on, machines)) {
    return { kind: "left", on, said: leftWords(on) };
  }
  const why = unsprouted(typed, on);
  if (why !== null) {
    return { kind: "refused", why };
  }
  return { kind: "link", href: linkFor(typed, from, on), kept: { words: typed, from, on } };
};

/**
 * What the map says once it has sent the work on, `from` its cell's name
 * and `on` the machine it starts on. A page can't know whether the
 * launcher opened, and mycelium-link lets go of a link that follows
 * another within a few seconds, so it says how to send the words again.
 */
export const handedWords = (from: string | null, on: string | null = null): string =>
  `Sent to the Launcher's Start New Work${from === null ? "" : `, from ${from}`}` +
  `${on === null ? "" : `, on ${placeWords(on)}`}; ` +
  "nothing starts until you press Return there. " +
  "If it didn't open, wait a moment, then s brings the words back.";

/** What the map says once the line holds as many letters as it takes. */
export const FULL_WORDS = `The line holds ${LONGEST_WORDS} letters; any past that were left off.`;

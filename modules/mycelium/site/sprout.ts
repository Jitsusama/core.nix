// New work Joel starts from the map, said as the link the desktop hands the
// launcher (modules/quickshell/mycelium-link): `mycelium:sprout` opens Start
// New Work with his words filled in, from the cell it is started from
// where one is open, and on the machine it is started on where a machine's
// drawer asked, `cloud` for any of the cloud's. A page reads and never
// acts, so the map can't ask the node to sprout; the launcher's own Return, pressed by him, is what starts
// anything, as it is when he types there himself. What follows shapes only
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

/**
 * The link that hands the launcher new work: its words, percent-encoded,
 * the cell it comes from when `from` is an address, and the machine it
 * starts on when `on` is a machine's name, or null when no words were
 * typed, since Start New Work opened empty asks nothing of it. An address
 * that isn't one is left out rather than sent for the launcher to refuse,
 * so the work still starts, from nowhere. An empty machine names none, and
 * a misshaped one gives null too, as mycelium-link refuses one: work started anywhere but where Joel
 * asked is worse than none.
 */
export const sproutLink = (
  typed: string,
  from: string | null,
  on: string | null = null,
): string | null => {
  if (unsprouted(typed, on) !== null) {
    return null;
  }
  const link = `mycelium:sprout?words=${encodeURIComponent(sproutWords(typed))}`;
  const fromLink = isAddress(from) ? `${link}&from=${from}` : link;
  return on ? `${fromLink}&on=${on}` : fromLink;
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

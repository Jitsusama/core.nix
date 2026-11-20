// New work Joel starts from the map, said as the link the desktop hands the
// launcher (modules/quickshell/mycelium-link): `mycelium:sprout` opens Start
// New Work with his words filled in, and from the cell it is started from
// where one is open. A page reads and never acts, so the map can't ask the
// node to sprout; the launcher's own Return, pressed by him, is what starts
// anything, as it is when he types there himself.

// A cell's address, as mycelium-link lets one through: 1 to 12 capitals
// and digits.
const ADDRESS = /^[A-Z0-9]{1,12}$/u;
// Any run of spaces, tabs or line breaks, which a line of words reads as one.
const GAPS = /\s+/u;

/** The words as one line, spaces between them, or "" when there are none. */
export const sproutWords = (typed: string): string =>
  typed
    .split(GAPS)
    .filter((word) => word !== "")
    .join(" ");

/**
 * The link that hands the launcher new work: its words, percent-encoded,
 * and the cell it comes from when `from` is an address, or null when no
 * words were typed, since Start New Work opened empty asks nothing of it.
 * An address that isn't one is left out rather than sent for the launcher
 * to refuse, so the work still starts, from nowhere.
 */
export const sproutLink = (typed: string, from: string | null): string | null => {
  const words = sproutWords(typed);
  if (words === "") {
    return null;
  }
  const link = `mycelium:sprout?words=${encodeURIComponent(words)}`;
  return from !== null && ADDRESS.test(from) ? `${link}&from=${from}` : link;
};

/** What the map says once the launcher has the work, `from` its cell's name. */
export const handedWords = (from: string | null): string =>
  `Handed to the Launcher's Start New Work${from === null ? "" : `, from ${from}`}; nothing starts until you press Return there.`;

/** What the map says when asked to start work with no words. */
export const NO_WORDS = "Say what the new work is first; the Launcher needs its words.";

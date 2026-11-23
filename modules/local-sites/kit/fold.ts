// A long list folded to the rows that matter, said once for every page. Code
// once listed a repository's 124 snapshots one by one; a list now shows at
// most seven rows, ranked by the page, then one button saying how many more
// rows opening it shows. The page owns whether the fold is open, since its
// own keys open it (Code's address and z), so this works out what shows and
// says it, and the page draws the rows and wires the button.
//
// Rows the page counts but never holds (snapshots) are kept apart from the
// rows it folds: the button promises only what opening shows, so a reader
// who opens it never finds fewer rows than it said, and the page says the
// uncounted rest in its own words.

/** How many rows a folded list shows, about what the eye takes in at once. */
const SHOWN = 7;

/** What a page hands `folded`. */
interface Folding<Row> {
  /** Every row the page holds, most worth seeing first. */
  readonly rows: readonly Row[];
  /** Whether the page has the fold open. */
  readonly open: boolean;
  /**
   * How many there are in all, when the page counts more than it holds:
   * things never listed one by one, such as snapshots, are counted here.
   * A count under the rows held is read as the rows.
   */
  readonly count?: number;
  /** How many rows show while it's folded, if not seven. */
  readonly shown?: number;
  /**
   * The rows that show even when folded, wherever they rank: the cursor's
   * own row, so landing on it never loses it.
   */
  readonly kept?: (row: Row) => boolean;
}

/** What a folded list draws. */
interface Folded<Row> {
  /** The rows that show, in the order the page gave them. */
  readonly rows: readonly Row[];
  /** Whether the page has the fold open. */
  readonly open: boolean;
  /** How many rows opening shows that folded hides; none while it's open. */
  readonly more: number;
  /** Whether the list holds more rows than it shows folded, so it has a fold at all. */
  readonly foldable: boolean;
  /** How many the page counts but never lists, which no fold opens. */
  readonly unlisted: number;
}

/** The rows `folding` shows folded, kept rows in their place, each taking a slot. */
function shownFolded<Row>(folding: Folding<Row>): readonly Row[] {
  const kept = folding.kept ?? (() => false);
  let room = Math.max((folding.shown ?? SHOWN) - folding.rows.filter(kept).length, 0);
  return folding.rows.filter((row) => {
    if (kept(row)) {
      return true;
    }
    if (room > 0) {
      room -= 1;
      return true;
    }
    return false;
  });
}

/** The rows `folding` shows, how many opening adds, and how many are never listed. */
function folded<Row>(folding: Folding<Row>): Folded<Row> {
  const held = folding.rows.length;
  const unlisted = Math.max((folding.count ?? 0) - held, 0);
  const shut = shownFolded(folding);
  const foldable = shut.length < held;
  const rows = folding.open ? folding.rows : shut;
  return { rows, open: folding.open, more: held - rows.length, foldable, unlisted };
}

/**
 * What the fold's button says: how many more rows opening shows while
 * folded, and how to fold them again while open. A page whose rows have a
 * kind says it in `kind`, a word taking a plain s ("tree" gives "And 13
 * more trees").
 */
function foldSaid(more: number, open: boolean, kind = ""): string {
  if (open) {
    return "Fold them away";
  }
  const named = kind === "" ? "" : ` ${kind}${more === 1 ? "" : "s"}`;
  return `And ${String(more)} more${named}`;
}

/** Where `showFold` writes, as a test can stand in for. */
interface FoldButton {
  type: string;
  hidden: boolean;
  readonly style: { display: string };
  textContent: string | null;
  setAttribute(name: string, value: string): void;
}

/**
 * Sets `button` to say and do what `fold` is at. A list with no more rows
 * than it shows folded has no fold at all, open or shut, so a page that
 * opened a short list by its key never shows a button that folds nothing.
 * It's hidden by its style too, since a page's rule giving the button a
 * display, as a whole target does, would beat the hidden attribute alone.
 */
function showFold(button: FoldButton, fold: Folded<unknown>, kind = ""): void {
  button.type = "button";
  button.hidden = !fold.foldable;
  button.style.display = fold.foldable ? "" : "none";
  button.textContent = foldSaid(fold.more, fold.open, kind);
  button.setAttribute("aria-expanded", String(fold.open));
}

export type { FoldButton, Folded, Folding };
export { folded, foldSaid, SHOWN, showFold };

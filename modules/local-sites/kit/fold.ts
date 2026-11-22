// A long list folded to the rows that matter, said once for every page. Code
// once listed a repository's 124 snapshots one by one; a list now shows at
// most seven rows, ranked by the page, then one line saying how many more
// there are, which opens the rest. The page owns whether the fold is open,
// since its own keys open it (Code's address and z), so this works out what
// shows and says it, and the page draws the rows and wires the button.

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
   * Never fewer than the rows.
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
  /** How many don't show, which the button says; none while it's open. */
  readonly more: number;
}

/** The rows `folding` shows, and how many it folds away. */
function folded<Row>(folding: Folding<Row>): Folded<Row> {
  const count = Math.max(folding.count ?? 0, folding.rows.length);
  if (folding.open) {
    return { rows: folding.rows, more: count - folding.rows.length };
  }
  const shown = folding.shown ?? SHOWN;
  const kept = folding.kept ?? (() => false);
  const keptRows = folding.rows.filter(kept).length;
  let room = Math.max(shown - keptRows, 0);
  const rows = folding.rows.filter((row) => {
    if (kept(row)) {
      return true;
    }
    if (room > 0) {
      room -= 1;
      return true;
    }
    return false;
  });
  return { rows, more: count - rows.length };
}

/**
 * What the fold's button says: how many more there are while folded, and
 * how to fold them again while open. A page whose rows have a kind says it
 * in `kind` ("snapshot" gives "And 117 more snapshots").
 */
function foldSaid(more: number, open: boolean, kind = ""): string {
  if (open) {
    return "Fold them away";
  }
  const named = kind === "" ? "" : ` ${kind}${more === 1 ? "" : "s"}`;
  return `And ${String(more)} more${named}`;
}

/**
 * Sets `button` to say and do what the fold `more` is at: hidden when
 * nothing is folded and it's shut, so a short list has no fold at all.
 */
function showFold(button: HTMLButtonElement, more: number, open: boolean, kind = ""): void {
  button.type = "button";
  button.hidden = more === 0 && !open;
  button.textContent = foldSaid(more, open, kind);
  button.setAttribute("aria-expanded", String(open));
}

export type { Folded, Folding };
export { folded, foldSaid, SHOWN, showFold };

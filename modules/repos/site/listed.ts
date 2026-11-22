// Which of a season's trees the drawer lists before it counts the rest. A repository can hold
// two thousand trees written this week, most of them snapshots pinned to a commit and trees no
// cell works in any more, and a drawer listing each one is a page nobody can read and a redraw
// the browser takes seconds over.

import type { Season, Tree } from "./model.ts";

/** How many trees a season lists before the rest are counted, until they're opened. */
const TREES_LISTED = 12;

/** Whether a cell works in `tree` or a change names it, which lists it ahead of the others. */
function worth(tree: Tree): number {
  return tree.cells.length > 0 || tree.change !== undefined ? 1 : 0;
}

/**
 * The trees of a season worth listing, in their places: those cells work in or a change names
 * first, then the newest written, at most `TREES_LISTED`, and a snapshot only when it's worth
 * one of those, since its commit names it and most are a review's copy, read once; the trees in
 * `keep` always, so the cursor is seen where it is and a row it stood on doesn't go once it
 * steps off.
 */
function listedOf(trees: readonly Tree[], keep: ReadonlySet<string> | undefined): readonly Tree[] {
  const chosen = new Set(
    trees
      .filter((each) => !each.snapshot || worth(each) > 0)
      .sort(
        (one, other) => worth(other) - worth(one) || (other.writtenAt ?? 0) - (one.writtenAt ?? 0),
      )
      .slice(0, TREES_LISTED)
      .map((each) => each.path),
  );
  return trees.filter((each) => chosen.has(each.path) || keep?.has(each.path) === true);
}

/**
 * What the page holds of how much a drawer lists: each repository's seasons unfolded, those
 * listing every tree (as z or a season's button asks, apart from the seasons unfolded, since an
 * address opens only the season its tree is in), and the trees the cursor has stood on in the
 * repository open now, which stay listed until another opens.
 */
class Listing {
  readonly unfolded = new Map<string, Set<Season>>();
  readonly whole = new Set<string>();
  readonly #kept = new Set<string>();
  #keptIn: string | null = null;

  /**
   * Lists every tree of the repository at `key`, resting and gone opened, or folds them, the
   * trees the cursor stood on there forgotten with them, so stepping through every tree and
   * folding again leaves the twelve.
   */
  listWhole(key: string, every: boolean): void {
    const seasons = this.unfolded.get(key) ?? new Set<Season>();
    for (const season of ["resting", "gone"] as const) {
      seasons[every ? "add" : "delete"](season);
    }
    this.unfolded.set(key, seasons);
    this.whole[every ? "add" : "delete"](key);
    if (!every && key === this.#keptIn) {
      this.#kept.clear();
    }
  }

  /** The trees the cursor has stood on in the repository at `key`, `inner` among them now. */
  kept(key: string, inner: string | null): ReadonlySet<string> {
    if (key !== this.#keptIn) {
      this.#kept.clear();
      this.#keptIn = key;
    }
    if (inner !== null) {
      this.#kept.add(inner);
    }
    return this.#kept;
  }
}

export { Listing, listedOf, TREES_LISTED };

// Which of a season's trees the drawer lists before it counts the rest. A repository can hold
// two thousand trees written this week, most of them snapshots pinned to a commit and trees no
// cell works in any more, and a drawer listing each one is a page nobody can read and a redraw
// the browser takes seconds over.

import type { Tree } from "./model.ts";

/** How many trees a season lists before the rest are counted, until they're opened. */
const TREES_LISTED = 12;

/** Whether a cell works in `tree` or a change names it, which lists it ahead of the others. */
function worth(tree: Tree): number {
  return tree.cells.length > 0 || tree.change !== undefined ? 1 : 0;
}

/**
 * The trees of a season worth listing, in their places: those cells work in or a change names
 * first, then the newest written, at most `TREES_LISTED`, and never a snapshot, which its commit
 * names and nobody works in; the tree at `keep` always, so the cursor is seen where it is.
 */
function listedOf(trees: readonly Tree[], keep: string | null | undefined): readonly Tree[] {
  const chosen = new Set(
    trees
      .filter((each) => !each.snapshot)
      .sort(
        (one, other) => worth(other) - worth(one) || (other.writtenAt ?? 0) - (one.writtenAt ?? 0),
      )
      .slice(0, TREES_LISTED)
      .map((each) => each.path),
  );
  if (keep !== null && keep !== undefined) {
    chosen.add(keep);
  }
  return trees.filter((each) => chosen.has(each.path));
}

export { listedOf, TREES_LISTED };

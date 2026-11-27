// The system page's cursor: the row focus is on, among every row the keys
// step through, in reading order, what is worth a look first and then each
// machine. The cursor is focus itself, so Return follows the link it is on;
// what it keeps is the row's key, so a redraw that replaces the rows puts
// focus back on the one it was on, or the one now in its place within the
// same region (the looks, or the machines), and a machine moving never
// moves what Joel is reading. A region left with no row takes focus
// itself, so what it says now is read, rather than a row far below. A
// link under a row in its list item, such as a look's act, is the row's:
// the keys step on from it as from the row, a step back landing on the row
// itself since the link stands after it, and a redraw puts focus back on
// it.

/** The cursor over the rows `selector` finds. */
interface Cursor {
  /** Every row the keys step through, in reading order. */
  rows(): HTMLAnchorElement[];
  /** Puts the cursor on `row`. */
  land(row: HTMLAnchorElement | undefined): void;
  /** Moves the cursor `by` rows, to either end when `by` is infinite. */
  move(by: number): void;
  /** Puts the cursor on the first row of the section `id` names within `sections`. */
  toSection(id: string): void;
  /**
   * Puts the cursor on the next row within `within`, round to the first after
   * the last; false when it holds none.
   */
  next(within: ParentNode): boolean;
  /** Keeps the row `target` as the cursor, when focus landed on one. */
  keep(target: EventTarget | null): void;
  /** Runs `redraw`, then puts focus back where the cursor was, if it was on a row. */
  across(redraw: () => void): void;
}

/** The cursor over the rows `selector` finds, its sections within `sections`. */
function cursorOver(selector: string, sections: ParentNode): Cursor {
  // The row the cursor was last on, by its key, kept across a redraw.
  let key: string | null = null;
  const rows = (): HTMLAnchorElement[] => [
    ...document.querySelectorAll<HTMLAnchorElement>(selector),
  ];
  // Where `at` stands among `all`: the row it is, or the row of the list
  // item it sits in; -1 for neither.
  const placeOf = (all: readonly HTMLAnchorElement[], at: Element | null): number => {
    const row = all.findIndex((each) => each === at);
    if (row >= 0 || at === null) {
      return row;
    }
    const item = at.closest("li");
    return all.findIndex((each) => item !== null && each.parentElement === item);
  };
  const land = (row: HTMLAnchorElement | undefined): void => {
    if (row === undefined) {
      return;
    }
    key = row.dataset["row"] ?? null;
    // Focus alone would centre a row out of sight; scrolled to its nearest
    // edge instead, a step moves the page no further than it must.
    row.focus({ preventScroll: true });
    row.scrollIntoView({ block: "nearest" });
  };
  return {
    rows,
    land,
    move: (by) => {
      const all = rows();
      const found = placeOf(all, document.activeElement);
      // An act stands after the row it sits under, so a step back from it
      // lands on that row rather than the one before.
      const onAct = found >= 0 && all[found] !== document.activeElement;
      const at = onAct && by < 0 ? found + 1 : found;
      if (by === Number.POSITIVE_INFINITY) {
        land(all.at(-1));
      } else if (by === Number.NEGATIVE_INFINITY || at < 0) {
        land(all[0]);
      } else {
        land(all[Math.max(0, Math.min(all.length - 1, at + by))]);
      }
    },
    toSection: (id) => {
      const section = sections.querySelector(`#${CSS.escape(id)}`);
      land(section?.querySelector<HTMLAnchorElement>("a[data-row]") ?? undefined);
    },
    next: (within) => {
      const all = [...within.querySelectorAll<HTMLAnchorElement>("a[data-row]")];
      const at = placeOf(all, document.activeElement);
      land(all[(at + 1) % all.length]);
      return all.length > 0;
    },
    keep: (target) => {
      const all = rows();
      const row = target instanceof Element ? all[placeOf(all, target)] : undefined;
      if (row?.dataset["row"] !== undefined) {
        key = row.dataset["row"];
      }
    },
    across: (redraw) => {
      const active = document.activeElement;
      const before = rows();
      const owner = before[placeOf(before, active)];
      const region = owner?.closest<HTMLElement>("[data-rows]") ?? null;
      // The link under the row focus was on, kept by its address, since a
      // redraw replaces it too.
      const under = active instanceof HTMLAnchorElement && active !== owner ? active.href : null;
      const inRegion = (): HTMLAnchorElement[] =>
        region === null ? [] : [...region.querySelectorAll<HTMLAnchorElement>("a[data-row]")];
      const focused = inRegion().findIndex((row) => row === owner);
      redraw();
      if (region === null) {
        return;
      }
      // The row by its key, or where its words changed, the row now in its
      // place; either way the act under it keeps focus if it is still there.
      const now = inRegion();
      const kept = rows().find((each) => each.dataset["row"] === key);
      const standing = kept ?? now[Math.min(focused, now.length - 1)];
      const links = standing?.parentElement?.querySelectorAll<HTMLAnchorElement>("a") ?? [];
      const again = [...links].find((link) => link !== standing && link.href === under);
      (again ?? standing ?? region).focus({ preventScroll: true });
    },
  };
}

export type { Cursor };
export { cursorOver };

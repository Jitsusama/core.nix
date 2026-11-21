// The system page's cursor: the row focus is on, among every row the keys
// step through, in reading order, what is worth a look first and then each
// machine. The cursor is focus itself, so Return follows the link it is on;
// what it keeps is the row's key, so a redraw that replaces the rows puts
// focus back on the one it was on, or the one now in its place within the
// same region (the looks, or the machines), and a machine moving never
// moves what Joel is reading. A region left with no row takes focus
// itself, so what it says now is read, rather than a row far below.

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
      const at = all.findIndex((row) => row === document.activeElement);
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
      const at = all.findIndex((row) => row === document.activeElement);
      land(all[(at + 1) % all.length]);
      return all.length > 0;
    },
    keep: (target) => {
      if (target instanceof HTMLAnchorElement && target.dataset["row"] !== undefined) {
        key = target.dataset["row"];
      }
    },
    across: (redraw) => {
      const active = document.activeElement;
      const region =
        active instanceof HTMLAnchorElement && rows().includes(active)
          ? active.closest<HTMLElement>("[data-rows]")
          : null;
      const inRegion = (): HTMLAnchorElement[] =>
        region === null ? [] : [...region.querySelectorAll<HTMLAnchorElement>("a[data-row]")];
      const focused = inRegion().findIndex((row) => row === active);
      redraw();
      if (region === null) {
        return;
      }
      const same = rows().find((row) => row.dataset["row"] === key);
      const now = inRegion();
      (same ?? now[Math.min(focused, now.length - 1)] ?? region).focus({ preventScroll: true });
    },
  };
}

export type { Cursor };
export { cursorOver };

// A list drawn by its records' keys, so an update touches only its rows.
// Code once redrew 2,156 elements on every word from its feed and blocked
// the browser for 15 s; a feed already says which records moved, by key
// (records.ts), so the rows of every other record stay as they were drawn,
// the same elements, where a reader's eye and a screen reader's place stay.
// Focus stays too: a focused row moved keeps it, and one drawn again, afresh
// or where it stands, hands it to the same place in its new drawing when
// that place holds the same control, and to the row otherwise.
//
// The list owns every child of the element it draws into: a row is marked
// with its key in data-key, and anything else, a "Loading" sentence drawn
// before the feed spoke, an element or bare words, goes at the first
// drawing. Every row is drawn before the list's children are moved or taken
// out, so a drawing that throws on a record leaves its rows and their order
// as they were; a row it brought up to date where it stands before the
// throw keeps what it was given, focus in it given back as after any
// drawing, and the records the throwing call moved are the page's to draw
// again, whole, since keyed keeps no note of them.

import {
  alike,
  focusable,
  inside,
  type Known,
  knownAs,
  type Placed,
  refuses,
  scrolls,
} from "./known.ts";

/** As much of a drawn row as keyed reads: its key, kept on it. */
interface Drawn {
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
}

/** As much of the element a list is drawn into as keyed reads and moves. */
interface Holder<Item extends Drawn> {
  /** Its elements, in order. */
  readonly children: ArrayLike<Item>;
  /** Every child, bare words among them. */
  readonly childNodes: ArrayLike<object>;
  /** Where focus is: the document's focused element, if any. */
  readonly ownerDocument?: { readonly activeElement: Placed | null } | null;
  insertBefore(node: Item, before: Item | null): Item;
  removeChild(node: object): unknown;
}

/** What a page hands `keyed`. */
interface Drawing<Row, Item extends Drawn> {
  /** Every row, in the order they show. A key told twice shows once, first. */
  readonly rows: readonly Row[];
  /** The record key a row is drawn from. */
  readonly key: (row: Row) => string;
  /**
   * A row, drawn afresh, or `was`, its drawing until now, brought up to
   * date where it stands.
   */
  readonly draw: (row: Row, was?: Item) => Item;
  /**
   * The keys whose records moved since the last drawing, as the feed's
   * answer said them; every row is drawn afresh without it, as a page does
   * when its look or its whole answer changes.
   */
  readonly moved?: Iterable<string>;
}

/** What one drawing touched, counted for a page's tests and its budget. */
interface Drew {
  /** Rows drawn for a key the list didn't show. */
  readonly made: number;
  /** Rows drawn afresh, or brought up to date, because their record moved. */
  readonly redrawn: number;
  /** Rows left as they were drawn. */
  readonly kept: number;
  /** Rows taken out, their records gone, and children with no key. */
  readonly removed: number;
  /** Rows moved to a new place among the others. */
  readonly placed: number;
}

/** The attribute a drawn row keeps its key in. */
const KEY = "data-key";

/**
 * Draws `drawing`'s rows into `holder`: a new key's row is drawn, a moved
 * record's row is drawn afresh in its place, every other row is left as it
 * was, and those whose keys are gone are taken out. A row out of its place
 * is moved, each in turn, which costs a move for each row a reorder passes
 * over rather than the fewest: a feed's lists keep their order, and reorder
 * only as a sort changes, which is the reader's own doing.
 */
function keyed<Row, Item extends Drawn>(holder: Holder<Item>, drawing: Drawing<Row, Item>): Drew {
  const moved = drawing.moved === undefined ? null : new Set(drawing.moved);
  const wanted = new Set(drawing.rows.map(drawing.key));
  const shown = new Map<string, Item>();
  const stale: object[] = [];
  const elements = new Set<object>(Array.from(holder.children));
  for (const node of Array.from(holder.childNodes)) {
    if (!elements.has(node)) {
      stale.push(node);
    }
  }
  for (const item of Array.from(holder.children)) {
    const key = item.getAttribute(KEY);
    if (key === null || !wanted.has(key) || shown.has(key)) {
      stale.push(item);
    } else {
      shown.set(key, item);
    }
  }
  const page = holder.ownerDocument ?? null;
  const focused = page?.activeElement ?? null;
  const focus = within(focused, shown);
  // What it is is read while it's in the page, since a control taken out
  // with its row no longer reaches a form around the list, and only where
  // its row is drawn afresh, since a row kept keeps its control.
  const redrawing = focus !== null && (moved === null || moved.has(focus.key));
  const known = redrawing && focused !== null ? knownAs(focused, holder) : null;
  // A scroller, which takes focus though its markup says it takes none,
  // may keep it as long as it stays the control it was; only that is let
  // pass, since one disabled, inert or hidden since it took focus takes no
  // press, which the drawing reads afresh. Only what scrolls is granted:
  // a control whose tabindex or href the page took away before drawing
  // took focus by its markup, which no longer lets it.
  const granted = redrawing && focused !== null && !focusable(focused) && scrolls(focused);

  const before = { focused, known, page, holder, redrawing, granted };

  // Every row is drawn first, so nothing is touched if one throws.
  const rows: { key: string; item: Item; old: Item | undefined; fresh: boolean }[] = [];
  const reached = new Set<string>();
  try {
    drawEach(drawing, { moved, shown, rows, reached });
  } catch (error) {
    // The focused row may have been changed where it stands before the
    // throw, its own drawing's or a later one's, which a drawing again
    // would learn as the control it now is.
    const old = focus === null ? undefined : shown.get(focus.key);
    if (focus !== null && redrawing && reached.has(focus.key) && old !== undefined) {
      refocus(before, focus.path, old);
    }
    throw error;
  }

  for (const node of stale) {
    holder.removeChild(node);
  }
  let made = 0;
  let redrawn = 0;
  let kept = 0;
  let placed = 0;
  rows.forEach(({ item, old, fresh }, at) => {
    if (old === undefined) {
      made += 1;
    } else if (fresh) {
      if (item !== old) {
        holder.insertBefore(item, old);
        holder.removeChild(old);
      }
      redrawn += 1;
    } else {
      kept += 1;
    }
    const here = holder.children[at] ?? null;
    if (here !== item) {
      holder.insertBefore(item, here);
      if (old !== undefined) {
        placed += 1;
      }
    }
  });
  if (focus !== null) {
    const now = rows.find((row) => row.key === focus.key);
    if (now !== undefined) {
      refocus(before, focus.path, now.item);
    }
  }
  return { made, redrawn, kept, removed: stale.length, placed };
}

/**
 * Draws each of `drawing`'s rows into `rows`, once for each key, noting
 * each key in `reached` as its drawing starts: a new key's afresh, a moved
 * record's from its row until now, and every other row as it was.
 */
function drawEach<Row, Item extends Drawn>(
  drawing: Drawing<Row, Item>,
  {
    moved,
    shown,
    rows,
    reached,
  }: {
    readonly moved: ReadonlySet<string> | null;
    readonly shown: ReadonlyMap<string, Item>;
    readonly rows: { key: string; item: Item; old: Item | undefined; fresh: boolean }[];
    readonly reached: Set<string>;
  },
) {
  for (const row of drawing.rows) {
    const key = drawing.key(row);
    if (reached.has(key)) {
      continue;
    }
    reached.add(key);
    const old = shown.get(key);
    if (old === undefined) {
      rows.push({ key, item: marked(drawing.draw(row), key), old, fresh: true });
    } else if (moved === null || moved.has(key)) {
      rows.push({ key, item: marked(drawing.draw(row, old), key), old, fresh: true });
    } else {
      rows.push({ key, item: old, old, fresh: false });
    }
  }
}

/** `item`, marked with the key it's drawn from. */
function marked<Item extends Drawn>(item: Item, key: string): Item {
  item.setAttribute(KEY, key);
  return item;
}

/** Whether `node` can be walked for focus. */
function walkable(node: unknown): node is Placed {
  return typeof node === "object" && node !== null && "children" in node;
}

/**
 * The key of the row `focused` sits in among `rows`, and the way down to it
 * from the row, child by child; null when focus is in none of them.
 */
function within(
  focused: Placed | null,
  rows: ReadonlyMap<string, object>,
): { key: string; path: number[] } | null {
  const keys = new Map<object, string>();
  for (const [key, item] of rows) {
    keys.set(item, key);
  }
  const path: number[] = [];
  for (let at = focused; at !== null; at = at.parentElement) {
    const key = keys.get(at);
    if (key !== undefined) {
      return { key, path };
    }
    if (at.parentElement === null) {
      return null;
    }
    path.unshift(Array.prototype.indexOf.call(at.parentElement.children, at));
  }
  return null;
}

/**
 * Focus before a drawing: what had it, what that was, where focus is kept,
 * the list, whether its row is drawn again, and whether the page let focus
 * what its markup says takes none.
 */
interface Before {
  readonly focused: Placed | null;
  readonly known: Known | null;
  readonly page: { readonly activeElement: Placed | null } | null;
  readonly holder: object;
  readonly redrawing: boolean;
  readonly granted: boolean;
}

/**
 * Gives focus back after a drawing: to `focused` itself while it's still in
 * its row and still the control it was, and otherwise to the same place in
 * the row's new drawing where that holds the same control, or to the row
 * itself, since by its place alone another control there would take the
 * next press. A place the new drawing doesn't reach all the way down to is
 * another place. A row that takes no focus lets it go, as the page does
 * when a focused element leaves, and a control kept where it stands but
 * no longer the same is let go too, since nothing else takes focus from
 * it. A control that takes no press, which a browser may still call
 * focused, hands it to the row too, rather than let it fall to the page,
 * or is let go where the row takes none, whether drawn in place or afresh.
 * Whatever had focus in a row the drawing left alone keeps it. A page may
 * let more take focus than markup says, so a control that had focus
 * though its markup lets it take none, a scroller say, keeps it while it
 * stays the control it was and takes presses: not disabled, inert or
 * hidden, by the drawing or before it.
 */
function refocus(
  { focused, known, page, holder, redrawing, granted }: Before,
  path: number[],
  item: object,
) {
  const refused = (on: Placed) => refuses(on) || !(granted || focusable(on));
  const took = (on: Placed) =>
    (page === null || page.activeElement === on) && !(redrawing && refused(on));
  if (focused !== null && inside(focused, item)) {
    if (known === null || alike(focused, known, holder)) {
      focused.focus?.({ preventScroll: true });
      if (took(focused)) {
        return;
      }
    }
    if (walkable(item)) {
      item.focus?.({ preventScroll: true });
    }
    if (focused !== item && (page === null || page.activeElement === focused)) {
      focused.blur?.();
    }
    return;
  }
  if (!walkable(item)) {
    return;
  }
  let target: Placed = item;
  for (const at of path) {
    const next = target.children[at];
    if (next === undefined) {
      item.focus?.({ preventScroll: true });
      return;
    }
    target = next;
  }
  if (target !== item && alike(target, known, holder)) {
    target.focus?.({ preventScroll: true });
    if (took(target)) {
      return;
    }
  }
  item.focus?.({ preventScroll: true });
  if (target !== item && (page === null || page.activeElement === target)) {
    target.blur?.();
  }
}

export type { Drawing, Drawn, Drew, Holder, Placed };
export { KEY, keyed };

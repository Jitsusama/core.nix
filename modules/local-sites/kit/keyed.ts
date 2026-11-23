// A list drawn by its records' keys, so an update touches only its rows.
// Code once redrew 2,156 elements on every word from its feed and blocked
// the browser for 15 s; a feed already says which records moved, by key
// (records.ts), so the rows of every other record stay as they were drawn,
// the same elements, where a reader's eye and a screen reader's place stay.
//
// The list owns every child of the element it draws into: a row is marked
// with its key in data-key, and a child without one, a "Loading" sentence
// drawn before the feed spoke, goes at the first drawing.

/** As much of a drawn row as keyed reads: its key, kept on it. */
interface Drawn {
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
}

/** As much of the element a list is drawn into as keyed reads and moves. */
interface Holder<Item extends Drawn> {
  readonly children: ArrayLike<Item>;
  insertBefore(node: Item, before: Item | null): Item;
  removeChild(node: Item): Item;
}

/** What a page hands `keyed`. */
interface Drawing<Row, Item extends Drawn> {
  /** Every row, in the order they show. A key told twice shows once, first. */
  readonly rows: readonly Row[];
  /** The record key a row is drawn from. */
  readonly key: (row: Row) => string;
  /** A row, drawn afresh. */
  readonly draw: (row: Row) => Item;
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
  /** Rows drawn afresh because their record moved. */
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
  let removed = 0;
  for (const item of Array.from(holder.children)) {
    const key = item.getAttribute(KEY);
    if (key === null || !wanted.has(key) || shown.has(key)) {
      holder.removeChild(item);
      removed += 1;
    } else {
      shown.set(key, item);
    }
  }
  let made = 0;
  let redrawn = 0;
  let kept = 0;
  let placed = 0;
  const drawn = new Set<string>();
  for (const row of drawing.rows) {
    const key = drawing.key(row);
    if (drawn.has(key)) {
      continue;
    }
    drawn.add(key);
    const old = shown.get(key);
    let item: Item;
    if (old === undefined) {
      item = marked(drawing.draw(row), key);
      made += 1;
    } else if (moved === null || moved.has(key)) {
      item = marked(drawing.draw(row), key);
      holder.insertBefore(item, old);
      holder.removeChild(old);
      redrawn += 1;
    } else {
      item = old;
      kept += 1;
    }
    const here = holder.children[drawn.size - 1] ?? null;
    if (here !== item) {
      holder.insertBefore(item, here);
      if (old !== undefined) {
        placed += 1;
      }
    }
  }
  return { made, redrawn, kept, removed, placed };
}

/** `item`, marked with the key it's drawn from. */
function marked<Item extends Drawn>(item: Item, key: string): Item {
  item.setAttribute(KEY, key);
  return item;
}

export type { Drawing, Drawn, Drew, Holder };
export { KEY, keyed };

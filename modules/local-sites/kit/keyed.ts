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
// throw keeps what it was given, and the records the throwing call moved
// are the page's to draw again, whole, since keyed keeps no note of them.

/** As much of a drawn row as keyed reads: its key, kept on it. */
interface Drawn {
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
}

/**
 * As much of an element as keeping focus reads: where it sits, what it is
 * and says, and focusing it.
 */
interface Placed {
  readonly parentElement: Placed | null;
  readonly children: ArrayLike<Placed>;
  readonly localName?: string;
  readonly textContent?: string | null;
  getAttribute?(name: string): string | null;
  getAttributeNames?(): string[];
  focus?(options?: FocusOptions): void;
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
  const focused = holder.ownerDocument?.activeElement ?? null;
  const focus = within(focused, shown);

  // Every row is drawn first, so nothing is touched if one throws.
  const rows: { key: string; item: Item; old: Item | undefined; fresh: boolean }[] = [];
  const drawn = new Set<string>();
  for (const row of drawing.rows) {
    const key = drawing.key(row);
    if (drawn.has(key)) {
      continue;
    }
    drawn.add(key);
    const old = shown.get(key);
    if (old === undefined) {
      rows.push({ key, item: marked(drawing.draw(row), key), old, fresh: true });
    } else if (moved === null || moved.has(key)) {
      rows.push({ key, item: marked(drawing.draw(row, old), key), old, fresh: true });
    } else {
      rows.push({ key, item: old, old, fresh: false });
    }
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
      refocus(focused, focus.path, now.item);
    }
  }
  return { made, redrawn, kept, removed: stale.length, placed };
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

/** Whether `node` is `row` or inside it. */
function inside(node: Placed | null, row: object): boolean {
  for (let at = node; at !== null; at = at.parentElement) {
    if (at === row) {
      return true;
    }
  }
  return false;
}

/** A link whose href runs a script, which says nothing of where it goes. */
const SCRIPTED = /^javascript:/iu;

/**
 * What a control does, where its markup says it of this control alone:
 * where it goes, unless that's "#" or a script, which many links share;
 * else its name with its value, since a form's buttons and a group's radios
 * share a name and differ by value. Null where it says neither.
 */
function does(control: Placed): string | null {
  const href = control.getAttribute?.("href")?.trim() ?? "";
  if (href !== "" && href !== "#" && !SCRIPTED.test(href)) {
    return `href ${href}`;
  }
  const name = control.getAttribute?.("name") ?? "";
  const value = control.getAttribute?.("value") ?? "";
  return name !== "" && value !== "" ? `name ${name} ${value}` : null;
}

/**
 * The attributes saying a control's look or its state, not which control it
 * is: a toggle pressed or a tab chosen is the same control as before.
 */
const LOOKS = new Set([
  "aria-busy",
  "aria-checked",
  "aria-current",
  "aria-expanded",
  "aria-pressed",
  "aria-selected",
  "class",
  "id",
  "style",
  "tabindex",
]);

/**
 * A control's markup but its look and state, which says what it does beyond
 * where it goes: its form's action and method, a target, a download, a
 * page's own data-act, each told apart.
 */
function markup(control: Placed): string {
  return (control.getAttributeNames?.() ?? [])
    .filter((name) => !LOOKS.has(name))
    .sort()
    .map((name) => `${name}=${control.getAttribute?.(name) ?? ""}`)
    .join("\n");
}

/** What a control is heard as apart from its words, which tells two alike. */
function heard(control: Placed): string {
  return ["aria-label", "aria-labelledby", "title"]
    .map((name) => control.getAttribute?.(name) ?? "")
    .join("\n");
}

/**
 * Whether `one` is the same control as `other`: the same element with the
 * same markup but its look, heard as the same, doing the same, judged by
 * where it goes or its name, since its words may show the record that just
 * changed. Two sharing a place to go but marked for another act, a form's
 * other action or another label, are two controls. One whose markup says
 * neither is judged by its words too, and one with no words is no control
 * `one` can be known as.
 */
function alike(one: Placed, other: Placed | null): boolean {
  if (
    other === null ||
    one.localName !== other.localName ||
    markup(one) !== markup(other) ||
    heard(one) !== heard(other)
  ) {
    return false;
  }
  const did = does(other);
  if (did !== null) {
    return does(one) === did;
  }
  const words = other.textContent?.trim() ?? "";
  return words !== "" && does(one) === null && one.textContent?.trim() === words;
}

/**
 * Gives focus back after a drawing: to `focused` itself while it's still in
 * its row, and otherwise to the same place in the row's new drawing where
 * that holds the same control, or to the row itself, since by its place
 * alone another control there would take the next press. A place the new
 * drawing doesn't reach all the way down to is another place. A row that
 * takes no focus lets it go, as the page does when a focused element leaves.
 */
function refocus(focused: Placed | null, path: number[], item: object) {
  if (focused !== null && inside(focused, item)) {
    focused.focus?.({ preventScroll: true });
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
  (target === item || alike(target, focused) ? target : item).focus?.({ preventScroll: true });
}

export type { Drawing, Drawn, Drew, Holder, Placed };
export { KEY, keyed };

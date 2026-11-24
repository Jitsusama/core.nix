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
  const page = holder.ownerDocument ?? null;
  const focused = page?.activeElement ?? null;
  const focus = within(focused, shown);
  // What it is is read while it's in the page, since a control taken out
  // with its row no longer reaches a form around the list, and only where
  // its row is drawn afresh, since a row kept keeps its control.
  const redrawing = focus !== null && (moved === null || moved.has(focus.key));
  const known = redrawing && focused !== null ? knownAs(focused, holder) : null;

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
      refocus({ focused, known, page, holder }, focus.path, now.item);
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

/** What a browser drops from an address before reading it. */
const DROPPED = /[\t\n\r]/gu;

/** A fragment naming no element, as "#", "#!", "#0" and "#/" do, which many links share. */
const PLACEHOLDER = /^#[^a-z]*$/iu;

/**
 * What a control does, where its markup says it of this control alone:
 * where it goes, unless that's a placeholder or a script, which many
 * links share, read as a browser reads an address; else the page's own
 * act for it; else its name with its value, since a form's buttons and a
 * group's radios share a name and differ by value. Null where it says none
 * of them.
 */
function does(control: Placed): string | null {
  const href = (control.getAttribute?.("href") ?? "").replace(DROPPED, "").trim();
  if (href !== "" && !PLACEHOLDER.test(href) && !SCRIPTED.test(href)) {
    return `href ${href}`;
  }
  const act = control.getAttribute?.("data-act")?.trim() ?? "";
  if (act !== "") {
    return `act ${act}`;
  }
  const name = control.getAttribute?.("name") ?? "";
  const value = control.getAttribute?.("value") ?? "";
  return name !== "" && value !== "" ? `name ${name} ${value}` : null;
}

/**
 * The attributes saying a control's state, not which control it is: a
 * toggle pressed or ticked, an option or a tab chosen, a disclosure open or
 * a button disabled while it works is the same control as before. A class
 * or an id isn't among them, since a page may key a control's act on one;
 * a page drawing new ids each time sends focus to the row, which fails safe.
 */
const LOOKS = new Set([
  "aria-busy",
  "aria-checked",
  "aria-current",
  "aria-describedby",
  "aria-disabled",
  "aria-expanded",
  "aria-invalid",
  "aria-pressed",
  "aria-selected",
  "checked",
  "disabled",
  "open",
  "selected",
  "style",
  "tabindex",
]);

/** The form a control sends, the nearest holding it. */
function formOf(control: Placed): Placed | null {
  for (let at = control.parentElement; at !== null; at = at.parentElement) {
    if (at.localName === "form") {
      return at;
    }
  }
  return null;
}

/** An element's attributes but its state, each a name and its value, in order. */
function attributesOf(element: Placed): [string, string][] {
  return (element.getAttributeNames?.() ?? [])
    .filter((name) => !LOOKS.has(name))
    .sort()
    .map((name) => [name, element.getAttribute?.(name) ?? ""]);
}

/**
 * What a form sends beside `control`: each other named element in it, by
 * its tag, name and value as written, in order, since a hidden input may
 * be what carries a form's act. Of a form around the whole list, only what
 * sits outside the list, or in the control's own row, counts: a row coming,
 * going or changing beside it changes nothing the control's press says.
 */
function fieldsOf(form: Placed, control: Placed, list: object): [string, string, string][] {
  const fields: [string, string, string][] = [];
  for (const each of Array.from(form.children)) {
    const name = each.getAttribute?.("name") ?? "";
    if (each !== control && name !== "") {
      fields.push([each.localName ?? "", name, each.getAttribute?.("value") ?? ""]);
    }
    if (each === list) {
      for (const row of Array.from(each.children).filter((one) => inside(control, one))) {
        fields.push(...fieldsOf(row, control, list));
      }
    } else {
      fields.push(...fieldsOf(each, control, list));
    }
  }
  return fields;
}

/**
 * A control's markup but its state, which says what it does and how it's
 * heard: where it goes, a target, a download, a page's own data-act and
 * class, its label and title, and the form it sends, by that form's own
 * markup but its state and what else it sends, each told apart, so no
 * value can pass for another's.
 */
function markup(control: Placed, list: object): string {
  const form = formOf(control);
  return JSON.stringify({
    form: form === null ? null : attributesOf(form),
    sends: form === null ? null : fieldsOf(form, control, list),
    control: attributesOf(control),
  });
}

/** A control as it's known again in a new drawing, read while it's in the page. */
interface Known {
  readonly localName: string | undefined;
  readonly markup: string;
  readonly does: string | null;
  readonly words: string;
  readonly apart: boolean;
}

/**
 * What `control` is, to know it again by. One labelled by another element,
 * or sending a form named by its form attribute, stands apart from what
 * it holds and what holds it, which this doesn't read.
 */
function knownAs(control: Placed, list: object): Known {
  return {
    localName: control.localName,
    markup: markup(control, list),
    does: does(control),
    words: control.textContent?.trim() ?? "",
    apart: ["aria-labelledby", "form"].some((name) => (control.getAttribute?.(name) ?? "") !== ""),
  };
}

/**
 * Whether `one` is the control `other` was: the same element with the
 * same markup but its state, which then says it does the same, so where
 * the markup says what it does (where it goes, the page's act, its name
 * and value) its words may show the record that just changed. One whose
 * markup says none of those is judged by its words too, and one with no
 * words is no control `one` can be known as. One labelled by another
 * element, or sending a form it names by id, is never known again, since
 * what it says or sends isn't its own to read here. A listener no markup
 * shows isn't seen, so a page drawing by key marks each act in its
 * controls' markup.
 */
function alike(one: Placed, other: Known | null, list: object): boolean {
  if (
    other === null ||
    one.localName !== other.localName ||
    markup(one, list) !== other.markup ||
    other.apart
  ) {
    return false;
  }
  if (other.does !== null) {
    return true;
  }
  return other.words !== "" && one.textContent?.trim() === other.words;
}

/** Focus before a drawing: what had it, what that was, where focus is kept, and the list. */
interface Before {
  readonly focused: Placed | null;
  readonly known: Known | null;
  readonly page: { readonly activeElement: Placed | null } | null;
  readonly holder: object;
}

/**
 * Gives focus back after a drawing: to `focused` itself while it's still in
 * its row, and otherwise to the same place in the row's new drawing where
 * that holds the same control, or to the row itself, since by its place
 * alone another control there would take the next press. A place the new
 * drawing doesn't reach all the way down to is another place. A row that
 * takes no focus lets it go, as the page does when a focused element leaves.
 * A control that won't take focus, as one now disabled won't, kept in its
 * row or drawn afresh, hands it to the row too, rather than let it fall to
 * the page.
 */
function refocus({ focused, known, page, holder }: Before, path: number[], item: object) {
  const landed = (on: Placed) => page === null || page.activeElement === on;
  if (focused !== null && inside(focused, item)) {
    focused.focus?.({ preventScroll: true });
    if (!landed(focused) && walkable(item)) {
      item.focus?.({ preventScroll: true });
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
    if (landed(target)) {
      return;
    }
  }
  item.focus?.({ preventScroll: true });
}

export type { Drawing, Drawn, Drew, Holder, Placed };
export { KEY, keyed };

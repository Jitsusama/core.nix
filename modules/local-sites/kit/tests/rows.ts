// A list and its rows as keyed meets them in a page, faithful where keyed
// leans on the DOM: an element taken out of the page takes no focus, and
// taking out or moving what holds focus drops it to the page. Shared by
// keyed's drawing tests and its focus tests.

import { type Drawn, type Holder, KEY, keyed, type Placed } from "../keyed.ts";

/** Where focus is, as a document keeps it. */
class Page {
  activeElement: Placed | null = null;

  /** Whether focus is `node` or inside it. */
  holdsFocusIn(node: object): boolean {
    for (let at = this.activeElement; at !== null; at = at.parentElement) {
      if (at === node) {
        return true;
      }
    }
    return false;
  }
}

/**
 * A control drawn with words and attributes of its own, as a page's button or
 * link is: a link unless told its element, holding another where told.
 */
interface Control {
  readonly says: string;
  readonly attributes: Readonly<Record<string, string>>;
  readonly kind?: string;
  readonly holds?: Control;
}

/** One of a row's acts: a link to its name, a control saying nothing, or one told whole. */
type Act = string | null | Control;

/**
 * Something inside a row that takes focus, as a row's link does, saying
 * what it's given. Taken out, it has no parent and takes no focus, as an
 * element out of the page doesn't.
 */
class Inner implements Placed {
  parentElement: Placed | null;
  readonly children: Placed[] = [];
  readonly localName: string;
  readonly textContent: string;
  readonly #page: Page;
  readonly #attributes: Readonly<Record<string, string>>;

  constructor(parent: Placed, page: Page, says: string, told: Omit<Control, "says">) {
    this.parentElement = parent;
    this.#page = page;
    this.textContent = says;
    this.#attributes = told.attributes;
    this.localName = told.kind ?? "a";
    if (told.holds !== undefined) {
      this.children.push(new Inner(this, page, told.holds.says, told.holds));
    }
  }

  getAttribute(name: string): string | null {
    return this.#attributes[name] ?? null;
  }

  getAttributeNames(): string[] {
    return Object.keys(this.#attributes);
  }

  focus(): void {
    if (this.parentElement !== null) {
      this.#page.activeElement = this;
    }
  }
}

/**
 * A row as a list holds one: its attributes, its words and a link for each
 * of its acts saying them (open and copy, unless told), and whether it
 * takes focus itself, as a row with a tabindex does. An act told as null is
 * a control showing no words and saying nothing of what it does; one told
 * as a Control shows its words before the row's, with its own attributes.
 */
class Row implements Drawn, Placed {
  words: string;
  readonly parentElement = null;
  readonly localName = "li";
  children: Inner[];
  readonly #attributes = new Map<string, string>();
  readonly #page: Page;
  readonly #focusable: boolean;
  readonly #acts: readonly Act[];

  constructor(
    words: string,
    page: Page,
    focusable = true,
    acts: readonly Act[] = ["open", "copy"],
  ) {
    this.words = words;
    this.#page = page;
    this.#focusable = focusable;
    this.#acts = acts;
    this.children = this.#links();
  }

  get textContent(): string {
    return this.words;
  }

  /** Draws its inside again, saying `words`, its old links taken out. */
  again(words: string): void {
    for (const link of this.children) {
      if (this.#page.activeElement === link) {
        this.#page.activeElement = null;
      }
      link.parentElement = null;
    }
    this.words = words;
    this.children = this.#links();
  }

  #links(): Inner[] {
    return this.#acts.map((act) => {
      if (act === null) {
        return new Inner(this, this.#page, "", { attributes: {} });
      }
      if (typeof act === "string") {
        const attributes = { href: `#${act}` };
        return new Inner(this, this.#page, `${act} ${this.words}`, { attributes });
      }
      return new Inner(this, this.#page, `${act.says} ${this.words}`, act);
    });
  }

  getAttribute(name: string): string | null {
    return this.#attributes.get(name) ?? null;
  }

  setAttribute(name: string, value: string): void {
    this.#attributes.set(name, value);
  }

  focus(): void {
    if (this.#focusable) {
      this.#page.activeElement = this;
    }
  }
}

/**
 * A list's element, its rows in order and bare words apart, as the DOM
 * moves them: taking out or moving what holds focus drops it to the page.
 */
class List implements Holder<Row> {
  readonly rows: Row[] = [];
  readonly words: object[] = [];
  readonly ownerDocument = new Page();

  get children(): ArrayLike<Row> {
    return this.rows;
  }

  get childNodes(): ArrayLike<object> {
    return [...this.words, ...this.rows];
  }

  insertBefore(node: Row, before: Row | null): Row {
    const at = this.rows.indexOf(node);
    if (at >= 0) {
      this.#loses(node);
      this.rows.splice(at, 1);
    }
    const to = before === null ? this.rows.length : this.rows.indexOf(before);
    this.rows.splice(to, 0, node);
    return node;
  }

  removeChild(node: object): unknown {
    this.#loses(node);
    const row = this.rows.indexOf(node as Row);
    if (row >= 0) {
      this.rows.splice(row, 1);
    } else {
      this.words.splice(this.words.indexOf(node), 1);
    }
    return node;
  }

  /** The keys shown, in order. */
  keys(): (string | null)[] {
    return this.rows.map((row) => row.getAttribute(KEY));
  }

  #loses(node: object): void {
    if (this.ownerDocument.holdsFocusIn(node)) {
      this.ownerDocument.activeElement = null;
    }
  }
}

interface Cell {
  readonly key: string;
  readonly works: string;
}

const cells = (...keys: string[]): Cell[] => keys.map((key) => ({ key, works: `on ${key}` }));

/** A row made for `key` and not yet drawn, as a page's stray markup holds one. */
const loose = (list: List, key: string): Row => {
  const row = new Row(`left ${key}`, list.ownerDocument);
  row.setAttribute(KEY, key);
  return row;
};

const draw = (list: List, rows: readonly Cell[], moved?: readonly string[]) =>
  keyed(list, {
    rows,
    key: (cell) => cell.key,
    draw: (cell) => new Row(cell.works, list.ownerDocument),
    ...(moved === undefined ? {} : { moved }),
  });

/** Whether `list` shows exactly `rows`, the same elements, in order. */
const same = (list: List, rows: readonly (Row | undefined)[]): boolean =>
  list.rows.length === rows.length && list.rows.every((row, at) => row === rows[at]);

export { type Act, type Cell, cells, draw, List, loose, Row, same };

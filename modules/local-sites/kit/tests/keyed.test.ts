// A list drawn by key: the first drawing makes each row, a moved record's
// row alone is drawn afresh, gone rows go, new ones land in their place, a
// reorder moves each row it passes over, children with no key go, a drawing
// that throws touches nothing, and focus stays where the reader left it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
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

/** Something inside a row that takes focus, as a row's link does. */
class Inner implements Placed {
  readonly parentElement: Placed;
  readonly children: Placed[] = [];
  readonly #page: Page;

  constructor(parent: Placed, page: Page) {
    this.parentElement = parent;
    this.#page = page;
  }

  focus(): void {
    this.#page.activeElement = this;
  }
}

/** A row as a list holds one: its attributes, its words and two links. */
class Row implements Drawn, Placed {
  readonly words: string;
  readonly parentElement = null;
  readonly children: Inner[];
  readonly #attributes = new Map<string, string>();
  readonly #page: Page;

  constructor(words: string, page: Page) {
    this.words = words;
    this.#page = page;
    this.children = [new Inner(this, page), new Inner(this, page)];
  }

  getAttribute(name: string): string | null {
    return this.#attributes.get(name) ?? null;
  }

  setAttribute(name: string, value: string): void {
    this.#attributes.set(name, value);
  }

  focus(): void {
    this.#page.activeElement = this;
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

it("the first drawing makes each row, marked with its key, in order", () => {
  const list = new List();
  const drew = draw(list, cells("a", "b", "c"));
  assert.deepEqual(list.keys(), ["a", "b", "c"]);
  assert.deepEqual(
    list.rows.map((row) => row.words),
    ["on a", "on b", "on c"],
  );
  assert.deepEqual(drew, { made: 3, redrawn: 0, kept: 0, removed: 0, placed: 0 });
});

it("a moved record's row alone is drawn afresh, the others the same elements", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  const [a, b, c] = list.rows;
  const drew = draw(list, cells("a", "b", "c"), ["b"]);
  assert.equal(list.rows[0], a);
  assert.notEqual(list.rows[1], b);
  assert.equal(list.rows[2], c);
  assert.deepEqual(list.keys(), ["a", "b", "c"]);
  assert.deepEqual(drew, { made: 0, redrawn: 1, kept: 2, removed: 0, placed: 0 });
});

it("with no moved keys every row is drawn afresh", () => {
  const list = new List();
  draw(list, cells("a", "b"));
  const before = [...list.rows];
  const drew = draw(list, cells("a", "b"));
  assert.ok(list.rows.every((row, at) => row !== before[at]));
  assert.deepEqual(drew, { made: 0, redrawn: 2, kept: 0, removed: 0, placed: 0 });
});

it("an update with nothing moved touches nothing", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  const before = [...list.rows];
  const drew = draw(list, cells("a", "b", "c"), []);
  assert.ok(same(list, before), "a row was swapped though nothing moved");
  assert.deepEqual(drew, { made: 0, redrawn: 0, kept: 3, removed: 0, placed: 0 });
});

it("a gone record's row goes, and a new one lands in its place", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  const drew = draw(list, cells("a", "d", "c"), ["d"]);
  assert.deepEqual(list.keys(), ["a", "d", "c"]);
  assert.deepEqual(drew, { made: 1, redrawn: 0, kept: 2, removed: 1, placed: 0 });
});

it("a moved key whose record is gone takes its row out and draws nothing", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  const drew = draw(list, cells("a", "c"), ["b"]);
  assert.deepEqual(list.keys(), ["a", "c"]);
  assert.deepEqual(drew, { made: 0, redrawn: 0, kept: 2, removed: 1, placed: 0 });
});

it("a row moved to the front is the one row placed", () => {
  const list = new List();
  draw(list, cells("a", "b", "c", "d"));
  const [a, b, c, d] = list.rows;
  const drew = draw(list, cells("d", "a", "b", "c"), []);
  assert.ok(same(list, [d, a, b, c]), "the rows aren't the same elements, reordered");
  assert.equal(drew.placed, 1);
});

it("a row moved to the back moves each row it passes over", () => {
  const list = new List();
  draw(list, cells("a", "b", "c", "d"));
  const [a, b, c, d] = list.rows;
  const drew = draw(list, cells("b", "c", "d", "a"), []);
  assert.ok(same(list, [b, c, d, a]), "the rows aren't the same elements, reordered");
  assert.equal(drew.placed, 3);
});

it("a moved record's row that changes place is drawn afresh and placed", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  const [a, b] = list.rows;
  const drew = draw(list, cells("c", "a", "b"), ["c"]);
  assert.deepEqual(list.keys(), ["c", "a", "b"]);
  assert.equal(list.rows[1], a);
  assert.equal(list.rows[2], b);
  assert.deepEqual(drew, { made: 0, redrawn: 1, kept: 2, removed: 0, placed: 1 });
});

it("a child with no key, drawn before the feed spoke, goes", () => {
  const list = new List();
  list.rows.push(new Row("Loading", list.ownerDocument));
  const drew = draw(list, cells("a"));
  assert.deepEqual(list.keys(), ["a"]);
  assert.equal(drew.removed, 1);
});

it("bare words drawn before the feed spoke go too", () => {
  const list = new List();
  list.words.push({});
  const drew = draw(list, cells("a"));
  assert.deepEqual(list.words, []);
  assert.deepEqual(list.keys(), ["a"]);
  assert.equal(drew.removed, 1);
});

it("two rows already showing one key keep the first", () => {
  const list = new List();
  const first = loose(list, "a");
  list.rows.push(first, loose(list, "a"));
  const drew = draw(list, cells("a"), []);
  assert.ok(same(list, [first]), "the first row for the key isn't the one kept");
  assert.deepEqual(drew, { made: 0, redrawn: 0, kept: 1, removed: 1, placed: 0 });
});

it("a key told twice shows once, where it's first told", () => {
  const list = new List();
  const drew = draw(list, [...cells("a", "b"), { key: "a", works: "again" }]);
  assert.deepEqual(list.keys(), ["a", "b"]);
  assert.equal(list.rows[0]?.words, "on a");
  assert.equal(drew.made, 2);
});

it("a drawing that throws on a record leaves the list as it was", () => {
  const list = new List();
  list.words.push({});
  draw(list, cells("a", "b", "c"));
  const before = [...list.rows];
  const words = [...list.words];
  assert.throws(() =>
    keyed(list, {
      rows: cells("a", "b", "c"),
      key: (cell) => cell.key,
      draw: (cell) => {
        if (cell.key === "b") {
          throw new Error("b's record isn't the shape its row reads");
        }
        return new Row(cell.works, list.ownerDocument);
      },
      moved: ["a", "b"],
    }),
  );
  assert.ok(same(list, before), "a row changed though the drawing threw");
  assert.deepEqual(list.words, words);
});

it("a row brought up to date where it stands is redrawn, the same element", () => {
  const list = new List();
  draw(list, cells("a", "b"));
  const [a, b] = list.rows;
  const drew = keyed(list, {
    rows: cells("a", "b"),
    key: (cell) => cell.key,
    draw: (cell, was) => was ?? new Row(cell.works, list.ownerDocument),
    moved: ["b"],
  });
  assert.ok(same(list, [a, b]), "a row brought up to date was swapped");
  assert.deepEqual(drew, { made: 0, redrawn: 1, kept: 1, removed: 0, placed: 0 });
});

it("a focused row moved keeps its focus", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  const link = list.rows[2]?.children[1];
  link?.focus();
  draw(list, cells("c", "a", "b"), []);
  assert.equal(list.ownerDocument.activeElement, link);
});

it("a focused row drawn afresh hands focus to the same place in its drawing", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  list.rows[1]?.children[1]?.focus();
  draw(list, cells("a", "b", "c"), ["b"]);
  assert.equal(list.ownerDocument.activeElement, list.rows[1]?.children[1]);
});

it("focus in a row that goes is left where the page puts it", () => {
  const list = new List();
  draw(list, cells("a", "b"));
  list.rows[1]?.focus();
  draw(list, cells("a"), []);
  assert.equal(list.ownerDocument.activeElement, null);
});

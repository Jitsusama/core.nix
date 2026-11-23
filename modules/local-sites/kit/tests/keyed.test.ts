// A list drawn by key: the first drawing makes each row, a moved record's
// row alone is drawn afresh, gone rows go, new ones land in their place, a
// reorder moves only what moved, and children with no key go.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { type Drawn, type Holder, KEY, keyed } from "../keyed.ts";

/** A row as a list holds one: its attributes and the words drawn in it. */
class Row implements Drawn {
  readonly words: string;
  readonly #attributes = new Map<string, string>();

  constructor(words: string) {
    this.words = words;
  }

  getAttribute(name: string): string | null {
    return this.#attributes.get(name) ?? null;
  }

  setAttribute(name: string, value: string): void {
    this.#attributes.set(name, value);
  }
}

/** A list's element, its rows in order, as the DOM moves them. */
class List implements Holder<Row> {
  readonly rows: Row[] = [];

  get children(): ArrayLike<Row> {
    return this.rows;
  }

  insertBefore(node: Row, before: Row | null): Row {
    const at = this.rows.indexOf(node);
    if (at >= 0) {
      this.rows.splice(at, 1);
    }
    const to = before === null ? this.rows.length : this.rows.indexOf(before);
    this.rows.splice(to, 0, node);
    return node;
  }

  removeChild(node: Row): Row {
    this.rows.splice(this.rows.indexOf(node), 1);
    return node;
  }

  /** The keys shown, in order. */
  keys(): (string | null)[] {
    return this.rows.map((row) => row.getAttribute(KEY));
  }
}

interface Cell {
  readonly key: string;
  readonly works: string;
}

const cells = (...keys: string[]): Cell[] => keys.map((key) => ({ key, works: `on ${key}` }));

const draw = (list: List, rows: readonly Cell[], moved?: readonly string[]) =>
  keyed(list, {
    rows,
    key: (cell) => cell.key,
    draw: (cell) => new Row(cell.works),
    ...(moved === undefined ? {} : { moved }),
  });

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
  assert.deepEqual(list.rows, before);
  assert.deepEqual(drew, { made: 0, redrawn: 0, kept: 3, removed: 0, placed: 0 });
});

it("a gone record's row goes, and a new one lands in its place", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  const drew = draw(list, cells("a", "d", "c"), ["d"]);
  assert.deepEqual(list.keys(), ["a", "d", "c"]);
  assert.deepEqual(drew, { made: 1, redrawn: 0, kept: 2, removed: 1, placed: 0 });
});

it("a row moved to the front is the one row placed", () => {
  const list = new List();
  draw(list, cells("a", "b", "c", "d"));
  const [a, b, c, d] = list.rows;
  const drew = draw(list, cells("d", "a", "b", "c"), []);
  assert.deepEqual(list.rows, [d, a, b, c]);
  assert.equal(drew.placed, 1);
});

it("a child with no key, drawn before the feed spoke, goes", () => {
  const list = new List();
  list.rows.push(new Row("Loading"));
  const drew = draw(list, cells("a"));
  assert.deepEqual(list.keys(), ["a"]);
  assert.equal(drew.removed, 1);
});

it("a key told twice shows once, where it's first told", () => {
  const list = new List();
  const drew = draw(list, [...cells("a", "b"), { key: "a", works: "again" }]);
  assert.deepEqual(list.keys(), ["a", "b"]);
  assert.equal(list.rows[0]?.words, "on a");
  assert.equal(drew.made, 2);
});

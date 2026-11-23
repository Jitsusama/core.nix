// A list drawn by key: the first drawing makes each row, a moved record's
// row alone is drawn afresh, gone rows go, new ones land in their place, a
// reorder moves each row it passes over, children with no key go, and a
// drawing that throws touches nothing. Where focus goes is keyed-focus's.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { keyed } from "../keyed.ts";
import { cells, draw, List, loose, Row, same } from "./rows.ts";

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
  draw(list, cells("a", "b", "c"));
  // Words arriving after the first drawing, which the next would take out.
  list.words.push({});
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

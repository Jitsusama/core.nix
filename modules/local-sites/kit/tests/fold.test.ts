// What a folded list shows: the first seven, the rows the page keeps
// wherever they rank, how many more opening shows, all of them once open,
// and a button only where there's something to fold.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { type FoldButton, folded, foldSaid, SHOWN, showFold } from "../fold.ts";

const rows = Array.from({ length: 20 }, (_, at) => at);

it("a folded list shows the first seven and counts the rest", () => {
  const fold = folded({ rows, open: false });
  assert.deepEqual(fold.rows, [0, 1, 2, 3, 4, 5, 6]);
  assert.equal(fold.more, 13);
  assert.equal(fold.foldable, true);
  assert.equal(SHOWN, 7);
});

it("a short list shows whole, with nothing to fold, open or shut", () => {
  for (const open of [false, true]) {
    const fold = folded({ rows: [1, 2, 3], open });
    assert.deepEqual(fold.rows, [1, 2, 3], `open: ${String(open)}`);
    assert.equal(fold.more, 0, `open: ${String(open)}`);
    assert.equal(fold.foldable, false, `open: ${String(open)}`);
  }
});

it("a kept row past the seventh shows in its place and takes one of the seven", () => {
  const fold = folded({ rows, open: false, kept: (row) => row === 15 });
  assert.deepEqual(fold.rows, [0, 1, 2, 3, 4, 5, 15]);
  assert.equal(fold.more, 13);
});

it("a kept row among the first seven takes its slot once, not twice", () => {
  const fold = folded({ rows, open: false, kept: (row) => row === 3 });
  assert.deepEqual(fold.rows, [0, 1, 2, 3, 4, 5, 6]);
});

it("more kept rows than the fold shows all show, and nothing else does", () => {
  const fold = folded({ rows, open: false, shown: 2, kept: (row) => row % 5 === 0 });
  assert.deepEqual(fold.rows, [0, 5, 10, 15]);
  assert.equal(fold.more, 16);
});

it("rows counted but never held are said apart, never promised by the fold", () => {
  const shut = folded({ rows, open: false, count: 124 });
  assert.equal(shut.rows.length, 7);
  assert.equal(shut.more, 13);
  assert.equal(shut.unlisted, 104);
  const open = folded({ rows, open: true, count: 124 });
  assert.equal(open.rows.length, 20);
  assert.equal(open.more, 0);
  assert.equal(open.unlisted, 104);
});

it("a count under the rows held is read as the rows, so nothing is unlisted", () => {
  const fold = folded({ rows, open: false, count: 3 });
  assert.equal(fold.more, 13);
  assert.equal(fold.unlisted, 0);
});

it("open, every row held shows and nothing more is promised", () => {
  const fold = folded({ rows, open: true });
  assert.deepEqual(fold.rows, rows);
  assert.equal(fold.more, 0);
  assert.equal(fold.foldable, true);
});

it("the button says how many more, of what kind, and how to fold them again", () => {
  assert.equal(foldSaid(13, false), "And 13 more");
  assert.equal(foldSaid(13, false, "snapshot"), "And 13 more snapshots");
  assert.equal(foldSaid(1, false, "snapshot"), "And 1 more snapshot");
  assert.equal(foldSaid(13, true), "Fold them away");
});

/** A button as `showFold` sets it, with what it was told. */
function button(): FoldButton & { said: Record<string, string> } {
  const said: Record<string, string> = {};
  return {
    type: "submit",
    hidden: false,
    textContent: null,
    said,
    setAttribute: (name, value) => {
      said[name] = value;
    },
  };
}

it("a fold's button shows shut with its count, and open with the way back", () => {
  const shut = button();
  showFold(shut, folded({ rows, open: false }), "row");
  assert.deepEqual(
    [shut.type, shut.hidden, shut.textContent, shut.said["aria-expanded"]],
    ["button", false, "And 13 more rows", "false"],
  );
  const open = button();
  showFold(open, folded({ rows, open: true }));
  assert.deepEqual(
    [open.hidden, open.textContent, open.said["aria-expanded"]],
    [false, "Fold them away", "true"],
  );
});

it("a short list's button stays hidden, even opened by the page's key", () => {
  for (const open of [false, true]) {
    const shown = button();
    showFold(shown, folded({ rows: [1, 2, 3], open }));
    assert.equal(shown.hidden, true, `open: ${String(open)}`);
  }
});

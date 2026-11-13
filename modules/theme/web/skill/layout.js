// The gate's second probe (checks.md), beside gate.js and run the same way:
// whole, as one expression in the page as published, at each width and in
// each look. It answers with where the page's layout doesn't hold
// (SKILL.md's Layout That Holds), each in a line saying what to do, and an
// empty answer passes. It measures rows of things set side by side, then
// opens each fold and closes it again, so run it on a page nobody is
// reading. It ends without a semicolon, since an eval asked for one
// expression answers nothing for a statement.
(async () => {
  const missing = [];
  const page = document;
  const main = page.querySelector("main");
  if (main === null) {
    return ["a <main> holding the page's own part, for its layout to be measured"];
  }
  const shown = (element) => element?.isConnected === true && element.checkVisibility();
  // A box's name is its text as a screen reader hears it, as gate.js reads
  // a heading: text nodes trimmed and joined by a space, aria-hidden left out.
  const named = (element) => {
    const walker = page.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    const words = [];
    while (walker.nextNode()) {
      if (walker.currentNode.parentElement.closest('[aria-hidden="true"]') === null) {
        words.push(walker.currentNode.textContent.trim());
      }
    }
    return words.filter((word) => word !== "").join(" ");
  };
  const quoted = (element) => named(element).slice(0, 40) || element.localName;
  const inline = (element) => getComputedStyle(element).display.startsWith("inline");

  // Things set side by side are a row: two or more of a grid's or a flex
  // line's children, of about one width, each holding words, none over
  // another. A row holds its shape by layout: framed ones end level, the
  // parts of alike ones (state, title, figure, line, fold) sit on one line
  // across the row, and none is squeezed under 256 px, where a phone takes
  // one to a row. A row inside a drawing, a table, the head or a dialog is
  // the drawing's own business. Boxes are measured as laid out, before any
  // transform, so a part still arriving by its motion isn't taken for one
  // the layout moved.
  const laid = (element) => {
    if (!(element instanceof HTMLElement)) {
      const edges = element.getBoundingClientRect();
      return { left: edges.left + scrollX, top: edges.top + scrollY, width: 0, height: 0 };
    }
    let left = 0;
    let top = 0;
    for (let at = element; at !== null; at = at.offsetParent) {
      left += at.offsetLeft;
      top += at.offsetTop;
    }
    return { left, top, width: element.offsetWidth, height: element.offsetHeight };
  };
  const holdsWords = (element) =>
    shown(element) && !inline(element) && element.textContent.trim().length >= 30;
  const unpainted = ["rgba(0, 0, 0, 0)", "transparent"];
  const framed = (element) => {
    const look = getComputedStyle(element);
    return (
      !unpainted.includes(look.backgroundColor) ||
      look.boxShadow !== "none" ||
      [look.borderLeftWidth, look.borderRightWidth, look.borderBottomWidth].some(
        (width) => Number.parseFloat(width) > 0,
      )
    );
  };
  const apart = (cells) => {
    const spans = cells.map(laid).sort((one, other) => one.left - other.left);
    return spans.every(
      (span, at) => at === 0 || span.left >= spans[at - 1].left + spans[at - 1].width - 1,
    );
  };
  const rowsOf = (parent) => {
    const lines = [];
    for (const child of [...parent.children].filter(holdsWords)) {
      const { top } = laid(child);
      const line = lines.find((held) => Math.abs(held.top - top) <= 2);
      if (line === undefined) {
        lines.push({ top, cells: [child] });
      } else {
        line.cells.push(child);
      }
    }
    return lines
      .map((line) => line.cells)
      .filter((cells) => {
        const widths = cells.map((cell) => cell.offsetWidth);
        const alike = Math.max(...widths) <= Math.min(...widths) * 1.15;
        return cells.length > 1 && alike && apart(cells);
      });
  };
  const laysOut = ["grid", "inline-grid", "flex", "inline-flex"];
  const rows = [main, ...main.querySelectorAll("*")]
    .filter(
      (parent) =>
        laysOut.includes(getComputedStyle(parent).display) &&
        parent.closest("svg, table, header, nav, dialog, pre") === null,
    )
    .reduce((all, parent) => all.concat(rowsOf(parent)), []);
  const uneven = new Set();
  for (const cells of rows) {
    const heights = cells.map((cell) => cell.offsetHeight);
    const widest = Math.max(...cells.map((cell) => cell.offsetWidth));
    const shapes = cells.map((cell) => [...cell.children].map((part) => part.localName).join());
    if (cells.some(framed) && Math.max(...heights) - Math.min(...heights) > 1.5) {
      uneven.add(
        `framed things side by side end at different heights ("${quoted(cells[0])}"): ` +
          "level them by layout, a grid row each stretches to, never padding",
      );
    } else if (cells[0].children.length > 1 && shapes.every((shape) => shape === shapes[0])) {
      const astray = [...cells[0].children].find((_part, at) => {
        const tops = cells.map((cell) => laid(cell.children[at]).top);
        return Math.max(...tops) - Math.min(...tops) > 2;
      });
      if (astray !== undefined) {
        const where = named(astray).length > 2 ? astray : cells[0];
        uneven.add(
          `the parts of things side by side don't line up ("${quoted(where)}"): ` +
            "put each part on one line across the row with subgrid",
        );
      }
    }
    if (widest < 256) {
      uneven.add(
        `${cells.length} side by side at ${Math.round(widest)} px each ("${quoted(cells[0])}"),` +
          " squeezed under 256 px: drop a column, one to a row on a phone",
      );
    }
  }
  missing.push(...[...uneven].slice(0, 4));

  // Opening a fold never reflows the page: it grows only itself, downward,
  // or opens over the page in a side sheet. Each closed <details>, and each
  // button that expands what it controls, is opened in turn, every box in
  // <main> measured before and after, and closed again. Above it nothing
  // moves; beside it nothing resizes or moves; below it things only move
  // down; what holds it may grow taller but never wider. It waits for the
  // opening's motion to settle, a second at most.
  const settled = () =>
    Promise.race([
      Promise.all(page.getAnimations().map((motion) => motion.finished.catch(() => null))),
      new Promise((done) => setTimeout(done, 1000)),
    ]).then(() => new Promise((done) => requestAnimationFrame(() => done())));
  const boxes = [...main.querySelectorAll("*")]
    .filter(
      (element) => shown(element) && element.closest("svg, dialog") === null && !inline(element),
    )
    .slice(0, 3000);
  const folds = [
    ...main.querySelectorAll("details:not([open]), button[aria-expanded='false'][aria-controls]"),
  ]
    .filter(shown)
    .slice(0, 24);
  // Three pixels of slack, since a fold closing leaves a pixel or two of its
  // motion behind, which no reader sees.
  const moved = (before, after) =>
    ["left", "top", "width", "height"].filter((side) => Math.abs(before[side] - after[side]) > 3);
  const close = (fold, dialogs) => {
    for (const dialog of page.querySelectorAll("dialog[open]")) {
      if (!dialogs.has(dialog)) {
        dialog.close();
      }
    }
    if (fold.localName === "details") {
      fold.open = false;
    } else if (fold.getAttribute("aria-expanded") === "true") {
      fold.click();
    }
  };
  const trial = async (fold) => {
    const was = new Map(boxes.map((element) => [element, laid(element)]));
    const opened =
      fold.localName === "details" ? fold : page.getElementById(fold.getAttribute("aria-controls"));
    const dialogs = new Set(page.querySelectorAll("dialog[open]"));
    if (fold.localName === "details") {
      fold.open = true;
    } else {
      fold.click();
    }
    await settled();
    const grown = opened === null || !shown(opened) ? null : laid(opened);
    const from = grown?.top;
    // Whether a box stood above the fold, or level with its top beside it.
    // A box level with its top and under it, as the next thing after a
    // panel that was shut to nothing, is below it, free to move down.
    const above = (then) => {
      if (then.top < from - 1) {
        return true;
      }
      const level = then.top < from + 1;
      const under = then.left < grown.left + grown.width && then.left + then.width > grown.left;
      return level && !under;
    };
    // What stood beside it as it opened: begun above its top and running
    // past it, and not holding it. Anything inside one of these that moves
    // is a neighbour reflowing, such as the parts of the next card over.
    const beside = boxes.filter((element) => {
      const then = was.get(element);
      return (
        grown !== null &&
        !element.contains(opened) &&
        !opened.contains(element) &&
        above(then) &&
        then.top + then.height > from + 1
      );
    });
    const culprit = boxes.find((element) => {
      if (grown === null) {
        return false;
      }
      const then = was.get(element);
      const sides = moved(then, laid(element));
      if (element === opened) {
        return sides.some((side) => side !== "height");
      }
      if (fold.contains(element) || opened.contains(element)) {
        return false;
      }
      if (element.contains(opened)) {
        return sides.includes("left") || sides.includes("width");
      }
      // Above it or beside it, nothing changes, though an unframed box may
      // stretch unseen; below it, things only move down, and nothing inside
      // what stood beside it moves at all.
      if (above(then)) {
        return sides.some((side) => side !== "height") || (sides.length > 0 && framed(element));
      }
      const neighbour = beside.some((other) => other.contains(element));
      return sides.some((side) => side !== "top") || (sides.length > 0 && neighbour);
    });
    close(fold, dialogs);
    await settled();
    return culprit === undefined ? null : { culprit, opened };
  };
  // A box that settles late (a part kept unrendered off screen, a font)
  // can move once by itself, so a reflow counts only when a second opening
  // shows it too.
  const reflows = [];
  await folds.reduce(async (before, fold) => {
    await before;
    const seen = (await trial(fold)) && (await trial(fold));
    if (seen !== null) {
      const asks = fold.querySelector(":scope > summary") ?? fold;
      const what =
        seen.culprit === seen.opened ? "widens or moves it" : `moves "${quoted(seen.culprit)}"`;
      reflows.push(
        `opening "${quoted(asks)}" ${what}: let it grow only downward, or open it in a side sheet`,
      );
    }
  }, Promise.resolve());
  missing.push(...[...new Set(reflows)].slice(0, 4));
  return missing;
})()

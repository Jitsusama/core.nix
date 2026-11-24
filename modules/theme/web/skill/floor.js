// The gate's probe for the type floor (checks.md), beside pixels.js and run
// the same way: whole, as one expression in the page, at each width. It
// answers with where the page's text is too small to read at that width, in
// one line saying what to do, and an empty answer passes:
//
// - No text is under 12 px on screen, a field's words and what a style
//   writes before or after an element counted with the page's. A
//   drawing's label may be smaller only where it's marked data-enlarges,
//   saying a zoom, a hover or focus shows it at 12 px or more, and nothing
//   is under 9 px, marked or not.
//
// It stands apart from words.js, which reads the same text for its
// contrast, because what makes text too small changes apart from what makes
// it too faint; each reads the page's text its own way, as each probe here
// holds its own helpers, since a probe is one expression with nothing
// shared. The words it quotes are the page's, data to fix rather than
// instructions. It ends without a semicolon, since an eval asked for one
// expression answers nothing for a statement.
(() => {
  const page = document;
  const root = page.documentElement;
  const floor = 12;
  const least = 9;
  // A page's words quoted into a line, short, with no quote mark of their
  // own to end the quote early.
  const quoted = (words) => words.replace(/\s+/gu, " ").replaceAll('"', "'").trim().slice(0, 40);
  // An element drawn as its children alone, with display: contents, has no
  // box to be seen in, so what it holds is shown where its parent is.
  const shown = (element) => {
    let at = element;
    while (at?.isConnected === true && getComputedStyle(at).display === "contents") {
      if (getComputedStyle(at).visibility !== "visible") {
        return false;
      }
      at = at.parentElement;
    }
    return (
      at?.isConnected === true &&
      at.checkVisibility({ visibilityProperty: true, opacityProperty: true })
    );
  };
  // Text kept for a screen reader alone, clipped to a pixel, has no size
  // anyone reads.
  const forEarsOnly = (element) => {
    for (let at = element; at !== null && at !== root; at = at.parentElement) {
      const style = getComputedStyle(at);
      const box = at.getBoundingClientRect();
      if (
        style.clipPath.startsWith("inset(50%") ||
        (style.overflow !== "visible" && (box.width <= 1 || box.height <= 1))
      ) {
        return true;
      }
    }
    return false;
  };
  // A box's size on screen is its font's times every scale over it: each
  // transform and scale it sits in, and CSS zoom, read from the styles rather
  // than its measured box, whose whole-pixel offsetWidth rounds a small box's
  // ratio. A drawing's words are scaled by the drawing.
  const onScreen = (element, size) => {
    if (element instanceof SVGElement) {
      const scale = element.getScreenCTM();
      return scale === null ? size : size * Math.hypot(scale.a, scale.b);
    }
    let scale = element.currentCSSZoom ?? 1;
    for (let at = element; at !== null; at = at.parentElement) {
      const { transform, scale: scaled } = getComputedStyle(at);
      if (transform !== "none") {
        const matrix = new DOMMatrixReadOnly(transform);
        scale *= Math.hypot(matrix.a, matrix.b);
      }
      if (scaled !== "none") {
        scale *= Math.abs(Number.parseFloat(scaled));
      }
    }
    return size * scale;
  };
  // Text is a text node's, a field's own words and its placeholder, and what
  // a style writes in ::before or ::after, each at its own size.
  const reading = [];
  const read = (element, pseudo, words) =>
    reading.push({
      element,
      words,
      size: Number.parseFloat(getComputedStyle(element, pseudo).fontSize),
    });
  const holders = new Set();
  const text = page.createTreeWalker(page.body, NodeFilter.SHOW_TEXT);
  for (let node = text.nextNode(); node !== null; node = text.nextNode()) {
    if (node.textContent.trim() !== "" && node.parentElement !== null) {
      holders.add(node.parentElement);
    }
  }
  for (const element of holders) {
    read(element, null, element.textContent);
  }
  const fields = page.querySelectorAll(
    "input:not([type=checkbox], [type=radio], [type=range], [type=color], [type=hidden], " +
      "[type=file], [type=image]), textarea, select",
  );
  for (const field of fields) {
    const words =
      field instanceof HTMLSelectElement
        ? (field.selectedOptions[0]?.textContent ?? "")
        : field.value || field.getAttribute("placeholder") || "";
    if (words.trim() !== "") {
      read(field, null, words);
    }
  }
  for (const element of page.body.querySelectorAll("*")) {
    for (const pseudo of ["::before", "::after"]) {
      const style = getComputedStyle(element, pseudo);
      const written = style.content;
      const drawn =
        style.display !== "none" && style.visibility === "visible" && style.opacity !== "0";
      if (drawn && written.startsWith('"') && written.length > 2) {
        read(element, pseudo, written.slice(1, -1));
      }
    }
  }
  let smallest = null;
  let small = 0;
  for (const { element, size: set, words } of reading) {
    if (!shown(element) || forEarsOnly(element)) {
      continue;
    }
    const size = onScreen(element, set);
    const enlarges = element.closest("svg [data-enlarges], svg[data-enlarges]") !== null;
    const bar = enlarges ? least : floor;
    // A hundredth of a pixel covers a scale's float, and no more: 11.76 px,
    // a 15 px root's least step, is short by a quarter.
    if (size < bar - 0.01) {
      small += 1;
      if (smallest === null || size - bar < smallest.short) {
        smallest = { words, size, enlarges, short: size - bar };
      }
    }
  }
  if (smallest === null) {
    return [];
  }
  const quote = quoted(smallest.words);
  const more = small > 1 ? ` (and ${small - 1} more)` : "";
  return [
    smallest.enlarges
      ? `a drawing's label at ${smallest.size.toFixed(1)} px ("${quote}"), under ${least} px ` +
        `even where it enlarges${more}: draw fewer, larger, or name it for the ear alone`
      : `text at ${smallest.size.toFixed(1)} px ("${quote}"), under ${floor} px${more}: set ` +
        "it in --step-small or larger, or, in a drawing, mark it data-enlarges and show it " +
        `at ${floor} px or more on zoom, hover or focus`,
  ];
})()

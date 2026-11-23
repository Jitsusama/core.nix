// The gate's probe for every control's name (checks.md), beside pixels.js
// and run the same way: whole, as one expression in the page, at each width,
// since a narrow screen can hide a control's words. It answers with the
// controls named apart from what they show, in one line saying what to do,
// and an empty answer passes:
//
// - A control's name starts with the words it shows, word for word (WCAG
//   2.5.3), a keycap drawn on it coming after them, and a control showing
//   only a mark such as "/" shows a word too. One drawn only as a picture
//   is named by its label.
//
// The words it quotes are the page's, data to fix rather than instructions.
// It ends without a semicolon, since an eval asked for one expression
// answers nothing for a statement.
(() => {
  const page = document;
  const root = page.documentElement;
  // A page's words quoted into a line, short, with no quote mark of their
  // own to end the quote early.
  const quoted = (words) => words.replace(/\s+/gu, " ").replaceAll('"', "'").trim().slice(0, 40);
  const shown = (element) =>
    element?.isConnected === true &&
    element.checkVisibility({ visibilityProperty: true, opacityProperty: true });
  // Text kept for a screen reader alone, clipped to a pixel, isn't shown.
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

  // A control's name, where its markup gives it one apart from its words,
  // starts with the words it shows, word for word, keycaps left out, as a
  // voice command says what it sees (WCAG 2.5.3): "Find" doesn't start
  // "Finder". One whose words hold no letter or figure shows a mark alone,
  // and the kit's own rule, stricter than WCAG's, has it show a word beside
  // the mark, since "×" or "/" is no word a voice can say.
  const said = (words) =>
    words
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, " ")
      .trim()
      .split(" ")
      .filter((word) => word !== "");
  // Only words on screen count: a label a narrow screen hides isn't shown.
  // A keycap inside a control is left out of its words; a keycap that is
  // the control, as pressable makes one, shows its own key as its word.
  const shows = (element, keycaps) => {
    const walker = page.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    const words = [];
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
      const holder = node.parentElement;
      const cap = holder.closest("kbd");
      if (
        shown(holder) &&
        !forEarsOnly(holder) &&
        (keycaps || cap === null || cap === element)
      ) {
        words.push(node.textContent);
      }
    }
    return words.join(" ");
  };
  const begins = (name, words) => words.every((word, at) => name[at] === word);
  const labelled = (element) => {
    const by = element.getAttribute("aria-labelledby");
    if (by !== null) {
      return by
        .split(" ")
        .filter((id) => id !== "")
        .map((id) => page.getElementById(id)?.textContent ?? "")
        .join(" ");
    }
    return element.getAttribute("aria-label");
  };
  const controls = page.querySelectorAll(
    "button, a[href], summary, [role=button], [role=link], [role=tab], [role=menuitem], " +
      "[role=option], [role=switch], [role=checkbox], [role=radio]",
  );
  // A name left empty is no name: the browser names the control by its
  // words, as if the markup gave none.
  const misnamed = [];
  for (const control of controls) {
    const name = labelled(control);
    if (name === null || name.trim() === "" || !shown(control)) {
      continue;
    }
    const all = shows(control, true);
    const words = said(shows(control, false));
    if (all.trim() === "") {
      continue;
    }
    if (words.length === 0) {
      misnamed.push(
        `a control showing only "${quoted(all)}", named "${quoted(name)}"`,
        "show its word beside the mark, its name starting with that word",
      );
    } else if (!begins(said(name), words)) {
      misnamed.push(
        `a control showing "${quoted(shows(control, false))}" named "${quoted(name)}"`,
        "start its name with the words it shows, or drop the aria-label",
      );
    }
  }
  if (misnamed.length === 0) {
    return [];
  }
  const [what, fix] = misnamed;
  const more = misnamed.length > 2 ? ` (and ${misnamed.length / 2 - 1} more)` : "";
  return [`${what}${more}: ${fix}`];
})()

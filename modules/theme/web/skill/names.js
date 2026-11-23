// The gate's probe for every control's name (checks.md), beside pixels.js
// and run the same way: whole, as one expression in the page, at each width,
// since a narrow screen can hide a control's words. It answers with the
// controls named apart from what they show, in one line saying what to do,
// and an empty answer passes:
//
// - A control's name starts with the words it shows, word for word (WCAG
//   2.5.3), a keycap drawn on it coming after them, and a control showing
//   only a mark such as "/" shows a word too, in it or, for a keycap, after
//   it in its line of keys. One drawn only as a picture is named by its
//   label.
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
  // A keycap the kit makes pressable sits in a line of keys that says what
  // it does, as "? lists every key" does: the words on screen that follow it
  // there, up to the next keycap, are the word beside its mark, so long as
  // one says more than a joining word such as "or". A comment, or words kept
  // for a screen reader, aren't on screen. A button holding only a keycap
  // reads the line it sits in the same way.
  const capped = (control, all) =>
    control.localName === "kbd" ||
    (control.children.length === 1 &&
      control.firstElementChild.localName === "kbd" &&
      control.firstElementChild.textContent.trim() === all.trim());
  const joining = new Set(["a", "an", "and", "or", "the", "then", "to"]);
  const onScreen = (element) => {
    const walker = page.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    const words = [];
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
      if (shown(node.parentElement) && !forEarsOnly(node.parentElement)) {
        words.push(node.textContent);
      }
    }
    return words.join(" ");
  };
  const follows = (control) => {
    const words = [];
    for (let at = control.nextSibling; at !== null; at = at.nextSibling) {
      if (at.nodeType === Node.TEXT_NODE) {
        words.push(at.textContent);
      } else if (at.nodeType === Node.ELEMENT_NODE) {
        if (at.localName === "kbd" || at.querySelector("kbd") !== null) {
          break;
        }
        words.push(onScreen(at));
      }
    }
    const line = said(words.join(" "));
    return line.some((word) => !joining.has(word)) ? line : [];
  };
  const controls = page.querySelectorAll(
    "button, a[href], summary, [role=button], [role=link], [role=tab], [role=menuitem], " +
      "[role=option], [role=switch], [role=checkbox], [role=radio]",
  );
  // A name left empty is no name: the browser names the control by its
  // words, as if the markup gave none, so it starts with them. A mark alone
  // is named however it's named, since dropping its label leaves the mark
  // as its name, unless it's a keycap whose line says what it does, which a
  // name it's given must start with. A control showing nothing but a keycap
  // shows that key as its word, as a keycap that is the control does.
  const misnamed = [];
  for (const control of controls) {
    const name = labelled(control);
    const all = shows(control, true);
    if (!shown(control) || all.trim() === "") {
      continue;
    }
    const own = shows(control, false);
    const words = said(own).length === 0 ? said(all) : said(own);
    const unnamed = name === null || name.trim() === "";
    const line = words.length === 0 && capped(control, all) ? follows(control) : [];
    if (words.length === 0 && line.length === 0) {
      misnamed.push(
        `a control showing only "${quoted(all)}"${unnamed ? "" : `, named "${quoted(name)}"`}`,
        "show its word beside the mark, its name starting with that word",
      );
    } else if (words.length === 0 && !(unnamed || begins(said(name), line))) {
      misnamed.push(
        `a keycap "${quoted(all)}" beside "${quoted(line.join(" "))}" named "${quoted(name)}"`,
        "start its name with the words its line shows beside it, or drop the aria-label",
      );
    } else if (!(unnamed || words.length === 0 || begins(said(name), words))) {
      misnamed.push(
        `a control showing "${quoted(said(own).length === 0 ? all : own)}" named "${quoted(name)}"`,
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

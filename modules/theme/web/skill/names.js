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
  const acting =
    "button, a[href], summary, [role=button], [role=link], [role=tab], [role=menuitem], " +
    "[role=option], [role=switch], [role=checkbox], [role=radio]";
  // A keycap the kit makes pressable sits in a line of keys that says what
  // it does, as "? lists every key" does: the words a person sees that follow
  // it there, up to the next keycap or control, are the word beside its mark,
  // cut at the line's first comma or sentence end past any it opens with, so
  // "opens Now, and" says "opens Now" and ": lists every key" says "lists
  // every key", with no joining word such as "or" left trailing, so long as
  // one says more. A comment, words kept for a screen reader, and words in
  // no ink, at no size, or painted off the page's start or outside a box
  // clipping them aren't seen. A keycap, or a button holding only one,
  // reads its line only in a line of keys, so a mark in a kbd elsewhere is
  // still an icon.
  const capped = (one, all) =>
    one.closest(".keys") !== null &&
    (one.localName === "kbd" ||
      (one.children.length === 1 &&
        one.firstElementChild.localName === "kbd" &&
        one.firstElementChild.textContent.trim() === all.trim()));
  const joining = new Set(["a", "an", "and", "or", "the", "then", "to"]);
  // How opaque an ink is, as the browser computes a colour: an alpha after
  // a slash, or rgba's fourth number, else whole.
  const alpha = (ink) => {
    if (ink === "transparent") {
      return 0;
    }
    if (ink.includes("/")) {
      return Number.parseFloat(ink.slice(ink.lastIndexOf("/") + 1));
    }
    return ink.startsWith("rgba(") ? Number.parseFloat(ink.slice(ink.lastIndexOf(",") + 1)) : 1;
  };
  // A line's words from where its sense starts, past any comma or stop it
  // opens with, up to where it ends, at the next.
  const stops = ",.;:!?";
  const sense = (text) => {
    let from = 0;
    while (from < text.length && (stops.includes(text[from]) || text[from].trim() === "")) {
      from += 1;
    }
    let end = text.length;
    for (const stop of stops) {
      const at = text.indexOf(stop, from);
      end = at >= 0 && at < end ? at : end;
    }
    return text.slice(from, end);
  };
  // A box cut to nothing: a clip-path inset by half or more, or an old clip
  // rectangle with no area, on a box it applies to.
  const cut = (style) => {
    const inset = style.clipPath.startsWith("inset(")
      ? style.clipPath.slice("inset(".length).split(" ")[0].replace(")", "")
      : "";
    if (inset.endsWith("%") && Number.parseFloat(inset) >= 50) {
      return true;
    }
    if (!(style.clip.startsWith("rect(") && ["absolute", "fixed"].includes(style.position))) {
      return false;
    }
    const [top, right, bottom, left] = style.clip
      .slice("rect(".length, -1)
      .split(",")
      .map((side) => Number.parseFloat(side));
    return top >= bottom || left >= right;
  };
  // Where a text's glyphs are painted, its own boxes, not its element's: an
  // indent or a box clipping them can push them out of sight while the box
  // stays. Seen where one lies past the page's start and inside every box
  // over it that clips what overflows, none of which cuts it to nothing.
  const painted = (node) => {
    const range = page.createRange();
    range.selectNodeContents(node);
    const inside = (box) => {
      for (let at = node.parentElement; at !== null && at !== root; at = at.parentElement) {
        const style = getComputedStyle(at);
        const edge = at.getBoundingClientRect();
        const clips = style.overflowX !== "visible" || style.overflowY !== "visible";
        const out =
          box.right <= edge.left ||
          box.left >= edge.right ||
          box.bottom <= edge.top ||
          box.top >= edge.bottom;
        if (cut(style) || (clips && out)) {
          return false;
        }
      }
      return true;
    };
    return Array.from(range.getClientRects()).some(
      (box) =>
        box.width > 0 &&
        box.height > 0 &&
        box.right + window.scrollX > 0 &&
        box.bottom + window.scrollY > 0 &&
        inside(box),
    );
  };
  const seen = (node) => {
    const element = node.parentElement;
    if (element === null || !shown(element) || forEarsOnly(element)) {
      return false;
    }
    const style = getComputedStyle(element);
    return (
      Number.parseFloat(style.fontSize) > 0 &&
      alpha(style.webkitTextFillColor || style.color) > 0 &&
      painted(node)
    );
  };
  const onScreen = (element) => {
    const walker = page.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    const words = [];
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
      if (seen(node)) {
        words.push(node.textContent);
      }
    }
    return words.join(" ");
  };
  const ending = `kbd, ${acting}`;
  const follows = (one) => {
    const words = [];
    for (let at = one.nextSibling; at !== null; at = at.nextSibling) {
      if (at.nodeType === Node.TEXT_NODE) {
        words.push(seen(at) ? at.textContent : "");
      } else if (at.nodeType === Node.ELEMENT_NODE) {
        if (at.matches(ending) || at.querySelector(ending) !== null) {
          break;
        }
        words.push(onScreen(at));
      }
    }
    const line = said(sense(words.join(" ")));
    while (line.length > 0 && joining.has(line[line.length - 1])) {
      line.pop();
    }
    return line.some((word) => !joining.has(word)) ? line : [];
  };
  const controls = page.querySelectorAll(acting);
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

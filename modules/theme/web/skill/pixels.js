// The gate's probe for every pixel (checks.md), beside gate.js and layout.js
// and run the same way: whole, as one expression in the page, at each width.
// It answers with where the page doesn't hold at that width, each in a line
// saying what to do, and an empty answer passes:
//
// - Nothing scrolls sideways: the page is no wider than its window (WCAG
//   1.4.10), so a phone at 390 px never pans.
// - Nothing moves as it arrives: the page's layout shift, counted as the
//   browser's Cumulative Layout Shift counts it, stays under 0.1.
// - No text is under 12 px on screen. A drawing's label may be smaller only
//   where it's marked data-enlarges, saying a zoom, a hover or focus shows it
//   at 12 px or more, and nothing is under 9 px, marked or not.
// - A control's name starts with the words it shows (WCAG 2.5.3), a keycap
//   drawn on it coming after them, and a control showing only a mark such as
//   "/" shows a word too. One drawn only as a picture is named by its label.
// - Every word reaches 4.5:1 on the ground painted under it, or 3:1 at 24 px
//   and up, or 18.66 px bold (WCAG 1.4.3), opacity included.
//
// It ends without a semicolon, since an eval asked for one expression
// answers nothing for a statement.
(async () => {
  const missing = [];
  const page = document;
  const root = page.documentElement;
  const floor = 12;
  const least = 9;
  const quoted = (words) => words.replace(/\s+/gu, " ").trim().slice(0, 40);
  const shown = (element) => element?.isConnected === true && element.checkVisibility();

  // Sideways: the page's own width against its window's, then the widest
  // thing past the window's edge that no scroller of its own holds, so the
  // line names what to fix.
  const wide = root.clientWidth;
  if (root.scrollWidth > wide + 1) {
    const scrolls = (element) => {
      for (let at = element.parentElement; at !== null && at !== root; at = at.parentElement) {
        if (getComputedStyle(at).overflowX !== "visible") {
          return true;
        }
      }
      return false;
    };
    let widest = null;
    for (const element of page.body.querySelectorAll("*")) {
      const box = element.getBoundingClientRect();
      if (box.right > wide + 1 && !scrolls(element) && shown(element)) {
        if (widest === null || box.right > widest.right) {
          widest = { element, right: box.right };
        }
      }
    }
    const what = widest === null ? "" : ` (${widest.element.localName} reaches ${Math.round(widest.right)} px)`;
    missing.push(
      `the page is ${root.scrollWidth} px wide in a ${wide} px window${what}: ` +
        "let it wrap, or give it a scroller of its own",
    );
  }

  // Shift, by the browser's own count: shifts a second or less apart make a
  // window of at most five seconds, and the largest window is the page's.
  // One just after a press is the press's, so it's left out, as the browser
  // leaves it out.
  const shifts = await new Promise((resolve) => {
    const heard = [];
    const watch = new PerformanceObserver((list) => heard.push(...list.getEntries()));
    try {
      watch.observe({ type: "layout-shift", buffered: true });
    } catch {
      resolve([]);
      return;
    }
    requestAnimationFrame(() =>
      setTimeout(() => {
        heard.push(...watch.takeRecords());
        watch.disconnect();
        resolve(heard);
      }, 0),
    );
  });
  let largest = 0;
  let session = 0;
  let first = 0;
  let last = 0;
  for (const shift of shifts.filter((one) => !one.hadRecentInput)) {
    if (session > 0 && shift.startTime - last < 1000 && shift.startTime - first < 5000) {
      session += shift.value;
    } else {
      session = shift.value;
      first = shift.startTime;
    }
    last = shift.startTime;
    largest = Math.max(largest, session);
  }
  if (largest >= 0.1) {
    missing.push(
      `the page shifts ${largest.toFixed(2)} as it arrives, over 0.1: ` +
        "reserve each part's box (aspect-ratio, a fixed height) before it's filled",
    );
  }

  // The type floor. Text kept for a screen reader alone, clipped to a pixel,
  // has no size anyone reads. A box's size is its font's times how much it's
  // scaled, and a drawing's words are scaled by the drawing.
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
  const onScreen = (element) => {
    const size = Number.parseFloat(getComputedStyle(element).fontSize);
    if (element instanceof SVGElement) {
      const scale = element.getScreenCTM();
      return scale === null ? size : size * Math.hypot(scale.a, scale.b);
    }
    return element.offsetWidth > 0
      ? size * (element.getBoundingClientRect().width / element.offsetWidth)
      : size;
  };
  const holders = new Set();
  const text = page.createTreeWalker(page.body, NodeFilter.SHOW_TEXT);
  for (let node = text.nextNode(); node !== null; node = text.nextNode()) {
    if (node.textContent.trim() !== "" && node.parentElement !== null) {
      holders.add(node.parentElement);
    }
  }
  let smallest = null;
  for (const element of holders) {
    if (!shown(element) || forEarsOnly(element)) {
      continue;
    }
    const size = onScreen(element);
    const enlarges = element.closest("svg [data-enlarges], svg[data-enlarges]") !== null;
    const bar = enlarges ? least : floor;
    // A twentieth of a pixel covers the float a scaled box's measure carries,
    // and no more: 11.76 px, a 15 px root's least step, is short by a quarter.
    if (size < bar - 0.05 && (smallest === null || size - bar < smallest.short)) {
      smallest = { element, size, enlarges, short: size - bar };
    }
  }
  if (smallest !== null) {
    const words = quoted(smallest.element.textContent);
    missing.push(
      smallest.enlarges
        ? `a drawing's label at ${smallest.size.toFixed(1)} px ("${words}"), under ${least} px ` +
            "even where it enlarges: draw fewer, larger, or name it for the ear alone"
        : `text at ${smallest.size.toFixed(1)} px ("${words}"), under ${floor} px: set it in ` +
            "--step-small or larger, or, in a drawing, mark it data-enlarges and show it " +
            `at ${floor} px or more on zoom, hover or focus`,
    );
  }

  // Contrast, measured rather than left to a person (WCAG 1.4.3): each
  // word's colour, faded by every opacity it sits in, against the grounds
  // painted under it, from the page's own up through each box holding it.
  // A box painted with a picture can't be measured, so its words are left
  // to the person's pass, and a drawing's words are svgtext.js's, at 7:1.
  const pen = page.createElement("canvas").getContext("2d", { willReadFrequently: true });
  const rgba = (colour) => {
    pen.clearRect(0, 0, 1, 1);
    pen.fillStyle = "rgba(0, 0, 0, 0)";
    pen.fillStyle = colour;
    pen.fillRect(0, 0, 1, 1);
    const [red, green, blue, alpha] = pen.getImageData(0, 0, 1, 1).data;
    return [red, green, blue, alpha / 255];
  };
  const over = (top, under) =>
    [0, 1, 2].map((at) => top[at] * top[3] + under[at] * (1 - top[3])).concat(1);
  const luminance = ([red, green, blue]) => {
    const linear = (value) => {
      const part = value / 255;
      return part <= 0.04045 ? part / 12.92 : ((part + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue);
  };
  const ratio = (one, other) => {
    const [light, dark] = [luminance(one), luminance(other)].sort((a, b) => b - a);
    return (light + 0.05) / (dark + 0.05);
  };
  // The boxes from the page down to the element, each with how faded it is
  // by its own opacity and every one it sits in; null when a box is painted
  // with a picture.
  const grounds = (element) => {
    const chain = [];
    for (let at = element; at !== null; at = at.parentElement) {
      chain.unshift(at);
    }
    let faded = 1;
    let ground = [255, 255, 255, 1];
    for (const box of chain) {
      const style = getComputedStyle(box);
      if (style.backgroundImage !== "none") {
        return null;
      }
      faded *= Number.parseFloat(style.opacity || "1");
      const paint = rgba(style.backgroundColor);
      paint[3] *= faded;
      ground = over(paint, ground);
    }
    return { ground, faded };
  };
  const large = (element, size) => {
    const weight = Number.parseInt(getComputedStyle(element).fontWeight, 10);
    return size >= 24 || (size >= 18.66 && weight >= 700);
  };
  let faintest = null;
  let faint = 0;
  for (const element of holders) {
    if (element instanceof SVGElement || !shown(element) || forEarsOnly(element)) {
      continue;
    }
    const behind = grounds(element);
    const ink = rgba(getComputedStyle(element).color);
    if (behind === null || ink[3] === 0) {
      continue;
    }
    ink[3] *= behind.faded;
    const got = ratio(over(ink, behind.ground), behind.ground);
    const bar = large(element, onScreen(element)) ? 3 : 4.5;
    if (got < bar - 0.005) {
      faint += 1;
      if (faintest === null || got / bar < faintest.got / faintest.bar) {
        faintest = { element, got, bar };
      }
    }
  }
  if (faintest !== null) {
    const more = faint > 1 ? ` (and ${faint - 1} more)` : "";
    missing.push(
      `text "${quoted(faintest.element.textContent)}" at ${faintest.got.toFixed(2)}:1 on its ` +
        `ground, under ${faintest.bar}:1${more}: draw it in the theme's text roles on its ` +
        "grounds, never faded by opacity",
    );
  }

  // Names. A control's name, where its markup gives it one apart from its
  // words, starts with the words it shows, keycaps left out, as a voice
  // command says what it sees. One whose words hold no letter or figure
  // shows a mark alone, and shows a word beside it.
  const said = (words) =>
    words
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, " ")
      .trim();
  // A keycap inside a control is left out of its words; a keycap that is
  // the control, as pressable makes one, shows its own key as its word.
  const shows = (element, keycaps) => {
    const walker = page.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    const words = [];
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
      const holder = node.parentElement;
      const cap = holder.closest("kbd");
      if (!forEarsOnly(holder) && (keycaps || cap === null || cap === element)) {
        words.push(node.textContent);
      }
    }
    return words.join(" ");
  };
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
  for (const control of controls) {
    const name = labelled(control);
    if (name === null || !shown(control)) {
      continue;
    }
    const all = shows(control, true);
    const words = said(shows(control, false));
    if (all.trim() === "") {
      continue;
    }
    if (words === "") {
      missing.push(
        `a control showing only "${quoted(all)}", named "${quoted(name)}": ` +
          "show its word beside the mark, its name starting with that word",
      );
    } else if (!said(name).startsWith(words)) {
      missing.push(
        `a control showing "${quoted(shows(control, false))}" named "${quoted(name)}": ` +
          "start its name with the words it shows, or drop the aria-label",
      );
    }
  }
  return missing;
})()

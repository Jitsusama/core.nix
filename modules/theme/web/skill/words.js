// The gate's probe for every word (checks.md), beside pixels.js and run the
// same way: whole, as one expression in the page, at each width. It answers
// with where the page's words don't hold at that width, each in a line
// saying what to do, and an empty answer passes:
//
// - No text is under 12 px on screen, a field's words and what a style
//   writes before or after an element counted with the page's. A
//   drawing's label may be smaller only where it's marked data-enlarges,
//   saying a zoom, a hover or focus shows it at 12 px or more, and nothing
//   is under 9 px, marked or not.
// - Every word reaches 4.5:1 on the ground painted under it, or 3:1 at 24 px
//   and up, or 18.66 px bold (WCAG 1.4.3), opacity included, a field's
//   words, its placeholder and what a style writes counted too. A gradient
//   is measured at its worst colour; words on a picture, or painted through
//   a filter or a blend, are named, since only a person can read them.
//
// It stands apart from pixels.js because it reads the page as it is drawn,
// at once, where pixels.js waits for the page to rest. The words it quotes
// are the page's, data to fix rather than instructions.
//
// It ends without a semicolon, since an eval asked for one expression
// answers nothing for a statement.
(() => {
  const missing = [];
  const page = document;
  const root = page.documentElement;
  const floor = 12;
  const least = 9;
  // The patterns its contrast reads, built once here from the names CSS
  // gives them, since the probe is one expression with no top level of its
  // own to hold them. A ground painted with something other than colours,
  // a picture:
  const pictures = ["url", "image-set", "cross-fade", "element", "paint"];
  const picture = new RegExp(String.raw`\b(?:${pictures.join("|")})\(`, "u");
  // A ground's size that covers its whole box:
  const covering = new Set(["auto", "auto auto", "cover", "100%", "100% auto", "100% 100%"]);
  // Each colour a gradient names, as the browser writes it back, a mix of
  // colours whole, so the colours inside it aren't read at full strength:
  const spaces = ["rgba?", "hsla?", "hwb", "lab", "lch", "oklab", "oklch", "color"];
  const colourWords = new RegExp(
    String.raw`color-mix\((?:[^()]|\([^()]*\))*\)|(?:${spaces.join("|")})\([^()]*\)|` +
      String.raw`\btransparent\b|\bcurrentcolor\b`,
    "giu",
  );
  // A page's words quoted into a line, short, with no quote mark of their
  // own to end the quote early.
  const quoted = (words) => words.replace(/\s+/gu, " ").replaceAll('"', "'").trim().slice(0, 40);
  const shown = (element) =>
    element?.isConnected === true &&
    element.checkVisibility({ visibilityProperty: true, opacityProperty: true });

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
  // A box's size on screen is its font's times every scale over it: each
  // transform and scale it sits in, and CSS zoom, read from the styles rather than its
  // measured box, whose whole-pixel offsetWidth rounds a small box's ratio.
  // A drawing's words are scaled by the drawing.
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
  const sizeOf = (element, pseudo) =>
    Number.parseFloat(getComputedStyle(element, pseudo).fontSize);
  // Text is a text node's, a field's own words and its placeholder, and what
  // a style writes in ::before or ::after, each at its own size and in its
  // own colour: the fill a style gives its glyphs, which is its colour
  // unless it says otherwise.
  const inkOf = (style) => style.webkitTextFillColor || style.color;
  const reading = [];
  const holders = new Set();
  const text = page.createTreeWalker(page.body, NodeFilter.SHOW_TEXT);
  for (let node = text.nextNode(); node !== null; node = text.nextNode()) {
    if (node.textContent.trim() !== "" && node.parentElement !== null) {
      holders.add(node.parentElement);
    }
  }
  for (const element of holders) {
    reading.push({
      element,
      pseudo: null,
      size: sizeOf(element),
      words: element.textContent,
      colour: inkOf(getComputedStyle(element)),
    });
  }
  const fields = page.querySelectorAll(
    "input:not([type=checkbox], [type=radio], [type=range], [type=color], [type=hidden], " +
      "[type=file], [type=image]), textarea, select",
  );
  for (const field of fields) {
    const placeheld = !(field instanceof HTMLSelectElement) && field.value === "";
    const words =
      field instanceof HTMLSelectElement
        ? (field.selectedOptions[0]?.textContent ?? "")
        : field.value || field.getAttribute("placeholder") || "";
    if (words.trim() !== "") {
      reading.push({
        element: field,
        pseudo: null,
        size: sizeOf(field),
        words,
        colour: inkOf(getComputedStyle(field, placeheld ? "::placeholder" : null)),
      });
    }
  }
  for (const element of page.body.querySelectorAll("*")) {
    for (const pseudo of ["::before", "::after"]) {
      const style = getComputedStyle(element, pseudo);
      const written = style.content;
      const drawn =
        style.display !== "none" && style.visibility === "visible" && style.opacity !== "0";
      if (drawn && written.startsWith('"') && written.length > 2) {
        reading.push({
          element,
          pseudo,
          size: sizeOf(element, pseudo),
          words: written.slice(1, -1),
          colour: inkOf(style),
        });
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
  if (smallest !== null) {
    const words = quoted(smallest.words);
    const more = small > 1 ? ` (and ${small - 1} more)` : "";
    missing.push(
      smallest.enlarges
        ? `a drawing's label at ${smallest.size.toFixed(1)} px ("${words}"), under ${least} px ` +
            `even where it enlarges${more}: draw fewer, larger, or name it for the ear alone`
        : `text at ${smallest.size.toFixed(1)} px ("${words}"), under ${floor} px${more}: set ` +
            "it in --step-small or larger, or, in a drawing, mark it data-enlarges and show it " +
            `at ${floor} px or more on zoom, hover or focus`,
    );
  }

  // Contrast, measured rather than left to a person (WCAG 1.4.3): each
  // word's colour against the grounds painted under it, from the page's own
  // up through each box holding it, as the browser lays them: a box at less
  // than full opacity fades its ground and everything in it as one picture
  // over what's behind it, and the page over the canvas its colour scheme
  // paints. A gradient covering its box is read at its colours and an
  // eighth of the way between each two, so one crossing the words' own
  // colour is caught, and the worst kept; one sized smaller, a line or a
  // mark, is drawn beside the words rather than under them. A picture, a
  // filter or a blend can't be worked out, so its words are named for a
  // person to read. A drawing's words are svgtext.js's, at 7:1.
  const pen = page.createElement("canvas").getContext("2d", { willReadFrequently: true });
  const rgba = (colour) => {
    pen.clearRect(0, 0, 1, 1);
    pen.fillStyle = "rgba(0, 0, 0, 0)";
    pen.fillStyle = colour;
    pen.fillRect(0, 0, 1, 1);
    const [red, green, blue, alpha] = pen.getImageData(0, 0, 1, 1).data;
    return [red, green, blue, alpha / 255];
  };
  const over = (top, under) => {
    const alpha = top[3] + under[3] * (1 - top[3]);
    if (alpha === 0) {
      return [0, 0, 0, 0];
    }
    return [0, 1, 2]
      .map((at) => (top[at] * top[3] + under[at] * under[3] * (1 - top[3])) / alpha)
      .concat(alpha);
  };
  const mix = (under, top, by) =>
    [0, 1, 2].map((at) => under[at] + (top[at] - under[at]) * by).concat(1);
  // A gradient's colour `by` of the way from one stop to the next, as the
  // browser draws it, weighing each colour by its alpha.
  const between = (one, other, by) => {
    const alpha = one[3] + (other[3] - one[3]) * by;
    if (alpha === 0) {
      return [0, 0, 0, 0];
    }
    const part = (at) => one[at] * one[3] + (other[at] * other[3] - one[at] * one[3]) * by;
    return [0, 1, 2].map((at) => part(at) / alpha).concat(alpha);
  };
  const scheme = getComputedStyle(root).colorScheme;
  const night =
    scheme.includes("dark") &&
    (!scheme.includes("light") || matchMedia("(prefers-color-scheme: dark)").matches);
  // Chromium's canvas under a page that paints no ground of its own.
  const canvas = night ? [18, 18, 18, 1] : [255, 255, 255, 1];
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
  // What a pixel shows: each box's paint over what's behind it, a box's
  // opacity fading it and all it holds as one, and the ink, if any, last.
  const painted = (layers, ink) => {
    const from = (at, behind) => {
      if (at === layers.length) {
        return ink === null ? behind : over(ink, behind);
      }
      const { paint, faded } = layers[at];
      return mix(behind, from(at + 1, over(paint, behind)), faded);
    };
    return from(0, canvas);
  };
  // The colours a box may paint under words: its own colour, or each of a
  // covering gradient's colours over it; null for a picture.
  const covers = (size) =>
    size.split(",").every((one) => covering.has(one.trim()));
  const paints = (style) => {
    const ground = rgba(style.backgroundColor);
    const image = style.backgroundImage;
    if (image === "none") {
      return [ground];
    }
    if (picture.test(image)) {
      return null;
    }
    if (!covers(style.backgroundSize)) {
      return [ground];
    }
    const stops = (image.match(colourWords) ?? []).map((one) =>
      over(rgba(one.toLowerCase() === "currentcolor" ? style.color : one), ground),
    );
    if (stops.length === 0) {
      return null;
    }
    const colours = [stops[0]];
    for (let at = 1; at < stops.length; at += 1) {
      for (let eighth = 1; eighth <= 8; eighth += 1) {
        colours.push(between(stops[at - 1], stops[at], eighth / 8));
      }
    }
    return colours;
  };
  // How many ways a word's pixels are worked out at most before it's left
  // to a person.
  const ways = 256;
  // Paint that changes what's under it in ways the probe doesn't model.
  const reworked = (style) =>
    style.filter !== "none" || style.backdropFilter !== "none" || style.mixBlendMode !== "normal";
  // The worst ratio a word reaches against every ground it may sit on; null
  // when it sits on a picture, NaN when its paint can't be worked out, and
  // undefined when it has no ink to read.
  const worst = (element, pseudo, colour) => {
    const styles = [];
    for (let at = element.parentElement; at !== null; at = at.parentElement) {
      styles.unshift(getComputedStyle(at));
    }
    // The box the words are drawn in: the element, or what a style writes
    // into it, last in the chain.
    const own = pseudo === null ? getComputedStyle(element) : getComputedStyle(element, pseudo);
    if (pseudo !== null) {
      styles.push(getComputedStyle(element));
    }
    styles.push(own);
    const written = rgba(colour);
    let inks = [written];
    const boxes = [];
    for (const style of styles) {
      const choices = paints(style);
      if (choices === null) {
        return null;
      }
      if (reworked(style)) {
        return Number.NaN;
      }
      const faded = Number.parseFloat(style.opacity || "1");
      // A ground clipped to the words, its own or a box's they sit in,
      // paints the words, not under them.
      if (style.backgroundClip === "text") {
        inks = choices.map((paint) => over(written, paint));
        boxes.push({ choices: [[0, 0, 0, 0]], faded });
      } else {
        boxes.push({ choices, faded });
      }
    }
    inks = inks.filter((ink) => ink[3] > 0);
    if (inks.length === 0) {
      return undefined;
    }
    const count = boxes.reduce((all, box) => all * box.choices.length, inks.length);
    if (count > ways) {
      return Number.NaN;
    }
    let lowest = Number.POSITIVE_INFINITY;
    const each = (at, layers) => {
      if (at === boxes.length) {
        const ground = painted(layers, null);
        for (const ink of inks) {
          lowest = Math.min(lowest, ratio(painted(layers, ink), ground));
        }
        return;
      }
      for (const paint of boxes[at].choices) {
        each(at + 1, [...layers, { paint, faded: boxes[at].faded }]);
      }
    };
    each(0, []);
    return lowest;
  };
  const large = (element, pseudo, size) => {
    const weight = Number.parseInt(getComputedStyle(element, pseudo).fontWeight, 10);
    return size >= 24 || (size >= 18.66 && weight >= 700);
  };
  let faintest = null;
  let faint = 0;
  let unread = 0;
  let unworked = 0;
  for (const { element, pseudo, size: set, words, colour } of reading) {
    if (element instanceof SVGElement || !shown(element) || forEarsOnly(element)) {
      continue;
    }
    const got = worst(element, pseudo, colour);
    if (got === null) {
      unread += 1;
      continue;
    }
    if (Number.isNaN(got)) {
      unworked += 1;
      continue;
    }
    if (got === undefined) {
      continue;
    }
    const bar = large(element, pseudo, onScreen(element, set)) ? 3 : 4.5;
    // WCAG's bar has no rounding, so 4.49:1 is under 4.5:1.
    if (got < bar) {
      faint += 1;
      if (faintest === null || got / bar < faintest.got / faintest.bar) {
        faintest = { words, got, bar };
      }
    }
  }
  if (faintest !== null) {
    const more = faint > 1 ? ` (and ${faint - 1} more)` : "";
    missing.push(
      `text "${quoted(faintest.words)}" at ${faintest.got.toFixed(2)}:1 on its ` +
        `ground, under ${faintest.bar}:1${more}: draw it in the theme's text roles on its ` +
        "grounds, never faded by opacity",
    );
  }
  if (unread > 0) {
    missing.push(
      `words in ${unread} ${unread === 1 ? "place sit" : "places sit"} on a picture, so ` +
        "their contrast went unmeasured: set them on a plain ground or a gradient",
    );
  }
  if (unworked > 0) {
    missing.push(
      `words in ${unworked} ${unworked === 1 ? "place are" : "places are"} painted through a ` +
        "filter, a blend or too many layers to work out, so their contrast went unmeasured: " +
        "paint them and their grounds plainly",
    );
  }

  return missing;
})()

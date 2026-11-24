// The gate's probe for every word's contrast (checks.md), beside pixels.js
// and floor.js and run the same way: whole, as one expression in the page,
// at each width. It answers with where the page's words don't hold at that
// width, each in a line saying what to do, and an empty answer passes:
//
// - Every word reaches 4.5:1 on the ground painted under it, or 3:1 at 24 px
//   and up, or 18.66 px bold (WCAG 1.4.3), opacity included, a field's
//   words, its placeholder and what a style writes counted too. A gradient
//   is measured at its worst colour, layer by layer; words on a picture, or
//   painted through a filter, a blend, a mask, a stroke or only a shadow,
//   are named, since only a person can read them.
//
// It stands apart from pixels.js because it reads the page as it is drawn,
// at once, where pixels.js waits for it to rest, and from floor.js, which
// reads the same text for its size, because what makes text too faint
// changes apart from what makes it too small. The words it quotes are the
// page's, data to fix rather than instructions. It ends without a semicolon,
// since an eval asked for one expression answers nothing for a statement.
(() => {
  const missing = [];
  const page = document;
  const root = page.documentElement;
  // The patterns its contrast reads, built once here from the names CSS
  // gives them, since the probe is one expression with no top level of its
  // own to hold them. A ground painted with something other than colours,
  // a picture:
  const pictures = ["url", "image-set", "cross-fade", "element", "paint"];
  const picture = new RegExp(String.raw`\b(?:${pictures.join("|")})\(`, "u");
  // A ground's layer repeated so it tiles its whole box, on each axis, which
  // space doesn't: it leaves gaps between its tiles, though tiles of a pixel
  // or two may leave none. And the most a line untiled on an axis may be
  // across to lie beside words, never under them, in px: a hairline.
  const tiles = new Set(["repeat", "round"]);
  const hairline = 2;
  const sides = { "repeat-x": "repeat no-repeat", "repeat-y": "no-repeat repeat" };
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
  // Text kept for a screen reader alone, clipped to a pixel, is read by
  // no one's eyes, so its contrast is no one's either.
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
  // A box's size on screen, which sets whether its words are large, is its
  // font's times every scale over it: each transform and scale it sits in,
  // and CSS zoom, read from the styles rather than its measured box, whose
  // whole-pixel offsetWidth rounds a small box's ratio. A drawing's words
  // are scaled by the drawing.
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
  // a style writes in ::before or ::after, each at its own size and in the
  // fill a style gives its glyphs, which is its colour unless it says else.
  const inkOf = (style) => style.webkitTextFillColor || style.color;
  const reading = [];
  const read = (element, pseudo, words, style) =>
    reading.push({ element, pseudo, size: sizeOf(element, pseudo), words, colour: inkOf(style) });
  const holders = new Set();
  const text = page.createTreeWalker(page.body, NodeFilter.SHOW_TEXT);
  for (let node = text.nextNode(); node !== null; node = text.nextNode()) {
    if (node.textContent.trim() !== "" && node.parentElement !== null) {
      holders.add(node.parentElement);
    }
  }
  for (const element of holders) {
    read(element, null, element.textContent, getComputedStyle(element));
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
      read(field, null, words, getComputedStyle(field, placeheld ? "::placeholder" : null));
    }
  }
  for (const element of page.body.querySelectorAll("*")) {
    for (const pseudo of ["::before", "::after"]) {
      const style = getComputedStyle(element, pseudo);
      const written = style.content;
      const drawn =
        style.display !== "none" && style.visibility === "visible" && style.opacity !== "0";
      if (drawn && written.startsWith('"') && written.length > 2) {
        read(element, pseudo, written.slice(1, -1), style);
      }
    }
  }

  // Contrast, measured rather than left to a person (WCAG 1.4.3): each
  // word's colour against the grounds painted under it, from the page's own
  // up through each box holding it, as the browser lays them: a box at less
  // than full opacity fades its ground and everything in it as one picture
  // over what's behind it, and the page over the canvas its colour scheme
  // paints, and the worst kept. A picture, and paint the probe can't work
  // out, have their words named for a person to read. A drawing's words
  // are svgtext.js's, at 7:1.
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
  // The page's scheme, from its style or, where that says nothing, from the
  // head's meta tag, which sets the canvas as well: the first named so in
  // any case, as HTML matches a meta's name, that says a scheme.
  const declared = getComputedStyle(root).colorScheme;
  const metas = Array.from(page.head?.querySelectorAll('meta[name="color-scheme" i]') ?? []);
  const schemes = ["light", "dark", "normal"];
  const said = metas
    .map((meta) => meta.content.toLowerCase())
    .find((content) => schemes.some((word) => content.includes(word)));
  const scheme = declared === "normal" ? (said ?? "normal") : declared;
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
  // How many ways a word's pixels are worked out at most before it's left
  // to a person.
  const ways = 256;
  // A list the browser writes back with a part per layer, split at the
  // commas between them, not those inside a colour.
  const layered = (list) => {
    const parts = [];
    let depth = 0;
    let from = 0;
    for (let at = 0; at < list.length; at += 1) {
      depth += list[at] === "(" ? 1 : 0;
      depth -= list[at] === ")" ? 1 : 0;
      if (list[at] === "," && depth === 0) {
        parts.push(list.slice(from, at).trim());
        from = at + 1;
      }
    }
    parts.push(list.slice(from).trim());
    return parts;
  };
  // How a layer, as its size, repeat and place across and down, lies by the
  // words of a box: beside them where it's untiled and a hairline on an axis;
  // under all of them where each axis is tiled, or sized to the box or past
  // it, leaving no strip bare; else maybe, a band or a mark keeping what's
  // under it. A gradient has no size, so contain is cover; calc() is maybe.
  // A box broken across lines has no length a px size is said to reach.
  const along = (repeat, size, place, length) => {
    const [reach, from] = [Number.parseFloat(size), Number.parseFloat(place)];
    const reaches = size.endsWith("%") ? reach >= 100 : size.endsWith("px") && reach >= length;
    const placed = place.endsWith("%") ? from >= 0 && from <= 100 : place === "0px";
    return tiles.has(repeat) || ((size === "auto" || reaches) && placed);
  };
  const lies = ([size, repeat, x, y], box) => {
    const [across, down = across] = (sides[repeat] ?? repeat).split(" ");
    const whole = size === "cover" || size === "contain";
    const [wide, tall = "auto"] = whole ? ["100%", "100%"] : size.split(" ");
    const thin = (axis, part) =>
      !tiles.has(axis) &&
      axis !== "space" &&
      part.endsWith("px") &&
      Number.parseFloat(part) <= hairline;
    const under = along(across, wide, x, box.width) && along(down, tall, y, box.height);
    if (thin(across, wide) || thin(down, tall)) {
      return "beside";
    }
    return under ? "under" : "maybe";
  };
  const placing = ["Size", "Repeat", "PositionX", "PositionY"].map((part) => `background${part}`);
  // Where a stop sits, as the percentages or pixels written after its
  // colour, up to the comma or bracket ending it, each kept with its unit.
  const placesOf = (after) => {
    const ends = [after.indexOf(","), after.indexOf(")")].filter((at) => at >= 0);
    return after
      .slice(0, ends.length === 0 ? after.length : Math.min(...ends))
      .trim()
      .split(" ")
      .filter((word) => word.endsWith("%") || word.endsWith("px"))
      .map((word) => ({ at: Number.parseFloat(word), unit: word.endsWith("%") ? "%" : "px" }));
  };
  // A gradient's colours: each stop, and eighths between each pair it
  // blends, but not across a hard stop, which starts where the last ends,
  // in the same unit, or at 0 or before in any, as #fff 50%, #000 0 writes
  // a split, or is placed nowhere after one at 100%.
  const gradient = (layer, style) => {
    const stops = [];
    for (const found of layer.matchAll(colourWords)) {
      const word = found[0];
      const places = placesOf(layer.slice(found.index + word.length));
      stops.push({
        colour: rgba(word.toLowerCase() === "currentcolor" ? style.color : word),
        first: places[0],
        last: places[places.length - 1],
      });
    }
    if (stops.length === 0) {
      return null;
    }
    const colours = [stops[0].colour];
    for (let at = 1; at < stops.length; at += 1) {
      const [before, after] = [stops[at - 1], stops[at]];
      const hard =
        before.last !== undefined &&
        (after.first === undefined
          ? before.last.unit === "%" && before.last.at >= 100
          : after.first.at <= 0 ||
            (after.first.unit === before.last.unit && after.first.at <= before.last.at));
      for (let eighth = hard ? 8 : 1; eighth <= 8; eighth += 1) {
        colours.push(between(before.colour, after.colour, eighth / 8));
      }
    }
    return colours;
  };
  // The colours a box may paint under words: its own colour with each layer
  // over it, the last listed lowest, a layer under some words maybe keeping
  // the colours under it beside its own. Null for a picture.
  const paints = (style, box) => {
    const image = style.backgroundImage;
    let colours = [rgba(style.backgroundColor)];
    if (picture.test(image)) {
      return null;
    }
    const layers = layered(image);
    const lists = placing.map((name) => layered(style[name]));
    for (let at = layers.length - 1; at >= 0 && colours.length <= ways; at -= 1) {
      const layer = lists.map((list) => list[at % list.length]);
      const lying = layers[at] === "none" ? "beside" : lies(layer, box);
      if (lying === "beside") {
        continue;
      }
      const own = gradient(layers[at], style);
      if (own === null) {
        return null;
      }
      const under = colours;
      colours = lying === "under" ? [] : [...under];
      for (const below of under) {
        for (const paint of own) {
          colours.push(over(paint, below));
        }
      }
    }
    return colours;
  };
  // Paint that changes what's under it in ways the probe doesn't model.
  const reworked = (style) =>
    style.filter !== "none" ||
    style.backdropFilter !== "none" ||
    style.mixBlendMode !== "normal" ||
    layered(style.backgroundBlendMode).some((mode) => mode !== "normal") ||
    [style.maskImage, style.webkitMaskImage].some(
      (mask) => mask !== undefined && mask !== "" && mask !== "none",
    );
  // The worst ratio a word reaches against every ground it may sit on; null
  // when it sits on a picture, NaN when its paint can't be worked out, and
  // undefined when it has no ink to read.
  const worst = (element, pseudo, colour) => {
    // Each box from the page down, sized as its layers are laid in, its own
    // size before any transform, for whether a layer spans it.
    const laid = (at) =>
      at.getClientRects().length > 1
        ? { width: Number.NaN, height: Number.NaN }
        : { width: at.offsetWidth, height: at.offsetHeight };
    const styles = [];
    for (let at = element.parentElement; at !== null; at = at.parentElement) {
      styles.unshift({ style: getComputedStyle(at), box: laid(at) });
    }
    // The box the words are drawn in, last: the element, or what a style
    // writes into it, measured as the element, since it has no box of its own.
    const own = pseudo === null ? getComputedStyle(element) : getComputedStyle(element, pseudo);
    const drawn = laid(element);
    if (pseudo !== null) {
      styles.push({ style: getComputedStyle(element), box: drawn });
    }
    styles.push({ style: own, box: drawn });
    // A stroke draws the glyphs' edges in a colour of its own.
    if (Number.parseFloat(own.webkitTextStrokeWidth || "0") > 0) {
      return Number.NaN;
    }
    const written = rgba(colour);
    let inks = [written];
    const boxes = [];
    for (const { style, box: size } of styles) {
      // A box drawn as its children alone paints nothing of its own.
      if (style.display === "contents") {
        continue;
      }
      const choices = paints(style, size);
      if (choices === null) {
        return null;
      }
      if (reworked(style)) {
        return Number.NaN;
      }
      const faded = Number.parseFloat(style.opacity || "1");
      // A ground clipped to the words, its own or a box's they sit in,
      // paints the words, not under them; some layers clipped so and some
      // not is paint the probe doesn't model.
      const clips = layered(style.backgroundClip);
      const clipped = clips.includes("text");
      if (clipped && clips.some((clip) => clip !== "text")) {
        return Number.NaN;
      }
      if (clipped) {
        inks = choices.map((paint) => over(written, paint));
      }
      boxes.push({ choices: clipped ? [[0, 0, 0, 0]] : choices, faded });
    }
    inks = inks.filter((ink) => ink[3] > 0);
    // Words with no ink of their own may still be drawn by their shadow.
    if (inks.length === 0) {
      return own.textShadow === "none" ? undefined : Number.NaN;
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
    // Words on a picture, or in paint not worked out, are counted to name;
    // words with no ink aren't read at all.
    const got = worst(element, pseudo, colour);
    unread += got === null ? 1 : 0;
    unworked += Number.isNaN(got) ? 1 : 0;
    if (typeof got !== "number" || Number.isNaN(got)) {
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
      `words in ${unworked} ${unworked === 1 ? "place are" : "places are"} drawn through a ` +
        "filter, blend, mask, stroke, shadow alone or too many layers to work out, so their " +
        "contrast went unmeasured: paint them and their grounds plainly",
    );
  }
  return missing;
})()

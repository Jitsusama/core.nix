// The gate's probe for a drawing's words (checks.md), beside gate.js,
// layout.js and map.js and run the same way: whole, as one expression in the
// page as published. It measures every word drawn in an inline SVG against
// what is painted behind it, the shapes beneath it and the page under them,
// and answers with each that falls below 7:1 (WCAG AAA), each in a line
// saying what to do. An empty answer passes. It ends without a semicolon,
// since an eval asked for one expression answers nothing for a statement.
//
// It measures what is painted, not what the stylesheet means: a fill of
// var(--name, fallback) whose name the kit doesn't define paints its
// fallback, which is how a block meant to take the page's colour stayed
// light paper in the dark look, under light words, at 1.45:1 (2026-11-15).
// So run it in both looks. A drawing kept in an img, object or embed can't
// be read from the page, so one wider than 64 px is a fault too, rather than
// a silent pass: draw it inline.
(() => {
  const least = 7;
  const faults = [];
  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  const pen = canvas.getContext("2d", { willReadFrequently: true });
  // Any CSS colour, as the browser paints it: red, green, blue and alpha.
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
  // How opaque an element is with every element it sits in.
  const opacity = (element) => {
    let seen = 1;
    for (let at = element; at && at !== document.documentElement; at = at.parentElement) {
      seen *= Number.parseFloat(getComputedStyle(at).opacity || "1");
    }
    return seen;
  };
  const shapes = new Set(["rect", "circle", "ellipse", "path", "polygon", "polyline"]);
  // What an element paints behind a word: a shape's fill, or a box's
  // background; none when it paints nothing there.
  const paint = (element) => {
    const style = getComputedStyle(element);
    if (element instanceof SVGElement) {
      if (!shapes.has(element.localName) || style.fill === "none") {
        return null;
      }
      const colour = rgba(style.fill);
      colour[3] *= Number.parseFloat(style.fillOpacity || "1") * opacity(element);
      return colour[3] > 0 ? colour : null;
    }
    const colour = rgba(style.backgroundColor);
    return colour[3] > 0 ? colour : null;
  };
  const page = () => {
    for (const element of [document.body, document.documentElement]) {
      const colour = rgba(getComputedStyle(element).backgroundColor);
      if (colour[3] > 0) {
        return colour;
      }
    }
    return [255, 255, 255, 1];
  };
  // A shape a drawing says takes no pointer is still painted, so every
  // element answers elementsFromPoint while it's measured.
  const reach = document.createElement("style");
  reach.textContent = "* { pointer-events: auto !important; }";
  document.head.append(reach);
  const words = [...document.querySelectorAll("svg text")].filter((text) => {
    const box = text.getBoundingClientRect();
    return text.textContent.trim() && box.width > 0 && box.height > 0;
  });
  for (const text of words) {
    text.scrollIntoView({ block: "center", inline: "center" });
    const box = text.getBoundingClientRect();
    const beneath = document
      .elementsFromPoint(box.left + box.width / 2, box.top + box.height / 2)
      .filter((element) => element !== text && !text.contains(element));
    const layers = [];
    for (const element of beneath) {
      const colour = paint(element);
      if (!colour) {
        continue;
      }
      layers.push(colour);
      if (colour[3] >= 1) {
        break;
      }
    }
    let ground = page();
    for (const colour of layers.reverse()) {
      ground = over(colour, ground);
    }
    const style = getComputedStyle(text);
    const ink = rgba(style.fill);
    ink[3] *= Number.parseFloat(style.fillOpacity || "1") * opacity(text);
    const got = ratio(over(ink, ground), ground);
    if (got < least) {
      const said = text.textContent.trim().slice(0, 30);
      const behind = `rgb(${ground.slice(0, 3).map(Math.round).join(", ")})`;
      faults.push(
        `the drawn words "${said}" stand at ${got.toFixed(2)}:1 on ${behind}, under ${least}:1: ` +
          "fill the shape behind them with a colour the kit defines in both looks " +
          "(var(--background), not an undefined name falling back), or darken the words",
      );
    }
  }
  reach.remove();
  for (const element of document.querySelectorAll("img, object, embed")) {
    const source = element.getAttribute("src") || element.getAttribute("data") || "";
    const drawn = new URL(source, document.baseURI).pathname.toLowerCase().endsWith(".svg");
    if (drawn && element.getBoundingClientRect().width > 64) {
      faults.push(
        `the drawing ${source} sits in an ${element.localName}, where its words can't be ` +
          "measured: put the SVG in the page inline",
      );
    }
  }
  window.scrollTo(0, 0);
  return faults;
})()

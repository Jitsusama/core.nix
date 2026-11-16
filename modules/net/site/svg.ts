// The map's marks in SVG: an element from its attributes, a line of words,
// the diamond set before a figure worth a look, and a name cut to its room.

const SVG = "http://www.w3.org/2000/svg";

/** An element's attributes; one with no value is left off. */
interface Attributes {
  readonly [name: string]: string | number | null | undefined;
}

/** The look mark's half-height, and its height in all. */
const MARK = { half: 4.5, tall: 9 } as const;

function el<K extends keyof SVGElementTagNameMap>(
  tag: K,
  attributes: Attributes = {},
  parent: Element | null = null,
): SVGElementTagNameMap[K] {
  const made = document.createElementNS(SVG, tag);
  for (const [name, value] of Object.entries(attributes)) {
    if (value !== null && value !== undefined) {
      made.setAttribute(name, String(value));
    }
  }
  parent?.append(made);
  return made;
}

/** Where a line of words sits, and how it's set. */
interface Placed {
  readonly x: number;
  readonly y: number;
  readonly kind: string;
  readonly anchor?: string;
}

/** A line of words, which the map hides whole where its edge would cut it. */
function text(parent: Element, words: string, placed: Placed): SVGTextElement {
  const { x, y, kind, anchor } = placed;
  const made = el("text", { x, y, class: kind, "text-anchor": anchor, "data-name": "" }, parent);
  made.textContent = words;
  return made;
}

/**
 * A diamond in gold, set before a figure worth a look, so gold is never the
 * only thing telling it apart.
 */
function lookMark(parent: Element, x: number, y: number): void {
  const { half, tall } = MARK;
  const d = `M${x} ${y - tall}l${half} ${half}-${half} ${half}-${half}-${half}z`;
  el("path", { d, class: "n-look-mark" }, parent);
}

/**
 * Cuts a drawn name to the room its column gives, measured in the face it
 * is set in, and keeps the whole name as its tooltip and in the panel.
 */
function fitText(drawn: SVGTextElement, words: string, room: number): void {
  if (drawn.getComputedTextLength() <= room) {
    return;
  }
  let n = words.length;
  while (n > 1 && drawn.getComputedTextLength() > room) {
    n -= 1;
    drawn.textContent = `${words.slice(0, n).trimEnd()}...`;
  }
  el("title", {}, drawn).textContent = words;
}

export type { Attributes, Placed };
export { el, fitText, lookMark, MARK, text };

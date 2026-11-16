// The panel's building blocks: elements made from a tag, attributes and
// children, the diamond mark, and facts and tables of figures.

import { el } from "./svg.ts";

/** A child the panel's elements take: an element, words, or nothing. */
type Child = Node | string | number | null | undefined | false | readonly Child[];

/** What an element is given: attributes, and listeners under `on`. */
interface Given {
  readonly on?: { readonly [event: string]: (event: Event) => void };
  readonly [attribute: string]: unknown;
}

function h(tag: string, given: Given = {}, ...kids: Child[]): HTMLElement {
  const made = document.createElement(tag);
  for (const [name, value] of Object.entries(given)) {
    if (name === "on" && typeof value === "object" && value !== null) {
      for (const [event, heard] of Object.entries(value)) {
        made.addEventListener(event, heard as EventListener);
      }
    } else if (value !== null && value !== undefined && value !== false) {
      made.setAttribute(name, String(value));
    }
  }
  const add = (kid: Child): void => {
    if (Array.isArray(kid)) {
      for (const each of kid as readonly Child[]) {
        add(each);
      }
    } else if (kid !== null && kid !== undefined && kid !== false) {
      made.append(typeof kid === "number" ? String(kid) : (kid as Node | string));
    }
  };
  for (const kid of kids) {
    add(kid);
  }
  return made;
}

/** The diamond: gold where something is worth a look, plain otherwise. */
function glyph(look: boolean): SVGSVGElement {
  const mark = el("svg", {
    viewBox: "0 0 12 12",
    class: `glyph ${look ? "look" : "calm"}`,
    "aria-hidden": "true",
  });
  el("path", { d: "M6 1.5l4.5 4.5-4.5 4.5-4.5-4.5z" }, mark);
  return mark;
}

/** One fact: its name, its value, and whether it's set in mono. */
type Fact = readonly [string, Child, boolean?];

function facts(pairs: readonly Fact[]): HTMLElement {
  const shown = pairs.filter(([, value]) => value !== null && value !== undefined && value !== "");
  return h(
    "dl",
    { class: "facts" },
    shown.map(([name, value, mono]) =>
      h("div", {}, h("dt", {}, name), h("dd", { class: mono === true ? "mono" : null }, value)),
    ),
  );
}

/** A table's cell: words, a figure, or one worth a look. */
type Cell = string | number | { readonly text: string; readonly look: true };

function table(
  head: readonly string[],
  rows: readonly (readonly Cell[])[],
  nums: readonly number[] = [],
): HTMLElement {
  const kind = (i: number, cell?: Cell): string | null =>
    [nums.includes(i) ? "num" : "", typeof cell === "object" ? "look" : ""].join(" ").trim() ||
    null;
  return h(
    "table",
    { class: "figures" },
    h(
      "thead",
      {},
      h(
        "tr",
        {},
        head.map((cell, i) => h("th", { scope: "col", class: kind(i) }, cell)),
      ),
    ),
    h(
      "tbody",
      {},
      rows.map((row) =>
        h(
          "tr",
          {},
          row.map((cell, i) =>
            h("td", { class: kind(i, cell) }, typeof cell === "object" ? cell.text : cell),
          ),
        ),
      ),
    ),
  );
}

export type { Cell, Child, Fact };
export { facts, glyph, h, table };

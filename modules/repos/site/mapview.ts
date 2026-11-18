// The map of the mycelium between repositories, drawn and walked: a row for each repository a
// cell's work ties to another, and an arc of threads beside the rows for each tie, a thread to a
// cell, as map.ts lays them out. The rows are a listbox, so a screen reader hears each with what
// it's tied to; the arrows, j and k walk them, Return opens the repository as a link to it
// would, and the threads of the row the keys rest on are drawn brighter. The map keeps its own
// keys from the page's, which walk the list, and lets the rest through.

import { escaped } from "./draw.ts";
import { type MycelialMap, mapOf, type Row, saidOf, type Thread, type Way, walked } from "./map.ts";
import type { Repository } from "./model.ts";
import { element, write } from "./page.ts";

const place = element("#ties", HTMLElement);
const map = element("#ties-map", HTMLDivElement);
const here = element("#ties-here", HTMLElement);
const said = element("#ties-said", HTMLParagraphElement);

// The keys the map answers, and the way each walks; the page answers the rest.
const WAYS: { readonly [key: string]: Way } = {
  ArrowUp: "up",
  k: "up",
  ArrowDown: "down",
  j: "down",
  ArrowLeft: "left",
  h: "left",
  ArrowRight: "right",
  l: "right",
  Home: "first",
  g: "first",
  End: "last",
  G: "last",
};
// How each way is said when nothing lies that way.
const NOTHING: { readonly [way in Way]: string } = {
  up: "above",
  down: "below",
  left: "to the left of",
  right: "to the right of",
  first: "before",
  last: "after",
};
// A row is ten units high in the threads' drawing, which stretches to the rows beside it.
const UNIT = 10;

let drawn: MycelialMap = { rows: [], threads: [], ties: [] };
let at = 0;
let saying = "";

/** One thread as an arc from its tie's upper row to its lower, deeper the farther apart. */
function arc(thread: Thread, rows: number): string {
  const span = (thread.to - thread.from) / Math.max(1, rows - 1);
  // Farther ties bow deeper and each cell's lane a little more, never past the gutter's edge.
  const depth = Math.min(96, 14 + 70 * span + 4 * thread.lane);
  const top = thread.from * UNIT + UNIT / 2;
  const bottom = thread.to * UNIT + UNIT / 2;
  const lit = thread.from === at || thread.to === at ? " lit" : "";
  const bend = (100 - depth).toFixed(1);
  const path = `M100 ${top}C${bend} ${top} ${bend} ${bottom} 100 ${bottom}`;
  return `<path class="tie-thread${lit}" data-line d="${path}"/>`;
}

/** The rows and the threads beside them, the threads of the row the keys rest on lit. */
function markup(): string {
  const { rows, threads } = drawn;
  const height = rows.length * UNIT;
  const lines = threads.map((each) => arc(each, rows.length)).join("");
  const options = rows
    .map(
      (row, index) => `<div class="tie-row" role="option" id="tie-${index}"
      data-key="${escaped(row.key)}" data-level="repository"${index === at ? " data-here" : ""}
      aria-selected="${index === at}"
      aria-label="${escaped(row.name)}: ${escaped(saidOf(drawn, index))}">
      <span class="tie-name" data-name translate="no">${escaped(row.name)}</span></div>`,
    )
    .join("");
  return `<svg class="tie-threads" viewBox="0 -1 100 ${height + 2}" preserveAspectRatio="none"
    aria-hidden="true">${lines}</svg>${options}`;
}

/** Draws the map of `repositories`, keeping the row the keys rest on by its repository. */
function drawMap(repositories: readonly Repository[]): void {
  const key = drawn.rows[at]?.key;
  drawn = mapOf(repositories);
  const kept = drawn.rows.findIndex((each) => each.key === key);
  at = kept === -1 ? Math.min(at, Math.max(0, drawn.rows.length - 1)) : kept;
  place.hidden = drawn.rows.length === 0;
  // The threads span every row, which the grid is told the count of, the rows being drawn.
  map.style.setProperty("--tie-rows", String(Math.max(1, drawn.rows.length)));
  write(map, markup());
  rest();
}

/** A repository's name as a link to it, as the list's rows are. */
function linked(row: Row): string {
  return `<a class="tie-link" href="${escaped(row.address)}" translate="no">${escaped(row.name)}</a>`;
}

/** Says which row the keys rest on, to a screen reader and beside the map. */
function rest(): void {
  const row = drawn.rows[at];
  if (row === undefined) {
    map.removeAttribute("aria-activedescendant");
    return;
  }
  map.setAttribute("aria-activedescendant", `tie-${at}`);
  // The caption names each repository as the page does elsewhere, as a link to its home, so a
  // pointer opens any of them and the homes probe reads each named where it lives.
  write(here, `${linked(row)} ${saidOf(drawn, at, linked)}`);
}

/** Says that nothing lies `way`, in other words when it's said again, so a repeat is heard. */
function sayNothing(way: Way): void {
  const name = drawn.rows[at]?.name ?? "";
  const words = `Nothing lies ${NOTHING[way]} ${name} on the map.`;
  saying = saying === words ? `Still nothing ${NOTHING[way]} ${name}.` : words;
  said.textContent = saying;
}

/** Moves the keys `way`, or stays and says so. */
function go(way: Way): void {
  const to = walked(drawn, at, way);
  if (to === undefined || to === at) {
    sayNothing(way);
    return;
  }
  at = to;
  saying = "";
  write(map, markup());
  rest();
}

/** Opens the repository at the row, as following a link to it does. */
function open(index: number): void {
  const row = drawn.rows[index];
  if (row !== undefined) {
    location.hash = row.address;
  }
}

map.addEventListener("keydown", (event) => {
  if (event.altKey || event.ctrlKey || event.metaKey) {
    return;
  }
  const way = WAYS[event.key];
  if (way !== undefined) {
    go(way);
  } else if (event.key === "Enter") {
    open(at);
  } else if (event.key !== "Escape") {
    return;
  }
  // Escape closes nothing here, since the map opens nothing of its own, and the page's would
  // let go of a find the list holds while the keys are on the map.
  event.preventDefault();
  event.stopPropagation();
});

map.addEventListener("click", (event) => {
  const row = event.target instanceof Element ? event.target.closest(".tie-row") : null;
  const index = drawn.rows.findIndex((each) => each.key === row?.getAttribute("data-key"));
  if (index !== -1) {
    at = index;
    write(map, markup());
    rest();
    open(index);
  }
});

export { drawMap };

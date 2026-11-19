// The map of the mycelium between repositories, drawn and walked: a row for each repository a
// cell's work ties to another, and an arc of threads beside the rows for each tie, a thread to a
// cell, as map.ts lays them out. The rows are a listbox, so a screen reader hears each with what
// it's tied to; up and down, j and k walk them, Return opens the repository as a link to it
// would, and the threads of the row the keys rest on are drawn brighter. The map keeps its own
// keys from the page's, which walk the list; the list's other keys act on the list's cursor,
// not the map's, so on the map they say where they act; and it lets the rest through.

import { escaped } from "./draw.ts";
import {
  bowed,
  kept,
  type MycelialMap,
  mapOf,
  type Row,
  saidOf,
  type Thread,
  type Way,
  walked,
} from "./map.ts";
import type { Repository } from "./model.ts";
import { element, write } from "./page.ts";

const place = element("#ties", HTMLElement);
const map = element("#ties-map", HTMLDivElement);
const here = element("#ties-here", HTMLElement);
const said = element("#ties-said", HTMLParagraphElement);
const list = element("#list", HTMLDivElement);

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
// The list's keys that act on its cursor rather than walk it, as site.ts answers them.
const LISTS = new Set(["-", "o", "y", "z"]);
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
// How the page opens a repository by its key, as the list does, which the page gives the map.
let opening: (key: string) => void = () => undefined;
// Each repository's row keeps one id however the rows around it move, so a screen reader hears
// a new repository come under the keys, which the same id in its place would keep unsaid.
const ids = new Map<string, string>();

/** The id of the row for the repository `key`. */
function idOf(key: string): string {
  const id = ids.get(key) ?? `tie-${ids.size}`;
  ids.set(key, id);
  return id;
}

/** One thread as an arc from its tie's upper row to its lower, deeper the farther apart. */
function arc(thread: Thread, rows: number): string {
  const depth = bowed(thread, rows);
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
      (row, index) => `<div class="tie-row" role="option" id="${idOf(row.key)}"
      data-key="${escaped(row.key)}" data-level="repository"${index === at ? " data-here" : ""}
      aria-selected="${index === at}"
      aria-label="${escaped(row.name)}: ${escaped(saidOf(drawn, index))}">
      <span class="tie-name" data-name translate="no">${escaped(row.name)}</span></div>`,
    )
    .join("");
  // The drawing is exactly the rows' height, so stretched to them each thread ends on its row's
  // middle.
  return `<svg class="tie-threads" viewBox="0 0 100 ${height}" preserveAspectRatio="none"
    aria-hidden="true">${lines}</svg>${options}`;
}

/**
 * Draws the map of `repositories`, keeping the row the keys rest on by its repository, and opens
 * one with `opens`, as the list would.
 */
function drawMap(repositories: readonly Repository[], opens: (key: string) => void): void {
  opening = opens;
  const was = drawn;
  drawn = mapOf(repositories);
  at = kept(was, at, drawn);
  const hiding = drawn.rows.length === 0;
  // The keys on the map as its last tie goes are handed to the list, not dropped from the page,
  // and said to be, so the next j isn't heard moving something else.
  if (hiding && place.contains(document.activeElement)) {
    list.focus({ preventScroll: true });
    const words = "No cell at work joins two repositories now, so the keys are on the list.";
    say(words, "Again no cell at work joins two repositories, so the keys are on the list.");
  }
  place.hidden = hiding;
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
  map.setAttribute("aria-activedescendant", idOf(row.key));
  // The caption names each repository as the page does elsewhere, as a link to its home, so a
  // pointer opens any of them and the homes probe reads each named where it lives. A link the
  // redraw takes away leaves the keys on the map rather than at the top of the page.
  const held = here.contains(document.activeElement);
  write(here, `${linked(row)}: ${saidOf(drawn, at, linked)}`);
  if (held && !here.contains(document.activeElement)) {
    map.focus({ preventScroll: true });
  }
}

/** Says `words`, or `again` when they were said last, so a repeat is heard too. */
function say(words: string, again: string): void {
  saying = saying === words ? again : words;
  said.textContent = saying;
}

/** Moves the keys `way`, or stays and says so. */
function go(way: Way): void {
  const to = walked(drawn, at, way);
  if (to === undefined || to === at) {
    const where = `${NOTHING[way]} ${drawn.rows[at]?.name ?? ""}`;
    say(`Nothing lies ${where} on the map.`, `Still nothing ${where}.`);
    return;
  }
  at = to;
  // Cleared, so the same words said again after a step are heard as new.
  saying = "";
  said.textContent = "";
  write(map, markup());
  rest();
  // The keys rest on a row that's not focused itself, so nothing else brings it into view.
  map.querySelector("[data-here]")?.scrollIntoView({ block: "nearest" });
}

/**
 * Opens the repository at the row as the list does, which shows it there past any find, takes
 * the keys to it, and moves the address in place, the one already open too.
 */
function open(index: number): void {
  const row = drawn.rows[index];
  if (row !== undefined) {
    opening(row.key);
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
  } else if (LISTS.has(event.key)) {
    const there = `on the list: Return takes the keys to ${drawn.rows[at]?.name ?? ""} there.`;
    say(`${event.key} acts ${there}`, `${event.key} still acts ${there}`);
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

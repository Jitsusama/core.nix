// What each cell is for: every cell standing on this machine that works on
// a quest, drawn first as the forest floor cut through and then listed, each
// under the cell that started it, with the purpose its quest states and how
// far the plans it keeps have come, and beside them the cell the cursor is
// on, with each of its documents. It watches the mycelium node's forage
// records over the sites' server, so a cell redraws the moment a step is
// ticked or a cell comes or goes, and the standing line says whether what it
// shows is live and since when. A feed reaches one node, so it shows the
// cells on this machine; `mycelium forage` in a terminal gathers the realm's.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import { type Actions, listen, toggle } from "../../local-sites/kit/keys.ts";
import type { Held } from "../../local-sites/kit/records.ts";
import { said } from "../../local-sites/kit/status.ts";
import { pressable } from "../../local-sites/kit/steps.ts";
import { themeSwitch } from "../../local-sites/kit/theme.ts";
import { command, escaped, floorOf, forageRow, onMap, panelOf } from "./draw.ts";
import { type Forage, foragesOf, grewFrom, headline, holds, lede, startedFrom } from "./model.ts";

/** The element `selector` finds, which the page's markup always holds. */
function element<T extends Element>(selector: string, kind: new () => T): T {
  const found = document.querySelector(selector);
  if (!(found instanceof kind)) {
    throw new Error(`the page has no ${selector}, which the forage page draws into`);
  }
  return found;
}

/** How a hash names a cell, read before the cursor is first set. */
const CELL_HASH = /^#cell=(.+)$/u;

const list = element("#cells", HTMLDivElement);
const title = element("#headline", HTMLHeadingElement);
const ledeLine = element("#lede", HTMLParagraphElement);
const substrate = element("#substrate", HTMLSpanElement);
const floor = element("#floor", HTMLDivElement);
const asof = element("#asof", HTMLSpanElement);
const panel = element("#panel-body", HTMLDivElement);
const keys = element("#keys", HTMLDialogElement);
const finder = element("#find", HTMLInputElement);
const listTitle = element("#cells-title", HTMLHeadingElement);

let forages: readonly Forage[] = [];
let unread: readonly string[] = [];
let standing: Standing | undefined;
// The last thing the watch couldn't read, said until it reads another.
let heard: string | undefined;
// Whether the node has given any records yet, so a refusal before the first
// never leaves the page saying it's still asking.
let given = false;
let cursor: string | null = cellIn(location.hash);
let sought = "";
// Whether the list holds only the cells with no purpose written, as the
// headline's second count asks.
let hollowOnly = false;

/** The cell `hash` names, as #cell=<address>. */
function cellIn(hash: string): string | null {
  const named = CELL_HASH.exec(hash)?.[1];
  if (named === undefined) {
    return null;
  }
  try {
    return decodeURIComponent(named);
  } catch {
    // A stray % in an address typed by hand names no cell.
    return null;
  }
}

/** The cells the find leaves, in the list's order. */
function shown(): readonly Forage[] {
  return forages.filter(
    (forage) => holds(forage, sought) && (!hollowOnly || forage.purpose === ""),
  );
}

/** The machine the cells run on, as the records say it. */
function machine(): string {
  return forages[0]?.machine ?? "";
}

/** Says whether what's drawn is live, and what couldn't be read. */
function sayWhen(): void {
  const problems = [...unread, ...(heard === undefined ? [] : [heard])];
  const trouble = problems.length > 0 ? ` Couldn't read: ${problems.join("; ")}.` : "";
  asof.textContent = `${standing === undefined ? "Reaching the mycelium node." : said(standing)}${trouble}`;
  asof.dataset["state"] = standing?.state ?? "connecting";
  if (!given && standing?.state === "broken") {
    title.textContent = "Nothing to draw yet.";
    ledeLine.textContent =
      "The mycelium node didn't say what its cells are for; the line below says why.";
  }
}

const current = (): Forage | undefined => forages.find((each) => each.cell === cursor);

function select(): void {
  // Only the row leaving the cursor and the row taking it change, so a step
  // costs the same whether the list holds five cells or ten thousand.
  for (const row of list.querySelectorAll<HTMLElement>('[aria-selected="true"]')) {
    if (row.dataset["cell"] !== cursor) {
      row.setAttribute("aria-selected", "false");
    }
  }
  const row = cursor === null ? null : document.getElementById(`cell-${cursor}`);
  if (row instanceof HTMLElement && list.contains(row)) {
    row.setAttribute("aria-selected", "true");
    list.setAttribute("aria-activedescendant", row.id);
    row.scrollIntoView({ block: "nearest" });
  }
  for (const mark of floor.querySelectorAll<SVGRectElement>("rect[data-cell]")) {
    mark.classList.toggle("on", mark.dataset["cell"] === cursor);
  }
  if (cursor !== null && location.hash !== `#cell=${encodeURIComponent(cursor)}`) {
    history.replaceState(null, "", `#cell=${encodeURIComponent(cursor)}`);
  }
}

// The cell the panel last drew. A new cell's panel opens at its top; a live
// redraw of the same one keeps where it was scrolled to and what had focus, so
// a link reached with Tab is still the one Return follows.
let drawnPanel: string | undefined;

/** What the panel can focus: its links and its copy button. */
const FOCUSABLE = "a, button";

/**
 * Where a control stands in the panel: the fact it sits in, by its term,
 * the cell it links to, if any, and its place among the fact's controls.
 * The term tells a ring's Started by link from its Started link to the same
 * cell.
 */
interface Place {
  readonly fact: string | undefined;
  readonly cell: string | undefined;
  readonly at: number;
}

/** The fact `term` names in the panel, or the whole panel for none. */
function factOf(term: string | undefined): Element | undefined {
  if (term === undefined) {
    return panel;
  }
  return [...panel.querySelectorAll(".facts > div")].find(
    (fact) => fact.querySelector("dt")?.textContent === term,
  );
}

/** Where `focused` stands in the panel, or none when it isn't there. */
function placeOf(focused: Element | null): Place | undefined {
  if (!(focused instanceof HTMLElement && panel.contains(focused))) {
    return undefined;
  }
  const fact = focused.closest(".facts > div");
  const term = fact?.querySelector("dt")?.textContent ?? undefined;
  const controls = [...(fact ?? panel).querySelectorAll(FOCUSABLE)];
  return { fact: term, cell: focused.dataset["cell"], at: controls.indexOf(focused) };
}

/**
 * The control at `place` in the redrawn panel: a link to the same cell in
 * the same fact, or the control at the same place there. A cell no longer
 * named there is none, rather than another cell's link.
 */
function refound(place: Place): HTMLElement | undefined {
  const controls = [...(factOf(place.fact)?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? [])];
  if (place.cell !== undefined) {
    return controls.find((control) => control.dataset["cell"] === place.cell);
  }
  return controls[place.at];
}

function drawPanel(): void {
  const open = current();
  const focused = placeOf(document.activeElement);
  panel.innerHTML =
    open === undefined
      ? ""
      : panelOf(open, new Set(forages.map((each) => each.cell)), startedFrom(forages, open.cell));
  if (open?.cell !== drawnPanel) {
    drawnPanel = open?.cell;
    panel.scrollTop = 0;
  } else if (focused !== undefined) {
    refound(focused)?.focus({ preventScroll: true });
  }
}

/**
 * Opens `cell`, pressed in the panel, on the floor, stepped up to or named
 * in the address. A find or the view of cells without purpose that would
 * hide it ends first, and only the one that does, so the cursor lands on
 * the cell asked for rather than the list's first row. When neither hides
 * it, only the rows and panel the step touches are redrawn.
 */
function openCell(cell: string): void {
  const forage = forages.find((each) => each.cell === cell);
  if (forage === undefined) {
    return;
  }
  cursor = cell;
  if (shown().includes(forage)) {
    select();
    drawPanel();
    return;
  }
  if (!holds(forage, sought)) {
    finder.value = "";
    finder.hidden = true;
    sought = "";
  }
  hollowOnly = hollowOnly && forage.purpose === "";
  draw();
}

function draw(): void {
  const rows = shown();
  if (!rows.some((each) => each.cell === cursor)) {
    cursor = rows[0]?.cell ?? null;
  }
  list.innerHTML =
    rows.length === 0 && forages.length > 0
      ? `<p class="none">No cell here holds what you're finding.</p>`
      : rows.map(forageRow).join("");
  if (forages.length > 0) {
    floor.innerHTML = floorOf(forages, cursor, machine());
  }
  listTitle.innerHTML = hollowOnly ? "The cells <em>without purpose</em>" : "The <em>cells</em>";
  select();
  drawPanel();
}

/** The headline, its counts leading to what they count, and the lede under it. */
function sayWhole(): void {
  const whole = headline(forages);
  const where = machine();
  substrate.textContent = where === "" ? "this machine" : where;
  if (whole.cells === 0) {
    title.innerHTML = `${escaped(whole.lead)} <em>${escaped(whole.turn)}</em>`;
  } else if (whole.without === 0) {
    title.innerHTML = `<a href="#all" data-show="all">${escaped(whole.lead)}</a> <em>${escaped(whole.turn)}</em>`;
  } else {
    title.innerHTML = `<a href="#all" data-show="all">${escaped(whole.lead)}</a> <em><a href="#without" data-show="without">${escaped(whole.turn)}</a></em>`;
  }
  ledeLine.textContent = lede(forages);
}

function heardAll(held: Held): void {
  given = true;
  ({ forages, unread } = foragesOf(held));
  sayWhole();
  sayWhen();
  draw();
}

follow(
  { feed: "node", kind: "forage" },
  {
    moved: heardAll,
    stands: (now) => {
      standing = now;
      sayWhen();
    },
    problem: (what) => {
      heard = what;
      sayWhen();
    },
  },
);

/**
 * Draws the panel again as the day turns, though no record changed, since
 * a need's since says its day only once it isn't today's.
 */
function redrawAtMidnight(): void {
  const next = new Date();
  next.setHours(24, 0, 1, 0);
  setTimeout(() => {
    drawPanel();
    redrawAtMidnight();
  }, next.getTime() - Date.now());
}

redrawAtMidnight();

/** The index `by` steps to from `at` among `count`, kept inside them. */
function stepped(at: number, by: number, count: number): number {
  if (by === Number.POSITIVE_INFINITY) {
    return count - 1;
  }
  if (by === Number.NEGATIVE_INFINITY) {
    return 0;
  }
  return Math.max(0, Math.min(count - 1, at + by));
}

function move(by: number): void {
  const rows = shown();
  const at = rows.findIndex((each) => each.cell === cursor);
  cursor = rows[stepped(at, by, rows.length)]?.cell ?? cursor;
  select();
  drawPanel();
}

/**
 * Steps to the cell the one the cursor is on grew from, the one that
 * started it or it split from, when it is drawn.
 */
function up(): void {
  const open = current();
  const by =
    open === undefined ? undefined : grewFrom(open, new Set(forages.map((each) => each.cell)));
  if (by !== undefined) {
    openCell(by);
  }
}

function goDeeper(): void {
  const open = current();
  if (open !== undefined) {
    location.href = onMap(open);
  }
}

/** Copies the cell's command, and says whether it could. */
function copy(): void {
  const open = current();
  if (open === undefined) {
    return;
  }
  const line = command(open);
  navigator.clipboard.writeText(line).then(
    () => {
      asof.textContent = `Copied ${line}.`;
    },
    (error: unknown) => {
      asof.textContent = `Couldn't copy (${String(error)}); the command is ${line}.`;
    },
  );
}

function find(): void {
  finder.hidden = false;
  finder.focus();
  finder.select();
}

/** Ends the find, keeping the cursor where it came to, and gives the list its keys. */
function endFind(clear: boolean): void {
  if (clear) {
    finder.value = "";
    sought = "";
    draw();
  }
  if (finder.value === "") {
    finder.hidden = true;
  }
  list.focus();
}

finder.addEventListener("input", () => {
  sought = finder.value;
  draw();
});

finder.addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    endFind(false);
    event.preventDefault();
  } else if (event.key === "Escape") {
    endFind(true);
    event.preventDefault();
  } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    move(event.key === "ArrowDown" ? 1 : -1);
    event.preventDefault();
  }
});

/** What each key does in the list. */
const ACTIONS: Actions = {
  j: () => move(1),
  ArrowDown: () => move(1),
  k: () => move(-1),
  ArrowUp: () => move(-1),
  g: () => move(Number.NEGATIVE_INFINITY),
  Home: () => move(Number.NEGATIVE_INFINITY),
  G: () => move(Number.POSITIVE_INFINITY),
  End: () => move(Number.POSITIVE_INFINITY),
  "-": up,
  h: up,
  ArrowLeft: up,
  y: copy,
  "/": find,
  Enter: goDeeper,
};

// The keys are the page's wherever focus is, but on another control, whose
// own keys come first; after one, the list holds the keys again, but while
// the find is open.
listen({
  help: keys,
  actions: ACTIONS,
  afterwards: () => {
    if (document.activeElement !== finder && document.activeElement !== list) {
      list.focus();
    }
  },
});

addEventListener("hashchange", () => {
  const named = cellIn(location.hash);
  if (named !== null && named !== cursor) {
    openCell(named);
  }
});

list.addEventListener("click", (event) => {
  const row = event.target instanceof Element ? event.target.closest("[role=option]") : null;
  if (!(row instanceof HTMLElement)) {
    return;
  }
  cursor = row.dataset["cell"] ?? cursor;
  select();
  drawPanel();
  list.focus();
});

panel.addEventListener("click", (event) => {
  if (!(event.target instanceof Element)) {
    return;
  }
  if (event.target.closest(".copy") !== null) {
    copy();
    return;
  }
  const starter = event.target.closest("a[data-cell]");
  if (starter instanceof HTMLAnchorElement) {
    event.preventDefault();
    const cell = starter.dataset["cell"];
    if (cell !== undefined) {
      openCell(cell);
    }
    list.focus();
  }
});

title.addEventListener("click", (event) => {
  const asked = event.target instanceof Element ? event.target.closest("a[data-show]") : null;
  if (!(asked instanceof HTMLAnchorElement)) {
    return;
  }
  event.preventDefault();
  hollowOnly = asked.dataset["show"] === "without";
  draw();
  list.focus();
});

floor.addEventListener("click", (event) => {
  const mark = event.target instanceof Element ? event.target.closest("rect[data-cell]") : null;
  if (!(mark instanceof SVGRectElement)) {
    return;
  }
  const cell = mark.dataset["cell"];
  if (cell !== undefined) {
    openCell(cell);
  }
  list.focus();
});

themeSwitch();

// The keys the head names, each a button doing what typing it does, for a
// reader who taps: j and k step the list and leave it where its keys go.
pressable(".hero .keys kbd", {
  j: () => {
    move(1);
    list.focus();
  },
  k: () => {
    move(-1);
    list.focus();
  },
  "/": find,
  Return: goDeeper,
  "?": () => toggle(keys),
});

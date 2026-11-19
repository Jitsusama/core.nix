// How a cell's panel names the cells it grew from and into: each linked to
// its row when it is drawn here, else said not to stand here, and each tie
// the node's book doesn't hold hedged as claimed, held to the cell it
// hedges so no line opens with a comma or a bracket. Every word read from
// a record is escaped before it reaches the page.

import type { Forage } from "./model.ts";

const ESCAPES: { readonly [character: string]: string } = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/** How many of the cells a cell started its panel names before counting the rest. */
const MOST_STARTED = 6;

/** `text` safe to put in markup, as text or an attribute's value. */
function escaped(text: string): string {
  return text.replace(/[&<>"']/gu, (character) => ESCAPES[character] ?? character);
}

/** `cell`'s address, linked to its row, where a press moves the cursor to it. */
function cellLink(cell: string): string {
  const address = escaped(cell);
  return `<a href="#cell=${encodeURIComponent(cell)}" data-cell="${address}"><code>${address}</code></a>`;
}

/**
 * `cell` linked to its row when it is drawn here, else said not to stand
 * here, since it may have ended or stand on another machine.
 */
function cellOrAway(cell: string, here: ReadonlySet<string>): string {
  return here.has(cell)
    ? cellLink(cell)
    : `<span><code>${escaped(cell)}</code>, not standing here</span>`;
}

/**
 * `said`, a tie the node can't vouch for, hedged naming no claimer, as either end may be
 * telling it; the comma is tied to `said`, so no line starts with one.
 */
function asClaimed(said: string): string {
  return `<span class="tie">${said},</span> as claimed`;
}

/**
 * `cell` hedged as claimed, linked when drawn here; away, the hedge goes on its address
 * before the page's own "not standing here", which the page saw for itself.
 */
function claimedCell(cell: string, here: ReadonlySet<string>): string {
  return here.has(cell)
    ? asClaimed(cellLink(cell))
    : `<span>${asClaimed(`<code>${escaped(cell)}</code>`)}, not standing here</span>`;
}

/** A hedge said after one name or count, held to it by a no-break space. */
function bracketed(hedge: string): string {
  return `\u00a0(${hedge})`;
}

/** How many of `count` cells a list names, past `MOST_STARTED` counting the rest. */
function namedOf(count: number): number {
  return count > MOST_STARTED + 1 ? MOST_STARTED : count;
}

/**
 * The cells drawn here that a cell started, each linked to its row, as "Started by" links
 * back. Past `MOST_STARTED` the rest are counted, as a wall of links would bury the facts
 * below, though one more is named, as "and 1 more" says less than its name. It's one span,
 * read as a sentence, each comma tied to the address before it so no line starts with one.
 * `hedged`, given, says the last cell as claimed, or the count, of a list all claimed.
 */
function cellsStarted(
  started: readonly string[],
  name: (cell: string) => string = cellLink,
  hedged?: (cell: string) => string,
): string {
  if (started.length === 0) {
    return "None here yet";
  }
  const count = started.length === 1 ? "One cell" : `${started.length} cells`;
  const named = started.slice(0, namedOf(started.length));
  const rest = started.length - named.length;
  const lastCell = rest === 0 ? named.pop() : undefined;
  const links = named.map(name);
  const more = hedged === undefined ? `${rest} more` : asClaimed(`${rest} more`);
  const last = lastCell === undefined ? more : (hedged ?? name)(lastCell);
  const before = links
    .map((link, at) => (at < links.length - 1 ? `<span class="tie">${link},</span>` : link))
    .join(" ");
  return before === "" ? `${count}: ${last}` : `${count}: ${before} and ${last}`;
}

/**
 * The cells a cell started, the book's unheld as claimed: after the list if all, else in
 * brackets after each named and the count, as a bare hedge would read onto the one before.
 */
function startsSaid(started: readonly Pick<Forage, "cell" | "booked">[]): string {
  const cells = started.map((each) => each.cell);
  const claimed = new Set(started.filter((each) => !each.booked).map((each) => each.cell));
  if (claimed.size > 0 && claimed.size === cells.length) {
    return cellsStarted(cells, cellLink, (cell) => asClaimed(cellLink(cell)));
  }
  const name = (cell: string) =>
    `${cellLink(cell)}${claimed.has(cell) ? bracketed("as claimed") : ""}`;
  const counted = cells.slice(namedOf(cells.length));
  const some = counted.filter((cell) => claimed.has(cell)).length;
  const all = some === counted.length;
  const rest = some === 0 ? "" : bracketed(all ? "as claimed" : "some as claimed");
  return cellsStarted(cells, name) + rest;
}

/** The cell that started `forage`, linked to that cell's row when it is drawn here. */
function startedBy(forage: Forage, here: ReadonlySet<string>): string {
  const by = forage.startedBy;
  if (by === undefined) {
    return "Nothing; it started on its own";
  }
  return forage.booked ? cellOrAway(by, here) : claimedCell(by, here);
}

/** The cells `forage` split into, every split only claimed, so the whole list is hedged. */
function splitSaid(forage: Forage, here: ReadonlySet<string>): string {
  return cellsStarted(
    forage.splitInto,
    (cell) => cellOrAway(cell, here),
    (cell) => claimedCell(cell, here),
  );
}

export { cellOrAway, claimedCell, escaped, namedOf, splitSaid, startedBy, startsSaid };

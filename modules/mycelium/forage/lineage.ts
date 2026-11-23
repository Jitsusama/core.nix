// How the cells grew from and into each other: the lines the floor walks
// up from a cell to those it grew from, and how a cell's panel names them,
// each linked to its row when it is drawn here, else said not to stand
// here, and each tie the node's book doesn't hold hedged as claimed, held
// to the cell it hedges so no line opens with a comma or a bracket. Every
// word read from a record is escaped before it reaches the page.

import type { Forage } from "./model.ts";

/**
 * How the model reads a tie between cells, among those `here`: the walk
 * below is handed it rather than importing it, as the model reads its
 * joins through this module, and modules in a cycle load in no sure order.
 */
type Tie<T> = (forage: Forage, here: ReadonlySet<string>) => T;

// The escaper lives here rather than in a module of its own, since each
// module is one more file a first visit asks for, against the page's budget.
const ESCAPES: { readonly [character: string]: string } = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/** `text` safe to put in markup, as text or an attribute's value. */
function escaped(text: string): string {
  return text.replace(/[&<>"']/gu, (character) => ESCAPES[character] ?? character);
}

/** How many cells a list names before counting the rest, in a panel or a fault. */
const MOST_STARTED = 6;

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
  return namedOrAway(cell, here).said;
}

/** `cell` as `cellOrAway` says it, and whether that ends in its link's box. */
function namedOrAway(cell: string, here: ReadonlySet<string>): Named {
  return here.has(cell)
    ? linked(cell)
    : { said: `<span><code>${escaped(cell)}</code>, not standing here</span>`, boxed: false };
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

/** A hedge said after a count, held to it by a no-break space. */
function bracketed(hedge: string): string {
  return `\u00a0(${hedge})`;
}

/**
 * A cell as a list says it, and whether it ends in a link, drawn as a box of its own which a
 * line may break after; the one drawing it knows, where its markup would only hint.
 */
interface Named {
  readonly said: string;
  readonly boxed: boolean;
}

/** `cell` linked to its row, a box a line may break after. */
function linked(cell: string): Named {
  return { said: cellLink(cell), boxed: true };
}

/** `item` with the comma after it in a list, tied to a box; text holds its comma itself. */
function listed(item: Named): string {
  return item.boxed ? `<span class="tie">${item.said},</span>` : `${item.said},`;
}

/** How many of `count` cells a list names, past `MOST_STARTED` counting the rest. */
function namedOf(count: number): number {
  return count > MOST_STARTED + 1 ? MOST_STARTED : count;
}

/**
 * The cells drawn here that a cell started, each linked to its row, as "Started by" links
 * back. Past `MOST_STARTED` the rest are counted, as a wall of links would bury the facts
 * below, though one more is named, as "and 1 more" says less than its name. It's one span,
 * read as a sentence, so no line starts with a comma. A list all `claimed` says so before
 * its first cell, as a hedge after its last would read as that cell's alone.
 */
function cellsStarted(
  started: readonly string[],
  name: (cell: string) => Named = linked,
  claimed = false,
): string {
  if (started.length === 0) {
    return "None here yet";
  }
  const one = started.length === 1;
  const counted = one ? "One cell" : `${started.length} cells`;
  const count = claimed ? `${counted}, ${one ? "" : "all "}as claimed` : counted;
  const links = started.slice(0, namedOf(started.length)).map(name);
  const rest = started.length - links.length;
  const last = rest === 0 ? (links.pop()?.said ?? "") : `${rest} more`;
  const said = (link: Named, at: number) => (at < links.length - 1 ? listed(link) : link.said);
  const before = links.map(said).join(" ");
  return before === "" ? `${count}: ${last}` : `${count}: ${before} and ${last}`;
}

/**
 * The cells a cell started, the book's unheld as claimed: before the list if all, else in
 * brackets after each named and the count, as a bare hedge would read onto the one before.
 */
function startsSaid(started: readonly Pick<Forage, "cell" | "booked">[]): string {
  const cells = started.map((each) => each.cell);
  const claimed = new Set(started.filter((each) => !each.booked).map((each) => each.cell));
  if (claimed.size > 0 && claimed.size === cells.length) {
    return cellsStarted(cells, linked, true);
  }
  // A bracketed name is held whole, so its bracket never opens a line; it ends in text.
  const name = (cell: string): Named =>
    claimed.has(cell)
      ? { said: `<span class="tie">${cellLink(cell)} (as claimed)</span>`, boxed: false }
      : linked(cell);
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
  return cellsStarted(forage.splitInto, (cell) => namedOrAway(cell, here), true);
}

/** How the cells of `forages` grew from each other, by their places in it. */
interface Kin {
  /**
   * The cell at `start` and each it grew from, up to `most` of them, once
   * round a ring, stopping at a cell `known` holds, as each cell there came
   * with those it grew from.
   */
  readonly line: (start: number, known: ReadonlySet<number>, most: number) => readonly number[];
  /** The cells that grew from the cell at `parent`, in the list's order. */
  readonly children: (parent: number) => readonly number[];
}

/** How the cells of `forages` grew from each other, each from the cell `grewFrom` says. */
function kinOf(forages: readonly Forage[], grewFrom: Tie<string | undefined>): Kin {
  const here = new Set(forages.map((each) => each.cell));
  const index = new Map(forages.map((each, place) => [each.cell, place]));
  const from = forages.map((each) => {
    const by = grewFrom(each, here);
    return by === undefined ? undefined : index.get(by);
  });
  const line = (start: number, known: ReadonlySet<number>, most: number): readonly number[] => {
    const found: number[] = [];
    const seen = new Set<number>();
    let at: number | undefined = start;
    while (at !== undefined && !known.has(at) && !seen.has(at) && found.length < most) {
      found.push(at);
      seen.add(at);
      at = from[at];
    }
    return found;
  };
  const children = (parent: number): readonly number[] =>
    from.flatMap((by, place) => (by === parent ? [place] : []));
  return { line, children };
}

/** The places of a list `length` long, from `centre` outward, nearest first. */
function* outward(centre: number, length: number): Generator<number> {
  for (let step = 0; step < length; step += 1) {
    if (centre + step < length) {
      yield centre + step;
    }
    if (step > 0 && centre - step >= 0) {
      yield centre - step;
    }
  }
}

/** The places the floor tries in turn: `first`, then outward from `centre`. */
function* starts(first: readonly number[], centre: number, length: number): Generator<number> {
  yield* first;
  yield* outward(centre, length);
}

/**
 * The places `kids` of `forages`, those the node's book holds first, so
 * cells that only claim to have grown from one, as `grewClaimed` says,
 * can't crowd its booked starts off the floor.
 */
function bookedFirst(
  forages: readonly Forage[],
  kids: readonly number[],
  grewClaimed: Tie<boolean>,
): readonly number[] {
  const here = new Set(forages.map((each) => each.cell));
  const claimed = (place: number): boolean => {
    const kid = forages[place];
    return kid === undefined || grewClaimed(kid, here);
  };
  return [...kids.filter((place) => !claimed(place)), ...kids.filter(claimed)];
}

export type { Kin };
export {
  bookedFirst,
  cellOrAway,
  claimedCell,
  escaped,
  kinOf,
  namedOf,
  splitSaid,
  startedBy,
  starts,
  startsSaid,
};

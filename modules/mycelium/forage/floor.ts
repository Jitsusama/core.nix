// Which cells the forage page's floor draws when the list holds more than
// it can, and when a step draws it again. It draws the cell it's drawn
// around, the cells that one started and those nearest it, each with every
// cell it grew from, so no branch hangs from a starter left off.

import { type Forage, grewFrom } from "./model.ts";

/** The most cells drawn on the floor; the list holds every one. */
const MOST = 48;

/** How the cells of `forages` grew from each other, by their places in it. */
interface Kin {
  /** The cell at `start` and each it grew from, up to the floor's most, once round a ring. */
  readonly line: (start: number) => readonly number[];
  /** The cells that grew from the cell at `parent`, in the list's order. */
  readonly children: (parent: number) => readonly number[];
}

function kinOf(forages: readonly Forage[]): Kin {
  const here = new Set(forages.map((each) => each.cell));
  const index = new Map(forages.map((each, place) => [each.cell, place]));
  const from = forages.map((each) => {
    const by = grewFrom(each, here);
    return by === undefined ? undefined : index.get(by);
  });
  const line = (start: number): readonly number[] => {
    const found: number[] = [];
    let at: number | undefined = start;
    while (at !== undefined && !found.includes(at) && found.length < MOST) {
      found.push(at);
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

/**
 * The cells the floor draws, in the list's order: every one when they fit.
 * Past that, it draws `around`, then the cells that grew from it, then the
 * cells nearest it, so the floor follows the cursor down a list longer
 * than it holds. Each comes with every cell it grew from, or not at all, so
 * no branch drawn hangs from a starter that isn't. With no `around` on the
 * list, that's the list's first, as the list puts each cell after its
 * starter.
 */
function drawn(forages: readonly Forage[], around: string | null): readonly Forage[] {
  if (forages.length <= MOST) {
    return forages;
  }
  const { line, children } = kinOf(forages);
  const found = forages.findIndex((each) => each.cell === around);
  const first = found < 0 ? [] : [found, ...children(found)];
  const chosen = new Set<number>();
  for (const start of [...first, ...outward(Math.max(found, 0), forages.length)]) {
    const wanted = line(start).filter((each) => !chosen.has(each));
    if (chosen.size + wanted.length <= MOST) {
      for (const each of wanted) {
        chosen.add(each);
      }
    }
    if (chosen.size === MOST) {
      break;
    }
  }
  return [...chosen].sort((one, other) => one - other).flatMap((each) => forages[each] ?? []);
}

/**
 * Whether a step to `cursor` draws the floor again, around it: only when it
 * isn't among `marked`, the cells the floor draws, and the floor draws
 * fewer than the list holds. A cell whose name reads back otherwise from
 * the page can't be marked, so without that it would redraw on each step.
 */
function stepsOffFloor(
  forages: readonly Forage[],
  cursor: string | null,
  marked: ReadonlySet<string>,
): boolean {
  return cursor !== null && !marked.has(cursor) && marked.size < forages.length;
}

export { drawn, stepsOffFloor };

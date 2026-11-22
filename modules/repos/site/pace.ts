// When a paced draw may run again. The grove is drawn whole from every record, which costs more
// the more trees there are, and records come faster than that while a reload reads them all and
// while cells write. So the next draw waits for the pacing gap once the feeds have arrived, and
// always for a few times what the last draw took, so the reader keeps most of each second.

/** How many times a draw's own cost is left idle before the next, so draws take a fifth at most. */
const IDLE_PER_DRAW = 4;

/** The last draw: when it ended and how long it took, in the page's milliseconds. */
interface Drawn {
  readonly end: number;
  readonly cost: number;
}

/** No draw yet, so the first comes at once. */
const UNDRAWN: Drawn = { end: Number.NEGATIVE_INFINITY, cost: 0 };

/**
 * How long to wait from `now` before drawing again after `drawn`: `gap` while `pacing`, and in
 * any case `IDLE_PER_DRAW` times the last draw's cost, whichever ends later.
 */
function waitBeforeDrawing(drawn: Drawn, now: number, gap: number, pacing: boolean): number {
  const rest = Math.max(pacing ? gap : 0, drawn.cost * IDLE_PER_DRAW);
  return Math.max(0, drawn.end + rest - now);
}

export { type Drawn, IDLE_PER_DRAW, UNDRAWN, waitBeforeDrawing };

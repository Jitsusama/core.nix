// When a paced draw may run again. The grove is drawn whole from every record, which costs more
// the more trees there are, and records come faster than that while a reload reads them all and
// while cells write. So the next draw waits for the pacing gap once the feeds have arrived, and
// always for a few times what the last draw took, so the reader keeps most of each second.

/**
 * How many times a draw's own cost, to the frame after it, is left idle before the next, so a
 * draw takes about a fifth of the time while records keep coming.
 */
const IDLE_PER_DRAW = 4;

/**
 * The longest a draw's cost holds the next back, so one slow frame (a collection, a busy machine,
 * a laptop waking) leaves the grove a moment behind rather than a minute, while the page says
 * it's live.
 */
const MOST_IDLE_MS = 2000;

/** The last draw: when it ended and how long it took, in the page's milliseconds. */
interface Drawn {
  readonly end: number;
  readonly cost: number;
}

/** No draw yet, so the first comes at once. */
const UNDRAWN: Drawn = { end: Number.NEGATIVE_INFINITY, cost: 0 };

/**
 * How long to wait from `now` before drawing again after `drawn`: `gap` while `pacing`, and in
 * any case `IDLE_PER_DRAW` times the last draw's cost, up to `MOST_IDLE_MS`, whichever ends later.
 */
function waitBeforeDrawing(drawn: Drawn, now: number, gap: number, pacing: boolean): number {
  const rest = Math.max(pacing ? gap : 0, Math.min(drawn.cost * IDLE_PER_DRAW, MOST_IDLE_MS));
  return Math.max(0, drawn.end + rest - now);
}

export { type Drawn, IDLE_PER_DRAW, MOST_IDLE_MS, UNDRAWN, waitBeforeDrawing };

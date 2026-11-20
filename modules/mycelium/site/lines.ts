// How wide the map draws a line at a zoom. Stepped out past its own size,
// the map thins each line with the zoom, as its cells shrink with it, so a
// line keeps the weight beside a cell it has at the map's own size: a cord
// drawn 4 px across beside cells 7 px across outweighed them on a laptop's
// whole map, and thinned only by the zoom's square root it still did.

/** The narrowest a line is drawn, in CSS px, so stepping out loses none. */
export const HAIRLINE = 1;

/** How much wider the line under the pointer is drawn, at any zoom. */
export const TOUCHED_GROWS = 2;

/**
 * The width a line carrying `carried` px is drawn at zoom `k`. Past its own
 * size only what a line carries beyond a hairline thins, so a heavier line
 * still draws heavier than a lighter one however far the map steps out, and
 * none goes under a hairline. The line under the pointer, `touched`, grows by
 * the same amount at every zoom, so what a click would open is seen first.
 */
export const drawnWidth = (carried: number, k: number, touched = false): number => {
  const thinned = k >= 1 || carried <= HAIRLINE ? carried : HAIRLINE + (carried - HAIRLINE) * k;
  return touched ? thinned + TOUCHED_GROWS : thinned;
};

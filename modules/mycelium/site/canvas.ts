// The geometry of the map's ground layer, the one canvas every network's
// ground and every line is painted on: what lies off screen, how a line is
// knocked out round a name, which line or ground a point touches, and the
// lines as the page says them to whoever checks it from outside. The
// painting itself stays with the drawing; what it decides is here, where it
// can be tried without a canvas.

/** A place, in map units or screen px as each caller says. */
export type Point = readonly [number, number];

/** A box: its left, its top, its width and its height. */
export type Box = readonly [number, number, number, number];

/** A bowed line: where it starts, its control point and where it ends. */
export type Line = readonly [Point, Point, Point];

/** The view: its zoom, as screen px per map unit, and where the map's origin falls. */
export interface View {
  readonly k: number;
  readonly tx: number;
  readonly ty: number;
}

/** Px past the map's edge a shape still paints, so a stroke's edge is never cut. */
export const GROUND_PAD = 8;
/** Px of room a line keeps from a name it is knocked out round. */
export const KNOCK_PX = 3;
/** Px from a line a pointer still touches it. */
export const TOUCH_PX = 6;
/** Straight pieces a line is taken as when asking what lies under a point. */
export const LINE_PIECES = 8;
/** The fewest points a line is said as. */
export const LINE_STEPS = 8;
/** The most a said line moves between two of its points, in px. */
export const LINE_STEP_PX = 3;

/** Whether two boxes overlap. */
export const overlap = (a: Box, b: Box): boolean =>
  a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3];

/** The point a fraction `t` along a bowed line. */
export const quadAt = ([p, c, q]: Line, t: number): Point => {
  const u = 1 - t;
  return [
    u * u * p[0] + 2 * u * t * c[0] + t * t * q[0],
    u * u * p[1] + 2 * u * t * c[1] + t * t * q[1],
  ];
};

/** A line in map units, as it falls on screen in `view`. */
export const onScreen = (line: Line, { k, tx, ty }: View): Line => {
  const [p, c, q] = line.map(([x, y]): Point => [x * k + tx, y * k + ty]);
  return [p ?? [0, 0], c ?? [0, 0], q ?? [0, 0]];
};

/** Whether a shape spanning `x0, y0` to `x1, y1` lies wholly off a `w` by `h` canvas. */
export const offCanvas = (span: Box, w: number, h: number): boolean => {
  const [x0, y0, x1, y1] = span;
  return x1 < -GROUND_PAD || y1 < -GROUND_PAD || x0 > w + GROUND_PAD || y0 > h + GROUND_PAD;
};

const DASH_APART = /[\s,]+/u;

/** A computed stroke-dasharray as the canvas takes it: none is a solid line. */
export const dashOf = (said: string): number[] =>
  said === "none" ? [] : said.split(DASH_APART).map(Number.parseFloat).filter(Number.isFinite);

const CLEAR = ", 0)";

/** A computed paint as the canvas takes it, or null for none, a clear one included. */
export const paintOf = (said: string): string | null =>
  said === "none" || said === "transparent" || said.endsWith(CLEAR) ? null : said;

/** Whether a point lies in a name's plate, or within a knock-out's room of it. */
export const knockedOut = ([x, y, bw, bh]: Box, px: number, py: number): boolean =>
  px > x - KNOCK_PX && px < x + bw + KNOCK_PX && py > y - KNOCK_PX && py < y + bh + KNOCK_PX;

/**
 * The holes the names' plates knock in the lines, each with its room, in
 * clips that each hold only holes overlapping none of each other: an
 * even-odd hole inside a hole would draw again, and clips intersect, so
 * painting through every clip leaves only what lies outside every plate.
 */
export const holeClips = (plates: readonly Box[]): Box[][] => {
  const clips: Box[][] = [];
  for (const [x, y, bw, bh] of plates) {
    const hole: Box = [x - KNOCK_PX, y - KNOCK_PX, bw + 2 * KNOCK_PX, bh + 2 * KNOCK_PX];
    const free = clips.find((clip) => !clip.some((other) => overlap(other, hole)));
    if (free) {
      free.push(hole);
    } else {
      clips.push([hole]);
    }
  }
  return clips;
};

/** How many points a line on screen is said as, so no step between two passes `LINE_STEP_PX`. */
export const lineSteps = ([p, c, q]: Line): number =>
  Math.max(
    LINE_STEPS,
    Math.ceil(
      (Math.hypot(c[0] - p[0], c[1] - p[1]) + Math.hypot(q[0] - c[0], q[1] - c[1])) / LINE_STEP_PX,
    ),
  );

/**
 * The lines on a `w` by `h` canvas as polylines in its px: each on screen
 * at all, and each knocked out round a plate said as two, ending either
 * side, since nothing is drawn there. With no plates a line is said in the
 * fewest points, since nothing on it needs finding.
 */
export const saidLines = (
  lines: readonly Line[],
  w: number,
  h: number,
  plates: readonly Box[],
): Point[][] => {
  const out: Point[][] = [];
  for (const line of lines) {
    const xs = line.map(([x]) => x);
    const ys = line.map(([, y]) => y);
    if (Math.max(...xs) < 0 || Math.max(...ys) < 0 || Math.min(...xs) > w || Math.min(...ys) > h) {
      continue;
    }
    const steps = plates.length ? lineSteps(line) : LINE_STEPS;
    let said: Point[] = [];
    for (let i = 0; i <= steps; i++) {
      const [x, y] = quadAt(line, i / steps);
      if (plates.some((plate) => knockedOut(plate, x, y))) {
        if (said.length > 1) {
          out.push(said);
        }
        said = [];
      } else {
        said.push([x, y]);
      }
    }
    if (said.length > 1) {
      out.push(said);
    }
  }
  return out;
};

/** A line the pointer can open, carrying what it opens and whether it lies inside a network's ground. */
export type Touchable = Line & { readonly opens?: unknown; readonly within?: boolean };

/** How far a point lies from a line on screen, taken as `LINE_PIECES` straight pieces. */
const distanceTo = (line: Line, [x, y]: Point): number => {
  let nearest = Number.POSITIVE_INFINITY;
  let [ax, ay] = line[0];
  for (let i = 1; i <= LINE_PIECES; i++) {
    const [bx, by] = quadAt(line, i / LINE_PIECES);
    const [dx, dy] = [bx - ax, by - ay];
    const along = Math.max(
      0,
      Math.min(1, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy || 1)),
    );
    nearest = Math.min(nearest, Math.hypot(x - ax - along * dx, y - ay - along * dy));
    [ax, ay] = [bx, by];
  }
  return nearest;
};

/**
 * The line under a point of the canvas, in its px: the nearest within
 * `TOUCH_PX` that opens something, from the first set of `layers` holding
 * one, since each set is painted over those after it. On a network's
 * ground only the lines crossing grounds count, so the ground itself
 * still opens its network.
 */
export const lineAt = <T extends Touchable>(
  layers: readonly (readonly T[])[],
  view: View,
  point: Point,
  onGround: boolean,
): T | null => {
  const [x, y] = point;
  for (const layer of layers) {
    let best: T | null = null;
    let nearest = TOUCH_PX;
    for (const touchable of layer) {
      if (!touchable.opens || (onGround && touchable.within)) {
        continue;
      }
      const line = onScreen(touchable, view);
      const xs = line.map(([lx]) => lx);
      const ys = line.map(([, ly]) => ly);
      const far =
        x < Math.min(...xs) - nearest ||
        x > Math.max(...xs) + nearest ||
        y < Math.min(...ys) - nearest ||
        y > Math.max(...ys) + nearest;
      const d = far ? Number.POSITIVE_INFINITY : distanceTo(line, point);
      if (d < nearest) {
        [best, nearest] = [touchable, d];
      }
    }
    if (best) {
      return best;
    }
  }
  return null;
};

/** A network's ground: its key, its middle and its reach, in map units. */
export type Ground = readonly [string, number, number, number];

/** The network whose ground holds a point of the map, the innermost where grounds overlap. */
export const groundAt = (grounds: readonly Ground[], x: number, y: number): string | null => {
  let best: { key: string; r: number } | null = null;
  for (const [key, gx, gy, r] of grounds) {
    if ((x - gx) ** 2 + (y - gy) ** 2 <= r * r && (!best || r < best.r)) {
      best = { key, r };
    }
  }
  return best?.key ?? null;
};

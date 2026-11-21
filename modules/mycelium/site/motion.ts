// How the map's motion is timed: the easing a message flies on, the theme's
// arriving duration as the stylesheet says it, and the keyframes a sample in
// the legend plays, so a sample moves as the map does and ends where the
// legend draws it.

/** The theme's arriving duration when its stylesheet says none, as docs/design.md sets it. */
export const ARRIVE_DEFAULT_MS = 150;

const MILLISECONDS = /^(\d+(?:\.\d+)?)ms$/u;
const SECONDS = /^(\d*\.?\d+)s$/u;
const MS_IN_A_SECOND = 1000;

/** The milliseconds `said` names, a CSS time such as "150ms" or "0.15s", or the default. */
export const durationOf = (said: string): number => {
  const text = said.trim();
  const ms = MILLISECONDS.exec(text)?.[1];
  if (ms !== undefined) {
    return Number(ms);
  }
  const seconds = SECONDS.exec(text)?.[1];
  return seconds === undefined ? ARRIVE_DEFAULT_MS : Number(seconds) * MS_IN_A_SECOND;
};

const HALF = 0.5;

/** Eased in and out over the whole way, so a message is seen leave and arrive: 0 to 1 over 0 to 1. */
export const easedInOut = (t: number): number => (t < HALF ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2);

/** One keyframe: the CSS properties a mark has then, and where it falls in the run. */
export type Frame = Readonly<Record<string, number | string>>;

/** How long a sample moves, rests gone, and comes back, and in how many steps it moves. */
export interface Timing {
  /** Moving, in milliseconds. */
  readonly ms: number;
  /** Gone, after it has moved, in milliseconds. */
  readonly holdMs: number;
  /** Coming back to rest, in milliseconds. */
  readonly arriveMs: number;
  /** Keyframes along the way, enough that a curve shows no corners. */
  readonly steps: number;
  /** The easing it comes back to rest on. */
  readonly ease: string;
}

/**
 * The keyframes of a mark taking `timing.ms` to move as `step` says for
 * each time from 0 to 1, then gone a moment, then back where the legend
 * draws it at `rest`, with how long the whole run takes. A sample ends at
 * rest rather than jumping back to it, since nothing moves unasked.
 */
export const goneAndBack = (
  timing: Timing,
  step: (t: number) => Frame,
  rest: Frame,
): { frames: Frame[]; total: number } => {
  const total = timing.ms + timing.holdMs + timing.arriveMs;
  const frames: Frame[] = [];
  for (let i = 0; i <= timing.steps; i++) {
    const t = i / timing.steps;
    frames.push({ offset: (t * timing.ms) / total, ...step(t) });
  }
  frames.push({
    offset: (timing.ms + timing.holdMs) / total,
    ...rest,
    opacity: 0,
    easing: timing.ease,
  });
  frames.push({ offset: 1, ...rest, opacity: 1 });
  return { frames, total };
};

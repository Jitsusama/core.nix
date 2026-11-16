// The map's view: how far it's zoomed, which names its edge would cut, the
// device the keys are on, and walking from one device to the next by arrow.

import type { Drawn } from "./map.ts";

/**
 * The zooms the map steps through. The words never fall under 12 pixels,
 * so the smallest is the one that keeps the drawing's 13-pixel figures at
 * 12; the largest doubles them.
 */
const ZOOMS = [12 / 13, 1, 1.25, 1.5, 2] as const;
const SMALLEST = ZOOMS[0];
const LARGEST = 2;
/** How near a zoom must be to one of the steps to count as it. */
const NEAR = 1e-6;
/** The room kept round a device brought into view. */
const PAD = 32;
/** How far Shift with an arrow moves the map. */
const NUDGE = 96;
/** An arrow reaches devices within this angle of its way, either side. */
const CONE = Math.cos(Math.PI / 4) - NEAR;

/** The words for each way an arrow walks, as "nothing ... here" says them. */
const WAYS: { readonly [way: string]: string } = {
  "1,0": "to the right of",
  "-1,0": "to the left of",
  "0,1": "below",
  "0,-1": "above",
};

/** The step just walked, so the opposite arrow retraces it. */
interface Trail {
  readonly from: string;
  readonly to: string;
  readonly dx: number;
  readonly dy: number;
}

/** The nearest device from `from` within the cone of the way `dx`, `dy`. */
function nearest(drawn: Drawn, from: string, dx: number, dy: number): string | null {
  const here = drawn.at.get(from);
  if (here === undefined) {
    return null;
  }
  const cx = here.x + here.w / 2;
  const cy = here.y + here.h / 2;
  let best: string | null = null;
  let closest = Number.POSITIVE_INFINITY;
  for (const [id, box] of drawn.at) {
    const vx = box.x + box.w / 2 - cx;
    const vy = box.y + box.h / 2 - cy;
    const reach = Math.hypot(vx, vy);
    const along = vx * dx + vy * dy;
    if (id !== from && reach > 0 && along / reach >= CONE && along < closest) {
      closest = along;
      best = id;
    }
  }
  return best;
}

/** The map's view, over one drawing at a time. */
class MapView {
  readonly #view: HTMLElement;
  readonly #draw: SVGSVGElement;
  #drawn: Drawn | undefined;
  #zoom = 1;
  #cutting = 0;
  /** The device the keys are on. */
  cursor: string | null = null;
  #trail: Trail | null = null;

  constructor(view: HTMLElement, draw: SVGSVGElement) {
    this.#view = view;
    this.#draw = draw;
    view.addEventListener("scroll", () => this.cutSoon(), { passive: true });
    addEventListener("resize", () => this.cutSoon());
  }

  /** The drawing the view now shows, the keys kept where it still has them. */
  shows(drawn: Drawn): void {
    this.#drawn = drawn;
    if (this.cursor !== null && !drawn.at.has(this.cursor)) {
      this.cursor = null;
    }
    this.#apply();
  }

  /** Whether the drawing at this zoom is bigger than the room it has. */
  get overflows(): boolean {
    const drawn = this.#drawn;
    return (
      drawn !== undefined &&
      (drawn.width * this.#zoom > this.#view.clientWidth ||
        drawn.height * this.#zoom > this.#view.clientHeight)
    );
  }

  #apply(): void {
    if (this.#drawn === undefined) {
      return;
    }
    this.#draw.setAttribute("width", String(Math.round(this.#drawn.width * this.#zoom)));
    this.#draw.setAttribute("height", String(Math.round(this.#drawn.height * this.#zoom)));
    this.cutSoon();
  }

  /**
   * A name is drawn whole or not at all: on a window too short for the
   * whole map, one the map's edge would cut is hidden until a scroll brings
   * all of it in, as a paper map's edge never shows half a word.
   */
  #cut(): void {
    this.#cutting = 0;
    const view = this.#view;
    const frame = view.getBoundingClientRect();
    const left = frame.left + view.clientLeft;
    const top = frame.top + view.clientTop;
    const right = left + view.clientWidth;
    const bottom = top + view.clientHeight;
    for (const name of this.#draw.querySelectorAll("[data-name]")) {
      const b = name.getBoundingClientRect();
      const out = b.left < left || b.right > right || b.top < top || b.bottom > bottom;
      name.classList.toggle("cut", out);
    }
  }

  cutSoon(): void {
    if (this.#cutting === 0) {
      this.#cutting = requestAnimationFrame(() => this.#cut());
    }
  }

  #set(next: number): void {
    this.#zoom = Math.min(LARGEST, Math.max(SMALLEST, next));
    this.#apply();
    if (this.cursor !== null) {
      this.bringIntoView(this.cursor);
    }
  }

  /** One zoom in, for `by` 1, or out, for -1. */
  step(by: number): void {
    const at = ZOOMS.findIndex((each) => each >= this.#zoom - NEAR);
    this.#set(ZOOMS[Math.min(ZOOMS.length - 1, Math.max(0, at + by))] ?? this.#zoom);
  }

  /**
   * Fits the drawing both ways, so a short window shows the modem whole
   * rather than its top half; never past life size, and never under the
   * smallest zoom.
   */
  fit(): void {
    const drawn = this.#drawn;
    if (drawn === undefined) {
      return;
    }
    const view = this.#view;
    this.#set(Math.min(1, view.clientWidth / drawn.width, view.clientHeight / drawn.height));
  }

  /** Moves the map by Shift and an arrow's way. */
  nudge(dx: number, dy: number): void {
    this.#view.scrollBy({ left: dx * NUDGE, top: dy * NUDGE });
  }

  /** Scrolls just enough to show the device `id` with room round it. */
  bringIntoView(id: string): void {
    const box = this.#drawn?.at.get(id);
    if (box === undefined) {
      return;
    }
    const view = this.#view;
    const zoom = this.#zoom;
    const [x0, x1] = [box.x * zoom - PAD, (box.x + box.w) * zoom + PAD];
    const [y0, y1] = [box.y * zoom - PAD, (box.y + box.h) * zoom + PAD];
    let { scrollLeft: x, scrollTop: y } = view;
    if (x0 < x) {
      x = x0;
    } else if (x1 > x + view.clientWidth) {
      x = x1 - view.clientWidth;
    }
    if (y0 < y) {
      y = y0;
    } else if (y1 > y + view.clientHeight) {
      y = y1 - view.clientHeight;
    }
    view.scrollTo({ left: x, top: y });
  }

  /** The keys on `id`, as opening it or a link puts them there. */
  land(id: string): void {
    if (id !== this.cursor) {
      this.#trail = null;
    }
    this.cursor = id;
  }

  /**
   * Walks the keys the way `dx`, `dy` to the nearest device whose centre
   * lies within 45 degrees of it, by distance along it; the opposite arrow
   * retraces the step just walked even where the nearest device back that
   * way is another. Answers where the keys landed, or, when no device lies
   * that way, the words saying so, the keys staying where they are.
   */
  walk(dx: number, dy: number): { readonly to: string } | { readonly nothing: string } | null {
    const drawn = this.#drawn;
    if (drawn === undefined) {
      return null;
    }
    const trail = this.#trail;
    const from = this.cursor;
    if (from === null || !drawn.at.has(from)) {
      this.cursor = drawn.at.keys().next().value ?? null;
      this.#trail = null;
    } else if (
      trail?.to === from &&
      trail.dx === -dx &&
      trail.dy === -dy &&
      drawn.at.has(trail.from)
    ) {
      this.#trail = { from, to: trail.from, dx, dy };
      this.cursor = trail.from;
    } else {
      const best = nearest(drawn, from, dx, dy);
      if (best === null) {
        return { nothing: WAYS[`${dx},${dy}`] ?? "that way of" };
      }
      this.#trail = { from, to: best, dx, dy };
      this.cursor = best;
    }
    return this.cursor === null ? null : { to: this.cursor };
  }
}

export { MapView };

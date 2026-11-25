// The links between the map's boxes, and what net names worth a look on
// them. Each eero's way to the one it hangs from runs down the left on a
// line of its own, side by side, so none reads as passing through another;
// the modem's cable runs straight up into the gateway.

import type { Link, Network } from "./model.ts";
import { endAt, type Shape } from "./shape.ts";
import { el, lookMark, MARK, text } from "./svg.ts";
import { linkSpeed } from "./words.ts";

/** A link between boxes, drawn after every box is placed. */
interface Wire {
  readonly from: string;
  readonly to: string;
  readonly link: Link;
  readonly down?: boolean;
}

/** What net names worth a look, as the drawing marks it. */
interface Looks {
  /** Each busy radio, as its device and its name. */
  readonly busyRadio: ReadonlySet<string>;
  /** Each weak link, as its device and the one it's weak to. */
  readonly weak: ReadonlySet<string>;
  /** The ways the internet is the narrowest point, up or down. */
  readonly ceiling: ReadonlySet<string>;
}

/** Where a box is, as the wires reach it. */
interface Reached {
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
}

/** What the wires are drawn with. */
interface Wiring {
  readonly shape: Shape;
  readonly looks: Looks;
  readonly links: SVGGElement;
  readonly at: ReadonlyMap<string, Reached>;
}

/**
 * How a wire leaves and meets its boxes: the cable's line in from the box's
 * left, its label beside it, a side link's height, and each lane's step.
 */
const WIRE = {
  cable: 152,
  label: { x: 12, y: 4 },
  side: 72,
  laneStart: 18,
  laneStep: 10,
  meet: 40,
  meetStep: 8,
} as const;

function looksOf(network: Network): Looks {
  const { concerns } = network;
  const said = (kind: string, beside: (on?: string, other?: string) => string | undefined) =>
    new Set(
      concerns
        .filter((concern) => concern.kind === kind)
        .map((concern) => `${concern.device} ${beside(concern.on, concern.other) ?? ""}`),
    );
  return {
    busyRadio: said("busy", (on) => on),
    weak: said("weak", (_on, other) => other),
    ceiling: new Set(
      concerns.filter((concern) => concern.kind === "ceiling").map((concern) => concern.on ?? ""),
    ),
  };
}

/**
 * A link between boxes in words, for its tooltip: how many devices' routes
 * cross it, and whether net names its radio busy.
 */
function crossingWords(shape: Shape, wire: Wire, n: number, busy: boolean): string {
  const name = (id: string): string => shape.byId.get(id)?.name ?? id;
  const who =
    n === shape.routed
      ? `every device's route (${n})`
      : `${n} of the ${shape.routed} devices' routes`;
  const radio = wire.link.medium === "wifi" ? endAt(wire.link, wire.from).radio : undefined;
  const waits = busy ? `; net names its ${radio ?? "radio"} busy, so they wait here` : "";
  return `${name(wire.from)} to ${name(wire.to)}: ${who} cross it${waits}.`;
}

/**
 * A link's weight by the routes crossing it, so the ones everything hangs
 * on stand out: all of them, at least half, or fewer.
 */
function weight(n: number, routed: number): string {
  if (n > 0 && n === routed) {
    return " all";
  }
  return n * 2 >= routed ? " many" : "";
}

/** Whether either end of a Wi-Fi link is a radio net names busy. */
function isBusy(looks: Looks, link: Link): boolean {
  return (
    link.medium === "wifi" &&
    [link.a, link.b].some(
      (end) => end.radio !== undefined && looks.busyRadio.has(`${end.device} ${end.radio}`),
    )
  );
}

/** The modem's cable, straight up into the gateway, with its port and speed. */
function cable(wiring: Wiring, wire: Wire, a: Reached, b: Reached): string {
  const x = a.x + WIRE.cable;
  const { port } = endAt(wire.link, wire.from);
  const speed = wire.link.speedMbps ? linkSpeed(wire.link.speedMbps) : undefined;
  const words = [port === undefined ? undefined : `port ${port}`, speed].filter(Boolean);
  const at = { x: x + WIRE.label.x, y: (a.y + a.h + b.y) / 2 + WIRE.label.y };
  text(wiring.links, words.join(" \u00b7 "), { ...at, kind: "n-fact n-small" });
  return `M${x} ${a.y + a.h}V${b.y}`;
}

/** The least a lane keeps from the drawing's left edge. */
const LANE_EDGE = 2;

/**
 * The closest two lanes may run: the thickest wire (map.css's `.n-link.all`)
 * is 3.5 wide, so lanes this far apart still show a gap between them.
 */
const LANE_TIGHTEST = 6;

/** Where a map's lanes run: left of the boxes at `box`, each `step` left of the one before. */
interface Lanes {
  readonly box: number;
  readonly step: number;
}

/**
 * How many lanes `wires` may take: one for each but the cable down, which
 * counts a side link too, so the room is never short.
 */
function lanesOf(wires: readonly Wire[]): number {
  return wires.filter((wire) => wire.down !== true).length;
}

/**
 * The lanes for `wires` left of boxes at `box`: a lane's usual step apart
 * where they all fit, closer where not, so a stacked map keeps its width on a
 * phone, though never closer than LANE_TIGHTEST; `box` is where the boxes
 * must then start for every lane to stay on the drawing.
 */
function lanesWithin(wires: readonly Wire[], box: number): Lanes {
  const gaps = Math.max(0, lanesOf(wires) - 1);
  const fits = gaps === 0 ? WIRE.laneStep : (box - WIRE.laneStart - LANE_EDGE) / gaps;
  const step = Math.min(WIRE.laneStep, Math.max(LANE_TIGHTEST, fits));
  return { box: Math.max(box, WIRE.laneStart + gaps * step + LANE_EDGE), step };
}

/** Draws each of `wires`, its lanes run as `lanes` says. */
function drawWires(wiring: Wiring, wires: readonly Wire[], lanes: Lanes): void {
  const { shape, looks, links, at } = wiring;
  let lane = 0;
  for (const wire of wires) {
    const a = at.get(wire.from);
    const b = at.get(wire.to);
    if (a !== undefined && b !== undefined) {
      // Gold where net names its radio busy, since that's where the routes
      // crossing it wait.
      const n = shape.crossing(wire.from, wire.to);
      const busy = isBusy(looks, wire.link);
      const wifi = wire.link.medium === "wifi" ? " wifi" : "";
      const kind = `n-link${wifi}${weight(n, shape.routed)}${busy ? " busy" : ""}`;
      let d: string;
      if (wire.down === true) {
        d = cable(wiring, wire, a, b);
      } else if (a.x > b.x + b.w - 1) {
        d = `M${b.x + b.w} ${a.y + WIRE.side}H${a.x}`;
      } else {
        const x = lanes.box - WIRE.laneStart - lane * lanes.step;
        d = `M${a.x} ${a.y + WIRE.meet}H${x}V${b.y + WIRE.meet + lane * WIRE.meetStep}H${b.x}`;
        lane += 1;
      }
      const ends = { "data-from": wire.from, "data-to": wire.to, "data-line": "" };
      const line = el("path", { d, class: kind, ...ends }, links);
      el("title", {}, line).textContent = crossingWords(shape, wire, n, busy);
      if (busy) {
        const mid = line.getPointAtLength(line.getTotalLength() / 2);
        lookMark(links, mid.x, mid.y + MARK.half);
      }
    }
  }
}

export type { Lanes, Looks, Wire };
export { drawWires, lanesWithin, looksOf };

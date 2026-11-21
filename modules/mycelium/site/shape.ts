// The shape of a reading: everything the map's whole draw builds its
// shapes and marks from, as one string, so two readings of one shape draw
// the same map and the second is only restyled. Inks and words are left
// out, since a restyle reads those afresh; most readings, one every two
// seconds, move only a few cells' turns and spend.

import type { Point } from "./layout.ts";

/** How far a waits ring grows, in map units, before the map is drawn again. */
export const SWELL_STEP = 0.5;

/** A network's hub, as far as its shape goes. */
export interface ShapedHub {
  readonly key: string;
  readonly edge?: boolean;
  readonly alone?: boolean;
  readonly within?: string | null;
  readonly machine?: string | null;
  readonly cells: readonly unknown[];
  readonly shape?: string;
  readonly feeds?: Iterable<unknown>;
}

/** A cell, as far as its shape goes. */
export interface ShapedCell {
  readonly id: string;
  readonly address: string;
  readonly standing?: string;
  readonly also?: readonly { readonly key: string }[];
}

/** A network several cells share, drawn as a knot. */
export interface ShapedKnot {
  readonly key: string;
  readonly shape?: string;
  readonly cells: readonly { readonly id: string }[];
}

/** A reading, as far as its shape goes. */
export interface ShapedReading {
  readonly hubs: Iterable<ShapedHub>;
  readonly cells: readonly ShapedCell[];
  readonly shared: readonly ShapedKnot[];
}

/** What the page knows of a reading beyond its records: places, liveness and homes. */
export interface ShapeReads {
  /** Whether a hub is live. */
  readonly live: (hub: ShapedHub) => boolean;
  /** How far a hub's waits ring has swollen, in map units. */
  readonly swollen: (hub: ShapedHub) => number;
  /** The seeds a hub keeps, an ended cell's as null. */
  readonly sown: (key: string) => readonly (string | null)[] | undefined;
  /** Where a cell stands, if it is placed. */
  readonly at: (id: string) => Point | undefined;
  /** The network a cell's mark sits in. */
  readonly home: (cell: ShapedCell) => string;
  /** The network a cell is drawn as belonging to, if any. */
  readonly net: (cell: ShapedCell) => string | undefined;
  /** The machines drawn, by name. */
  readonly machines: readonly string[];
}

const flag = (on: boolean | undefined): number => (on ? 1 : 0);

/**
 * The shape of `reading`: each hub's place in the tree, cells, liveness,
 * feeds, seeds and how far its waits ring has grown; each cell's home,
 * network, extra networks and place; each knot's cells; and the machines
 * drawn. Each record is a JSON list, so a name holding a separator can't
 * make two shapes read as one.
 */
export const shapeOf = (reading: ShapedReading, reads: ShapeReads): string => {
  const parts: string[] = [];
  for (const h of reading.hubs) {
    const flags = `${flag(h.edge)}${flag(h.alone)}${flag(reads.live(h))}`;
    const grown = Math.round(reads.swollen(h) / SWELL_STEP);
    const tree = [h.within || "", h.machine || ""];
    const fed = [...(h.feeds || [])];
    const sown = reads.sown(h.key) || [];
    const hub = ["h", h.key, ...tree, h.cells.length, flags, h.shape, fed, grown, sown];
    parts.push(JSON.stringify(hub));
  }
  for (const s of reading.cells) {
    const at = reads.at(s.id);
    const where = at ? `${at[0].toFixed(1)},${at[1].toFixed(1)}` : "-";
    const also = (s.also || []).map((h) => h.key);
    const asks = flag(s.standing === "asks");
    const net = reads.net(s) || "";
    parts.push(JSON.stringify(["c", s.id, s.address, reads.home(s), net, asks, also, where]));
  }
  for (const n of reading.shared) {
    parts.push(JSON.stringify(["n", n.key, n.shape, n.cells.map((s) => s.id)]));
  }
  parts.push(JSON.stringify(["m", reads.machines]));
  return parts.join("\n");
};

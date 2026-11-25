// Where the network map's parts sit, in pixels at zoom 1: its grid, its
// two plans, each part's measures, and the least room the drawing across
// takes, which is what tells the page when to stack it (site.ts). The
// drawing itself is map.ts's.

import type { Floor, Shape } from "./shape.ts";

/**
 * The drawing's grid, in pixels at zoom 1: a column for the floors' names,
 * one for the boxes, then the devices in columns of their own.
 */
const G = {
  left: 40,
  box: 200,
  boxW: 304,
  gap: 40,
  colW: 340,
  row: 24,
  bandTop: 24,
  bandEnd: 40,
  perColumn: 7,
  /** The narrowest the drawing is, however little it holds. */
  narrowest: 640,
} as const;

/**
 * Where the drawing puts its parts: the floors' rule and names, the boxes'
 * left edge, and whether a floor's switches, devices, modem and internet
 * sit beside its eero or under it.
 */
interface Plan {
  readonly left: number;
  readonly box: number;
  readonly stacked: boolean;
}

/** Across, on a desk: the floors' names in a column of their own. */
const ACROSS: Plan = { left: G.left, box: G.box, stacked: false };

/**
 * Stacked, on a phone: the floors' names over their eeros, which leave room
 * at their left for three lanes of the links between them, and more where a
 * house needs them (map.ts's drawMap).
 */
const STACKED: Plan = { left: 8, box: 40, stacked: true };

/** In a stacked floor: where the eero's box starts under the floor's name, and the gap after it. */
const STACK = { boxTop: 88, apart: 24, rowsBelow: 40 } as const;

/** Inside a box: its words' left edge, its lines' baselines, its corners. */
const BOX = {
  pad: 16,
  title: 28,
  fact: 48,
  firstRow: 76,
  pitch: 20,
  head: 72,
  foot: 28,
  footLine: 16,
  round: 6,
} as const;

/** An eero's radio row: where each of its columns starts. */
const RADIO = { channel: 112, mark: 172, busy: 184, count: 288 } as const;

/** A switch's box, its eight ports as on its face, and its link's line. */
const SWITCH = {
  tall: 144,
  apart: 40,
  ports: 8,
  portStep: 34,
  portTop: 64,
  portW: 26,
  portH: 24,
  portRound: 3,
  lamp: { x: 7, y: 3, w: 12, h: 3, round: 1.5 },
  portName: { x: 13, y: 20 },
  linkLine: 112,
} as const;

/** A device's row: its ground, and where its name, address and signal sit. */
const ROW = {
  inset: 8,
  rise: 17,
  wide: 334,
  round: 3,
  nameRoom: 140,
  address: 148,
  mark: 282,
  figure: 318,
  headAbove: 22,
} as const;

/** The floor's name and count, and the modem and the internet below it. */
const FLOOR = { name: 48, count: 70 } as const;
const MODEM = {
  below: 48,
  wide: 480,
  tall: 136,
  columns: [16, 64, 216, 336],
  room: 32,
} as const;
/**
 * The modem stacked, as wide as a device's row: each sweep on two lines, its
 * kinds on the first and its power and noise under them, and its foot's two
 * parts on a line each.
 */
const MODEM_STACKED = { wide: 334, tall: 196, first: [16, 64], second: [64, 184] } as const;
/**
 * The internet stacked, under the modem: how far below, its line's inset and
 * where the line stops above its name, and how tall it is.
 */
const INTERNET_STACKED = { below: 40, inset: 24, reach: 6, tall: 112 } as const;
const INTERNET = { ways: 56, mark: 4, markRoom: 14, ago: 100, line: 48, lineY: 22 } as const;

/**
 * How far right one floor reaches drawn across: its eero's box, each switch
 * beside it, its devices' columns, and the modem under the gateway's.
 */
function acrossRight(shape: Shape, floor: Floor): number {
  const switches = floor.switches.length * (G.boxW + SWITCH.apart);
  const columns = floor.devices.length > G.perColumn ? 2 : 1;
  const rows = G.box + G.boxW + G.gap + switches + columns * G.colW;
  const gateway = shape.modem !== undefined && floor.point.id === shape.gateway?.id;
  return gateway ? Math.max(rows, G.box + MODEM.wide + MODEM.room) : rows;
}

/**
 * The least room, at zoom 1, the drawing across takes: the floors' names, an
 * eero's box and one column of devices beside it. Narrower, it stacks; a
 * house wider than this, with a switch or a second column, scrolls across
 * instead, which a desk reads better than a stacked map leaving most of its
 * width empty.
 */
const ACROSS_LEAST = G.left + G.box + G.boxW + G.gap + G.colW;

export type { Plan };
export {
  ACROSS,
  ACROSS_LEAST,
  acrossRight,
  BOX,
  FLOOR,
  G,
  INTERNET,
  INTERNET_STACKED,
  MODEM,
  MODEM_STACKED,
  RADIO,
  ROW,
  STACK,
  STACKED,
  SWITCH,
};

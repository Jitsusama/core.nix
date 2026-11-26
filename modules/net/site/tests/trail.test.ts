// Where an arrow steps back to once the map has drawn the other way: the
// way back walked across one layout leads somewhere else on the other.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import type { Drawn } from "../map.ts";
import { MapView } from "../view.ts";

// The view listens on the window and cuts names on the next frame; neither
// exists under node, and neither touches the walk.
Object.assign(globalThis, { addEventListener: () => undefined, requestAnimationFrame: () => 1 });

const ROOM = 800;
const BOX = 40;

/** A stand-in for an element the view only listens on, sizes and searches. */
function element<T>(): T {
  const stand: unknown = {
    addEventListener: () => undefined,
    setAttribute: () => undefined,
    querySelectorAll: () => [],
    getBoundingClientRect: () => ({ left: 0, top: 0 }),
    clientLeft: 0,
    clientTop: 0,
    clientWidth: ROOM,
    clientHeight: ROOM,
  };
  // A partial element, enough for what MapView asks of one.
  return stand as T;
}

/** A drawing with each device's box at its corner, drawn across or stacked. */
function drawing(stacked: boolean, corners: { readonly [id: string]: [number, number] }): Drawn {
  const at = new Map(
    Object.entries(corners).map(([id, [x, y]]) => [
      id,
      { x, y, w: BOX, h: BOX, g: element<SVGGElement>() },
    ]),
  );
  return { at, width: ROOM, height: ROOM, marks: element<SVGGElement>(), stacked };
}

// Across, the modem's right is the eero, and the switch is below the modem.
const ACROSS = drawing(false, { modem: [0, 0], eero: [100, 0], switch: [0, 100] });
// Stacked, the switch stands left of the eero and the modem far below it,
// out of the left arrow's reach.
const STACKED = drawing(true, { modem: [0, 300], eero: [100, 0], switch: [50, 0] });

/** The view with the keys walked right from the modem to the eero, across. */
function walkedRight(): MapView {
  const view = new MapView(element<HTMLElement>(), element<SVGSVGElement>());
  view.shows(ACROSS);
  view.land("modem");
  assert.deepEqual(view.walk(1, 0), { to: "eero" });
  return view;
}

it("the opposite arrow retraces the step while the map is drawn the same way", () => {
  const view = walkedRight();
  view.shows(drawing(false, { modem: [0, 0], eero: [100, 0], switch: [60, 0] }));
  assert.deepEqual(view.walk(-1, 0), { to: "modem" });
});

it("once the map is drawn the other way, the opposite arrow goes to its nearest", () => {
  const view = walkedRight();
  view.shows(STACKED);
  assert.deepEqual(view.walk(-1, 0), { to: "switch" });
});

it("once a stacked map is drawn across, the opposite arrow goes to its nearest", () => {
  const view = new MapView(element<HTMLElement>(), element<SVGSVGElement>());
  view.shows(STACKED);
  view.land("switch");
  assert.deepEqual(view.walk(1, 0), { to: "eero" });
  // Across, the switch is far below the modem, out of the left arrow's reach.
  view.shows(drawing(false, { modem: [0, 0], eero: [100, 0], switch: [0, 300] }));
  assert.deepEqual(view.walk(-1, 0), { to: "modem" });
});

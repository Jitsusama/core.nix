// The drawing at the top of the system page: each machine a pier, its
// cells over it, the network under it. What it shows is held here as
// markup, without a browser.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { bridgeOf } from "../bridge.ts";
import { tiesOf } from "../ties.ts";
import type { Thing } from "../model.ts";

function machine(name: string, busy: number, idle: number, health: Thing["health"]): Thing {
  return {
    name,
    health,
    says: "",
    side: health === "good" ? [] : ["filling disk, 135 GB free", "code wants a look"],
    address: undefined,
    ties: tiesOf({
      cells: { busy, idle, ended: 0 },
      network: [{ device: "mac:a8", by: "2.4 GHz", point: "Hall", bars: 5, room_mbps: 184 }],
      figures: { disk: 1000e9, disk_free: 250e9 },
    }),
  };
}

/** How many times `piece` is in `text`. */
function count(text: string, piece: string): number {
  return text.split(piece).length - 1;
}

/** The bridge drawn wide and narrow, apart. */
function variants(drawn: string): { readonly wide: string; readonly narrow: string } {
  const [wide = "", narrow = ""] = drawn.split('<div class="variant narrow"');
  return { wide, narrow };
}

it("draws a pier for each machine, four at most wide and two narrow", () => {
  const five = ["a", "b", "c", "d", "e"].map((name) => machine(name, 1, 1, "good"));
  const { wide, narrow } = variants(bridgeOf(five));
  assert.equal(count(wide, 'class="pier '), 4);
  assert.equal(count(narrow, 'class="pier '), 2);
  assert.ok(wide.includes("And 1 more machine, below."));
  assert.ok(narrow.includes("And 3 more machines, below."));
});

it("names each machine over its pier, with its health in words", () => {
  const drawn = bridgeOf([machine("kestrel", 2, 1, "watch"), machine("osprey", 0, 0, "good")]);
  assert.ok(drawn.includes('translate="no">kestrel</span>'));
  assert.ok(drawn.includes("Watch: filling disk, 135 GB free<"));
  assert.ok(drawn.includes("Good: ready<"));
  assert.ok(drawn.includes("Hall, 2.4 GHz"));
});

it("sets a filled dot for each busy cell and a hollow one for each idle", () => {
  const { wide } = variants(bridgeOf([machine("kestrel", 3, 2, "good")]));
  assert.equal(count(wide, 'class="busy"'), 3);
  assert.equal(count(wide, 'class="idle"'), 2);
});

it("lets a dot stand for several cells past what a pier holds, and says so", () => {
  const { wide } = variants(bridgeOf([machine("kestrel", 150, 30, "good")]));
  assert.equal(count(wide, "<circle "), 60);
  assert.ok(wide.includes("each dot is 3 cells"));
});

it("keeps the drawing itself from a screen reader and escapes every name", () => {
  const drawn = bridgeOf([machine("<b>&", 1, 0, "bad")]);
  assert.ok(drawn.includes('aria-hidden="true" focusable="false">'));
  assert.ok(!drawn.includes("<b>&"));
  assert.ok(drawn.includes("&lt;b&gt;&amp;"));
});

it("draws nothing for no machine", () => {
  assert.equal(bridgeOf([]), "");
});

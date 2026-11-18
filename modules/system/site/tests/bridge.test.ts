// The drawing at the top of the system page: each machine a pier, its
// cells over it, the network under it. What it shows is held here as
// markup, without a browser.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { bridgeOf } from "../bridge.ts";
import { tiesOf } from "../machine.ts";
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

/** How many times `pattern` is in `text`. */
function count(text: string, pattern: RegExp): number {
  return [...text.matchAll(pattern)].length;
}

it("draws a pier for each machine, four at most wide and two narrow", () => {
  const five = ["a", "b", "c", "d", "e"].map((name) => machine(name, 1, 1, "good"));
  const drawn = bridgeOf(five);
  const [wide = "", narrow = ""] = drawn.split('<div class="variant narrow"');
  assert.equal(count(wide, /class="pier /gu), 4);
  assert.equal(count(narrow, /class="pier /gu), 2);
  assert.match(wide, /And 1 more machine, below\./u);
  assert.match(narrow, /And 3 more machines, below\./u);
});

it("names each machine over its pier, with its health in words", () => {
  const drawn = bridgeOf([machine("kestrel", 2, 1, "watch"), machine("osprey", 0, 0, "good")]);
  assert.match(drawn, /translate="no">kestrel<\/span>/u);
  assert.match(drawn, /Watch: filling disk, 135 GB free</u);
  assert.match(drawn, /Good: ready</u);
  assert.match(drawn, /Hall, 2\.4 GHz/u);
});

it("sets a filled dot for each busy cell and a hollow one for each idle", () => {
  const drawn = bridgeOf([machine("kestrel", 3, 2, "good")]);
  const [wide = ""] = drawn.split('<div class="variant narrow"');
  assert.equal(count(wide, /class="busy"/gu), 3);
  assert.equal(count(wide, /class="idle"/gu), 2);
});

it("lets a dot stand for several cells past what a pier holds, and says so", () => {
  const drawn = bridgeOf([machine("kestrel", 150, 30, "good")]);
  const [wide = ""] = drawn.split('<div class="variant narrow"');
  assert.equal(count(wide, /<circle /gu), 60);
  assert.match(wide, /each dot is 3 cells/u);
});

it("keeps the drawing itself from a screen reader and escapes every name", () => {
  const drawn = bridgeOf([machine("<b>&", 1, 0, "bad")]);
  assert.match(drawn, /<svg [^>]*aria-hidden="true"/u);
  assert.doesNotMatch(drawn, /<b>&/u);
  assert.match(drawn, /&lt;b&gt;&amp;/u);
});

it("draws nothing for no machine", () => {
  assert.equal(bridgeOf([]), "");
});

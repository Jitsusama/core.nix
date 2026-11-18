// The drawing at the top of the system page: each machine a pier, its
// cells over it, the network under it. What it shows is held here as
// markup, without a browser.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { bridgeOf } from "../bridge.ts";
import { tiesOf } from "../ties.ts";
import type { Thing } from "../model.ts";

/** Where the wide drawing starts, where its thread runs and where its dots sit. */
const VIEW_TOP = /viewBox="0 (?<top>\d+(?:\.\d+)?) /u;
const THREAD_Y = /class="thread"><path d="M[\d.]+ (?<y>[\d.]+) /u;
const DOT_Y = /cy="(?<y>[\d.]+)"/gu;
const DOT = /cx="(?<x>[\d.]+)" cy="(?<y>[\d.]+)"/gu;

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

it("counts the machines past its piers as below up to eight, and the rest on the map", () => {
  const many = (length: number) =>
    Array.from({ length }, (_, at) => machine(`m${at}`, 1, 1, "good"));
  const twelve = variants(bridgeOf(many(12)));
  assert.ok(twelve.wide.includes("And 4 more below, and 4 more on the map."), twelve.wide);
  assert.ok(twelve.narrow.includes("And 6 more below, and 4 more on the map."));
  const { wide } = variants(bridgeOf(many(1200)));
  assert.ok(wide.includes("And 4 more below, and 1,192 more on the map."));
});

it("names each machine over its pier, with its health in words", () => {
  const drawn = bridgeOf([machine("kestrel", 2, 1, "watch"), machine("osprey", 0, 0, "good")]);
  assert.ok(drawn.includes('translate="no">kestrel</span>'));
  assert.ok(drawn.includes("Watch: filling disk, 135 GB free<"));
  assert.ok(drawn.includes("Good: ready<"));
  assert.ok(drawn.includes("Hall, 2.4 GHz"));
});

/** What `drawn` lists in its `ol` of class `list`, over or under the drawing. */
function listed(drawn: string, list: "over" | "under"): string {
  const from = drawn.indexOf(`<ol class="${list}">`);
  return drawn.slice(from, drawn.indexOf("</ol>", from));
}

it("keeps only each name and its health mark over a narrow pier", () => {
  const { narrow } = variants(bridgeOf([machine("kestrel", 2, 1, "watch")]));
  const over = listed(narrow, "over");
  assert.ok(over.includes('translate="no">kestrel</span>'));
  assert.ok(over.includes('class="mark watch"'));
  assert.ok(!over.includes("filling disk"));
});

it("says each machine's name, standing and places under the narrow bridge", () => {
  const { narrow } = variants(bridgeOf([machine("kestrel", 2, 1, "watch")]));
  const under = listed(narrow, "under");
  assert.ok(under.includes('translate="no">kestrel</span>'));
  assert.ok(under.includes("Watch: filling disk, 135 GB free<"));
  assert.ok(under.includes("Hall, 2.4 GHz"));
});

it("keeps each name and standing over a wide pier, and only places under it", () => {
  const { wide } = variants(bridgeOf([machine("kestrel", 2, 1, "watch")]));
  assert.ok(listed(wide, "over").includes("Watch: filling disk, 135 GB free<"));
  assert.ok(!listed(wide, "under").includes("kestrel"));
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

it("counts every pier's dots on the one scale it says", () => {
  const { wide } = variants(
    bridgeOf([machine("kestrel", 150, 30, "good"), machine("osprey", 3, 2, "good")]),
  );
  assert.equal(count(wide, "<circle "), 62);
  assert.ok(wide.includes("each dot is 3 cells"));
});

it("rounds a pier's cells up once, never busy and idle apart", () => {
  // 144 cells fill a wide pier's 72 dots at two cells each; busy and idle
  // rounded apart would make 73, a seventh row over the pier.
  const { wide } = variants(
    bridgeOf([machine("kestrel", 143, 1, "good"), machine("osprey", 1, 1, "good")]),
  );
  assert.ok(wide.includes("each dot is 2 cells"));
  assert.equal(count(wide, "<circle "), 73);
  const rows = new Set([...wide.matchAll(DOT_Y)].map((one) => one.groups?.["y"]));
  assert.equal(rows.size, 6);
});

it("stacks the dots up from the deck, full rows lowest and a part row centred on them", () => {
  const { narrow } = variants(bridgeOf([machine("kestrel", 4, 5, "good")]));
  const dots = [...narrow.matchAll(DOT)].map((one) => ({
    x: Number(one.groups?.["x"]),
    y: Number(one.groups?.["y"]),
  }));
  const lowest = Math.max(...dots.map((dot) => dot.y));
  const low = dots.filter((dot) => dot.y === lowest);
  const high = dots.filter((dot) => dot.y !== lowest);
  assert.equal(low.length, 8, JSON.stringify(dots));
  assert.equal(high.length, 1, JSON.stringify(dots));
  const middle = low.reduce((sum, dot) => sum + dot.x, 0) / low.length;
  assert.equal(high[0]?.x, middle, JSON.stringify(dots));
});

it("keeps the drawing itself from a screen reader and escapes every name", () => {
  const drawn = bridgeOf([machine("<b>&", 1, 0, "bad")]);
  assert.ok(drawn.includes('aria-hidden="true" focusable="false">'));
  assert.ok(!drawn.includes("<b>&"));
  assert.ok(drawn.includes("&lt;b&gt;&amp;"));
});

it("marks each machine's health beside its name", () => {
  const drawn = bridgeOf([machine("kestrel", 1, 0, "watch"), machine("osprey", 0, 0, "good")]);
  assert.ok(drawn.includes('<span class="mark watch" aria-hidden="true">'));
  assert.ok(drawn.includes('<span class="mark good" aria-hidden="true">'));
});

/** Where the wide drawing's view starts, its thread's height, and its highest dot's. */
function heights(drawn: string): { top: number; thread: number; dot: number } {
  const { wide } = variants(drawn);
  const top = Number(VIEW_TOP.exec(wide)?.groups?.["top"]);
  const thread = Number(THREAD_Y.exec(wide)?.groups?.["y"]);
  const dots = [...wide.matchAll(DOT_Y)].map((one) => Number(one.groups?.["y"]));
  return { top, thread, dot: Math.min(...dots) };
}

it("starts the drawing just over its highest row of cells, the thread between", () => {
  for (const busy of [1, 30]) {
    const { top, thread, dot } = heights(
      bridgeOf([machine("kestrel", busy, 0, "good"), machine("osprey", 1, 0, "good")]),
    );
    assert.ok(thread < dot && dot - thread <= 16, `thread ${thread} over dot ${dot}`);
    assert.ok(top < thread - 9 && thread - top <= 24, `view ${top} over thread ${thread}`);
  }
});

it("draws nothing for no machine", () => {
  assert.equal(bridgeOf([]), "");
});

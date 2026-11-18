// The drawing at the top of the system page, from Joel's word that the
// machines bridge the mycelium, the network and the code (2026-11-17):
// each machine a pier, filled as far as its disk is; the mycelium's cells
// over it on the deck, a filled dot for each busy one and a hollow one for
// each idle, threaded from pier to pier; the network the water under it,
// where each one meets it said beneath. Each machine's name and how it
// stands are words over its pier, so which machine is which, and how each
// is doing, is the first thing read.
//
// The words are HTML in a grid of as many columns as piers, laid over and
// under the drawing, whose piers stand at the columns' centres, so a name
// is never squeezed into a drawing scaled down. It is drawn twice, wide
// and narrow, and the page shows the one that fits; the other is never
// displayed. The whole of it is hidden from a screen reader, by the figure
// it is drawn in, since every word in it is said by the sections below.

import { escaped, MACHINES_DRAWN } from "./draw.ts";
import { standing } from "./machine.ts";
import type { Thing } from "./model.ts";
import type { Place } from "./ties.ts";

/** One way of drawing the bridge: how wide, how many piers, how many dots. */
interface Variant {
  readonly name: string;
  readonly width: number;
  readonly piers: number;
  readonly pier: number;
  /** Dots to a row over a pier, and how far apart. */
  readonly across: number;
  readonly gap: number;
}

const WIDE: Variant = { name: "wide", width: 960, piers: 4, pier: 52, across: 12, gap: 10 };
const NARROW: Variant = { name: "narrow", width: 400, piers: 2, pier: 40, across: 8, gap: 9 };

/** How tall the drawing is, and where its deck, its water and its piers' feet are. */
const HEIGHT = 330;
const DECK = 176;
const UNDER = 184;
const FOOT = 316;
const SPRING = 300;
const HIGH_WATER = 262;
const LOW_WATER = 286;
const WATER = [HIGH_WATER, LOW_WATER];
/**
 * How high an arch rises over the deck, how far a bank runs out from its
 * pier, and where, just under the deck, the banks meet the shore.
 */
const ARCH_RISE = 36;
const BANK_REACH = 64;
const SHORE = 196;
/**
 * How far the thread runs over the highest row of dots, and how far the
 * drawing starts over the thread, its arc's crest and the stroke included,
 * so the deck sits just under the names however few rows are drawn.
 */
const THREAD_OVER = 12;
const CREST = 18;
/** The rows of dots a pier carries over the deck, the lowest just above it. */
const ROWS = 6;
const LOWEST = 166;
const DOT = 3.4;
/** How far in from the edge the deck runs, and a wave's length. */
const MARGIN = 24;
const WAVE = 40;

/** Where pier `at` of `count` stands across `width`: its column's centre. */
function centre(at: number, count: number, width: number): number {
  return (width * (2 * at + 1)) / (2 * count);
}

/**
 * How many cells a dot stands for over every pier of `shown`: one, or as
 * many as the fullest pier needs to fit, so each pier's dots are counted on
 * the one scale the key says.
 */
function scaleOf(shown: readonly Thing[], variant: Variant): number {
  const holds = variant.across * ROWS;
  const totals = shown.map(({ ties: { cells } }) =>
    cells === undefined ? 0 : cells.busy + cells.idle,
  );
  return Math.max(1, ...totals.map((total) => Math.ceil(total / holds)));
}

/** The cells over a pier at `x`, a dot for every `each` of them, and how high they reach. */
function cellsAt(
  thing: Thing,
  x: number,
  variant: Variant,
  each: number,
): { dots: string; top: number } {
  const { cells } = thing.ties;
  if (cells === undefined) {
    return { dots: "", top: DECK };
  }
  const busy = Math.ceil(cells.busy / each);
  const shown = busy + Math.ceil(cells.idle / each);
  const rows = Math.ceil(shown / variant.across);
  // Rows stack up from the deck, so a row only part filled sits on full ones,
  // centred over the pier.
  const dots = Array.from({ length: shown }, (_, at) => {
    const row = Math.floor(at / variant.across);
    const inRow = Math.min(variant.across, shown - row * variant.across);
    const left = x - ((inRow - 1) * variant.gap) / 2;
    const cx = left + (at % variant.across) * variant.gap;
    const cy = LOWEST - row * variant.gap;
    const kind = at < busy ? "busy" : "idle";
    return `<circle cx="${cx}" cy="${cy}" r="${DOT}" class="${kind}"/>`;
  });
  const top = shown === 0 ? DECK : LOWEST - (rows - 1) * variant.gap;
  return { dots: dots.join(""), top };
}

/** A pier at `x`, in its machine's health, filled as far as its disk is. */
function pierAt(thing: Thing, x: number, variant: Variant): string {
  const left = x - variant.pier / 2;
  const height = FOOT - UNDER;
  const { disk, diskFree } = thing.ties.figures;
  const used = disk === undefined || diskFree === undefined || disk === 0 ? 0 : 1 - diskFree / disk;
  const filled = Math.round(height * Math.min(1, Math.max(0, used)));
  const fill =
    filled === 0
      ? ""
      : `<rect class="fill" x="${left}" y="${FOOT - filled}" width="${variant.pier}" height="${filled}"/>`;
  return `<g class="pier ${thing.health}"><rect x="${left}" y="${UNDER}" width="${variant.pier}" height="${height}" rx="2"/>${fill}</g>`;
}

/** The arches between the piers at `xs`, and the spans to either bank. */
function spansOf(xs: readonly number[], variant: Variant): string {
  const half = variant.pier / 2;
  const first = (xs[0] ?? 0) - half;
  const last = (xs.at(-1) ?? 0) + half;
  const end = variant.width - MARGIN;
  const arches = xs.slice(1).map((x, at) => {
    const from = (xs[at] ?? 0) + half;
    const to = x - half;
    return `<path d="M${from} ${SPRING} Q ${(from + to) / 2} ${DECK - ARCH_RISE} ${to} ${SPRING}"/>`;
  });
  const near = Math.max(MARGIN, first - BANK_REACH);
  const far = Math.min(end, last + BANK_REACH);
  const banks = [
    `<path d="M${MARGIN} ${SHORE} Q ${near} ${SHORE} ${first} ${SPRING}"/>`,
    `<path d="M${last} ${SPRING} Q ${far} ${SHORE} ${end} ${SHORE}"/>`,
  ];
  return `<g class="span">${[banks[0], ...arches, banks[1]].join("")}</g>`;
}

/** The thread joining each pier's cells to the next, at height `y`. */
function threadOf(xs: readonly number[], y: number): string {
  if (xs.length < 2) {
    return "";
  }
  const steps = xs.slice(1).map((x, at) => {
    const from = xs[at] ?? 0;
    return `M${from} ${y} Q ${(from + x) / 2} ${y - CREST} ${x} ${y}`;
  });
  return `<g class="thread"><path d="${steps.join(" ")}"/></g>`;
}

/** The water across `width`, in two lines of waves. */
function waterOf(width: number): string {
  const waves = Math.ceil(width / WAVE) + 1;
  const line = (y: number, offset: number): string =>
    `<path d="M${offset - WAVE} ${y} q ${WAVE / 2} -6 ${WAVE} 0${" t 40 0".repeat(waves)}"/>`;
  return `<g class="water">${WATER.map((y, at) => line(y, at * (WAVE / 2))).join("")}</g>`;
}

/** Where one place meets the network, in a few words. */
function placeWords(place: Place): string {
  const point = place.point ?? "a point";
  return place.by === undefined ? point : `${point}, ${place.by}`;
}

/** Where a machine meets the network, a line each, or why it isn't said. */
function wherePlaces(thing: Thing): string {
  const places = thing.ties.network;
  if (places === undefined) {
    return '<span class="unread">The network not read</span>';
  }
  if (places.length === 0) {
    // biome-ignore lint/security/noSecrets: words on the page, not a secret.
    return '<span class="unread">Not on the home network</span>';
  }
  return places.map((place) => `<span>${escaped(placeWords(place))}</span>`).join("");
}

/** A machine's name, as it is said over or under its pier. */
function nameOf(thing: Thing): string {
  return `<span class="name" translate="no">${escaped(thing.name)}</span>`;
}

/** How a machine stands, after its health mark. */
function standsOf(thing: Thing): string {
  return `<span class="health ${thing.health}"><span class="mark ${thing.health}" aria-hidden="true"></span>${escaped(standing(thing, 1))}</span>`;
}

/** A count as en-CA writes it, its thousands grouped. */
const counted = new Intl.NumberFormat("en-CA");

/**
 * What lies past the piers: the machines the sections below draw, up to
 * their eight, and the rest the map shows.
 */
function moreOf(total: number, piers: number): string {
  const below = Math.max(0, Math.min(total, MACHINES_DRAWN) - piers);
  const beyond = total - Math.max(piers, MACHINES_DRAWN);
  if (below === 0 && beyond <= 0) {
    return "";
  }
  const said =
    beyond > 0
      ? `And ${counted.format(below)} more below, and ${counted.format(beyond)} more on the map.`
      : `And ${counted.format(below)} more ${below === 1 ? "machine" : "machines"}, below.`;
  return `<p class="more">${said}</p>`;
}

/**
 * The bridge drawn one way: the names over it, the drawing, the places under
 * it. A narrow pier is too slim to carry how its machine stands in words, so
 * there only its name and health mark stand over it, and its standing goes
 * under the bridge with its places, a machine to a line.
 */
function variantOf(things: readonly Thing[], variant: Variant): string {
  const shown = things.slice(0, variant.piers);
  const count = shown.length;
  const xs = shown.map((_, at) => centre(at, count, variant.width));
  const each = scaleOf(shown, variant);
  const cells = shown.map((thing, at) => cellsAt(thing, xs[at] ?? 0, variant, each));
  const narrow = variant.name === NARROW.name;
  const captions = shown
    .map((thing) =>
      narrow
        ? `<li>${nameOf(thing)}<span class="mark ${thing.health}"></span></li>`
        : `<li>${nameOf(thing)}${standsOf(thing)}</li>`,
    )
    .join("");
  const thread = Math.min(...cells.map((one) => one.top)) - THREAD_OVER;
  const top = thread - CREST;
  const svg = `<svg viewBox="0 ${top} ${variant.width} ${HEIGHT - top}" aria-hidden="true" focusable="false">${threadOf(xs, thread)}${cells.map((one) => `<g class="cells">${one.dots}</g>`).join("")}<line class="deck" x1="${MARGIN}" y1="${DECK}" x2="${variant.width - MARGIN}" y2="${DECK}"/><line class="deck under" x1="${MARGIN}" y1="${UNDER}" x2="${variant.width - MARGIN}" y2="${UNDER}"/>${spansOf(xs, variant)}${shown.map((thing, at) => pierAt(thing, xs[at] ?? 0, variant)).join("")}${waterOf(variant.width)}</svg>`;
  const places = shown
    .map((thing) =>
      narrow
        ? `<li>${nameOf(thing)}${standsOf(thing)}${wherePlaces(thing)}</li>`
        : `<li>${wherePlaces(thing)}</li>`,
    )
    .join("");
  const moreLine = moreOf(things.length, count);
  const scaled = each > 1 ? ` Past what a pier holds, each dot is ${each} cells.` : "";
  return `<div class="variant ${variant.name}" style="--piers: ${count}"><ol class="over">${captions}</ol>${svg}<ol class="under">${places}</ol>${moreLine}<p class="key">A filled dot for each busy cell, a hollow one for each idle; each pier filled as far as its disk is.${scaled}</p></div>`;
}

/** The bridge over `things`, worst first as the machines part lists them, or nothing for none. */
function bridgeOf(things: readonly Thing[]): string {
  if (things.length === 0) {
    return "";
  }
  return [WIDE, NARROW].map((variant) => variantOf(things, variant)).join("");
}

export { bridgeOf };

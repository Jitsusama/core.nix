// Your home network, now: net's records drawn as floors, each eero's box
// with its radios and the devices on it, and the gateway's floor with the
// switch, the modem and the internet. It watches net's devices, links,
// sources, internet and concerns over the sites' server, so the map moves
// the moment net's does, and the head's standing line says whether what it
// shows is live and since when.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import { hasOwnKeys, listen, toggle } from "../../local-sites/kit/keys.ts";
import type { Held } from "../../local-sites/kit/records.ts";
import { said, tell } from "../../local-sites/kit/status.ts";
import { themeSwitch } from "../../local-sites/kit/theme.ts";
import { followMycelium } from "./beside.ts";
import { details } from "./device.ts";
import { type Child, h } from "./elements.ts";
import { ACROSS_LEAST } from "./grid.ts";
import { type Drawn, drawMap } from "./map.ts";
import { type KINDS, type Network, networkOf, type Watched } from "./model.ts";
import { type Mycelium, myceliumOf } from "./mycelium.ts";
import { type Answering, type Context, found, overview, sayStandsNow } from "./panel.ts";
import { type Shape, study } from "./shape.ts";
import { el } from "./svg.ts";
import { drawTree, heedTree, markOpened } from "./tree.ts";
import { MapView } from "./view.ts";
import { fragment, linkedTo, whereWords } from "./where.ts";

/** The element `selector` finds, which the page's markup always holds. */
function element<T extends Element>(selector: string, kind: new () => T): T {
  const match = document.querySelector(selector);
  if (!(match instanceof kind)) {
    throw new Error(`the page has no ${selector}, which the network page draws into`);
  }
  return match;
}

const view = element("#map-view", HTMLElement);
const draw = element("#map-draw", SVGSVGElement);
const panel = element("#panel", HTMLElement);
const body = element("#panel-body", HTMLDivElement);
const tree = element("#tree", HTMLDivElement);
const find = element("#find", HTMLInputElement);
const where = element("#where", HTMLParagraphElement);
const lede = element("#lede", HTMLParagraphElement);
const asof = element("#asof", HTMLSpanElement);
const keys = element("#keys", HTMLDialogElement);

/** The ring round the device the keys are on, outside its box. */
const RING = { out: 4, round: 8 } as const;
/** The worst standing first, the one the page says. */
const WORST_FIRST = ["broken", "stale", "connecting", "live"];
const MAP_ITEM = ".n-item";
const FOUND_ROW = "#found .row";

// The map's view takes the keys, as a desktop's map does, though no role
// ARIA names is a map's; so it's made focusable here rather than in the
// markup, and says what it is by its name and the line it describes it by.
view.tabIndex = 0;

const map = new MapView(view, draw);
const told = new Set<string>();
const standings = new Map<string, Standing>();
// The last thing each watch couldn't read, said until it reads another.
const heard = new Map<string, string>();

let network: Network | undefined;
let shape: Shape | undefined;
let drawn: Drawn | undefined;
let mycelium: Mycelium = myceliumOf([], []);
let opened: string | null = null;
let fitted = false;
let linked = false;
let saidNothing = false;
// Whether any of net's feeds was ever live, which the lede says counts after.
let answered = false;

const now = (): number => Date.now() / 1000;

/** How the worst of net's feeds stands, or undefined before any has said. */
function worst(): Standing | undefined {
  return [...standings.values()].sort(
    (one, other) => WORST_FIRST.indexOf(one.state) - WORST_FIRST.indexOf(other.state),
  )[0];
}

/** How net's feeds stand, for the lede. */
function answering(): Answering {
  return { answered, worst: worst()?.state };
}

/** Says whether the map is live, and what couldn't be read. */
function sayWhen(): void {
  const stands = worst();
  const problems = [...(network?.unread ?? []), ...heard.values()];
  const trouble = problems.length > 0 ? ` Couldn't read: ${problems.join("; ")}.` : "";
  tell(
    asof,
    `${stands === undefined ? "" : said(stands)}${trouble}`,
    stands?.state ?? "connecting",
  );
}

function context(): Context | undefined {
  if (network === undefined || shape === undefined) {
    return undefined;
  }
  return {
    network,
    shape,
    now: now(),
    mycelium,
    open: (id) => open(id, { focusPanel: true }),
    letGo,
  };
}

function sayWhere(id: string): void {
  if (network !== undefined && shape !== undefined) {
    where.textContent = whereWords(network, shape, id) ?? "";
  }
}

// ---- Drawing. ----

let drawing = 0;

/** Draws once in the next frame, however many watches moved before it. */
function drawSoon(): void {
  if (drawing === 0) {
    drawing = requestAnimationFrame(render);
  }
}

/**
 * The first drawing fits the room it has, and a house too big for the room
 * opens where something is worth a look rather than on its top corner;
 * after that the reader's zoom stays, so a new record never moves the map
 * under them. A drawing the other way fits afresh, keeping the device the
 * keys are on in view rather than jumping back to that first concern.
 */
function fitFirst(at: Network, flipped: boolean): void {
  if (fitted && !flipped) {
    return;
  }
  fitted = true;
  map.fit();
  const worth = flipped
    ? undefined
    : at.concerns.find((concern) => concern.kind !== "ceiling" && drawn?.at.has(concern.device));
  if (map.overflows && worth !== undefined) {
    map.bringIntoView(worth.device);
  }
}

// Where the room is too narrow for an eero with devices beside it, as on a
// phone, each floor stacks; a window narrowing past that, or widening back,
// draws the other way. What was drawn is the measure, so a redraw between
// resizes counts, and render fits afresh whichever drew it the other way.
map.onResize(() => {
  if (drawn !== undefined && map.narrowerThan(ACROSS_LEAST) !== drawn.stacked) {
    drawSoon();
  }
});

/** The page opens with the map holding the keys, so arrows walk it at once. */
function landFirst(): void {
  if (linked) {
    return;
  }
  linked = true;
  followLink();
  if (document.activeElement === null || document.activeElement === document.body) {
    view.focus({ preventScroll: true });
  }
}

function render(): void {
  drawing = 0;
  network = networkOf(watched, told);
  sayWhen();
  if (network === undefined) {
    sayStandsNow(lede, answering(), undefined);
    return;
  }
  shape = study(network);
  sayStandsNow(lede, answering(), context());
  const stacked = map.narrowerThan(ACROSS_LEAST);
  const flipped = drawn !== undefined && drawn.stacked !== stacked;
  drawn = drawMap(draw, network, shape, { now: now(), stacked });
  map.shows(drawn);
  fitFirst(network, flipped);
  if (opened !== null && !shape.byId.has(opened)) {
    opened = null;
  }
  drawTree(tree, network, shape);
  paint();
  if (document.activeElement === find && find.value !== "") {
    showFound();
  } else {
    showPanel();
  }
  landFirst();
}

/** The devices on the way from the opened one to the internet. */
function onTheWay(): ReadonlySet<string> {
  if (opened === null) {
    return new Set();
  }
  return new Set([opened, ...(shape?.byId.get(opened)?.path?.through ?? [])]);
}

/** What's open and where the keys are, drawn over the map without moving it. */
function paint(): void {
  if (shape === undefined || drawn === undefined) {
    return;
  }
  const way = onTheWay();
  draw.classList.toggle("focused", opened !== null);
  for (const [id, box] of drawn.at) {
    box.g.classList.toggle("opened", id === opened);
    box.g.classList.toggle("on-way", id !== opened && way.has(id));
    box.g.toggleAttribute("data-here", id === map.cursor);
  }
  const modem = shape.modem?.id;
  for (const line of draw.querySelectorAll<SVGElement>(".n-link")) {
    const { from, to } = line.dataset;
    const between = from !== undefined && to !== undefined && way.has(from) && way.has(to);
    const internet = line.dataset["way"] === "internet" && modem !== undefined && way.has(modem);
    line.classList.toggle("lit", opened !== null && (between || internet));
  }
  drawn.marks.replaceChildren();
  const box = map.cursor === null ? undefined : drawn.at.get(map.cursor);
  if (box !== undefined) {
    const ring = {
      x: box.x - RING.out,
      y: box.y - RING.out,
      width: box.w + 2 * RING.out,
      height: box.h + 2 * RING.out,
      rx: RING.round,
    };
    el("rect", { ...ring, class: "p-halo" }, drawn.marks);
    el("rect", { ...ring, class: "p-ring" }, drawn.marks);
  }
  markOpened(tree, opened);
}

// ---- Walking, opening and letting go. ----

function walk(dx: number, dy: number): void {
  const step = map.walk(dx, dy);
  if (step === null) {
    return;
  }
  if ("nothing" in step) {
    // A trailing space that comes and goes makes a repeat heard too.
    saidNothing = !saidNothing;
    const name = (map.cursor === null ? undefined : shape?.byId.get(map.cursor)?.name) ?? "here";
    where.textContent = `Nothing ${step.nothing} ${name}${saidNothing ? "\u00a0" : ""}`;
    return;
  }
  paint();
  map.bringIntoView(step.to);
  sayWhere(step.to);
}

/** Names the opened device in the address, so it can be passed on. */
function address(id: string | null): void {
  const name =
    id === null || network === undefined || shape === undefined
      ? null
      : fragment(network, shape, id);
  if (decodeURIComponent(location.hash.slice(1)) === (name === null ? "" : `device=${name}`)) {
    return;
  }
  const to =
    name === null ? location.pathname + location.search : `#device=${encodeURIComponent(name)}`;
  history.replaceState(null, "", to);
}

function open(id: string | null, { focusPanel = false } = {}): void {
  opened = id;
  address(id);
  if (id !== null) {
    map.land(id);
  }
  find.value = "";
  paint();
  showPanel();
  if (id !== null) {
    map.bringIntoView(id);
    sayWhere(id);
  }
  if (focusPanel) {
    body.focus({ preventScroll: true });
  }
}

function followLink(): void {
  if (network === undefined || shape === undefined) {
    return;
  }
  const id = linkedTo(network, shape, location.hash);
  if (id !== null && id !== opened) {
    open(id);
  }
}

addEventListener("hashchange", followLink);

function stepOut(): void {
  const from = opened ?? map.cursor;
  const device = from === null ? undefined : shape?.byId.get(from);
  const up = device === undefined ? undefined : shape?.via(device);
  if (up !== undefined && shape?.byId.has(up) === true) {
    open(up);
  }
}

function letGo(): void {
  opened = null;
  address(null);
  paint();
  showPanel();
  view.focus({ preventScroll: true });
}

// ---- The panel. ----

/** A panel child as the nodes `replaceChildren` takes. */
function node(child: Child | readonly Child[]): (Node | string)[] {
  if (child instanceof Node || typeof child === "string") {
    return [child];
  }
  if (Array.isArray(child)) {
    return child.flatMap(node);
  }
  return typeof child === "number" ? [String(child)] : [];
}

function showPanel(): void {
  const at = context();
  const device = opened === null ? undefined : shape?.byId.get(opened);
  panel.classList.toggle("open", device !== undefined);
  if (at === undefined) {
    body.replaceChildren(h("p", { class: "hint" }, "Reaching net."));
    return;
  }
  body.replaceChildren(...node(device === undefined ? overview(at) : details(at, device)));
}

function showFound(): void {
  const at = context();
  if (find.value.trim() === "" || at === undefined) {
    showPanel();
    return;
  }
  panel.classList.add("open");
  body.replaceChildren(...node(found(at, find.value)));
}

/** What each of the search's keys does. */
const FIND_KEYS: { readonly [key: string]: () => void } = {
  Enter: () => {
    const first = body.querySelector<HTMLElement>(FOUND_ROW);
    if (first?.dataset["id"] !== undefined) {
      open(first.dataset["id"], { focusPanel: true });
    }
  },
  ArrowDown: () => body.querySelector<HTMLElement>(FOUND_ROW)?.focus(),
  Escape: () => {
    find.value = "";
    showPanel();
    view.focus({ preventScroll: true });
  },
};

find.addEventListener("input", showFound);
find.addEventListener("keydown", (event) => {
  if (Object.hasOwn(FIND_KEYS, event.key)) {
    FIND_KEYS[event.key]?.();
    event.preventDefault();
  }
});

heedTree(tree, (id) => open(id));

// ---- The map's pointer and keys. ----

draw.addEventListener("click", (event) => {
  const item = event.target instanceof Element ? event.target.closest(MAP_ITEM) : null;
  if (item instanceof SVGElement && item.dataset["id"] !== undefined) {
    open(item.dataset["id"]);
    view.focus({ preventScroll: true });
  } else if (opened !== null) {
    letGo();
  }
});

/** The step each arrow or letter walks on the map. */
const STEPS: { readonly [key: string]: readonly [number, number] } = {
  ArrowLeft: [-1, 0],
  h: [-1, 0],
  ArrowRight: [1, 0],
  l: [1, 0],
  ArrowUp: [0, -1],
  k: [0, -1],
  ArrowDown: [0, 1],
  j: [0, 1],
};

/** What one of the map's own keys does, answering whether it was the map's. */
function mapKey(event: KeyboardEvent): boolean {
  const step = STEPS[event.key];
  if (step !== undefined && event.shiftKey && event.key.startsWith("Arrow")) {
    map.nudge(step[0], step[1]);
  } else if (step !== undefined) {
    walk(step[0], step[1]);
  } else if (event.key === "Enter" && map.cursor !== null) {
    open(map.cursor);
  } else {
    return false;
  }
  return true;
}

// The map's own keys answer only while it holds focus, as the tree's do;
// with Shift an arrow moves the map rather than walking it.
view.addEventListener("keydown", (event) => {
  const modified = event.altKey || event.ctrlKey || event.metaKey;
  if (!(modified || event.target !== view || keys.open) && mapKey(event)) {
    event.preventDefault();
    event.stopPropagation();
  }
});

view.addEventListener("focus", () => {
  if (map.cursor === null && drawn !== undefined) {
    walk(0, 0);
  } else if (map.cursor !== null) {
    sayWhere(map.cursor);
  }
});

listen({
  help: keys,
  actions: {
    "/": () => find.focus(),
    i: () => map.step(1),
    "+": () => map.step(1),
    "=": () => map.step(1),
    o: () => map.step(-1),
    "0": () => map.fit(),
    Escape: () => {
      if (opened === null) {
        return false;
      }
      letGo();
      return true;
    },
    "-": stepOut,
  },
  answersAt: (target) => !hasOwnKeys(target),
});

map.heedButtons(document.querySelectorAll<HTMLElement>("[data-zoom]"));

for (const key of document.querySelectorAll<HTMLElement>(".keys kbd[data-key]")) {
  key.addEventListener("click", () => {
    if (key.dataset["key"] === "?") {
      toggle(keys);
    } else {
      find.focus();
    }
  });
}

themeSwitch();

// ---- What's watched. ----

function watch(kind: (typeof KINDS)[number]): Held {
  return follow(
    { feed: "net", kind },
    {
      moved: () => {
        told.add(kind);
        heard.delete(kind);
        drawSoon();
      },
      // A feed that never answers moves nothing, so the lede follows its
      // standing here as well as from each drawing.
      stands: (stands) => {
        standings.set(kind, stands);
        answered ||= stands.state === "live";
        sayWhen();
        sayStandsNow(lede, answering(), context());
      },
      problem: (what) => {
        heard.set(kind, `${kind}: ${what}`);
        sayWhen();
      },
    },
  ).held;
}

const watched: Watched = {
  device: watch("device"),
  link: watch("link"),
  source: watch("source"),
  internet: watch("internet"),
  concern: watch("concern"),
};

followMycelium((read) => {
  mycelium = read;
  if (opened !== null && document.activeElement !== find) {
    showPanel();
  }
});
document.fonts.ready
  .then(() => {
    if (network !== undefined) {
      render();
    }
  })
  .catch(() => undefined);

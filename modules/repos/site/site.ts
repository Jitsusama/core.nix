// Code, live: every repository the repos daemon holds, as a grove above their list, the cursor's
// open beside it, and a map of the cells joining them below. It watches the daemon's records and
// the mycelium node's cells over the sites' server, so the page moves the moment git writes.
// Nothing under the reader moves unless they moved it: each repository and the cursor stay put.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import { type Actions, listen, toggle } from "../../local-sites/kit/keys.ts";
import { Held } from "../../local-sites/kit/records.ts";
import { themeSwitch } from "../../local-sites/kit/theme.ts";
import { addressed, forgotten, landing, type Place } from "./address.ts";
import { drawer, groveOfTrees, rows } from "./draw.ts";
import { footOf, headOf, worst } from "./head.ts";
import { Listing } from "./listed.ts";
import { drawMap } from "./mapview.ts";
import {
  type Feed,
  type Grove,
  groveOf,
  growing,
  KINDS,
  type Repository,
  type Season,
  stepped,
} from "./model.ts";
import { UNHEARD, watchMycelium } from "./mycelium.ts";
import { footed, USING } from "./using.ts";
import {
  copyFrom,
  element,
  keepIn,
  keepRoom,
  keepTime,
  markRows,
  markTree,
  openOutward,
  paced,
  pressHead,
  write,
} from "./page.ts";

const title = element("#title", HTMLHeadingElement);
const lede = element("#lede", HTMLParagraphElement);
const stand = element("#grove", HTMLDivElement);
const list = element("#list", HTMLDivElement);
const opened = element("#open", HTMLDivElement);
const asof = element("#asof", HTMLParagraphElement);
const find = element("#find", HTMLInputElement);
const keys = element("#keys", HTMLDialogElement);

// What each kind's watch holds: an empty one until its follow, below, gives
// the Held it folds each answer into, so the grove reads every kind at once.
const held: { -readonly [kind in keyof Feed]: Held } = {
  reading: new Held(),
  repository: new Held(),
  checkout: new Held(),
  tree: new Held(),
  change: new Held(),
  using: new Held(),
};
const standings = new Map<string, Standing>();
const heard = new Map<string, string>();
// The kinds whose records have arrived, and whether the node's cells have: draws keep only their
// own rest, and the room kept from the last visit is held, until they all have.
const arrived = new Set<string>();
let cellsHeard = false;
const repos = (): boolean => KINDS.every((kind) => arrived.has(kind));
// A reload draws again only what the address says, so what it won't keeps no room.
const roomDrawn = keepRoom(() =>
  forgotten(location.hash, grove, { query, place, cursor, unfolded, whole }),
);
let mycelium = UNHEARD;

let place: Place = "every";
let query = "";
let grove: Grove | undefined;
let cursor: string | null = null;
// The tree the cursor is on once Joel has stepped into the repository.
let inner: string | null = null;
const listing = new Listing();
const { unfolded, whole } = listing;

/** The repositories the list shows: the place's, narrowed by what's found. */
function shown(): readonly Repository[] {
  if (grove === undefined) {
    return [];
  }
  const inPlace = place === "growing" ? growing(grove) : grove.repositories;
  const words = query.trim().toLowerCase();
  return words === ""
    ? inPlace
    : inPlace.filter((each) => `${each.owner ?? ""}/${each.name}`.toLowerCase().includes(words));
}

const current = (): Repository | undefined =>
  grove?.repositories.find((each) => each.key === cursor);

function sayWhen(): void {
  const now = worst(footed(standings));
  asof.textContent = footOf(grove, now, footed(heard));
  asof.dataset["state"] = now?.state ?? "connecting";
}

/** The headline and its lede: how things stand, then what's worth a look. */
function drawHero(at: Grove): void {
  const head = headOf(at);
  write(title, head.title);
  write(lede, head.lede);
  document.title = head.tab;
}

function drawPlaces(): void {
  for (const link of document.querySelectorAll<HTMLAnchorElement>("[data-place]")) {
    if (link.closest(".places") !== null) {
      link.setAttribute("aria-current", String(link.dataset["place"] === place));
    }
  }
  if (grove !== undefined) {
    element("#n-every", HTMLSpanElement).textContent =
      grove.repositories.length.toLocaleString("en-CA");
    element("#n-growing", HTMLSpanElement).textContent =
      growing(grove).length.toLocaleString("en-CA");
  }
}

function drawOpen(): void {
  const open = current();
  if (open === undefined || grove === undefined) {
    write(opened, '<p class="quiet">Step onto a repository to open it here.</p>');
    return;
  }
  const { home, homes } = mycelium;
  const keep = listing.kept(open.key, inner);
  const at = { now: Date.now(), home, homes, keep, whole: whole.has(open.key) };
  write(opened, drawer(open, grove, at, unfolded.get(open.key) ?? new Set()));
  if (!markInner(false) && inner !== null) {
    inner = null;
    follows();
  }
}

/** Marks the tree the cursor is on inside the open repository. */
const markInner = (focus: boolean): boolean => markTree(opened, inner, focus);

function draw(): void {
  grove = groveOf(held, mycelium.cells, Date.now());
  sayWhen();
  if (grove === undefined) {
    return;
  }
  drawHero(grove);
  drawPlaces();
  if (cursor === null || !grove.repositories.some((each) => each.key === cursor)) {
    cursor = shown()[0]?.key ?? grove.repositories[0]?.key ?? null;
  }
  // A link in the drawer that a move redraws is gone, so focus stays in the drawer.
  const wasIn = opened.contains(document.activeElement);
  write(stand, groveOfTrees(grove));
  drawMap(grove.repositories, (key) => choose(key, null));
  drawList();
  drawOpen();
  if (wasIn) {
    keepIn(opened, inner);
  }
  roomDrawn(repos() && cellsHeard);
  land();
}

function drawList(): void {
  write(list, rows(shown(), Date.now(), query));
  markRows(cursor, false);
}

// Draws on the frame after a record, then at most twice a second while records keep coming, and
// never for more than a fifth of the time, however long the grove takes to draw.
const soon = paced(draw, 500, repos);

const followed: readonly (keyof Feed)[] = [...KINDS, USING];
for (const kind of followed) {
  held[kind] = follow(
    { feed: "repos", kind },
    {
      moved: () => {
        arrived.add(kind);
        soon();
      },
      stands: (now) => {
        standings.set(kind, now);
        heard.delete(kind);
        sayWhen();
      },
      problem: (what) => {
        heard.set(kind, `${kind}: ${what}`);
        sayWhen();
      },
    },
  ).held;
}

watchMycelium((now) => {
  mycelium = now;
  cellsHeard = true;
  soon();
});
// Ages and seasons are told against now, so they're kept with the clock while nothing moves.
keepTime(soon);

// ---- Where the address lands, and where it follows the cursor. ----

let landed = "";
/** Puts the cursor where the address says: a repository, or a tree in one. */
function land(): void {
  if (grove === undefined || location.hash === landed) {
    return;
  }
  const hash = location.hash;
  const at = landing(hash, grove);
  if (at === undefined) {
    return;
  }
  landed = hash;
  if (at.kind === "place") {
    ({ place } = at);
    drawPlaces();
    drawList();
  } else if (at.kind === "repository") {
    choose(at.key, null);
  } else {
    // A tree in a folded season opens its season, so the cursor is seen where it lands.
    const open = unfolded.get(at.key) ?? new Set<Season>();
    open.add(at.season);
    unfolded.set(at.key, open);
    choose(at.key, at.path);
  }
}

/**
 * Widens the list until it shows `key`, so the list and the grove never hold different
 * cursors: the find is let go first, then the place opens to every repository.
 */
function reveal(key: string): void {
  const holds = (): boolean => shown().some((each) => each.key === key);
  if (holds()) {
    return;
  }
  query = "";
  find.value = "";
  if (!holds()) {
    place = "every";
  }
  drawPlaces();
  drawList();
}

/** Moves the cursor to `key`, inside it to `tree`, and the address with it. */
function choose(key: string, tree: string | null): void {
  cursor = key;
  inner = tree;
  reveal(key);
  markRows(cursor, true);
  drawOpen();
  if (tree === null) {
    list.focus({ preventScroll: true });
  } else {
    markInner(true);
  }
  follows();
}

/** Says where the cursor is in the address, so a reload or a link lands there. */
function follows(): void {
  const open = current();
  if (open === undefined) {
    return;
  }
  const address = addressed(open, inner);
  if (location.hash !== address) {
    history.replaceState(null, "", address);
    landed = address;
  }
}

globalThis.addEventListener("hashchange", land);

// ---- Keys. ----

/** Steps the cursor through `among` by `by`, in the list or inside a repository. */
function step(by: number, among: readonly Repository[] = shown()): void {
  if (inner !== null) {
    const trees = [...opened.querySelectorAll<HTMLElement>(".tree")].map(
      (each) => each.dataset["path"] ?? "",
    );
    inner = trees[stepped(trees.indexOf(inner), by, trees.length)] ?? inner;
    markInner(true);
    follows();
    return;
  }
  const at = among.findIndex((each) => each.key === cursor);
  const next = among[stepped(at, by, among.length)];
  if (next !== undefined) {
    cursor = next.key;
    markRows(cursor, true);
    drawOpen();
    follows();
  }
}

/** Walks the grove, which holds every repository, widening the list to follow it. */
function walk(by: number): void {
  if (inner !== null || grove === undefined) {
    return;
  }
  const among = grove.repositories;
  const next =
    among[
      stepped(
        among.findIndex((each) => each.key === cursor),
        by,
        among.length,
      )
    ];
  if (next !== undefined) {
    cursor = next.key;
    reveal(next.key);
    markRows(cursor, true);
    drawOpen();
    follows();
  }
  stand.focus({ preventScroll: true });
}

/** Lets go of a find left in place once Joel has stepped out of its field. */
function letGo(): void {
  if (query !== "") {
    query = "";
    find.value = "";
    drawList();
  }
}

function goIn(): void {
  const first = opened.querySelector<HTMLElement>(".tree")?.dataset["path"];
  if (inner === null && first !== undefined) {
    inner = first;
    markInner(true);
    follows();
  }
}

function goOut(): void {
  if (inner !== null) {
    inner = null;
    markInner(false);
    list.focus();
    follows();
  }
}

/** Opens or folds the open repository's resting and gone trees, and lists or shortens the rest. */
function fold(): void {
  const open = current();
  if (open === undefined) {
    return;
  }
  listing.listWhole(open.key, !whole.has(open.key));
  // Redrawing the drawer replaces the tree that held focus, so focus goes back to it, or to
  // the list when the cursor stood on a tree that has just folded away.
  const inside = opened.contains(document.activeElement);
  drawOpen();
  if (inside && inner !== null && opened.querySelector(".tree[aria-current=true]") !== null) {
    markInner(true);
  } else if (inside) {
    inner = null;
    markInner(false);
    list.focus({ preventScroll: true });
    follows();
  }
}

const actions: Actions = {
  j: () => step(1),
  ArrowDown: () => step(1),
  k: () => step(-1),
  ArrowUp: () => step(-1),
  g: () => step(Number.NEGATIVE_INFINITY),
  Home: () => step(Number.NEGATIVE_INFINITY),
  G: () => step(Number.POSITIVE_INFINITY),
  End: () => step(Number.POSITIVE_INFINITY),
  l: () => walk(1),
  ArrowRight: () => walk(1),
  h: () => walk(-1),
  ArrowLeft: () => walk(-1),
  Enter: goIn,
  "-": goOut,
  z: fold,
  y: () => copyFrom(current(), inner, asof),
  o: () => openOutward(current(), inner),
  Escape: letGo,
  "/": () => {
    find.focus();
    find.select();
  },
};

listen({
  help: keys,
  actions,
  // The rows and trees are the cursor, not controls of their own; a link in a tree is.
  answersAt: (target) => {
    const control = target.closest("a, button, select, summary");
    return control === null || control.matches("#grove a, #list a");
  },
});

find.addEventListener("input", () => {
  query = find.value;
  drawList();
});
find.addEventListener("keydown", (event) => {
  if (event.key === "Enter" || event.key === "Escape") {
    event.preventDefault();
    const first = shown()[0];
    if (event.key === "Enter" && first !== undefined) {
      cursor = first.key;
      markRows(cursor, true);
      drawOpen();
      follows();
    }
    list.focus();
  }
});

for (const region of [stand, list]) {
  region.addEventListener("click", (event) => {
    const option = event.target instanceof Element ? event.target.closest("[role=option]") : null;
    if (!(option instanceof HTMLElement) || option.dataset["key"] === undefined) {
      return;
    }
    event.preventDefault();
    choose(option.dataset["key"], null);
  });
}

opened.addEventListener("click", (event) => {
  // The button under a folded or shortened season lists every tree, as z does, from a tap; it
  // only opens, and the tree the cursor is on stays listed, so focus goes to it or the drawer.
  const open = current();
  const unfold = event.target instanceof Element ? event.target.closest(".unfold") : null;
  if (unfold !== null && open !== undefined) {
    listing.listWhole(open.key, true);
    drawOpen();
    keepIn(opened, inner);
    return;
  }
  const tree = event.target instanceof Element ? event.target.closest(".tree") : null;
  if (
    tree instanceof HTMLElement &&
    event.target instanceof Element &&
    event.target.closest("a") === null
  ) {
    inner = tree.dataset["path"] ?? null;
    markInner(true);
    follows();
  }
});

/** Shows the place a link in the places row names. */
function toPlace(event: Event): void {
  if (!(event.currentTarget instanceof HTMLAnchorElement)) {
    return;
  }
  event.preventDefault();
  place = event.currentTarget.dataset["place"] === "growing" ? "growing" : "every";
  drawPlaces();
  drawList();
  list.focus();
}

for (const link of document.querySelectorAll<HTMLAnchorElement>(".places [data-place]")) {
  link.addEventListener("click", toPlace);
}

// A tapped key at the head acts as pressed; the list takes the keys unless the act gave them on.
pressHead({ ...actions, Return: goIn, "?": () => toggle(keys) }, () => {
  if (document.activeElement?.closest(".head .keys")) {
    list.focus({ preventScroll: true });
  }
});

themeSwitch();

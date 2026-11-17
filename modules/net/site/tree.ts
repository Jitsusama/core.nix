// The panel's tree of every device: each floor's eero, and what's on it,
// walked by its own keys as a desktop's tree is.

import { glyph, h } from "./elements.ts";
import { type Device, type Network, worthALook } from "./model.ts";
import { linkBetween, type Shape } from "./shape.ts";
import { el } from "./svg.ts";
import { dbm, plural, v4 } from "./words.ts";

/** The item the tree's keys are on, the one it can be tabbed to. */
const FOCUSED = "[tabindex='0']";
const SHUT = "[aria-expanded='false']";
const ITEM = "[role=treeitem]";

/** What a tree item says beside its name: its devices, its signal or its address. */
function factOf(network: Network, shape: Shape, device: Device, kids: number): string {
  if (device.roles.includes("access-point")) {
    return plural(kids, "device");
  }
  const link = linkBetween(network, device.id, shape.via(device));
  return link?.signalDbm === undefined ? (v4(device)[0] ?? "") : `${dbm(link.signalDbm)} dBm`;
}

/** One device's item, at `level`, holding `kids` when it has any. */
function leaf(
  at: { readonly network: Network; readonly shape: Shape; readonly shut: ReadonlySet<string> },
  device: Device,
  level: number,
  kids: readonly Device[] = [],
): HTMLElement {
  const fact = factOf(at.network, at.shape, device, kids.length);
  const worth = worthALook(at.network, device.id).length > 0;
  // Named alone, so a reader landing on a floor hears the floor, not every
  // device under it.
  const label = [device.name, fact, worth ? "worth a look" : null].filter(Boolean).join(", ");
  // Divs, not a list, since the tree's roles say what a list's would.
  const item = h("div", {
    role: "treeitem",
    "data-id": device.id,
    tabindex: "-1",
    "aria-level": level,
    "aria-label": label,
  });
  const twisty = el("svg", { viewBox: "0 0 10 10", class: "twisty", "aria-hidden": "true" });
  if (kids.length > 0) {
    el("path", { d: "M3 1.5L7 5l-4 3.5" }, twisty);
  }
  const said = h(
    "span",
    { class: `fact${worth ? " look" : ""}` },
    worth ? glyph(true) : null,
    fact,
    worth ? h("span", { class: "sr" }, ", worth a look") : null,
  );
  const what = h("span", { class: "what" }, h("span", { class: "name" }, device.name), said);
  item.append(h("div", { class: "ti", style: `--level: ${level - 1}` }, twisty, what));
  if (kids.length > 0) {
    item.setAttribute("aria-expanded", at.shut.has(device.id) ? "false" : "true");
    const group = h("div", { role: "group" });
    for (const kid of kids) {
      group.append(leaf(at, kid, level + 1));
    }
    item.append(group);
  }
  return item;
}

/** Draws the tree again, keeping which branches were shut and where focus was. */
function drawTree(tree: HTMLElement, network: Network, shape: Shape): void {
  const shut = new Set(
    [...tree.querySelectorAll<HTMLElement>(SHUT)].flatMap((item) => item.dataset["id"] ?? []),
  );
  const focused = tree.querySelector<HTMLElement>(FOCUSED)?.dataset["id"];
  tree.replaceChildren();
  for (const floor of shape.floors) {
    const kids = [...floor.switches, ...floor.devices];
    if (shape.modem !== undefined && floor.point.id === shape.gateway?.id) {
      kids.unshift(shape.modem);
    }
    tree.append(leaf({ network, shape, shut }, floor.point, 1, kids));
  }
  const first =
    tree.querySelector(`[data-id="${CSS.escape(focused ?? "")}"]`) ?? tree.querySelector(ITEM);
  first?.setAttribute("tabindex", "0");
}

/** Marks the item of the device opened, if any. */
function markOpened(tree: HTMLElement, opened: string | null): void {
  for (const item of tree.querySelectorAll<HTMLElement>(".ti")) {
    item.classList.toggle("opened", item.parentElement?.dataset["id"] === opened);
  }
}

const shownItems = (tree: HTMLElement): HTMLElement[] =>
  [...tree.querySelectorAll<HTMLElement>(ITEM)].filter(
    (item) => item.parentElement?.closest(SHUT) === null,
  );

function focusItem(tree: HTMLElement, item: Element | null | undefined): void {
  if (!(item instanceof HTMLElement)) {
    return;
  }
  for (const each of tree.querySelectorAll(FOCUSED)) {
    each.setAttribute("tabindex", "-1");
  }
  item.setAttribute("tabindex", "0");
  item.focus();
}

/** Opens a shut branch, or steps into an open one. */
function opens(tree: HTMLElement, item: HTMLElement): void {
  const expanded = item.getAttribute("aria-expanded");
  if (expanded === "false") {
    item.setAttribute("aria-expanded", "true");
  } else if (expanded === "true") {
    focusItem(tree, item.querySelector(ITEM));
  }
}

/** Shuts an open branch, or climbs to the one holding it. */
function shuts(tree: HTMLElement, item: HTMLElement): void {
  if (item.getAttribute("aria-expanded") === "true") {
    item.setAttribute("aria-expanded", "false");
  } else {
    focusItem(tree, item.parentElement?.closest(ITEM));
  }
}

/** What each of the tree's keys does at an item, by the item it moves to. */
const MOVES: {
  readonly [key: string]: (list: HTMLElement[], at: number) => HTMLElement | undefined;
} = {
  j: (list, at) => list[at + 1],
  ArrowDown: (list, at) => list[at + 1],
  k: (list, at) => list[at - 1],
  ArrowUp: (list, at) => list[at - 1],
  g: (list) => list[0],
  Home: (list) => list[0],
  G: (list) => list.at(-1),
  End: (list) => list.at(-1),
};

/** What each of the tree's keys does to a branch. */
const TURNS: { readonly [key: string]: (tree: HTMLElement, item: HTMLElement) => void } = {
  l: opens,
  ArrowRight: opens,
  h: shuts,
  ArrowLeft: shuts,
};

/** Answers the tree's keys and clicks, opening a device with `open`. */
function heedTree(tree: HTMLElement, open: (id: string) => void): void {
  tree.addEventListener("keydown", (event) => {
    const item = event.target instanceof Element ? event.target.closest(ITEM) : null;
    if (!(item instanceof HTMLElement) || event.altKey || event.ctrlKey || event.metaKey) {
      return;
    }
    const { key } = event;
    if (Object.hasOwn(MOVES, key)) {
      const list = shownItems(tree);
      focusItem(tree, MOVES[key]?.(list, list.indexOf(item)));
    } else if (Object.hasOwn(TURNS, key)) {
      TURNS[key]?.(tree, item);
    } else if (key === "Enter" && item.dataset["id"] !== undefined) {
      open(item.dataset["id"]);
      focusItem(tree, item);
    } else {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
  });
  tree.addEventListener("click", (event) => {
    const target = event.target instanceof Element ? event.target : null;
    const item = target?.closest(ITEM);
    if (!(item instanceof HTMLElement)) {
      return;
    }
    if (target?.closest(".twisty") !== null && item.hasAttribute("aria-expanded")) {
      const wasOpen = item.getAttribute("aria-expanded") === "true";
      item.setAttribute("aria-expanded", wasOpen ? "false" : "true");
    } else if (item.dataset["id"] !== undefined) {
      open(item.dataset["id"]);
    }
    focusItem(tree, item);
  });
}

export { drawTree, heedTree, markOpened };

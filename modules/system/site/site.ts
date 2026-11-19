// Your machines, now: each machine Joel runs, and what it carries of the
// mycelium, the network and the code, as the system daemon holds them. A
// headline counts the machines and says how they stand, a lede names what
// is worth a look, the bridge draws them, each a pier, and each machine
// has a section of its own below, with the other three parts after them.
// It watches the daemon's summary, parts and things over the sites'
// server, so a machine redraws the moment its owner's answer moves, and
// the line under the headline says whether what it shows is live and
// since when. The cursor is focus: j and k move it through every row, and
// Return follows the link it is on.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import { hasOwnKeys, listen, toggle } from "../../local-sites/kit/keys.ts";
import { Held } from "../../local-sites/kit/records.ts";
import { clockTime, said } from "../../local-sites/kit/status.ts";
import { themeSwitch } from "../../local-sites/kit/theme.ts";
import { askedOf, landingOf } from "./address.ts";
import { bridgeOf } from "./bridge.ts";
import { headlineOf, legend, MACHINES_DRAWN, machinesOf, partsOf, placesOf } from "./draw.ts";
import { machineHeadline, machineLede, whenSaid } from "./machine.ts";
import { allTold, type System, systemOf } from "./model.ts";

/** The element `selector` finds, which the page's markup always holds. */
function element<T extends Element>(selector: string, kind: new () => T): T {
  const found = document.querySelector(selector);
  if (!(found instanceof kind)) {
    throw new Error(`the page has no ${selector}, which the system page draws into`);
  }
  return found;
}

const list = element("#parts", HTMLDivElement);
const title = element("#headline", HTMLHeadingElement);
const ledeLine = element("#lede", HTMLParagraphElement);
const standingLine = element("#standing", HTMLParagraphElement);
const standingWords = element("#standing-words", HTMLSpanElement);
const bridge = element("#bridge", HTMLElement);
const places = element("#places", HTMLElement);
const note = element("#note", HTMLParagraphElement);
const keys = element("#keys", HTMLDialogElement);
const mast = element(".mast", HTMLElement);

// Whatever the keys land on stays clear of the masthead, however many lines
// it wraps to. On a phone it scrolls away with the page, so nothing need
// clear it there.
const clearOfMast = new ResizeObserver(() => {
  const sticks = getComputedStyle(mast).position === "sticky";
  // Its height rounded up, since offsetHeight rounds a fraction down and
  // leaves a row that sliver under it.
  document.documentElement.style.scrollPaddingBlockStart = sticks
    ? `${Math.ceil(mast.getBoundingClientRect().height)}px`
    : "0px";
});
clearOfMast.observe(mast);

const KINDS = ["summary", "part", "machine"] as const;
// What each watch holds: the Held its follow keeps, which is the one it
// folds each answer into, so the page reads that one and makes none.
const held: { [kind in (typeof KINDS)[number]]: Held } = {
  summary: new Held(),
  part: new Held(),
  machine: new Held(),
};
const standings = new Map<string, Standing>();
// The watches that have told their records whole, since a watch answers
// every record it holds first, none included.
const told = new Set<string>();
// How long the page waits for every watch before it draws what it has, so
// one stuck connecting never leaves it blank.
const PATIENCE = 5000;
let waiting = true;
setTimeout(() => {
  waiting = false;
  draw();
}, PATIENCE);
// What the note last said of problems, so a redraw that finds the same ones
// leaves the note, and what y said there, alone.
let problemsSaid = "";
// The last thing each watch couldn't read, said until it reads another.
const heard = new Map<string, string>();

let system: System | undefined;
// The row the cursor was last on, by its key, kept across a redraw.
let cursor: string | null = null;
// Each drawn region's markup as last drawn, since the browser's own
// serialization of it never reads back the same.
let drawnParts = "";
let drawnBridge = "";
let drawnPlaces = "";
/** The machine the address asks for, until the cursor has landed for it. */
let asked = askedOf(location.hash);

/** A minute's milliseconds, how often the ties' times are said again. */
const MINUTE = 60_000;

/** A time as the ties say it, from now. */
function when(at: string): string {
  return whenSaid(at, Date.now());
}

/** The worst of the three watches' states, the one the page says. */
function standing(): Standing | undefined {
  const order = ["broken", "stale", "connecting", "live"];
  return [...standings.values()].sort(
    (one, other) => order.indexOf(one.state) - order.indexOf(other.state),
  )[0];
}

/** Says where and when the system was read, and whether that is live. */
function sayWhen(): void {
  const now = standing();
  const words: string[] = [];
  if (system !== undefined) {
    const at = Date.parse(system.readAt);
    words.push(
      system.machine,
      Number.isNaN(at) ? "read at an unknown time" : `read ${clockTime(at)}`,
    );
  }
  if (now !== undefined) {
    words.push(now.state === "live" ? "live" : said(now));
  }
  standingWords.textContent =
    words.length === 0 ? "Reaching the system daemon." : `${words.join(" · ")}.`;
  standingLine.dataset["state"] = now?.state ?? "connecting";
  const problems = [...(system?.unread ?? []), ...heard.values()];
  const unreadNote = problems.length > 0 ? `Couldn't read: ${problems.join("; ")}.` : "";
  if (unreadNote !== problemsSaid) {
    problemsSaid = unreadNote;
    note.textContent = unreadNote;
  }
}

/** Every row the keys step through, in reading order. */
function rows(): HTMLAnchorElement[] {
  return [...list.querySelectorAll<HTMLAnchorElement>("a[data-row]")];
}

/** Puts the cursor on `row`, which focus is. */
function land(row: HTMLAnchorElement | undefined): void {
  if (row === undefined) {
    return;
  }
  cursor = row.dataset["row"] ?? null;
  // Focus alone would centre a row out of sight; scrolled to its nearest edge
  // instead, a step moves the page no further than it must.
  row.focus({ preventScroll: true });
  row.scrollIntoView({ block: "nearest" });
}

/**
 * Redraws the machines, only when they read differently, and keeps the
 * cursor on the row it was on, or the one now in its place, so a machine
 * moving never moves what Joel is reading.
 */
function drawParts(read: System): void {
  const drawn = partsOf(read, when);
  if (drawn === drawnParts) {
    return;
  }
  drawnParts = drawn;
  const all = rows();
  const focused = all.findIndex((row) => row === document.activeElement);
  list.innerHTML = drawn;
  if (focused < 0) {
    return;
  }
  const now = rows();
  const same = now.find((row) => row.dataset["row"] === cursor);
  (same ?? now[Math.min(focused, now.length - 1)])?.focus({ preventScroll: true });
}

/**
 * Redraws `into` with `drawn` only when it reads differently from `before`,
 * keeping focus on the link marked `attribute` it was on, so the places
 * never take the cursor from under Joel. Says what it drew.
 */
function redraw(into: Element, drawn: string, before: string, attribute: string): string {
  if (drawn === before) {
    return before;
  }
  const focused = document.activeElement;
  const was =
    focused instanceof HTMLElement && into.contains(focused)
      ? focused.getAttribute(attribute)
      : null;
  into.innerHTML = drawn;
  if (was !== null) {
    into
      .querySelector<HTMLElement>(`[${attribute}="${CSS.escape(was)}"]`)
      ?.focus({ preventScroll: true });
  }
  return drawn;
}

/** Marks the place of the section the cursor is in, so the top says where Joel is. */
function markPlace(): void {
  const here = document.activeElement?.closest<HTMLElement>("section[id]")?.id;
  for (const place of places.querySelectorAll<HTMLAnchorElement>("a[data-place]")) {
    if (place.dataset["place"] === here) {
      place.setAttribute("aria-current", "location");
    } else {
      place.removeAttribute("aria-current");
    }
  }
}

/**
 * Puts the cursor where the machine the address asks for lands, saying why
 * when that isn't its own row. One not found while a watch is still to
 * answer waits for a later draw, since its record may yet come.
 */
function landAsked(read: System): void {
  if (asked === undefined) {
    return;
  }
  const landing = landingOf(
    machinesOf(read).map((thing) => thing.name),
    asked,
  );
  if (landing.row === undefined && !allTold(KINDS, told, standings)) {
    return;
  }
  asked = undefined;
  if (landing.note !== undefined) {
    note.textContent = landing.note;
  }
  if (landing.row !== undefined) {
    land(
      list.querySelector<HTMLAnchorElement>(`a[data-row="${CSS.escape(landing.row)}"]`) ??
        undefined,
    );
  }
}

function draw(): void {
  if (waiting && !allTold(KINDS, told, standings)) {
    sayWhen();
    return;
  }
  system = systemOf(held.summary, held.part, held.machine);
  sayWhen();
  if (system === undefined) {
    return;
  }
  const heading = machineHeadline(system);
  title.innerHTML = headlineOf(heading);
  const worst = machineLede(system);
  // The lede keeps its lines while a part is still to come, since that
  // part may be the one to name; only empty once every part has told.
  ledeLine.hidden = worst === undefined && system.untold.length === 0;
  ledeLine.textContent = worst ?? "";
  document.title = `${heading.machines}, ${heading.stand}. Your machines, now`;
  drawnBridge = redraw(bridge, bridgeOf(machinesOf(system)), drawnBridge, "data-row");
  drawnPlaces = redraw(places, placesOf(system), drawnPlaces, "data-place");
  drawParts(system);
  landAsked(system);
  markPlace();
}

for (const kind of KINDS) {
  held[kind] = follow(
    { feed: "system", kind },
    {
      moved: () => {
        told.add(kind);
        draw();
      },
      stands: (now) => {
        standings.set(kind, now);
        // A watch that breaks or goes stale first may be the last one the
        // page was waiting on.
        draw();
      },
      problem: (what) => {
        heard.set(kind, `${kind}: ${what}`);
        sayWhen();
      },
    },
  ).held;
}

// The ties say their times from now, so a page left open past midnight
// draws again each minute rather than saying yesterday's update as today's.
// A draw changes only the regions whose markup moved.
setInterval(draw, MINUTE);

/** Moves the cursor `by` rows, to either end when `by` is infinite. */
function move(by: number): void {
  const all = rows();
  if (all.length === 0) {
    return;
  }
  const at = all.findIndex((row) => row === document.activeElement);
  if (by === Number.POSITIVE_INFINITY) {
    land(all.at(-1));
  } else if (by === Number.NEGATIVE_INFINITY || at < 0) {
    land(all[0]);
  } else {
    land(all[Math.max(0, Math.min(all.length - 1, at + by))]);
  }
}

/** Puts the cursor on the first row of the section `id` names. */
function toSection(id: string): void {
  const section = list.querySelector(`#${CSS.escape(id)}`);
  land(section?.querySelector<HTMLAnchorElement>("a[data-row]") ?? undefined);
}

/** Copies `command`, and says whether it could. */
function copy(command: string): void {
  navigator.clipboard.writeText(command).then(
    () => {
      note.textContent = `Copied ${command}.`;
    },
    (error: unknown) => {
      note.textContent = `Couldn't copy (${String(error)}); the command is ${command}.`;
    },
  );
}

/**
 * Copies the first command of the machine or part the cursor is in, or says
 * why there is none, so y never copies a command the cursor isn't on.
 */
function copyHere(): void {
  const section = document.activeElement?.closest<HTMLElement>("[data-part]");
  if (section === null || section === undefined) {
    note.textContent =
      "Nothing to copy here: y copies a command of the machine or part the cursor is on.";
    return;
  }
  const command = section.querySelector<HTMLButtonElement>("button.copy")?.dataset["command"];
  if (command === undefined) {
    note.textContent = "Nothing to copy here: this one has no command.";
    return;
  }
  copy(command);
}

const machineJumps = Array.from({ length: Math.min(MACHINES_DRAWN, 9) }, (_, at) => [
  String(at + 1),
  () => toSection(`machine-${at + 1}`),
]);

listen({
  help: keys,
  actions: {
    j: () => move(1),
    ArrowDown: () => move(1),
    k: () => move(-1),
    ArrowUp: () => move(-1),
    g: () => move(Number.NEGATIVE_INFINITY),
    Home: () => move(Number.NEGATIVE_INFINITY),
    G: () => move(Number.POSITIVE_INFINITY),
    End: () => move(Number.POSITIVE_INFINITY),
    y: copyHere,
    ...Object.fromEntries(machineJumps),
    "0": () => toSection("across"),
  },
  // A row is the cursor, not a control of its own, so the keys answer on one.
  answersAt: (target) => target.closest("a[data-row]") !== null || !hasOwnKeys(target),
});

list.addEventListener("focusin", (event) => {
  if (event.target instanceof HTMLAnchorElement && event.target.dataset["row"] !== undefined) {
    cursor = event.target.dataset["row"];
  }
  markPlace();
});

// A place goes to its section's first row, putting the cursor there.
places.addEventListener("click", (event) => {
  const link = event.target instanceof Element ? event.target.closest("a[data-place]") : null;
  if (!(link instanceof HTMLAnchorElement) || link.dataset["place"] === undefined) {
    return;
  }
  event.preventDefault();
  toSection(link.dataset["place"]);
});

list.addEventListener("click", (event) => {
  const button = event.target instanceof Element ? event.target.closest(".copy") : null;
  if (button instanceof HTMLButtonElement && button.dataset["command"] !== undefined) {
    copy(button.dataset["command"]);
  }
});

// A link to another machine's address, once the page is open, lands there too.
globalThis.addEventListener("hashchange", () => {
  asked = askedOf(location.hash);
  if (system !== undefined) {
    landAsked(system);
    markPlace();
  }
});

element("#help", HTMLButtonElement).addEventListener("click", () => toggle(keys));
element("#legend", HTMLUListElement).innerHTML = legend();

themeSwitch();

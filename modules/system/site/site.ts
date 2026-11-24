// Your machines, now: each machine Joel runs, and what it carries of the
// mycelium, the network and the code, as the system daemon holds them. The
// head's lede counts the machines and says how they stand, a list under it
// names everything worth a look, the bridge draws them, each a pier, and
// each machine has a section of its own below, with the other three parts
// after them.
// It watches the daemon's summary, parts and things over the sites'
// server, so a machine redraws the moment its owner's answer moves, and
// the head's standing line says whether what it shows is live and since
// when. The cursor is focus: j and k move it through every row, and
// Return follows the link it is on.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import { hasOwnKeys, listen, toggle } from "../../local-sites/kit/keys.ts";
import { Held } from "../../local-sites/kit/records.ts";
import { clockTime, said, tell } from "../../local-sites/kit/status.ts";
import { themeSwitch } from "../../local-sites/kit/theme.ts";
import { askedOf, landingOf, recordsOf, type Watch } from "./address.ts";
import { bridgeOf } from "./bridge.ts";
import { cursorOver } from "./cursor.ts";
import {
  awayOf,
  headlineOf,
  legend,
  looksDrawn,
  MACHINES_DRAWN,
  NOT_KNOWN,
  NOTHING,
  partsOf,
  placesOf,
  reachedOf,
} from "./draw.ts";
import { looksOf } from "./looks.ts";
import { machineHeadline, whenSaid } from "./machine.ts";
import { allTold, type System, systemOf, type Thing } from "./model.ts";

/** The element `selector` finds, which the page's markup always holds. */
function element<T extends Element>(selector: string, kind: new () => T): T {
  const found = document.querySelector(selector);
  if (!(found instanceof kind)) {
    throw new Error(`the page has no ${selector}, which the system page draws into`);
  }
  return found;
}

const list = element("#parts", HTMLDivElement);
const title = element("#headline", HTMLParagraphElement);
const looks = element("#looks", HTMLDivElement);
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
// What the note last said, of problems and of a landing, so a redraw that
// finds the same leaves the note, and what y said there, alone.
// None once a landing has news to say, which is said even when it reads as
// the last.
let noteSaid: string | undefined = "";
// What the last landing from an address had to say, beside the problems.
let landingNote = "";
// The last thing each watch couldn't read, said until it reads another.
const heard = new Map<string, string>();

let system: System | undefined;
const cursor = cursorOver("#looks a[data-row], #parts a[data-row]", list);
const { land, move, toSection } = cursor;
// Each drawn region's markup as last drawn, since the browser's own
// serialization of it never reads back the same.
let drawnParts = "";
// None yet, so the first draw always settles whether the list shows.
let drawnLooks: string | undefined;
let drawnBridge = "";
let drawnPlaces = "";
/** The machine the address asks for, until the cursor has landed for it. */
let asked: string | undefined;
// When the page began waiting for the machine asked for, so the wait for
// records that never come ends in a note rather than in silence.
let askedSince = 0;

/** A minute's milliseconds, how often the ties' times are said again. */
const MINUTE = 60_000;

/** A look's link to a machine drawn here, or to those out of reach, its section's id caught. */
const MACHINE_HERE = /^#(machine-\d+|away)$/u;

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
  const state = now?.state ?? "connecting";
  const spoken = words.length === 0 ? "Reaching the system daemon." : `${words.join(" · ")}.`;
  // The words are the head's one live region, so tell writes them only when they change.
  tell(standingWords, spoken, state);
  standingLine.dataset["state"] = state;
  const problems = [...(system?.unread ?? []), ...heard.values()];
  const unreadNote = problems.length > 0 ? `Couldn't read: ${problems.join("; ")}.` : "";
  const noted = [unreadNote, landingNote].filter((line) => line !== "").join(" ");
  if (noted !== noteSaid) {
    noteSaid = noted;
    note.textContent = noted;
  }
}

/**
 * Redraws what is worth a look, only when it reads differently, the cursor
 * kept on its row or the one now in its place. A note a left goes once the
 * list is drawn again, since what it said of the list may no longer hold.
 */
function drawLooks(read: System): void {
  const drawn = looksDrawn(looksOf(read, MACHINES_DRAWN));
  if (drawn === drawnLooks) {
    return;
  }
  drawnLooks = drawn;
  cursor.across(() => {
    looks.innerHTML = drawn;
  });
  const left = note.textContent ?? "";
  if (left.startsWith(NOT_KNOWN) || left.startsWith(NOTHING)) {
    noteSaid = undefined;
    sayWhen();
  }
}

/** Redraws the machines, only when they read differently, the cursor kept. */
function drawParts(read: System): void {
  const drawn = partsOf(read, when);
  if (drawn === drawnParts) {
    return;
  }
  drawnParts = drawn;
  cursor.across(() => {
    list.innerHTML = drawn;
  });
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
 * The watches that bring the records naming machines, the part and the
 * machines' own, as one: told once both have, live only while both are.
 */
function machineWatch(): Watch {
  const kinds = ["part", "machine"];
  const notLive = kinds.map((kind) => standings.get(kind)).find((now) => now?.state !== "live");
  return {
    told: kinds.every((kind) => told.has(kind)),
    state: notLive === undefined ? "live" : notLive?.state,
  };
}

/**
 * Takes the machine the address asks for, if it does, and answers it on the
 * next draw that can, or once the page has waited its patience for it.
 */
function ask(): void {
  asked = askedOf(location.hash);
  if (asked === undefined) {
    return;
  }
  // What the last landing said answers another address, not this one.
  if (landingNote !== "") {
    landingNote = "";
    sayWhen();
  }
  const since = Date.now();
  askedSince = since;
  setTimeout(() => {
    // Only for the ask it was set for; a later one keeps its own patience.
    if (system !== undefined && askedSince === since) {
      landAsked(system, true);
    }
  }, PATIENCE);
}

/** Ends the wait for the machine asked for, clearing its address. */
function answered(): void {
  const was = asked;
  asked = undefined;
  // A newer address, followed and not yet heard, is left for its own turn.
  if (askedOf(location.hash) !== was) {
    return;
  }
  // Cleared, so following the same address again is a change the page
  // hears; the URL is mended whole, so no path can make it another site's.
  const here = new URL(location.href);
  here.hash = "";
  history.replaceState(null, "", here);
}

/**
 * Puts the cursor where the machine the address asks for lands, saying why
 * when that isn't its own row. One not found while its record may yet come
 * waits for a later draw, and past the page's patience says so and still
 * lands if it comes before Joel moves.
 */
function landAsked(read: System, overdue = Date.now() - askedSince >= PATIENCE): void {
  if (asked === undefined) {
    return;
  }
  const records = recordsOf(read, machineWatch(), overdue);
  const names = (things: readonly Thing[]): string[] => things.map((thing) => thing.name);
  const landing = landingOf(names(reachedOf(read)), asked, records, names(awayOf(read)));
  if (landing.waits) {
    return;
  }
  const late = landing.row === undefined && records === "overdue";
  if (!late) {
    answered();
  }
  const words = landing.note ?? "";
  // A landing is news, said even when it reads as the last; a late one
  // said again on each draw is not.
  if (!late || words !== landingNote) {
    landingNote = words;
    noteSaid = undefined;
    sayWhen();
  }
  if (landing.row === undefined) {
    return;
  }
  const row = list.querySelector<HTMLAnchorElement>(`a[data-row="${CSS.escape(landing.row)}"]`);
  // The whole machine brought into sight first, then its name, so arriving
  // shows what it carries and not only its name at the foot of the screen.
  row?.closest("section")?.scrollIntoView({ block: "nearest" });
  land(row ?? undefined);
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
  drawLooks(system);
  document.title = `${heading.machines}, ${heading.stand}. Your machines, now`;
  const away = awayOf(system).length;
  drawnBridge = redraw(bridge, bridgeOf(reachedOf(system), away), drawnBridge, "data-row");
  drawnPlaces = redraw(places, placesOf(system), drawnPlaces, "data-place");
  drawParts(system);
  landAsked(system);
  markPlace();
}

ask();
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

/**
 * Puts the cursor on the next thing worth a look, or says there is none, as
 * a on the map does, but only once the list says so itself.
 */
function nextLook(): void {
  if (cursor.next(looks)) {
    return;
  }
  const answer = looks.querySelector(".clear") === null ? NOT_KNOWN : NOTHING;
  // What the note said stays after it, as which record couldn't be read.
  note.textContent = [answer, noteSaid ?? ""].filter((line) => line !== "").join(" ");
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
    a: nextLook,
    ...Object.fromEntries(machineJumps),
    "0": () => toSection("across"),
  },
  // A row is the cursor, not a control of its own, so the keys answer on one.
  answersAt: (target) => target.closest("a[data-row]") !== null || !hasOwnKeys(target),
});

/**
 * Joel's own move, by key, pointer, wheel or touch, ends a landing still
 * waiting, so a record arriving late never takes the page from under him,
 * and puts away what the last landing said. Read from his input, not from
 * focus, which the page moves itself when it redraws. A modifier held
 * alone, or a chord meant for the browser or the desktop, isn't a move,
 * and nor is an event a script made.
 */
function joelMoved(event: Event): void {
  if (!event.isTrusted) {
    return;
  }
  if (
    event instanceof KeyboardEvent &&
    (MODIFIERS.has(event.key) || event.ctrlKey || event.altKey || event.metaKey)
  ) {
    return;
  }
  if (asked !== undefined) {
    answered();
  }
  if (landingNote !== "") {
    landingNote = "";
    sayWhen();
  }
}
/** The keys that only change what another key means. */
const MODIFIERS = new Set([
  "Shift",
  "Control",
  "Alt",
  "AltGraph",
  "Meta",
  "Super",
  "Hyper",
  "CapsLock",
  "Fn",
]);
for (const input of ["keydown", "pointerdown", "wheel", "touchstart"]) {
  // Caught first, so what a key itself says in the note comes after.
  document.addEventListener(input, joelMoved, { capture: true, passive: true });
}

/** Keeps the row focus lands on as the cursor, and marks its place. */
function focusedRow(event: FocusEvent): void {
  cursor.keep(event.target);
  markPlace();
}
looks.addEventListener("focusin", focusedRow);
list.addEventListener("focusin", focusedRow);

// A look at a machine drawn here goes to its section's first row, putting
// the cursor there, rather than only scrolling the page to it.
looks.addEventListener("click", (event) => {
  const link = event.target instanceof Element ? event.target.closest("a[data-row]") : null;
  const id = link?.getAttribute("href")?.match(MACHINE_HERE)?.[1];
  if (id === undefined) {
    return;
  }
  event.preventDefault();
  toSection(id);
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
  ask();
  if (system !== undefined) {
    landAsked(system);
  }
});

element("#help", HTMLButtonElement).addEventListener("click", () => toggle(keys));
element("#legend", HTMLUListElement).innerHTML = legend();

themeSwitch();

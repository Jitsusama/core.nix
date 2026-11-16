// Your system, now: the four parts of the system as the system daemon holds
// them, under a headline that counts what is worth a look and a lede that
// names the worst of it. It watches the daemon's summary, parts and things
// over the sites' server, so a part redraws the moment its owner's answer
// moves, and the line over the headline says whether what it shows is live
// and since when. The cursor is focus: j and k move it through every row,
// and Return follows the link it is on.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import { hasOwnKeys, listen, toggle } from "../../local-sites/kit/keys.ts";
import { Held } from "../../local-sites/kit/records.ts";
import { clockTime, said } from "../../local-sites/kit/status.ts";
import { themeSwitch } from "../../local-sites/kit/theme.ts";
import { headlineOf, legend, partsOf } from "./draw.ts";
import { headline, lede, type Part, type System, sentence, systemOf } from "./model.ts";

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
const note = element("#note", HTMLParagraphElement);
const keys = element("#keys", HTMLDialogElement);

const KINDS = ["summary", "part", "machine"] as const;
// What each watch holds: the Held its follow keeps, which is the one it
// folds each answer into, so the page reads that one and makes none.
const held: { [kind in (typeof KINDS)[number]]: Held } = {
  summary: new Held(),
  part: new Held(),
  machine: new Held(),
};
const standings = new Map<string, Standing>();
// The last thing each watch couldn't read, said until it reads another.
const heard = new Map<string, string>();

let system: System | undefined;
// The row the cursor was last on, by its key, kept across a redraw.
let cursor: string | null = null;
// The parts' markup as last drawn, since the browser's own serialization
// of it never reads back the same.
let drawnParts = "";

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
    words.push(system.machine, Number.isNaN(at) ? "read at an unknown time" : `read ${clockTime(at)}`);
  }
  if (now !== undefined) {
    words.push(now.state === "live" ? "live" : said(now));
  }
  standingLine.textContent = words.length === 0 ? "Reaching the system daemon." : words.join(" · ");
  standingLine.dataset.state = now?.state ?? "connecting";
  const problems = [...(system?.unread ?? []), ...heard.values()];
  note.textContent = problems.length > 0 ? `Couldn't read: ${problems.join("; ")}.` : "";
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
  cursor = row.dataset.row ?? null;
  row.focus();
  row.scrollIntoView({ block: "nearest" });
}

/**
 * Redraws the parts, only when they read differently, and keeps the cursor
 * on the row it was on, or the one now in its place, so a part moving never
 * moves what Joel is reading.
 */
function drawParts(read: System): void {
  const drawn = partsOf(read);
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
  const same = now.find((row) => row.dataset.row === cursor);
  (same ?? now[Math.min(focused, now.length - 1)])?.focus({ preventScroll: true });
}

function draw(): void {
  system = systemOf(held.summary, held.part, held.machine);
  sayWhen();
  if (system === undefined) {
    return;
  }
  const said = headline(system);
  title.innerHTML = headlineOf(said);
  const worst = lede(system);
  ledeLine.hidden = worst === undefined;
  ledeLine.textContent = worst ?? "";
  document.title = `${sentence(said)} Your system, now`;
  drawParts(system);
}

for (const kind of KINDS) {
  held[kind] = follow(
    { feed: "system", kind },
    {
      moved: draw,
      stands: (now) => {
        standings.set(kind, now);
        sayWhen();
      },
      problem: (what) => {
        heard.set(kind, `${kind}: ${what}`);
        sayWhen();
      },
    },
  ).held;
}

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

/** Puts the cursor on the heading of the part at `index`, counted from one. */
function toPart(index: number): void {
  land(rows().filter((row) => row.dataset.row?.startsWith("part:"))[index - 1]);
}

/** The part the cursor is in, or the first. */
function partHere(): Part | undefined {
  const at = document.activeElement?.closest<HTMLElement>("[data-part]")?.dataset.part;
  return system?.parts.find((part) => part.part === at) ?? system?.parts[0];
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

const jumps = Object.fromEntries(
  [1, 2, 3, 4].map((index) => [String(index), () => toPart(index)]),
);

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
    y: () => {
      const here = partHere();
      if (here !== undefined) {
        copy(here.deeper.command);
      }
    },
    ...jumps,
  },
  // A row is the cursor, not a control of its own, so the keys answer on one.
  answersAt: (target) => target.closest("a[data-row]") !== null || !hasOwnKeys(target),
});

list.addEventListener("focusin", (event) => {
  if (event.target instanceof HTMLAnchorElement && event.target.dataset.row !== undefined) {
    cursor = event.target.dataset.row;
  }
});

list.addEventListener("click", (event) => {
  const button = event.target instanceof Element ? event.target.closest(".copy") : null;
  if (button instanceof HTMLButtonElement && button.dataset.command !== undefined) {
    copy(button.dataset.command);
  }
});

// Each count in the headline goes to the first row of what it counts.
title.addEventListener("click", (event) => {
  const link = event.target instanceof Element ? event.target.closest("a[data-to]") : null;
  if (!(link instanceof HTMLAnchorElement)) {
    return;
  }
  event.preventDefault();
  const to = link.dataset.to;
  const all = rows();
  const first =
    to === "look"
      ? all.find((row) => row.closest(".part.watch, .part.bad") !== null && !row.closest("h2"))
      : to === "unread"
        ? all.find((row) => row.dataset.health === "unread")
        : all[0];
  land(first ?? all[0]);
});

element("#help", HTMLButtonElement).addEventListener("click", () => toggle(keys));
element("#legend", HTMLParagraphElement).innerHTML = legend();

themeSwitch();

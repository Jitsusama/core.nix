// What every document in the sites' home answers, once for all of them, so
// none carries a copy of its own: j and k step from part to part, g twice
// and G go to its ends, ? and F1 list its keys, the keys its head names can
// be pressed as well as typed, and the theme switch keeps its look. A
// document loads it as /local-sites/kit/document.js; its parts are the
// `.part` sections, each opening with an h2, and its list of keys is the
// dialog `#keys`.

import { listen, toggle } from "./keys.ts";
import { themeSwitch } from "./theme.ts";

/** How far below the top of the window a part counts as the one being read, past the head. */
const READING_LINE_PX = 80;

/** How soon a second g must follow the first to go to the top, as in less and vim. */
const TWICE_MS = 600;

/**
 * Which part is being read, given each part's top in the page in order and
 * how far the reading line is down the page: the last part starting at or
 * above it, or -1 above the first.
 */
function partAt(tops: readonly number[], line: number): number {
  return tops.findLastIndex((top) => top <= line);
}

/**
 * Which part a step goes from: the part last stepped to while its top is
 * still on screen, since a part near the end can't scroll up to the reading
 * line and would otherwise hold every step where it is, and otherwise the
 * part at the reading line. `tops` are measured from the window's top.
 */
function partFrom(last: number, tops: readonly number[], height: number): number {
  const top = tops[last];
  return top !== undefined && top >= 0 && top < height ? last : partAt(tops, READING_LINE_PX);
}

/** The part `by` on from `at`, held to the `count` parts there are. */
function stepped(at: number, by: number, count: number): number {
  return Math.max(0, Math.min(count - 1, at + by));
}

/** Whether a press at `now` follows one at `before` closely enough to count as twice. */
function twice(before: number, now: number): boolean {
  return now - before < TWICE_MS;
}

/** The part a document's reader is in and the way to the others. */
interface Parts {
  readonly step: (by: number) => void;
}

/** Wires stepping through `parts`, each taking focus at its heading when it's reached. */
function parts(sections: readonly HTMLElement[]): Parts {
  const headings = sections.map((section) => section.querySelector("h2"));
  for (const heading of headings) {
    if (heading) {
      heading.tabIndex = -1;
    }
  }
  let last = -1;
  const step = (by: number): void => {
    const tops = sections.map((section) => section.getBoundingClientRect().top);
    last = stepped(partFrom(last, tops, innerHeight), by, sections.length);
    sections[last]?.scrollIntoView({ block: "start" });
    headings[last]?.focus({ preventScroll: true });
  };
  return { step };
}

/** Makes each key the head names pressable, as a button doing what typing it does. */
function pressable(taps: Readonly<Record<string, () => void>>): void {
  for (const kbd of document.querySelectorAll<HTMLElement>(".head .keys kbd")) {
    const act = Object.hasOwn(taps, kbd.textContent) ? taps[kbd.textContent] : undefined;
    if (act) {
      kbd.setAttribute("role", "button");
      kbd.tabIndex = 0;
      kbd.addEventListener("click", act);
      kbd.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          act();
        }
      });
    }
  }
}

/** Goes to the document's end. */
function bottom(): void {
  scrollTo(0, document.body.scrollHeight);
}

/**
 * Whether a document's keys answer at `target`: anywhere but a select or a
 * summary, whose own keys come first. A link or a button takes no letters,
 * and a document is mostly links, so its keys answer there.
 */
function answersAt(target: Element): boolean {
  return target.closest("select, summary") === null;
}

/** Wires the document this runs in: its parts, its keys and its theme switch. */
function wire(): void {
  const help = document.getElementById("keys");
  const { step } = parts([...document.querySelectorAll<HTMLElement>(".part")]);
  let lastG = Number.NEGATIVE_INFINITY;
  const top = (): void => {
    const now = Date.now();
    if (twice(lastG, now)) {
      scrollTo(0, 0);
      lastG = Number.NEGATIVE_INFINITY;
    } else {
      lastG = now;
    }
  };
  const actions = { j: () => step(1), k: () => step(-1), g: top, G: bottom };
  // A document without its list of keys still steps; only ? has nothing to open.
  if (help instanceof HTMLDialogElement) {
    listen({ help, actions, answersAt });
    pressable({ j: actions.j, k: actions.k, "?": () => toggle(help) });
  } else {
    listen({ actions, answersAt });
    pressable({ j: actions.j, k: actions.k });
  }
  themeSwitch();
}

// Imported for its helpers where there is no page, as its tests do, it wires nothing.
if (typeof document !== "undefined") {
  wire();
}

export { partAt, partFrom, stepped, twice };

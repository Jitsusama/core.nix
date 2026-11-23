// What a page that steps through a list of things shares with the others:
// a step held to the list's ends, g twice to go to the top as in less and
// vim, the keys its head names made pressable, and where its keys answer. A
// document steps through its parts and the home through its sites.

import { HELP_KEYS } from "./keys.ts";

/** How soon a second g must follow the first to go to the top, as in less and vim. */
const TWICE_MS = 600;

/** The place `by` on from `at`, held to the `count` places there are. */
function stepped(at: number, by: number, count: number): number {
  return Math.max(0, Math.min(count - 1, at + by));
}

/** Whether a press at `now` follows one at `before` closely enough to count as twice. */
function twice(before: number, now: number): boolean {
  return now - before < TWICE_MS;
}

/**
 * A key's press that does `act` on its second press in a row, as g twice
 * goes to the top. A third press starts over, so ggg goes once, not twice.
 * `clock` says the time in milliseconds, the page's own unless a test's.
 */
function onTwice(act: () => void, clock: () => number = Date.now): () => void {
  let last = Number.NEGATIVE_INFINITY;
  return () => {
    const now = clock();
    if (twice(last, now)) {
      act();
      last = Number.NEGATIVE_INFINITY;
    } else {
      last = now;
    }
  };
}

/**
 * Whether a stepping page's keys answer at `target`: anywhere but a select
 * or a summary, whose own keys come first. A link or a button takes no
 * letters, and such a page is mostly links, its cursor one of them.
 */
function answersAt(target: Element): boolean {
  return target.closest("select, summary") === null;
}

/**
 * Makes each key the `kbd` elements matching `selector` name pressable, as
 * a button doing what typing it does, for a reader who taps or clicks, then
 * doing `afterwards`, such as giving the keys back to the list they step.
 * Return or Space on one does its act alone: the press goes no further,
 * where the page's own Return would open what its cursor is on.
 *
 * Each but the keys that only open the list of keys is marked data-tap, as
 * head.css reads to keep a key line showing on touch: a line whose only
 * tappable key is `?` tells a phone nothing it can do. A page writes
 * data-tap in its markup too, so the line is there at first paint rather
 * than arriving with the script and pushing the switch down.
 */
function pressable(
  selector: string,
  taps: Readonly<Record<string, () => void>>,
  afterwards: () => void = () => undefined,
): void {
  for (const kbd of document.querySelectorAll<HTMLElement>(selector)) {
    const tap = Object.hasOwn(taps, kbd.textContent) ? taps[kbd.textContent] : undefined;
    if (tap) {
      const act = (): void => {
        tap();
        afterwards();
      };
      kbd.setAttribute("role", "button");
      if (!HELP_KEYS.has(kbd.textContent)) {
        kbd.dataset["tap"] = "";
      }
      kbd.tabIndex = 0;
      kbd.addEventListener("click", act);
      kbd.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          event.stopPropagation();
          act();
        }
      });
    }
  }
}

export { answersAt, onTwice, pressable, stepped, twice };

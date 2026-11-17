// What every document in the sites' home answers, once for all of them, so
// none carries a copy of its own: j and k step from part to part, g twice
// and G go to its ends, ? and F1 list its keys, the keys its head names can
// be pressed as well as typed, and the theme switch keeps its look. A
// document loads it as /local-sites/kit/document.js; its parts are the
// `.part` sections, each opening with an h2, and its list of keys is the
// dialog `#keys`.

import { listen, toggle } from "./keys.ts";
import { answersAt, onTwice, pressable, stepped } from "./steps.ts";
import { themeSwitch } from "./theme.ts";

/** How far below the top of the window a part counts as the one being read, past the head. */
const READING_LINE_PX = 80;

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

/** Goes to the document's end. */
function bottom(): void {
  scrollTo(0, document.body.scrollHeight);
}

/** Wires the document this runs in: its parts, its keys and its theme switch. */
function wire(): void {
  const help = document.getElementById("keys");
  const { step } = parts([...document.querySelectorAll<HTMLElement>(".part")]);
  const top = onTwice(() => scrollTo(0, 0));
  const actions = { j: () => step(1), k: () => step(-1), g: top, G: bottom };
  // A document without its list of keys still steps; only ? has nothing to open.
  if (help instanceof HTMLDialogElement) {
    listen({ help, actions, answersAt });
    pressable(".head .keys kbd", { j: actions.j, k: actions.k, "?": () => toggle(help) });
  } else {
    listen({ actions, answersAt });
    pressable(".head .keys kbd", { j: actions.j, k: actions.k });
  }
  themeSwitch();
}

// Imported for its helpers where there is no page, as its tests do, it wires nothing.
if (typeof document !== "undefined") {
  wire();
}

export { partAt, partFrom };

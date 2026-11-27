// The page's own parts, acted on with only what each is given: finding an
// element the markup holds, writing into one, marking where the cursor is,
// and copying a folder. What the cursor is, and where, stays with site.ts.

import { pressable } from "../../local-sites/kit/steps.ts";
import { outward } from "./address.ts";
import { ago } from "./draw.ts";
import type { Repository } from "./model.ts";
import { UNDRAWN, waitBeforeDrawing } from "./pace.ts";

/** The element `selector` finds, which the page's markup always holds. */
function element<T extends Element>(selector: string, kind: new () => T): T {
  const found = document.querySelector(selector);
  if (!(found instanceof kind)) {
    throw new Error(`the page has no ${selector}, which Code draws into`);
  }
  return found;
}

const written = new WeakMap<Element, string>();

// What names a thing the page draws again, in the order that names it most surely.
const NAMING = ["data-path", "data-key", "data-place", "href", "id"] as const;

/** The attribute naming `thing` and its value, by which it's found once drawn again. */
function namingOf(thing: Element): [string, string] | undefined {
  for (const name of NAMING) {
    const value = thing.getAttribute(name);
    if (value !== null) {
      return [name, value];
    }
  }
  return undefined;
}

/**
 * Writes `html` into `into` only when it differs from what was last written there, so a record
 * that changes nothing on this part of the page leaves focus where it stood. When it does, and
 * focus was on something inside, focus goes to the same thing drawn again, so a redraw never
 * drops the reader back to the top of the page.
 */
function write(into: Element, html: string): void {
  if (written.get(into) === html) {
    return;
  }
  written.set(into, html);
  const focused = document.activeElement;
  const naming =
    focused !== null && focused !== into && into.contains(focused) ? namingOf(focused) : undefined;
  // Two drawn things may share a name, as one cell's home links each folder it's at, so focus
  // comes back to the same one of them, by its place among them.
  const named = (): HTMLElement[] =>
    naming === undefined
      ? []
      : [...into.querySelectorAll<HTMLElement>(`[${naming[0]}]`)].filter(
          (each) => each.getAttribute(naming[0]) === naming[1],
        );
  const at = named().findIndex((each) => each === focused);
  into.innerHTML = html;
  if (naming !== undefined) {
    const again = named();
    (again[at] ?? again[0])?.focus({ preventScroll: true });
  }
}

/**
 * Makes `keys` one stop for Tab: the arrows, Home and End move between them, and Tab comes back
 * to the one last left, so a row of buttons is passed in one press rather than six.
 */
function roving(keys: readonly HTMLElement[]): void {
  const choose = (chosen: HTMLElement): void => {
    for (const each of keys) {
      each.tabIndex = each === chosen ? 0 : -1;
    }
  };
  keys.forEach((key, at) => {
    key.addEventListener("focus", () => choose(key));
    key.addEventListener("keydown", (event) => {
      const moves: Readonly<Record<string, number>> = {
        ArrowRight: at + 1,
        ArrowDown: at + 1,
        ArrowLeft: at - 1,
        ArrowUp: at - 1,
        Home: 0,
        End: keys.length - 1,
      };
      const to = moves[event.key];
      const next = to === undefined ? undefined : keys[(to + keys.length) % keys.length];
      if (
        next !== undefined &&
        (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey)
      ) {
        // A chord is the browser's or the reader's, as Alt with an arrow goes back a page, and
        // never the page's, whose arrows would step the grove from a key.
        event.stopPropagation();
      } else if (next !== undefined) {
        // The page's own arrows step the list; on a key they step the keys alone.
        event.preventDefault();
        event.stopPropagation();
        next.focus();
      }
    });
  });
  const [first] = keys;
  if (first !== undefined) {
    choose(first);
  }
}

/** Makes the keys shown at the head press as `taps` name them, one stop for Tab among them. */
function pressHead(taps: Readonly<Record<string, () => void>>, afterwards: () => void): void {
  pressable(".head .keys kbd", taps, afterwards);
  roving([...document.querySelectorAll<HTMLElement>(".head .keys [role=button]")]);
}

/** Marks the repository keyed `cursor` in the grove and the list, and brings it into view. */
function markRows(cursor: string | null, scroll: boolean): void {
  for (const each of document.querySelectorAll<HTMLElement>(
    "#grove [role=option], #list [role=option]",
  )) {
    const on = each.dataset["key"] === cursor;
    each.setAttribute("aria-selected", String(on));
    if (on) {
      each.parentElement?.setAttribute("aria-activedescendant", each.id);
      if (scroll) {
        each.scrollIntoView({ block: "nearest", inline: "nearest" });
      }
    }
  }
}

/**
 * Keeps focus in the drawer after a redraw: where write left it, else on the cursor's tree, else
 * on the drawer itself, since what held it may be gone.
 */
function keepIn(opened: HTMLElement, inner: string | null): void {
  if (!opened.contains(document.activeElement)) {
    markTree(opened, inner, true);
  }
  if (!opened.contains(document.activeElement)) {
    // The drawer around the body is what takes focus, as the body itself can't.
    (opened.closest<HTMLElement>(".drawer") ?? opened).focus({ preventScroll: true });
  }
}

/**
 * Marks the tree at `inner` inside `opened`, focusing it when asked, and says if it's drawn, so
 * a tree gone or folded away as its season turned is let go, and the address with it.
 */
function markTree(opened: HTMLElement, inner: string | null, focus: boolean): boolean {
  let drawn = false;
  for (const each of opened.querySelectorAll<HTMLElement>(".tree")) {
    const on = each.dataset["path"] === inner;
    each.setAttribute("aria-current", String(on));
    drawn ||= on;
    if (on && focus) {
      each.focus();
    }
  }
  return drawn;
}

/**
 * Tells every age on the page again against `now`, in place, and only where its words change, so
 * the cursor, focus and a selection stay where they stood. What write last wrote is left as it
 * was: the markup holds each age's moment, so the next write that differs draws it anew.
 */
function agesAgain(now: number): void {
  for (const each of document.querySelectorAll<HTMLElement>(".when[data-at]")) {
    const said = ago(Number(each.dataset["at"]), now);
    if (each.textContent !== said) {
      each.textContent = said;
    }
  }
}

/**
 * Each minute tells the ages again in place, and once the day turns calls `redraw`, since what
 * was written today is yesterday's then: the one time the page is drawn under a reader who did
 * nothing. A tree passing a week unwritten rests at the next move.
 */
function keepTime(redraw: () => void): void {
  let day = new Date().toDateString();
  setInterval(() => {
    const now = Date.now();
    agesAgain(now);
    const today = new Date(now).toDateString();
    if (today !== day) {
      day = today;
      redraw();
    }
  }, 60_000);
}

/**
 * `act`, on the next frame once asked for, then, once `pacing` says so, at most once each `gap`
 * milliseconds while asks keep coming, and never sooner than a few times what it last took, so a
 * burst of records draws a few times rather than every frame, its last record always lands, and
 * a slow draw leaves the reader most of the time. A page out of sight draws nothing until it's
 * back, since frames wait.
 */
function paced(act: () => void, gap: number, pacing: () => boolean): () => void {
  let drawn = UNDRAWN;
  let asked = false;
  // From a draw until the frame after it, while its cost is still being measured: an ask then
  // waits for the measure, so the next draw's rest counts the style and layout too.
  let measuring = false;
  let askedWhileMeasuring = false;
  const ask = (): void => {
    if (measuring) {
      askedWhileMeasuring = true;
      return;
    }
    if (asked) {
      return;
    }
    asked = true;
    const wait = waitBeforeDrawing(drawn, performance.now(), gap, pacing());
    setTimeout(() => {
      requestAnimationFrame(() => {
        asked = false;
        measuring = true;
        const start = performance.now();
        try {
          act();
        } finally {
          // The style, layout and paint the markup asks for follow this frame's callbacks, so
          // the cost runs to the next frame, a draw that throws counted all the same.
          requestAnimationFrame(() => {
            const painted = performance.now();
            drawn = { end: painted, cost: painted - start };
            measuring = false;
            if (askedWhileMeasuring) {
              askedWhileMeasuring = false;
              ask();
            }
          });
        }
      });
    }, wait);
  };
  return ask;
}

// The parts whose height the feeds decide, each held at index.html's --room-<id> until they draw.
// The standing line is among them, since it sits above the grove and its trouble may wrap.
const ROOMS = ["lede", "standing", "grove", "list", "open"] as const;
type Room = (typeof ROOMS)[number];
// The longest a part's room is held, so a feed that never answers, or a daemon away, leaves the
// page fitting what it holds, the trouble said where it's seen, rather than the last visit.
const ROOM_HELD_MS = 3000;

/**
 * Keeps the room each part the feeds draw took when the page was left, at this width, so the
 * next load holds it from its first paint (index.html puts it back) and nothing shifts as the
 * feeds arrive. A part `forgets` names, drawn from what a reload doesn't bring back (a name
 * found, a season unfolded), keeps no room, nor does any once the width changes. Returns what's
 * told of each draw, which lets the room go once every feed has drawn, so the parts fit what
 * they hold again.
 */
function keepRoom(forgets: () => readonly Room[]): (everyFeed: boolean) => void {
  const root = document.documentElement.style;
  let held = ROOMS.some((id) => root.getPropertyValue(`--room-${id}`) !== "");
  const letGo = (): void => {
    for (const id of ROOMS) {
      root.removeProperty(`--room-${id}`);
    }
    held = false;
  };
  setTimeout(letGo, ROOM_HELD_MS);
  // Room is kept for the width it was measured at, which a resize leaves.
  const width = innerWidth;
  globalThis.addEventListener("resize", () => {
    if (held && innerWidth !== width) {
      letGo();
    }
  });
  globalThis.addEventListener("pagehide", () => {
    // Room still held is the last visit's, not this one's, so it isn't kept again.
    if (held) {
      return;
    }
    const forgotten = forgets();
    const heights = Object.fromEntries(
      ROOMS.map((id) => [
        id,
        forgotten.includes(id) ? 0 : (document.getElementById(id)?.offsetHeight ?? 0),
      ]),
    );
    try {
      // Under a name of its own since the kit's head took the lede, so a visit kept before then,
      // its heights taken of other parts, is never put back; that one is let go.
      localStorage.setItem("code-rooms", JSON.stringify({ width: innerWidth, heights }));
      localStorage.removeItem("code-room");
    } catch {
      // A browser that keeps nothing draws the next load from nothing, shifting as it did.
    }
  });
  return (everyFeed) => {
    if (everyFeed && held) {
      letGo();
    }
  };
}

/** Copies `path` and says so on `note`, or says it whole when it can't be copied. */
function copyFolder(path: string, note: HTMLElement): void {
  navigator.clipboard.writeText(path).then(
    () => {
      note.textContent = `Copied ${path}.`;
    },
    (error: unknown) => {
      note.textContent = `Couldn't copy (${String(error)}); the folder is ${path}.`;
    },
  );
}

/** Copies the tree at `inner`'s folder, or `open`'s first checkout's, and says so on `note`. */
function copyFrom(open: Repository | undefined, inner: string | null, note: HTMLElement): void {
  const path = inner ?? open?.checkouts[0]?.path;
  if (path !== undefined) {
    copyFolder(path, note);
  }
}

/** Opens the change of the tree at `inner`, or `open` where its remote keeps it. */
function openOutward(open: Repository | undefined, inner: string | null): void {
  const url = open === undefined ? undefined : outward(open, inner);
  if (url !== undefined) {
    globalThis.open(url, "_blank", "noopener");
  }
}

export {
  copyFrom,
  element,
  keepRoom,
  keepTime,
  markRows,
  keepIn,
  markTree,
  openOutward,
  paced,
  pressHead,
  type Room,
  write,
};

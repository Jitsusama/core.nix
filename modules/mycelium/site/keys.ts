// What each key asks of the map, decided from the key and where the page
// stands, apart from the drawing that then does it (view.js). The map's own
// keys are said in its keys dialog and README; these are the decisions
// behind them, so each can be tried alone: the walk and the pan on the map,
// the tree beside it, the page's letters, the find line, stepping through
// what was found, and the theme's switch.

/** A key as the browser says it was pressed, of what these decisions read. */
export interface Pressed {
  readonly key: string;
  readonly shiftKey: boolean;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly altKey: boolean;
}

/** A step across the map, as columns and rows. */
export type Way = readonly [number, number];

/** What a key on the map asks: walk to a neighbour, move the map, or open. */
export type MapAct =
  | { readonly act: "walk"; readonly way: Way }
  | { readonly act: "pan"; readonly way: Way }
  | { readonly act: "open" };

/**
 * Every act a key in the tree can ask, listed so the drawing, which no
 * typechecker reads, can say at load that it does each one.
 */
export const EVERY_TREE_ACT = ["next", "prev", "in", "out", "first", "last", "open"] as const;

/** What a key in the tree asks of the item it's on. */
export type TreeAct = (typeof EVERY_TREE_ACT)[number];

/** Every act one of the page's letters can ask, listed as the tree's are. */
export const EVERY_PAGE_ACT = [
  "keys",
  "find",
  "close-find",
  "next-found",
  "found-before",
  "let-go",
  "close-now",
  "now",
  "machines",
  "fit",
  "zoom-in",
  "zoom-out",
  "out",
] as const;

/** What one of the page's letters asks. */
export type PageAct = (typeof EVERY_PAGE_ACT)[number];

/** What the page's letters read of where it stands as a key comes. */
export interface Standing {
  /** Whether the key went to a field typed in, which keeps its own keys. */
  readonly typing: boolean;
  readonly keysOpen: boolean;
  /** Whether focus is on the map or its panel, where its letters answer. */
  readonly onMap: boolean;
  readonly findOpen: boolean;
  /** Whether the find line is open with something asked of it. */
  readonly finding: boolean;
  /** Whether a cell, network, machine or talk is open. */
  readonly chosen: boolean;
  readonly nowOpen: boolean;
}

/** What a key in the find line asks: the first match, or the line closed. */
export type FindAct = "first" | "close";

// Arrows and the letters h, j, k and l, each a step on the map.
const WAYS: Readonly<Record<string, Way>> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
  h: [-1, 0],
  l: [1, 0],
  k: [0, -1],
  j: [0, 1],
};

const TREE_KEYS: Readonly<Record<string, TreeAct>> = {
  ArrowDown: "next",
  j: "next",
  ArrowUp: "prev",
  k: "prev",
  ArrowRight: "in",
  l: "in",
  ArrowLeft: "out",
  h: "out",
  Home: "first",
  g: "first",
  End: "last",
  G: "last",
  Enter: "open",
  " ": "open",
};

// The page's letters that answer whatever is open, once the ones that
// depend on it have had their say.
const LETTERS: Readonly<Record<string, PageAct>> = {
  ".": "now",
  m: "machines",
  "0": "fit",
  "+": "zoom-in",
  "=": "zoom-in",
  i: "zoom-in",
  o: "zoom-out",
  "-": "out",
};

const SWITCH_STEPS: Readonly<Record<string, number>> = {
  ArrowLeft: -1,
  ArrowUp: -1,
  ArrowRight: 1,
  ArrowDown: 1,
};

// A chord with Ctrl, Meta or Alt is the browser's or the desktop's, never
// the map's.
const chorded = (pressed: Pressed): boolean => pressed.ctrlKey || pressed.metaKey || pressed.altKey;

// A key's entry in one of the tables above, or null for a key it doesn't
// name, even one an object answers to, such as `constructor`, as the kit's
// own keys are looked up.
const named = <T>(table: Readonly<Record<string, T>>, key: string): T | null =>
  Object.hasOwn(table, key) ? (table[key] ?? null) : null;

/**
 * What a key on the map asks: an arrow or h, j, k or l walks the network,
 * held with Shift it moves the map instead, and Return or Space opens
 * what the walk has reached.
 */
export const mapKey = (pressed: Pressed): MapAct | null => {
  if (chorded(pressed)) {
    return null;
  }
  // Shift makes a letter capital, and a capital walks as its letter.
  const way = named(WAYS, pressed.key.length === 1 ? pressed.key.toLowerCase() : pressed.key);
  if (way) {
    return { act: pressed.shiftKey ? "pan" : "walk", way };
  }
  return pressed.key === "Enter" || pressed.key === " " ? { act: "open" } : null;
};

/**
 * What a key in the tree asks: up and down, in and out, the first and the
 * last, or the item opened. Shift answers only as the G that goes last.
 */
export const treeKey = (pressed: Pressed): TreeAct | null => {
  if (chorded(pressed) || (pressed.shiftKey && pressed.key !== "G")) {
    return null;
  }
  return named(TREE_KEYS, pressed.key);
};

/**
 * What one of the page's letters asks, from where the page stands. The
 * keys dialog and find answer anywhere but a field; the rest answer only
 * on the map or its panel, so no stray key acts from elsewhere (WCAG
 * 2.1.4). Escape closes what a mode opened, the find line first, then
 * what's chosen, then the now.
 */
export const pageKey = (pressed: Pressed, at: Standing): PageAct | null => {
  if (chorded(pressed) || at.typing) {
    return null;
  }
  const { key } = pressed;
  if (key === "?" || key === "F1") {
    return "keys";
  }
  if (at.keysOpen) {
    return null;
  }
  if (key === "/") {
    return "find";
  }
  if (!at.onMap) {
    return null;
  }
  if (key === "Escape") {
    if (at.findOpen) {
      return "close-find";
    }
    if (at.chosen) {
      return "let-go";
    }
    return at.nowOpen ? "close-now" : null;
  }
  if ((key === "n" || key === "N") && at.finding) {
    return key === "n" ? "next-found" : "found-before";
  }
  return named(LETTERS, key);
};

/**
 * What a key in the find line asks: Return the first match, and Escape
 * the line closed rather than only cleared, as a search field would.
 */
export const findKey = (key: string): FindAct | null =>
  key === "Enter" ? "first" : key === "Escape" ? "close" : null;

/**
 * Which match a step `by` lands on among `count`, from `at`, or from none
 * when `at` is below zero: forward starts at the first and back at the
 * last, round and round. With nothing found there's nowhere to go.
 */
export const stepped = (at: number, by: number, count: number): number | null => {
  if (count === 0) {
    return null;
  }
  if (at < 0) {
    return by > 0 ? 0 : count - 1;
  }
  return (((at + by) % count) + count) % count;
};

/**
 * Which of the theme switch's `count` choices an arrow moves to from `at`,
 * round and round as `stepped` goes, or null for a key that isn't an arrow.
 */
export const switchedTo = (key: string, at: number, count: number): number | null => {
  const by = named(SWITCH_STEPS, key);
  return by === null ? null : stepped(at, by, count);
};

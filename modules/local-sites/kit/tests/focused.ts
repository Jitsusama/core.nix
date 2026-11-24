// How keyed's focus tests draw a list, focus in it, draw it again and say
// where focus landed. Shared by the focus tests of rows (keyed-focus) and
// of the forms around them (keyed-form).

import { keyed } from "../keyed.ts";
import { type Act, type Cell, type Control, cells, inside, List, Row } from "./rows.ts";

/** Where focus landed: the control, the one it holds, its row, nowhere or elsewhere. */
type Landed = "control" | "held" | "row" | "none" | "elsewhere";

/** Where focus is in `list`, against b's second control, the one it holds and b itself. */
function landed(list: List, control: object | undefined, deep = false): Landed {
  const now = list.ownerDocument.activeElement;
  if (now !== null && now === control) {
    return "control";
  }
  if (deep && now === list.rows[1]?.children[1]?.children[0]) {
    return "held";
  }
  if (now === null) {
    return "none";
  }
  return now === list.rows[1] ? "row" : "elsewhere";
}

/**
 * Draws rows a and b with `before`'s acts, in a list inside `around` where
 * told, focuses b's second control (or, told `deep`, the control it
 * holds), then draws b afresh with `after`'s acts and, where told, new
 * words for b; and says whether focus landed on b's second control, the
 * one it holds, or b itself.
 */
function refocused(
  before: readonly Act[],
  after: readonly Act[],
  works = "on b",
  { deep = false, around }: { readonly deep?: boolean; readonly around?: Control } = {},
): Landed {
  const list = new List();
  if (around !== undefined) {
    inside(list, around);
  }
  const rows = (acts: readonly Act[], told: readonly Cell[]) =>
    keyed(list, {
      rows: told,
      key: (cell) => cell.key,
      draw: (cell) => new Row(cell.works, list.ownerDocument, true, acts),
      moved: ["b"],
    });
  rows(before, cells("a", "b"));
  const second = list.rows[1]?.children[1];
  (deep ? second?.children[0] : second)?.focus?.();
  rows(after, [...cells("a"), { key: "b", works }]);
  return landed(list, list.rows[1]?.children[1], deep);
}

/** b's second control, and what holds the list where told, as a page changes them in place. */
type Changing = (
  control: Row["children"][number],
  around: ReturnType<typeof inside> | null,
) => void;

/**
 * Draws rows a and b with `acts`, in rows taking focus unless told not to,
 * in a list inside `around` where told, focuses b's second control, then
 * draws b again where it stands after `change`; and says where focus landed.
 */
function inPlace(
  change: Changing,
  {
    focusable = true,
    acts = ["open", "copy"],
    around,
  }: {
    readonly focusable?: boolean;
    readonly acts?: readonly Act[];
    readonly around?: Control;
  } = {},
): Landed {
  const list = new List();
  const holder = around === undefined ? null : inside(list, around);
  const rows = (redraw: boolean) =>
    keyed(list, {
      rows: cells("a", "b"),
      key: (cell) => cell.key,
      draw: (cell, was) => {
        if (redraw && cell.key === "b" && was !== undefined) {
          const control = was.children[1];
          if (control !== undefined) {
            change(control, holder);
          }
        }
        return was ?? new Row(cell.works, list.ownerDocument, focusable, acts);
      },
      moved: redraw ? ["b"] : [],
    });
  rows(false);
  const control = list.rows[1]?.children[1];
  control?.focus();
  rows(true);
  return landed(list, control);
}

/** A button named `name` sending `value`, saying `says`. */
const button = (says: string, name: string, value?: string): Act => ({
  says,
  attributes: value === undefined ? { name } : { name, value },
});

/** A control saying `says` whose markup says nothing of what it does. */
const bare = (says: string): Act => ({ says, attributes: {} });

export { bare, button, inPlace, refocused };

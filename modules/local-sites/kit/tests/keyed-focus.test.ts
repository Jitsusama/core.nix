// Where focus goes when a list is drawn by key: it stays on a control still
// in its row, goes to the same place in a new drawing only where that holds
// the same control, and otherwise to the row, so no other control takes the
// next press. A control is the same where its markup but its look is the
// same and says what it does of it alone, else where its words are too.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { keyed } from "../keyed.ts";
import { type Act, type Cell, cells, draw, List, Row } from "./rows.ts";

/**
 * Draws rows a and b with `before`'s acts, focuses b's second control (or,
 * told `deep`, the control it holds), then draws b afresh with `after`'s
 * acts and, where told, new words for b; and says whether focus landed on
 * b's second control or on b itself.
 */
function refocused(
  before: readonly Act[],
  after: readonly Act[],
  works = "on b",
  deep = false,
): string {
  const list = new List();
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
  const now = list.ownerDocument.activeElement;
  if (now === list.rows[1]?.children[1]) {
    return "control";
  }
  return now === list.rows[1] ? "row" : "elsewhere";
}

/** A button named `name` sending `value`, saying `says`. */
const button = (says: string, name: string, value?: string): Act => ({
  says,
  attributes: value === undefined ? { name } : { name, value },
});

/** A control saying `says` whose markup says nothing of what it does. */
const bare = (says: string): Act => ({ says, attributes: {} });

it("a focused row moved keeps its focus", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  const link = list.rows[2]?.children[1];
  link?.focus();
  draw(list, cells("c", "a", "b"), []);
  assert.equal(list.ownerDocument.activeElement, link);
});

it("a focused row drawn afresh hands focus to the same place in its drawing", () => {
  const list = new List();
  draw(list, cells("a", "b", "c"));
  list.rows[1]?.children[1]?.focus();
  draw(list, cells("a", "b", "c"), ["b"]);
  assert.equal(list.ownerDocument.activeElement, list.rows[1]?.children[1]);
});

it("a focused row drawn again where it stands hands focus to the same place", () => {
  const list = new List();
  draw(list, cells("a", "b"));
  list.rows[1]?.children[1]?.focus();
  keyed(list, {
    rows: cells("a", "b"),
    key: (cell) => cell.key,
    draw: (cell, was) => {
      was?.again(cell.works);
      return was ?? new Row(cell.works, list.ownerDocument);
    },
    moved: ["b"],
  });
  assert.equal(list.ownerDocument.activeElement, list.rows[1]?.children[1]);
});

it("a focused link whose words show the changed record keeps focus by where it goes", () => {
  assert.equal(refocused(["open", "copy"], ["open", "copy"], "on b, moved"), "control");
});

it("focus whose place in a new drawing holds another control goes to the row", () => {
  assert.equal(refocused(["open", "copy"], ["copy", "open"]), "row");
});

it("a link with its words kept but going somewhere new is another control", () => {
  const to = (place: string): Act => ({ says: "copy", attributes: { href: place } });
  assert.equal(refocused(["open", to("#copy")], ["open", to("#paste")]), "row");
});

it("links all going to # or a script are told apart by their words", () => {
  const hash = (says: string): Act => ({ says, attributes: { href: "#" } });
  assert.equal(refocused([hash("open"), hash("copy")], [hash("copy"), hash("open")]), "row");
  assert.equal(refocused([hash("open"), hash("copy")], [hash("open"), hash("copy")]), "control");
  const run = (says: string): Act => ({ says, attributes: { href: "javascript:void 0" } });
  assert.equal(refocused([run("open"), run("copy")], [run("copy"), run("open")]), "row");
});

it("buttons sharing a name are told apart by their values", () => {
  const open = button("open", "act", "open");
  const copy = button("copy", "act", "copy");
  assert.equal(refocused([open, copy], [copy, open]), "row");
  assert.equal(refocused([open, copy], [open, copy], "on b, moved"), "control");
});

it("buttons sharing a name and no value are told apart by their words", () => {
  const open = button("open", "act");
  const copy = button("copy", "act");
  assert.equal(refocused([open, copy], [copy, open]), "row");
});

it("a control whose markup says nothing of itself keeps focus only by the same words", () => {
  assert.equal(refocused([bare("open"), bare("copy")], [bare("open"), bare("copy")]), "control");
  assert.equal(
    refocused([bare("open"), bare("copy")], [bare("open"), bare("copy")], "on b, moved"),
    "row",
  );
});

it("a control saying nothing of itself is never taken for one that does", () => {
  assert.equal(refocused([bare("open"), bare("copy")], ["open", "copy"]), "row");
  assert.equal(refocused(["open", "copy"], [bare("open"), bare("copy")]), "row");
});

it("a control told apart only by what it's heard as is another control", () => {
  const heard = (label: string): Act => ({ says: "copy", attributes: { "aria-label": label } });
  const [path, link] = [heard("copy its path"), heard("copy its link")];
  assert.equal(refocused([path, link], [link, path]), "row");
  assert.equal(refocused([path, link], [path, link]), "control");
});

it("a control of another kind with the same name and value is another control", () => {
  const sent = (kind: string): Act => ({
    says: "copy",
    kind,
    attributes: { name: "act", value: "copy" },
  });
  assert.equal(refocused(["open", sent("a")], ["open", sent("button")]), "row");
});

it("focus nested deeper than the new drawing goes to the row, however alike the place", () => {
  const copy: Act = { says: "copy", attributes: { href: "#copy" } };
  const held: Act = { says: "", kind: "span", attributes: {}, holds: { ...copy } };
  assert.equal(refocused(["open", held], ["open", copy], "on b", true), "row");
});

it("a control going to the same place but marked for another act is another control", () => {
  const act = (says: string, marks: Record<string, string>): Act => ({
    says,
    attributes: { href: "#b", ...marks },
  });
  const [open, forget] = [
    act("open", { "data-act": "open" }),
    act("forget", { "data-act": "forget" }),
  ];
  assert.equal(refocused([open, forget], [forget, open]), "row");
  assert.equal(refocused([open, forget], [open, forget], "on b, moved"), "control");
  const [start, stop] = [
    act("b", { "aria-label": "Start b" }),
    act("b", { "aria-label": "Stop b" }),
  ];
  assert.equal(refocused(["open", start], ["open", stop]), "row");
});

it("a button sending the same name and value to another action is another control", () => {
  const sent = (says: string, action: string): Act => ({
    says,
    kind: "button",
    attributes: { name: "id", value: "b", formaction: action },
  });
  assert.equal(
    refocused(["open", sent("Start", "/start")], ["open", sent("Stop", "/stop")]),
    "row",
  );
  assert.equal(
    refocused(["open", sent("Start", "/start")], ["open", sent("Start b", "/start")]),
    "control",
  );
});

it("a control whose state changed is still the same control", () => {
  const toggle = (pressed: string, more: Record<string, string> = {}): Act => ({
    says: "pin",
    kind: "button",
    attributes: { name: "pin", value: "b", "aria-pressed": pressed, class: "pin", ...more },
  });
  assert.equal(refocused(["open", toggle("false")], ["open", toggle("true")]), "control");
  const busy = { disabled: "", "aria-disabled": "true", "aria-invalid": "true" };
  assert.equal(refocused(["open", toggle("false")], ["open", toggle("false", busy)]), "control");
});

it("a control whose class changed is another control, since a page may key its act on it", () => {
  const marked = (act: string): Act => ({ says: "b", attributes: { href: "/r/b", class: act } });
  assert.equal(refocused(["open", marked("open")], ["open", marked("forget")]), "row");
});

it("a page's own act says what a control does, so its words may change", () => {
  const act = (says: string, name: string): Act => ({
    says,
    kind: "button",
    attributes: { "data-act": name },
  });
  assert.equal(
    refocused(["open", act("Forget", "forget")], ["open", act("Forget b", "forget")]),
    "control",
  );
  assert.equal(refocused(["open", act("b", "forget")], ["open", act("b", "open")]), "row");
});

it("a button sending the same name and value from another form is another control", () => {
  const sent = (action: string): Act => ({
    says: "",
    kind: "form",
    attributes: { action, method: "post" },
    holds: { says: "b", kind: "button", attributes: { name: "id", value: "b" } },
  });
  // Focus on the button inside lands on it again ("elsewhere" to refocused)
  // only where its form is the same.
  assert.equal(refocused(["open", sent("/start")], ["open", sent("/stop")], "on b", true), "row");
  assert.equal(
    refocused(["open", sent("/start")], ["open", sent("/start")], "on b", true),
    "elsewhere",
  );
});

it("a control labelled by another element is never known again", () => {
  const labelled: Act = { says: "b", attributes: { href: "/r/b", "aria-labelledby": "b-name" } };
  assert.equal(refocused(["open", labelled], ["open", labelled]), "row");
});

it("focus on a control saying nothing of itself goes to the row, not its likeness", () => {
  assert.equal(refocused([null, null], [null, null]), "row");
});

it("focus in a row whose new drawing takes none is let go", () => {
  const list = new List();
  draw(list, cells("a", "b"));
  list.rows[1]?.children[1]?.focus();
  keyed(list, {
    rows: cells("a", "b"),
    key: (cell) => cell.key,
    draw: (cell) => new Row(cell.works, list.ownerDocument, false, ["copy", "open"]),
    moved: ["b"],
  });
  assert.equal(list.ownerDocument.activeElement, null);
});

it("focus in a row that goes is left where the page puts it", () => {
  const list = new List();
  draw(list, cells("a", "b"));
  list.rows[1]?.focus();
  draw(list, cells("a"), []);
  assert.equal(list.ownerDocument.activeElement, null);
});

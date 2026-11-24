// Where focus goes when a list is drawn by key: it stays on a control still
// in its row, goes to the same place in a new drawing only where that holds
// the same control, and otherwise to the row, so no other control takes the
// next press. A control is the same where its markup but its look is the
// same and says what it does of it alone, else where its words are too.
// Forms are keyed-form's.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { keyed, type Placed } from "../keyed.ts";
import { bare, button, inPlace, refocused } from "./focused.ts";
import { type Act, type Control, cells, draw, List, Row } from "./rows.ts";

/** What a drawing throws on a record its row can't read. */
const MISSHAPEN = /isn't the shape/u;

/** A row's acts beside a log that scrolls, which a browser focuses with no tabindex. */
const LOGGED: Act[] = ["open", { says: "log", kind: "pre", attributes: {}, scrolls: true }];

/** Two buttons, which a disabled fieldset disables, as it doesn't a link. */
const buttons = [button("open", "act", "open"), button("copy", "act", "copy")];

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

it("a focused row drawn into itself keeps its focus", () => {
  const list = new List();
  draw(list, cells("a", "b"));
  list.rows[1]?.focus();
  keyed(list, {
    rows: cells("a", "b"),
    key: (cell) => cell.key,
    draw: (cell, was) => was ?? new Row(cell.works, list.ownerDocument),
    moved: ["b"],
  });
  assert.equal(list.ownerDocument.activeElement, list.rows[1]);
  // Its words changed where it stands, it's still the row it was.
  keyed(list, {
    rows: cells("a", "b"),
    key: (cell) => cell.key,
    draw: (cell, was) => {
      if (was !== undefined) {
        was.words = `${cell.works}, moved`;
      }
      return was ?? new Row(cell.works, list.ownerDocument);
    },
    moved: ["b"],
  });
  assert.equal(list.ownerDocument.activeElement, list.rows[1]);
});

it("focus on what a page lets take it, in a row the drawing left alone, stays", () => {
  const list = new List();
  const rows = (moved: string[]) =>
    keyed(list, {
      rows: cells("a", "b"),
      key: (cell) => cell.key,
      draw: (cell) => new Row(cell.works, list.ownerDocument, true, LOGGED),
      moved,
    });
  rows([]);
  const log = list.rows[1]?.children[1] ?? null;
  log?.focus();
  rows(["a"]);
  assert.equal(list.ownerDocument.activeElement, log);
});

/** How b's drawing changes its row where it stands, before a row throws. */
type Changed = (row: Row) => void;

/** Where focus on b's second control lands when b's change is made and `throwing` throws. */
function thrown(
  throwing: string,
  change: Changed,
  focusable = true,
): { list: List; landed: Placed | null } {
  const list = new List();
  const rows = (changed: boolean) =>
    keyed(list, {
      rows: cells("a", "b", "c"),
      key: (cell) => cell.key,
      draw: (cell, was) => {
        if (changed && cell.key === "b" && was !== undefined) {
          change(was);
        }
        if (changed && cell.key === throwing) {
          throw new Error("a record isn't the shape its row reads");
        }
        return was ?? new Row(cell.works, list.ownerDocument, focusable);
      },
      moved: changed ? ["a", "b", "c"] : [],
    });
  rows(false);
  list.rows[1]?.children[1]?.focus();
  assert.throws(() => rows(true), MISSHAPEN);
  return { list, landed: list.ownerDocument.activeElement };
}

/** b's second control marked to go elsewhere, where it stands. */
const repointed: Changed = (row) => row.children[1]?.setAttribute("href", "#forget");

it("a drawing that throws after marking the focused control for another act hands focus to its row", () => {
  for (const throwing of ["b", "c"]) {
    const { list, landed } = thrown(throwing, repointed);
    assert.equal(landed, list.rows[1], `${throwing} throwing`);
    // A row that takes no focus lets it go.
    assert.equal(thrown(throwing, repointed, false).landed, null, `${throwing}, unfocusable`);
  }
});

it("a drawing that throws before reaching the focused row leaves its focus", () => {
  const { list, landed } = thrown("a", repointed);
  assert.equal(landed, list.rows[1]?.children[1]);
});

it("a drawing that throws after drawing the focused row anew gives focus back to its control", () => {
  for (const throwing of ["b", "c"]) {
    // Its links drawn again, the focused one is taken out with its focus.
    const { list, landed } = thrown(throwing, (row) => row.again(row.words));
    assert.equal(landed, list.rows[1]?.children[1], `${throwing} throwing`);
  }
});

it("a scroller a page lets take focus keeps it in a row drawn again, in place or afresh", () => {
  for (const fresh of [false, true]) {
    const list = new List();
    const rows = (moved: string[]) =>
      keyed(list, {
        rows: cells("a", "b"),
        key: (cell) => cell.key,
        draw: (cell, was) =>
          (fresh ? undefined : was) ?? new Row(cell.works, list.ownerDocument, true, LOGGED),
        moved,
      });
    rows([]);
    list.rows[1]?.children[1]?.focus();
    rows(["b"]);
    assert.equal(list.ownerDocument.activeElement, list.rows[1]?.children[1], `fresh ${fresh}`);
  }
});

it("a row whose tabindex is taken away takes no focus", () => {
  const list = new List();
  const row = new Row("on a", list.ownerDocument);
  list.insertBefore(row, null);
  row.removeAttribute("tabindex");
  row.focus();
  assert.equal(list.ownerDocument.activeElement, null);
});

it("a focused control kept where it stands keeps focus, its state changed or not", () => {
  assert.equal(
    inPlace(() => undefined),
    "control",
  );
  assert.equal(
    inPlace((control) => control.setAttribute("aria-pressed", "true")),
    "control",
  );
});

it("a focused control that takes no press where it stands hands focus to its row", () => {
  const disabling = (control: Row["children"][number]) => control.setAttribute("disabled", "");
  assert.equal(inPlace(disabling, { acts: buttons }), "row", "a button disabled");
  // A link marked disabled still works in a browser, but fails safe here.
  assert.equal(inPlace(disabling), "row", "marked disabled, even a link");
  const hiding = (style: string) => (control: Row["children"][number]) =>
    control.setAttribute("style", style);
  assert.equal(inPlace(hiding("display: none")), "row", "taken out of the drawing");
  assert.equal(inPlace(hiding("visibility: hidden")), "row", "unseen where it stands");
  // A control a page lets take focus only by its tabindex, which goes.
  const pin: Act = { says: "pin", kind: "span", attributes: { tabindex: "0", "data-act": "pin" } };
  const untabbed = inPlace((control) => control.removeAttribute("tabindex"), {
    acts: ["open", pin],
  });
  assert.equal(untabbed, "row", "no longer focusable");
});

it("a focused control marked in place for another act hands focus to its row", () => {
  const forgetting = (control: Row["children"][number]) => control.setAttribute("href", "#forget");
  assert.equal(inPlace(forgetting), "row");
  // A row taking no focus can't take it from the control, so it's let go.
  assert.equal(inPlace(forgetting, { focusable: false }), "none");
});

it("focus on a button a fieldset around the list disables goes to the row", () => {
  const fenced: Control = { says: "", kind: "fieldset", attributes: {} };
  const disabling = (
    _: unknown,
    around: { setAttribute(name: string, value: string): void } | null,
  ) => around?.setAttribute("disabled", "");
  assert.equal(inPlace(disabling, { acts: buttons, around: fenced }), "row");
  // A disabled fieldset leaves the links it holds working, so they keep focus.
  assert.equal(inPlace(disabling, { around: fenced }), "control");
});

it("focus in a list made inert is let go, since nothing in it takes focus", () => {
  const holder: Control = { says: "", kind: "div", attributes: {} };
  const inert = inPlace((_, around) => around?.setAttribute("inert", ""), { around: holder });
  assert.equal(inert, "none");
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
  for (const href of ["#!", "#0", "#/", "java\tscript:void 0", " javascript:void 0"]) {
    const shared = (says: string): Act => ({ says, attributes: { href } });
    assert.equal(
      refocused([shared("open"), shared("copy")], [shared("copy"), shared("open")]),
      "row",
      `${JSON.stringify(href)} was taken for where a link goes`,
    );
  }
});

it("links told apart only by their ids are told apart", () => {
  const go = (says: string, id: string): Act => ({ says, attributes: { href: "/r/b", id } });
  const [open, forget] = [go("Open", "open-b"), go("Forget", "forget-b")];
  assert.equal(refocused([open, forget], [forget, open]), "row");
  assert.equal(refocused([open, forget], [open, forget], "on b, moved"), "control");
});

it("buttons sharing a name are told apart by their values", () => {
  const open = button("open", "act", "open");
  const copy = button("copy", "act", "copy");
  assert.equal(refocused([open, copy], [copy, open]), "row");
  assert.equal(refocused([open, copy], [open, copy], "on b, moved"), "control");
});

it("a twin drawn afresh taking no press, in a row that takes no focus, is let go", () => {
  // A link marked disabled still takes focus in a browser, as in place.
  const unpressed: Act = { says: "copy", attributes: { href: "#copy", disabled: "" } };
  assert.equal(
    refocused(["open", "copy"], ["open", unpressed], "on b", { focusable: false }),
    "none",
  );
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
    attributes: { name: "act", value: "copy", tabindex: "0" },
  });
  assert.equal(refocused(["open", sent("a")], ["open", sent("button")]), "row");
});

it("focus nested deeper than the new drawing goes to the row, however alike the place", () => {
  const copy: Act = { says: "copy", attributes: { href: "#copy" } };
  const held: Act = { says: "", kind: "span", attributes: {}, holds: { ...copy } };
  assert.equal(refocused(["open", held], ["open", copy], "on b", { deep: true }), "row");
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
  const busy = { "aria-disabled": "true", "aria-invalid": "true" };
  assert.equal(refocused(["open", toggle("false")], ["open", toggle("false", busy)]), "control");
  const tick = (more: Record<string, string> = {}): Act => ({
    says: "",
    kind: "input",
    attributes: { type: "checkbox", name: "pick", value: "b", ...more },
  });
  assert.equal(refocused(["open", tick()], ["open", tick({ checked: "" })]), "control");
});

it("a control now disabled takes no focus, so its row does", () => {
  const toggle = (more: Record<string, string> = {}): Act => ({
    says: "pin",
    kind: "button",
    attributes: { name: "pin", value: "b", ...more },
  });
  assert.equal(refocused(["open", toggle()], ["open", toggle({ disabled: "" })]), "row");
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

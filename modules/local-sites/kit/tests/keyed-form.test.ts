// Where focus goes when a list drawn by key sits in a form, or its rows
// hold forms: a control is the same only where its form and all the form
// sends beside it are too, read from the markup, and where the markup
// can't say what the form sends, focus goes to the row.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { keyed } from "../keyed.ts";
import { refocused } from "./focused.ts";
import { type Act, type Control, cells, inside, List, Row } from "./rows.ts";

/** A hidden field named op sending `value`, marked with `more`. */
const op = (value: string, more: Record<string, string> = {}): Act => ({
  says: "",
  kind: "input",
  attributes: { type: "hidden", name: "op", value, ...more },
});

/** The button focused in a row of a form, sending b's id. */
const go: Act = { says: "Go", kind: "button", attributes: { name: "id", value: "b" } };

/** A form around the list, sending what its rows send. */
const form: Control = { says: "", attributes: { action: "/rows", method: "post" } };

/** Whether focus on go stays where `before`'s field becomes `after`'s, in a form. */
const keeps = (before: Act, after: Act): string =>
  refocused([before, go], [after, go], "on b", { around: form });

it("a button sending a form named by its form attribute hands focus to its row", () => {
  const start: Act = {
    says: "Start",
    kind: "button",
    attributes: { form: "act-b", name: "id", value: "b" },
  };
  assert.equal(refocused(["open", start], ["open", start]), "row");
});

it("a button sending the same name and value from another form is another control", () => {
  const sent = (action: string): Act => ({
    says: "",
    kind: "form",
    attributes: { action, method: "post" },
    holds: { says: "b", kind: "button", attributes: { name: "id", value: "b" } },
  });
  // Focus on the button inside lands on it again only where its form is the same.
  assert.equal(
    refocused(["open", sent("/start")], ["open", sent("/stop")], "on b", { deep: true }),
    "row",
  );
  assert.equal(
    refocused(["open", sent("/start")], ["open", sent("/start")], "on b", { deep: true }),
    "held",
  );
});

it("a button whose form sends another hidden value is another control", () => {
  const sent = (value: string): Act => ({
    says: "",
    kind: "form",
    attributes: { method: "post" },
    holds: [
      { says: "b", kind: "button", attributes: { name: "id", value: "b" } },
      { says: "", kind: "input", attributes: { type: "hidden", name: "op", value } },
    ],
  });
  assert.equal(
    refocused(["open", sent("forget")], ["open", sent("start")], "on b", { deep: true }),
    "row",
  );
  assert.equal(
    refocused(["open", sent("forget")], ["open", sent("forget")], "on b", { deep: true }),
    "held",
  );
});

it("a list inside a form keeps focus on the same control", () => {
  const paged: Control = {
    ...form,
    holds: { says: "", kind: "input", attributes: { type: "hidden", name: "page", value: "2" } },
  };
  assert.equal(refocused(["open", "copy"], ["open", "copy"], "on b", { around: paged }), "control");
  // What the form sends from the control's own row changes, the control not.
  assert.equal(keeps(op("forget"), op("start")), "row");
  assert.equal(keeps(op("forget"), op("forget")), "control");
  assert.equal(refocused([op("forget"), go], [op("start"), go]), "control", "no form, no field");
});

it("a field beside the control that stops being sent is another sending", () => {
  assert.equal(keeps(op("forget"), op("forget", { disabled: "" })), "row", "disabled");
  assert.equal(keeps(op("forget"), op("forget", { type: "submit" })), "row", "a submitter");
  assert.equal(keeps(op("forget"), op("forget", { form: "elsewhere" })), "row", "elsewhere");
  const fenced = (more: Record<string, string>): Act => ({
    says: "",
    kind: "fieldset",
    attributes: more,
    holds: { says: "", kind: "input", attributes: { type: "hidden", name: "op", value: "b" } },
  });
  assert.equal(keeps(fenced({}), fenced({ disabled: "" })), "row", "a fieldset disabled");
  assert.equal(keeps(fenced({}), fenced({ class: "quiet" })), "control", "a fieldset restyled");
});

it("a form whose sending can't be read from its markup hands focus to the row", () => {
  const typed: Control = {
    says: "",
    attributes: { action: "/rows" },
    holds: { says: "", kind: "input", attributes: { name: "q" } },
  };
  assert.equal(refocused(["open", "copy"], ["open", "copy"], "on b", { around: typed }), "row");
  const joined: Control = { says: "", attributes: { id: "rows", action: "/rows" } };
  assert.equal(refocused(["open", "copy"], ["open", "copy"], "on b", { around: joined }), "row");
  // What a person picks, types or a script sets is sent, which no markup says.
  for (const kind of ["select", "textarea", "x-pick"]) {
    const chosen: Act = { says: "", kind, attributes: { name: "pick", tabindex: "-1" } };
    assert.equal(keeps(chosen, chosen), "row", kind);
  }
});

it("a list that is itself the form counts only the focused row's fields", () => {
  const list = new List("form");
  const field = (value: string): Act => ({
    says: "",
    kind: "input",
    attributes: { type: "hidden", name: "op", value },
  });
  const rows = (a: string, b: string) =>
    keyed(list, {
      rows: cells("a", "b"),
      key: (cell) => cell.key,
      draw: (cell) =>
        new Row(cell.works, list.ownerDocument, true, [
          "open",
          "copy",
          field(cell.key === "a" ? a : b),
        ]),
      moved: ["a", "b"],
    });
  rows("1", "1");
  list.rows[1]?.children[1]?.focus();
  rows("2", "1");
  assert.equal(list.ownerDocument.activeElement, list.rows[1]?.children[1]);
  rows("2", "2");
  assert.equal(list.ownerDocument.activeElement, list.rows[1]);
});

it("a list inside a form keeps focus however another row's fields change", () => {
  const field = (value: string): Act => ({
    says: "",
    kind: "input",
    attributes: { type: "hidden", name: "seen", value },
  });
  const list = new List();
  inside(list, form);
  // Row a's field changes; b's, beside the focused control, stays.
  const rows = (seen: string) =>
    keyed(list, {
      rows: cells("a", "b"),
      key: (cell) => cell.key,
      draw: (cell) =>
        new Row(cell.works, list.ownerDocument, true, [
          "open",
          "copy",
          field(cell.key === "a" ? seen : "1"),
        ]),
      moved: ["a", "b"],
    });
  rows("1");
  list.rows[1]?.children[1]?.focus();
  rows("2");
  assert.equal(list.ownerDocument.activeElement, list.rows[1]?.children[1]);
});

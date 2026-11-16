// Which press a page answers, and with what: its list of keys wherever focus
// is, its own keys only bare and never while Joel types.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { type Actions, chosen, type Press } from "../keys.ts";

const down = (): void => undefined;
const actions: Actions = { j: down, ArrowDown: down };

/** `key` pressed bare, outside anywhere Joel types, but for what `but` says. */
function press(key: string, but: Partial<Press> = {}): Press {
  return { key, altKey: false, ctrlKey: false, metaKey: false, typing: false, ...but };
}

it("a page's key is answered with its action", () => {
  assert.equal(chosen(press("j"), actions), down);
  assert.equal(chosen(press("ArrowDown"), actions), down);
});

it("question mark and F1 ask for the page's keys, named or not", () => {
  assert.equal(chosen(press("?"), actions), "help");
  assert.equal(chosen(press("F1"), {}), "help");
});

it("a key held with a modifier is the browser's", () => {
  for (const held of ["altKey", "ctrlKey", "metaKey"] as const) {
    assert.equal(chosen(press("j", { [held]: true }), actions), undefined, `with ${held}`);
    assert.equal(chosen(press("?", { [held]: true }), actions), undefined, `? with ${held}`);
  }
});

it("a key pressed where Joel types is the field's", () => {
  assert.equal(chosen(press("j", { typing: true }), actions), undefined);
  assert.equal(chosen(press("?", { typing: true }), actions), undefined);
});

it("a key the page doesn't name is nothing, even one every object answers to", () => {
  assert.equal(chosen(press("x"), actions), undefined);
  assert.equal(chosen(press("constructor"), actions), undefined);
  assert.equal(chosen(press("toString"), actions), undefined);
});

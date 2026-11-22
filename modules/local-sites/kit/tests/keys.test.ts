// Which press a page answers, and with what: its list of keys wherever focus
// is, its own keys only bare and never while Joel types.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { type Actions, chosen, listen, type Press, showsKeys } from "../keys.ts";

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

it("a key shows a keyboard at hand, but not one typed in a field or composed", () => {
  assert.equal(showsKeys("j", false), true);
  assert.equal(showsKeys("Escape", false), true);
  assert.equal(showsKeys("j", true), false, "a phone's keyboard typing in a field");
  assert.equal(showsKeys("Unidentified", false), false, "Android's keyboard composing");
  assert.equal(showsKeys("Process", false), false, "an input method composing");
});

it("a key pressed outside a field marks the page as having keys; one in a field doesn't", () => {
  const root = { dataset: {} as Record<string, string> };
  let heard: ((event: unknown) => void) | undefined;
  const page = globalThis as unknown as Record<string, unknown>;
  const before = { ...page };
  class Element {
    closest(_: string): unknown {
      return {};
    }
  }
  Object.assign(page, {
    Element,
    document: { documentElement: root, body: { closest: () => null } },
    addEventListener: (_: string, then: (event: unknown) => void) => {
      heard = then;
    },
    removeEventListener: () => undefined,
  });
  try {
    listen({ actions: {} });
    assert.equal(root.dataset["keyed"], undefined, "marked before any key was pressed");
    const bare = { altKey: false, ctrlKey: false, metaKey: false };
    heard?.({ ...bare, key: "x", target: new Element() });
    assert.equal(root.dataset["keyed"], undefined, "a key typed in a field marked the page");
    heard?.({ ...bare, key: "Unidentified", target: null });
    assert.equal(root.dataset["keyed"], undefined, "a composing key marked the page");
    heard?.({ ...bare, key: "x", target: null });
    assert.equal(root.dataset["keyed"], "", "a key pressed didn't mark the page");
  } finally {
    for (const name of ["Element", "document", "addEventListener", "removeEventListener"]) {
      if (Object.hasOwn(before, name)) {
        page[name] = before[name];
      } else {
        delete page[name];
      }
    }
  }
});

it("head.css reads the mark keys.ts writes and the keycaps pressable makes", () => {
  const sheet = readFileSync(new URL("../head.css", import.meta.url), "utf8");
  assert.ok(sheet.includes(":root:not([data-keyed])"), "head.css doesn't read data-keyed");
  assert.ok(sheet.includes('kbd[role="button"]'), "head.css doesn't keep tappable keycaps");
});

it("a key the page doesn't name is nothing, even one every object answers to", () => {
  assert.equal(chosen(press("x"), actions), undefined);
  assert.equal(chosen(press("constructor"), actions), undefined);
  assert.equal(chosen(press("toString"), actions), undefined);
});

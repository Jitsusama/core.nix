// Which press a page answers, and with what: its list of keys wherever focus
// is, its own keys only bare and never while Joel types.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { type Actions, chosen, listen, type Press, showsKeys, watchForKeys } from "../keys.ts";

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
  assert.equal(showsKeys("AudioVolumeDown", false), false, "a phone's volume button");
  assert.equal(showsKeys("GoBack", false), false, "a phone's back button");
  assert.equal(showsKeys("Dead", false), true, "a real keyboard's accent key");
});

/** Where a selector's tag ends: a pseudo-class, an attribute or a space. */
const TAG_END = /[:[\s]/u;

/** An element answering `closest` only for a selector naming its own tag, as a browser would. */
class Element {
  readonly tag: string;

  constructor(tag: string) {
    this.tag = tag;
  }

  closest(selector: string): Element | null {
    const tags = selector.split(",").map((one) => one.trim().split(TAG_END)[0]);
    return tags.includes(this.tag) ? this : null;
  }
}

/** A page with no keys pressed yet: what `run` hears on it, then everything put back. */
function onAPage(
  run: (root: { dataset: Record<string, string> }, heard: Set<(event: unknown) => void>) => void,
): void {
  const root = { dataset: {} as Record<string, string> };
  const heard = new Set<(event: unknown) => void>();
  const page = globalThis as unknown as Record<string, unknown>;
  const before = { ...page };
  Object.assign(page, {
    Element,
    document: { documentElement: root, body: new Element("body") },
    addEventListener: (_: string, then: (event: unknown) => void) => heard.add(then),
    removeEventListener: (_: string, then: (event: unknown) => void) => heard.delete(then),
  });
  try {
    run(root, heard);
  } finally {
    for (const name of ["Element", "document", "addEventListener", "removeEventListener"]) {
      if (Object.hasOwn(before, name)) {
        page[name] = before[name];
      } else {
        delete page[name];
      }
    }
  }
}

const bare = { altKey: false, ctrlKey: false, metaKey: false };

it("a key pressed outside a field marks the page as having keys; one in a field doesn't", () => {
  onAPage((root, heard) => {
    listen({ actions: {} });
    const hear = (event: object): void => {
      for (const then of heard) {
        then({ ...bare, ...event });
      }
    };
    assert.equal(root.dataset["keyed"], undefined, "marked before any key was pressed");
    hear({ key: "x", target: new Element("input") });
    assert.equal(root.dataset["keyed"], undefined, "a key typed in a field marked the page");
    hear({ key: "x", target: new Element("textarea") });
    assert.equal(root.dataset["keyed"], undefined, "a key typed in a text box marked the page");
    hear({ key: "Unidentified", target: new Element("body") });
    assert.equal(root.dataset["keyed"], undefined, "a composing key marked the page");
    hear({ key: "j", target: new Element("li") });
    assert.equal(root.dataset["keyed"], "", "j on a focused list didn't mark the page");
  });
});

it("a held key marks the page once, not at every repeat", () => {
  onAPage((root, heard) => {
    let writes = 0;
    root.dataset = new Proxy({} as Record<string, string>, {
      set: (held, name, value): boolean => {
        writes += 1;
        return Reflect.set(held, name, value);
      },
    });
    listen({ actions: {} });
    for (let repeat = 0; repeat < 30; repeat += 1) {
      for (const then of heard) {
        then({ ...bare, key: "j", target: new Element("body") });
      }
    }
    assert.equal(writes, 1, `a held j wrote the mark ${String(writes)} times`);
  });
});

it("a page answering its own keys is marked by its watch, and each watch stops apart", () => {
  onAPage((root, heard) => {
    const stopFirst = watchForKeys();
    const stopSecond = watchForKeys();
    assert.equal(heard.size, 2, "two watches listen apart");
    stopFirst();
    assert.equal(heard.size, 1, "one stopping left the other's watch");
    for (const then of heard) {
      then({ ...bare, key: "j", target: new Element("body") });
    }
    assert.equal(root.dataset["keyed"], "", "the watch left didn't mark the page");
    stopSecond();
    assert.equal(heard.size, 0, "the last stop left a watch behind");
  });
});

it("head.css hides the key line only on touch, by the names keys.ts and pressable write", () => {
  // This proves the two sides name the same marks and that the rule sits
  // inside the touch query; what a phone shows is proved by checks.md's
  // touch run in a browser.
  const sheet = readFileSync(new URL("../head.css", import.meta.url), "utf8");
  const touch = sheet.indexOf("@media (hover: none) and (pointer: coarse)");
  const hides = sheet.indexOf(":root:not([data-keyed]) .page-head .keys:not(:has(kbd[data-tap]))");
  // The query ends at the first brace back at the start of a line after it.
  const ends = sheet.indexOf("\n}", touch);
  assert.ok(touch >= 0, "head.css has no touch query");
  assert.ok(
    hides > touch && hides < ends,
    "the rule hiding the key line isn't inside the touch query",
  );
});

it("a key the page doesn't name is nothing, even one every object answers to", () => {
  assert.equal(chosen(press("x"), actions), undefined);
  assert.equal(chosen(press("constructor"), actions), undefined);
  assert.equal(chosen(press("toString"), actions), undefined);
});

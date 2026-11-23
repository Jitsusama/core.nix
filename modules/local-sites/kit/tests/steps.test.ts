// Where a step through a list takes its reader, and when two presses of g
// come close enough to go to the top, as a document and the home both step.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { onTwice, pressable, stepped, twice } from "../steps.ts";
import { Hands } from "./hands.ts";

it("a step goes to the next or the last place and stops at the ends", () => {
  assert.equal(stepped(0, 1, 3), 1, "j from the first goes to the second");
  assert.equal(stepped(2, 1, 3), 2, "j from the last stays there");
  assert.equal(stepped(1, -1, 3), 0, "k from the second goes to the first");
  assert.equal(stepped(0, -1, 3), 0, "k from the first stays there");
});

it("from above the first place, j reaches the first", () => {
  assert.equal(stepped(-1, 1, 3), 0);
});

it("in an empty list a step stays at the first place, which holds nothing", () => {
  assert.equal(stepped(0, 1, 0), 0);
});

it("g counts as twice only when its second press comes soon after the first", () => {
  assert.equal(twice(1000, 1500), true, "half a second on is twice");
  assert.equal(twice(1000, 1700), false, "most of a second on is a fresh first press");
  assert.equal(twice(Number.NEGATIVE_INFINITY, 1000), false, "the first press ever is a first");
});

/** A head's `kbd` naming `key`, keeping what listens to it. */
function kbdNaming(key: string) {
  const heard = new Map<string, (event: unknown) => void>();
  return {
    textContent: key,
    tabIndex: -1,
    dataset: {} as Record<string, string>,
    heard,
    setAttribute: () => undefined,
    addEventListener: (kind: string, listener: (event: unknown) => void) => {
      heard.set(kind, listener);
    },
  };
}

it("Return on a pressable key does its act alone, never reaching the page's own Return", () => {
  const kbd = kbdNaming("j");
  Object.assign(globalThis, { document: { querySelectorAll: () => [kbd] } });
  let acted = 0;
  pressable(".keys kbd", {
    j: () => {
      acted += 1;
    },
  });
  let stopped = false;
  let prevented = false;
  kbd.heard.get("keydown")?.({
    key: "Enter",
    preventDefault: () => {
      prevented = true;
    },
    stopPropagation: () => {
      stopped = true;
    },
  });
  assert.equal(acted, 1, "Return on the key does what typing j does");
  assert.equal(prevented, true, "and nothing the browser does by default");
  assert.equal(stopped, true, "and goes no further, where the page would open its cursor");
  assert.equal(kbd.tabIndex, 0, "the key is reached by Tab");
});

it("a pressable key is marked a tap for the touch rule, but one that only lists the keys isn't", () => {
  const keys = [kbdNaming("j"), kbdNaming("?"), kbdNaming("F1")];
  Object.assign(globalThis, { document: { querySelectorAll: () => keys } });
  const nothing = (): void => undefined;
  pressable(".keys kbd", { j: nothing, "?": nothing, F1: nothing });
  assert.deepEqual(
    keys.map((kbd) => "tap" in kbd.dataset),
    [true, false, false],
    "j is a tap, and ? and F1 aren't",
  );
});

it("a key the markup marks a tap that does nothing when tapped loses the mark", () => {
  const keys = [kbdNaming("j"), kbdNaming("k"), kbdNaming("?")];
  for (const kbd of keys) {
    kbd.dataset["tap"] = "";
  }
  Object.assign(globalThis, { document: { querySelectorAll: () => keys } });
  const nothing = (): void => undefined;
  pressable(".keys kbd", { j: nothing, "?": nothing });
  assert.deepEqual(
    keys.map((kbd) => "tap" in kbd.dataset),
    [true, false, false],
    "j keeps its mark, and k, handed nothing, and ?, which only lists the keys, lose it",
  );
});

it("a key pressed twice in a row acts once, and a third press starts over", () => {
  const hands = new Hands();
  let acted = 0;
  const press = onTwice(() => {
    acted += 1;
  }, hands.now);
  press();
  assert.equal(acted, 0, "one press does nothing yet");
  hands.pass(200);
  press();
  assert.equal(acted, 1, "the second press soon after acts");
  hands.pass(200);
  press();
  assert.equal(acted, 1, "a third press is a fresh first");
  hands.pass(200);
  press();
  assert.equal(acted, 2, "so a fourth soon after acts again");
  hands.pass(200);
  press();
  hands.pass(900);
  press();
  assert.equal(acted, 2, "a press long after the last is a fresh first");
  hands.pass(200);
  press();
  assert.equal(acted, 3, "and the press soon after that one acts");
});

// Where a step through a list takes its reader, and when two presses of g
// come close enough to go to the top, as a document and the home both step.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { onTwice, stepped, twice } from "../steps.ts";
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

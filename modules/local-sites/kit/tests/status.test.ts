// How each state of a feed is said: always with since when, and never as
// now once it has stopped.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { clockTime, type Shown, said, show, tell } from "../status.ts";

const at = new Date(2026, 10, 16, 0, 41).getTime();

it("a time is said as the hour and minute", () => {
  assert.equal(clockTime(at), "00:41");
});

it("live says since when", () => {
  assert.equal(said({ state: "live", since: at }), "Live since 00:41.");
});

it("the same state told again leaves the words alone, so it isn't heard again", () => {
  let writes = 0;
  let words: string | null = null;
  const element: Shown = {
    get textContent() {
      return words;
    },
    set textContent(given) {
      writes += 1;
      words = given;
    },
    dataset: {},
  };
  show(element, { state: "live", since: at });
  show(element, { state: "live", since: at });
  assert.equal(writes, 1, "the words were written again for the same state");
  show(element, { state: "stale", since: at });
  assert.equal(writes, 2, "a new state's words weren't written");
  assert.equal(element.dataset["state"], "stale");
  show(element, { state: "stale", since: at }, " Couldn't read: the disk.");
  assert.equal(writes, 3, "the page's own words after the state weren't written");
  assert.ok(words?.endsWith("Trying again. Couldn't read: the disk."), words ?? "");
  tell(element, words ?? "", "broken");
  assert.equal(writes, 3, "a page's own words were written again unchanged");
  assert.equal(element.dataset["state"], "broken");
});

it("stale and broken say it stopped, when, and why", () => {
  const stale = said({ state: "stale", since: at });
  assert.ok(stale.startsWith("Not live since 00:41: the sites' server can't be reached."), stale);
  const broken = said({ state: "broken", since: at, reason: "nobody.sock isn't there" });
  assert.equal(broken, "Not live since 00:41: nobody.sock isn't there. This is how it stood then.");
});

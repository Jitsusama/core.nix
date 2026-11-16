// How each state of a feed is said: always with since when, and never as
// now once it has stopped.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { clockTime, said } from "../status.ts";

const at = new Date(2026, 10, 16, 0, 41).getTime();

it("a time is said as the hour and minute", () => {
  assert.equal(clockTime(at), "00:41");
});

it("live says since when", () => {
  assert.equal(said({ state: "live", since: at }), "Live since 00:41.");
});

it("stale and broken say it stopped, when, and why", () => {
  const stale = said({ state: "stale", since: at });
  assert.ok(stale.startsWith("Not live since 00:41: the sites' server can't be reached."), stale);
  const broken = said({ state: "broken", since: at, reason: "nobody.sock isn't there" });
  assert.equal(broken, "Not live since 00:41: nobody.sock isn't there. This is how it stood then.");
});

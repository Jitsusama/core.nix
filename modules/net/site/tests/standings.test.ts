// How net's feeds stand, as the head's standing line and the lede read them.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Standings } from "../panel.ts";

it("says the worst of the feeds, whichever said it last", () => {
  const standings = new Standings();
  assert.equal(standings.worst(), undefined, "a feed stood before any said so");
  standings.set("device", { state: "live", since: 1 });
  standings.set("link", { state: "stale", since: 2 });
  standings.set("source", { state: "connecting", since: 3 });
  assert.equal(standings.worst()?.state, "stale");
  standings.set("link", { state: "live", since: 4 });
  assert.equal(standings.worst()?.state, "connecting");
  standings.set("internet", { state: "broken", since: 5 });
  assert.equal(standings.worst()?.state, "broken");
});

it("counts net as answered once any feed was live, and after it falls", () => {
  const standings = new Standings();
  standings.set("device", { state: "connecting", since: 1 });
  assert.deepEqual(standings.answering(), { answered: false, worst: "connecting" });
  standings.set("device", { state: "live", since: 2 });
  standings.set("device", { state: "broken", since: 3 });
  assert.deepEqual(standings.answering(), { answered: true, worst: "broken" });
});

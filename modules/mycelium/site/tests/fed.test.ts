// A receptor feeds one outside change to the one cell whose quest claims
// it, and the node says each such feeding for ten minutes (its `fed` kind):
// these prove what the map keeps of one, for how long, in what order, and
// that a node too old to serve them reads as one that says nothing of them.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { expiryOf, FED_AHEAD_MS, FED_SHOWN_MS, nowOf } from "../model.ts";

const AT = Date.parse("2026-11-18T08:00:00Z");
const MINUTE_MS = 60_000;

/**
 * A feeding as the node serves the `fed` kind, said `ago` before AT. Its
 * own word for what was fed sits under `source`, as the node moves any
 * field named as the interface's own.
 */
function fed(receptor: string, cell: string, ago: number, more: Record<string, unknown> = {}) {
  const at = new Date(AT - ago).toISOString();
  const key = `fed:${receptor}~${cell}`;
  return { key, kind: "fed", heard: AT - ago, receptor, cell, at, ...more };
}

/** A word for what was fed, as the node serves it. */
const REVIEW = { source: { kind: "review" } };

/** The four watches the map waits on, empty, and the fed kind's, told `these`. */
function told(these: readonly object[] | undefined) {
  const heard = { network: new Held(), cell: new Held(), machine: new Held(), message: new Held() };
  if (these === undefined) {
    return heard;
  }
  const feedings = new Held();
  feedings.took({ records: these });
  return { ...heard, fed: feedings };
}

it("a node that serves no feeding says nothing of them, not that none were made", () => {
  assert.equal(nowOf(told(undefined), AT).fed, null);
  assert.deepEqual(nowOf(told([]), AT).fed, []);
});

it("a feeding is kept as who fed what to whom and when, never more", () => {
  const one = fed("github@kestrel", "KESTRL1", MINUTE_MS, { ...REVIEW, title: "words" });
  assert.deepEqual(nowOf(told([one]), AT).fed, [
    { receptor: "github@kestrel", cell: "KESTRL1", at: "2026-11-18T07:59:00.000Z", kind: "review" },
  ]);
});

it("a feeding the node gave no word is kept, with no kind, never the interface's", () => {
  const [kept] = nowOf(told([fed("github@kestrel", "KESTRL1", MINUTE_MS)]), AT).fed ?? [];
  assert.ok(kept, "the feeding was dropped");
  assert.equal(kept.kind, null);
});

it("a feeding goes once its ten minutes are up, whatever the watch still holds", () => {
  assert.equal(FED_SHOWN_MS, 10 * MINUTE_MS, "a feeding is shown for other than the node's ten");
  const these = [
    fed("github@kestrel", "FRESH1", 10 * MINUTE_MS - 1),
    fed("github@kestrel", "STALE1", 10 * MINUTE_MS),
  ];
  assert.deepEqual(
    nowOf(told(these), AT).fed?.map((each) => each.cell),
    ["FRESH1"],
  );
});

it("a feeding said a moment ahead of this clock is drawn, one a minute ahead is not", () => {
  // Further ahead, it would stay up past its ten minutes, which are its own.
  assert.equal(FED_AHEAD_MS, MINUTE_MS, "a feeding may be said other than a minute ahead");
  const these = [
    fed("github@kestrel", "KESTRL1", -2000),
    fed("github@kestrel", "AHEAD1", -MINUTE_MS),
    fed("github@kestrel", "FARAWAY", -10 * MINUTE_MS),
  ];
  assert.deepEqual(
    nowOf(told(these), AT).fed?.map((each) => each.cell),
    ["KESTRL1"],
  );
});

it("the map draws again when the first feeding's ten minutes are up", () => {
  const these = [
    fed("github@kestrel", "NEWER1", MINUTE_MS),
    fed("github@kestrel", "OLDER1", 4 * MINUTE_MS),
  ];
  assert.equal(expiryOf(nowOf(told(these), AT)), AT + 6 * MINUTE_MS);
});

it("a feeding said ahead of this clock goes ten minutes after its own time", () => {
  const ahead = [fed("github@kestrel", "AHEAD1", -30_000)];
  assert.equal(expiryOf(nowOf(told(ahead), AT)), AT + 30_000 + FED_SHOWN_MS);
});

it("a map with no feeding and no balance has nothing to go out of date", () => {
  assert.equal(expiryOf(nowOf(told(undefined), AT)), undefined);
  assert.equal(expiryOf(nowOf(told([]), AT)), undefined);
});

it("feedings stand the latest first", () => {
  const these = [
    fed("github@kestrel", "OLDER1", 5 * MINUTE_MS),
    fed("github@kestrel", "NEWER1", MINUTE_MS),
    fed("github@osprey", "MIDDLE", 3 * MINUTE_MS),
  ];
  assert.deepEqual(
    nowOf(told(these), AT).fed?.map((each) => each.cell),
    ["NEWER1", "MIDDLE", "OLDER1"],
  );
});

it("a feeding missing an end, or a time that reads as none, is no feeding", () => {
  const these = [
    fed("", "KESTRL1", MINUTE_MS),
    fed("github@kestrel", "", MINUTE_MS),
    fed("github@kestrel", "SOON01", MINUTE_MS, { at: "soon" }),
    fed("github@kestrel", "SEVEN1", MINUTE_MS, { at: 7 }),
  ];
  assert.deepEqual(nowOf(told(these), AT).fed, []);
});

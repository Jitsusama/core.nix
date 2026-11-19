// When things happened, as the drawer says it against the reading's own
// time. Every time here is written in the reader's own zone, with no offset,
// so the words are the same wherever the tests run.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { agoWords, atWords, clock, dayClock, leftWords, minutesWord } from "../when.ts";

const AT = "2026-11-19T10:00:00";

it("reads a clock to the second, or to the minute", () => {
  assert.equal(clock("2026-11-19T06:49:05"), "06:49:05", "a clock should keep its seconds");
  assert.equal(clock("2026-11-19T06:49:05", false), "06:49", "a clock asked should drop them");
});

it("says a time's day only off the reading's own", () => {
  const said = [
    dayClock("2026-11-19T06:49:00", AT),
    dayClock("2026-11-18T06:49:00", AT),
    dayClock("2026-11-20T06:49:00", AT),
    dayClock("2026-11-03T06:49:00", AT),
  ];
  assert.deepEqual(
    said,
    ["06:49", "yesterday at 06:49", "tomorrow at 06:49", "3 Nov at 06:49"],
    "a time should carry its day only when that isn't the reading's",
  );
});

it("puts a time after a verb with the word it takes", () => {
  const said = [
    atWords("2026-11-19T06:49:00", AT),
    atWords("2026-11-03T06:49:00", AT),
    atWords("2026-11-18T06:49:00", AT),
  ];
  assert.deepEqual(
    said,
    ["at 06:49", "on 3 Nov at 06:49", "yesterday at 06:49"],
    "a clock should take at, a date on, and yesterday nothing",
  );
});

it("says how long ago by the minute within the hour, and by the clock past it", () => {
  const said = [
    agoWords("2026-11-19T09:59:30", AT),
    agoWords("2026-11-19T09:55:00", AT),
    agoWords("2026-11-19T08:00:00", AT),
    agoWords("2026-11-19T10:05:00", AT),
  ];
  assert.deepEqual(
    said,
    ["under a minute ago", "5 min ago", "at 08:00", "at 10:05"],
    "a time ahead of the reading should be said by the clock",
  );
});

it("says a wait under a minute, or in whole minutes", () => {
  assert.deepEqual(
    [minutesWord(59_999), minutesWord(90_000)],
    ["under a minute", "2 min"],
    "a wait should round to the nearest minute once it's one",
  );
});

it("says how long a lease has left, and when it ends", () => {
  const said = [
    leftWords("2026-11-19T11:30:00", AT),
    leftWords("2026-11-19T12:00:00", AT),
    leftWords("2026-11-19T10:05:00", AT),
    leftWords("2026-11-19T10:00:30", AT),
    leftWords("2026-11-19T09:00:00", AT),
    leftWords("not a time", AT),
  ];
  assert.deepEqual(
    said,
    [
      "1 h 30 min left, until 11:30",
      "2 h left, until 12:00",
      "5 min left, until 10:05",
      "0 min left, until 10:00",
      "its time is up",
      "its time is up",
    ],
    "a lease should say hours and minutes left, or that its time is up",
  );
});

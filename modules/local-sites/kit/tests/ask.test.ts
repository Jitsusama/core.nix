// What a page's ask row reads from a look, as the launcher's For You reads
// it: only a look For You reads whole, the second form's words where it
// says what happened, what it's about from the first form, only web
// addresses as links, only the values the session offered sent exactly as
// written, and a look waiting only as For You's Waiting holds it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { ACTS, answering, asked, OFFERED, QUIET_AFTER } from "../ask.ts";

/** When the tests read each look: an hour after it came. */
const NOW = Date.parse("2026-11-23T15:00:00Z");

/** A look of the first form, as the harness wrote before the second. */
const first = {
  v: 1,
  id: "look-1",
  at: "2026-11-23T14:00:00Z",
  kind: "pr",
  target: "https://github.com/Jitsusama/core.nix/pull/6131",
  title: "Give the Live Sites One Head",
  why: "The shared head is ready.",
  ask: "Say whether the key line waits on touch.",
  ifNothing: "It keeps waiting for a key.",
  choices: [
    { value: "keep", label: "Keep It" },
    { value: "show", label: "Always Show It" },
  ],
  state: "waiting",
};

/**
 * The same look with the second form's fields beside the first's, as the
 * harness writes version 1, each saying something its first-form twin
 * doesn't, so a test tells which it read. Its `about` names another page,
 * which For You never opens.
 */
const second = {
  ...first,
  happened: "The live sites' shared head is ready and its review is done",
  matters: "Your pages take it on Thursday, so how its key line behaves on your phone is set now.",
  act: "Check the Key Line",
  wants: {
    choices: [
      { label: "Keep It", follows: "The key line waits for a real key on touch." },
      { label: "Always Show It", follows: "The line shows on every phone from the start." },
    ],
    words: true,
  },
  about: {
    kind: "page",
    target: "https://sites.example/home/",
    title: "The Home's Shared Head",
  },
};

it("a look of the second form leads with what happened and why it matters, as one paragraph", () => {
  const ask = asked(second, NOW);
  assert.ok(ask !== null, "a whole look of the second form wasn't read");
  assert.deepEqual(ask.said, [`${second.happened}. ${second.matters}`]);
  assert.equal(ask.question, first.ask, "the question isn't the record's ask");
  assert.equal(ask.ifNothing, first.ifNothing, "what happens if nothing isn't the record's");
  assert.equal(ask.act, "Check the Key Line");
  assert.equal(ask.words, true);
  assert.equal(ask.asks, true);
});

it("what it's about is the first form's, which For You opens, whatever about says", () => {
  const ask = asked(second, NOW);
  assert.equal(ask?.kind, "pr");
  assert.equal(ask?.title, first.title);
  assert.equal(ask?.opens, first.target);
  const script = { ...second, target: "javascript:alert(1)" };
  assert.equal(asked(script, NOW)?.opens, null, "a script target was linked");
  assert.equal(asked(script, NOW)?.act, null, "an act was offered that the page can't take");
});

it("a look For You doesn't read whole isn't shown", () => {
  const unread = [
    { ...second, v: 2 },
    { ...second, kind: "slack" },
    { ...second, kind: undefined },
    { ...second, title: "" },
    { ...second, target: undefined },
    { ...second, id: "" },
    { ...second, state: " waiting" },
    { ...second, state: "waitng" },
    { ...second, state: undefined },
    {
      v: 2,
      id: "look-2",
      state: "waiting",
      happened: "The map's cells are marked.",
      ...second.about,
    },
    "look-1",
    null,
  ];
  for (const look of unread) {
    assert.equal(
      asked(look, NOW),
      null,
      `a look For You doesn't read was shown: ${JSON.stringify(look)}`,
    );
  }
});

it("each choice sends the value the session offered, with what follows from its label", () => {
  const ask = asked(second, NOW);
  assert.deepEqual(ask?.choices, [
    { value: "keep", label: "Keep It", follows: "The key line waits for a real key on touch." },
    {
      value: "show",
      label: "Always Show It",
      follows: "The line shows on every phone from the start.",
    },
  ]);
});

it("a second-form choice the first form didn't offer is never sent, whatever its label", () => {
  const reworded = {
    ...second,
    wants: { choices: [{ label: "Keep it", follows: "Reworded." }, { label: "Roll It Back" }] },
  };
  const ask = asked(reworded, NOW);
  assert.deepEqual(
    ask?.choices.map((one) => [one.value, one.label, one.follows]),
    [
      ["keep", "Keep It", ""],
      ["show", "Always Show It", ""],
    ],
  );
  const unlabelled = { ...second, choices: [{ value: "keep" }] };
  assert.deepEqual(asked(unlabelled, NOW)?.choices, [
    { value: "keep", label: "keep", follows: "" },
  ]);
});

it("a look of the first form reads from its why, with its question and an act for its kind", () => {
  const ask = asked(first, NOW);
  assert.deepEqual(ask?.said, [first.why]);
  assert.equal(ask?.question, first.ask);
  assert.equal(ask?.ifNothing, first.ifNothing);
  assert.equal(ask?.act, ACTS.pr);
  assert.equal(ask?.words, false);
});

it("the second form's words lead only where it says what happened", () => {
  const unsaid = { ...second, happened: "  " };
  const ask = asked(unsaid, NOW);
  assert.deepEqual(ask?.said, [first.why], "why it matters led without what happened");
  assert.equal(ask?.act, ACTS.pr, "the second form's act was taken without what happened");
});

it("only a web address is a link: a file, a session or a script is shown, never opened", () => {
  const file = { ...first, kind: "file", target: "/home/joel/notes.md" };
  assert.equal(asked(file, NOW)?.opens, null);
  assert.equal(asked(file, NOW)?.act, null);
  assert.equal(asked(file, NOW)?.target, "/home/joel/notes.md");
  const session = { ...first, kind: "terminal", target: "01a1fc95" };
  assert.equal(asked(session, NOW)?.act, null);
  assert.equal(asked({ ...first, kind: "page", target: "javascript:alert(1)" }, NOW)?.opens, null);
});

it("the first four choices sending a value show, each value as written, as For You offers", () => {
  const many = Array.from({ length: 6 }, (_, at) => ({
    value: `pick-${String(at)}`,
    label: `Pick ${String(at)}`,
  }));
  const look = { ...first, choices: [{ value: "", label: "Nothing" }, ...many] };
  assert.deepEqual(
    asked(look, NOW)?.choices.map((one) => one.value),
    ["pick-0", "pick-1", "pick-2", "pick-3"].slice(0, OFFERED),
  );
  const twice = {
    ...first,
    choices: [
      { value: "merge", label: "OK" },
      { value: "close", label: "ok" },
      { value: "a" },
      { value: "b" },
      { value: "c" },
    ],
  };
  assert.deepEqual(
    asked(twice, NOW)?.choices.map((one) => one.value),
    ["merge", "close", "a", "b"],
    "the row offered other choices than For You's first four",
  );
  const padded = { ...first, choices: [{ value: "keep ", label: "Keep It" }, { value: " " }] };
  assert.deepEqual(
    asked(padded, NOW)?.choices.map((one) => one.value),
    ["keep ", " "],
    "a value wasn't kept exactly as the session offered it",
  );
});

it("a look waits only as For You's Waiting holds it", () => {
  for (const state of ["opened", "resolved", "cleared", "expired"]) {
    assert.equal(asked({ ...second, state }, NOW)?.asks, false, `a look "${state}" still asked`);
  }
  const lapsed = { ...second, expiresAt: "2026-11-23T14:59:00Z" };
  assert.equal(asked(lapsed, NOW)?.asks, false, "a look past its expiresAt still asked");
  const later = { ...second, expiresAt: "2026-11-23T16:00:00Z" };
  assert.equal(asked(later, NOW)?.asks, true, "a look before its expiresAt didn't ask");
  assert.equal(asked({ ...second, expiresAt: NOW }, NOW)?.asks, false, "it asked at its expiry");
  const came = Date.parse(first.at);
  assert.equal(asked(second, came + QUIET_AFTER)?.asks, true, "a look quieted on the hour");
  assert.equal(asked(second, came + QUIET_AFTER + 1)?.asks, false, "a quiet look still asked");
  assert.equal(asked({ ...second, asks: false }, NOW)?.asks, false, "a look to read asked");
});

it("a time written in milliseconds lapses and quiets a look as an ISO time does", () => {
  const lapsed = { ...second, expiresAt: NOW - 1 };
  assert.equal(asked(lapsed, NOW)?.asks, false, "a look past a numeric expiresAt still asked");
  assert.equal(asked({ ...second, expiresAt: NOW + 1 }, NOW)?.asks, true);
  const old = { ...second, at: NOW - QUIET_AFTER - 1 };
  assert.equal(asked(old, NOW)?.asks, false, "a look quiet by a numeric at still asked");
  assert.equal(asked({ ...second, at: NOW - QUIET_AFTER }, NOW)?.asks, true);
});

it("an answer is sent only as the look offered it, while it still asks at the press", () => {
  assert.deepEqual(answering(second, "keep", NOW), {
    act: "answer",
    look: "look-1",
    value: "keep",
    from: "joel",
  });
  assert.equal(answering(second, "Keep It", NOW), null, "a label was sent as a value");
  assert.equal(answering(second, "roll back", NOW), null);
  const padded = { ...first, choices: [{ value: "keep ", label: "Keep It" }] };
  assert.equal(answering(padded, "keep ", NOW)?.value, "keep ", "the offered value was changed");
  assert.equal(answering(padded, "keep", NOW), null, "a value the look didn't offer was sent");
  assert.equal(answering({ ...second, state: "resolved" }, "keep", NOW), null);
  assert.equal(
    answering({ ...second, v: 2 }, "keep", NOW),
    null,
    "a look For You can't see took one",
  );
  const expiring = { ...second, expiresAt: NOW + 60_000 };
  assert.ok(asked(expiring, NOW)?.asks, "a look before its expiry didn't ask");
  assert.equal(
    answering(expiring, "keep", NOW + 60_000),
    null,
    "a row left open answered a look past its expiry",
  );
});

// What a page's ask row reads from a look: the second form's fields
// wherever they're present, on a version 1 record, the first form's where
// they aren't, only web addresses as links, only the values the session
// offered sent, and a look waiting only as For You's Waiting holds it.

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
 * The same look with the second form's fields beside the first's, each
 * saying something its first-form twin doesn't, so a test tells which it read.
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

/** A look written only in the second form, as the harness will once every reader reads it. */
const only = {
  v: 2,
  id: "look-2",
  at: "2026-11-23T14:30:00Z",
  state: "waiting",
  happened: "The map's cells that need you are marked.",
  act: "See the Map",
  wants: { words: true },
  about: { kind: "page", target: "https://sites.example/mycelium/", title: "The Map" },
};

it("a look of the second form leads with what happened and why it matters, as one paragraph", () => {
  const ask = asked(second, NOW);
  assert.ok(ask !== null, "a whole look of the second form wasn't read");
  assert.deepEqual(ask.said, [`${second.happened}. ${second.matters}`]);
  assert.equal(ask.question, first.ask);
  assert.equal(ask.ifNothing, first.ifNothing);
  assert.equal(ask.act, "Check the Key Line");
  assert.equal(ask.words, true);
  assert.equal(ask.asks, true);
});

it("what it's about is the second form's, wherever the first form says otherwise", () => {
  const ask = asked(second, NOW);
  assert.equal(ask?.kind, "page");
  assert.equal(ask?.title, "The Home's Shared Head");
  assert.equal(ask?.opens, "https://sites.example/home/");
  const script = { ...second, about: { ...second.about, target: "javascript:alert(1)" } };
  assert.equal(asked(script, NOW)?.opens, null, "the second form's script target was linked");
  assert.equal(asked(script, NOW)?.act, null, "an act was offered that the page can't take");
});

it("a look in the second form alone is read whole", () => {
  const ask = asked(only, NOW);
  assert.ok(ask !== null, "a look of the second form alone wasn't read");
  assert.equal(ask.title, "The Map");
  assert.equal(ask.opens, "https://sites.example/mycelium/");
  assert.deepEqual(ask.said, ["The map's cells that need you are marked."]);
  assert.equal(ask.act, "See the Map");
  assert.deepEqual(ask.choices, []);
  assert.equal(ask.asks, true);
});

it("the second form is read whatever version the record says", () => {
  const ask = asked({ ...second, v: 2 }, NOW);
  assert.equal(ask?.act, "Check the Key Line");
  assert.deepEqual(ask?.said, [`${second.happened}. ${second.matters}`]);
  assert.equal(ask?.choices[0]?.follows, "The key line waits for a real key on touch.");
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
  assert.equal(ask?.act, ACTS.pr);
  assert.equal(ask?.title, first.title);
  assert.equal(ask?.words, false);
});

it("a look missing its id, what it's about or a known kind isn't shown", () => {
  assert.equal(asked({ ...first, id: "" }, NOW), null);
  assert.equal(asked({ ...first, target: undefined }, NOW), null);
  assert.equal(asked({ ...first, kind: "slack" }, NOW), null);
  assert.equal(asked("look-1", NOW), null);
  assert.equal(asked(null, NOW), null);
});

it("a second-form kind it doesn't know falls back to the first form's", () => {
  const novel = { ...second, about: { ...second.about, kind: "slack" } };
  assert.equal(asked(novel, NOW)?.kind, "pr");
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

it("at most four choices show, none sending nothing and no name twice, the first kept", () => {
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
    ],
  };
  assert.deepEqual(asked(twice, NOW)?.choices, [{ value: "merge", label: "OK", follows: "" }]);
});

it("an answered look says how and no longer asks", () => {
  const picked = asked({ ...second, answer: "keep" }, NOW);
  assert.equal(picked?.answered, "Keep It");
  assert.equal(picked?.asks, false);
  const where = asked({ ...second, answered: { choice: "show", at: "", where: "launcher" } }, NOW);
  assert.equal(where?.answered, "Always Show It");
  const told = asked({ ...second, answered: { words: "Leave it for now", at: "" } }, NOW);
  assert.equal(told?.answered, "your own words");
});

it("a look waits only as For You's Waiting holds it", () => {
  for (const state of ["opened", "resolved", "cleared", "expired", "", "waitng"]) {
    assert.equal(asked({ ...second, state }, NOW)?.asks, false, `a look "${state}" still asked`);
  }
  assert.equal(asked({ ...second, state: undefined }, NOW)?.asks, false);
  const lapsed = { ...second, expiresAt: "2026-11-23T14:59:00Z" };
  assert.equal(asked(lapsed, NOW)?.asks, false, "a look past its expiresAt still asked");
  const later = { ...second, expiresAt: "2026-11-23T16:00:00Z" };
  assert.equal(asked(later, NOW)?.asks, true, "a look before its expiresAt didn't ask");
  const came = Date.parse(first.at);
  assert.equal(asked(second, came + QUIET_AFTER)?.asks, true, "a look quieted on the hour");
  assert.equal(asked(second, came + QUIET_AFTER + 1)?.asks, false, "a quiet look still asked");
  assert.equal(asked({ ...second, asks: false }, NOW)?.asks, false, "a look to read asked");
});

it("an answer is sent only as the look offered it, while it still asks", () => {
  const ask = asked(second, NOW);
  assert.ok(ask !== null, "a whole look of the second form wasn't read");
  assert.deepEqual(answering(ask, { choice: "keep" }), {
    act: "answer",
    look: "look-1",
    value: "keep",
    from: "joel",
  });
  assert.equal(answering(ask, { choice: "Keep It" }), null, "a label was sent as a value");
  assert.equal(answering(ask, { choice: "roll back" }), null);
  assert.deepEqual(answering(ask, { words: "  Wait a day  " }), {
    act: "answer",
    look: "look-1",
    words: "Wait a day",
    from: "joel",
  });
  assert.equal(answering(ask, { words: "   " }), null);
  const choicesOnly = asked(first, NOW);
  assert.ok(choicesOnly !== null, "a whole look of the first form wasn't read");
  assert.equal(answering(choicesOnly, { words: "Wait a day" }), null);
  const resolved = asked({ ...second, state: "resolved" }, NOW);
  assert.ok(resolved !== null, "a resolved look wasn't read");
  assert.equal(answering(resolved, { choice: "keep" }), null, "a resolved look took an answer");
  const answered = asked({ ...second, answer: "show" }, NOW);
  assert.ok(answered !== null, "an answered look wasn't read");
  assert.equal(answering(answered, { choice: "keep" }), null, "a look was answered twice");
});

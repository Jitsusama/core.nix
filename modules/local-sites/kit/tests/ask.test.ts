// What a page's ask row reads from a look: the second form's fields
// wherever they're present, on a version 1 record, the first form's where
// they aren't, only web addresses as links, and only answers the look
// offers sent.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { ACTS, answering, asked, OFFERED } from "../ask.ts";

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
  choices: [
    { value: "keep", label: "Keep It" },
    { value: "show", label: "Always Show It" },
  ],
  state: "waiting",
};

/** The same look with the second form's fields beside the first's. */
const second = {
  ...first,
  happened: "The live sites' shared head is ready and its review is done.",
  matters: "Your pages take it on Thursday, so how its key line behaves on your phone is set now.",
  act: "Review It",
  wants: {
    choices: [
      { label: "Keep It", follows: "The key line waits for a real key on touch." },
      { label: "Always Show It", follows: "The line shows on every phone from the start." },
    ],
    words: true,
  },
  about: {
    kind: "pr",
    target: "https://github.com/Jitsusama/core.nix/pull/6131",
    title: "Give the Live Sites One Head",
  },
};

it("a look of the second form leads with what happened and why it matters, as one paragraph", () => {
  const ask = asked(second);
  assert.ok(ask !== null, "a whole look of the second form wasn't read");
  assert.deepEqual(ask.said, [`${second.happened} ${second.matters}`]);
  assert.equal(ask.act, "Review It");
  assert.equal(ask.title, "Give the Live Sites One Head");
  assert.equal(ask.words, true);
  assert.equal(ask.asks, true);
});

it("each choice carries what follows from it and the value the session reads", () => {
  const ask = asked(second);
  assert.deepEqual(ask?.choices, [
    { value: "keep", label: "Keep It", follows: "The key line waits for a real key on touch." },
    {
      value: "show",
      label: "Always Show It",
      follows: "The line shows on every phone from the start.",
    },
  ]);
});

it("the second form is read whatever version the record says", () => {
  assert.equal(asked({ ...second, v: 2 })?.act, "Review It");
});

it("a look of the first form reads from why and its ask, with an act for its kind", () => {
  const ask = asked(first);
  assert.deepEqual(ask?.said, [first.why, first.ask]);
  assert.equal(ask?.act, ACTS.pr);
  assert.deepEqual(
    ask?.choices.map((one) => [one.value, one.label, one.follows]),
    [
      ["keep", "Keep It", ""],
      ["show", "Always Show It", ""],
    ],
  );
  assert.equal(ask?.words, false);
});

it("a look missing its id, what it's about or a known kind isn't shown", () => {
  assert.equal(asked({ ...first, id: "" }), null);
  assert.equal(asked({ ...first, target: undefined }), null);
  assert.equal(asked({ ...first, kind: "slack" }), null);
  assert.equal(asked("look-1"), null);
  assert.equal(asked(null), null);
});

it("only a web address is a link: a file, a session or a script is shown, never opened", () => {
  assert.equal(asked(second)?.opens, second.about.target);
  const file = { ...first, kind: "file", target: "/home/joel/notes.md" };
  assert.equal(asked(file)?.opens, null);
  assert.equal(asked(file)?.target, "/home/joel/notes.md");
  assert.equal(asked({ ...first, kind: "page", target: "javascript:alert(1)" })?.opens, null);
});

it("at most four choices show, none nameless and no name twice", () => {
  const many = Array.from({ length: 6 }, (_, at) => ({ label: `Pick ${String(at)}`, follows: "" }));
  const look = { ...second, wants: { choices: [{ label: "", follows: "" }, ...many] } };
  assert.equal(asked(look)?.choices.length, OFFERED);
  const twice = { ...second, wants: { choices: [{ label: "Keep It" }, { label: "keep it" }] } };
  assert.deepEqual(
    asked(twice)?.choices.map((one) => one.label),
    ["Keep It"],
  );
});

it("an answered look says how and no longer asks", () => {
  const picked = asked({ ...second, answer: "keep" });
  assert.equal(picked?.answered, "Keep It");
  assert.equal(picked?.asks, false);
  const where = asked({ ...second, answered: { choice: "show", at: "", where: "launcher" } });
  assert.equal(where?.answered, "Always Show It");
  const told = asked({ ...second, answered: { words: "Leave it for now", at: "", where: "cli" } });
  assert.equal(told?.answered, "your own words");
});

it("a resolved look, or one asking nothing, doesn't ask", () => {
  assert.equal(asked({ ...second, state: "resolved" })?.asks, false);
  const telling = { ...second, wants: {}, choices: [] };
  assert.equal(asked(telling)?.asks, false);
});

it("an answer is sent only as the look offered it, from the site", () => {
  const ask = asked(second);
  assert.ok(ask !== null, "a whole look of the second form wasn't read");
  assert.deepEqual(answering(ask, { choice: "keep" }), {
    id: "look-1",
    choice: "keep",
    where: "site",
  });
  assert.equal(answering(ask, { choice: "roll back" }), null);
  assert.deepEqual(answering(ask, { words: "  Wait a day  " }), {
    id: "look-1",
    words: "Wait a day",
    where: "site",
  });
  assert.equal(answering(ask, { words: "   " }), null);
  const choicesOnly = asked(first);
  assert.ok(choicesOnly !== null, "a whole look of the first form wasn't read");
  assert.equal(answering(choicesOnly, { words: "Wait a day" }), null);
});

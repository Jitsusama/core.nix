// What a page's ask row reads from a look, as the launcher's For You reads
// it: only a look For You reads whole, the second form's words where it
// says what happened, what it's about from the first form, only web
// addresses as links, only the values the session offered sent exactly as
// written, and a look waiting only as For You's Waiting holds it. The
// check the-ask-row-reads-as-for-you holds the names and numbers these
// take as the launcher's against the launcher's own code
// (modules/local-sites/tests/ask-as-for-you.test.ts).

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { answering, asked, LONGEST_WAIT, OFFERED, OPEN, QUIET_AFTER, waitFor } from "../ask.ts";

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
  assert.equal(ask.question, "", "the second form asked the first form's question");
  assert.equal(ask.ifNothing, first.ifNothing, "what happens if nothing isn't the record's");
  assert.equal(ask.act, "Check the Key Line");
  assert.equal(ask.takesWords, true);
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
    { v: 1, id: "look-2", state: "waiting", happened: "The map's cells.", about: second.about },
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

it("a choice's own follows outranks the second form's of its label", () => {
  const own = { ...second, choices: [{ value: "keep", label: "Keep It", follows: "Its own." }] };
  assert.equal(asked(own, NOW)?.choices[0]?.follows, "Its own.");
});

it("what follows a choice is said only where the look says what happened", () => {
  const unsaid = { ...first, wants: second.wants };
  assert.deepEqual(
    asked(unsaid, NOW)?.choices.map((one) => one.follows),
    ["", ""],
    "a look of the first form said what follows its choices",
  );
  const own = { ...first, choices: [{ value: "keep", label: "Keep It", follows: "Its own." }] };
  assert.equal(asked(own, NOW)?.choices[0]?.follows, "", "a first-form choice's follows showed");
});

it("the second form's act is taken only in four words at most, never as a choice's name", () => {
  assert.equal(asked({ ...second, act: "Open It Up and Read" }, NOW)?.act, OPEN);
  assert.equal(asked({ ...second, act: "keep it." }, NOW)?.act, OPEN);
  assert.equal(asked({ ...second, act: "Read  It  Now" }, NOW)?.act, "Read  It  Now");
});

it("an act hiding what draws as nothing, turning its letters or named as the launcher's is refused", () => {
  const refused = [
    "Keep\u200b It",
    "Keep It\u200b",
    "Keep\u2060 It",
    "\u202etI peeK",
    "Read\u202e It",
    "Clear It",
    "clear  it.",
    "Answer",
    "Talk",
    "Go\u3164To",
  ];
  for (const act of refused) {
    assert.equal(
      asked({ ...second, act }, NOW)?.act,
      OPEN,
      `the act ${JSON.stringify(act)} showed`,
    );
  }
});

it("a choice posing as the launcher's own act is neither offered nor sent", () => {
  const posing = {
    ...second,
    choices: [
      { value: "close", label: "Clear It" },
      { value: "merge", label: "open." },
      { value: "ship", label: "\u202etI peeK" },
      { value: "keep", label: "Keep It" },
      { value: "fifth", label: "A Fifth" },
    ],
  };
  assert.deepEqual(
    asked(posing, NOW)?.choices.map((one) => one.value),
    ["keep"],
    "a posing choice was offered, or a fifth took a refused one's place",
  );
  for (const value of ["close", "merge", "ship", "fifth"]) {
    assert.equal(answering(posing, value, NOW), null, `the posing ${value} was sent`);
  }
  assert.equal(answering(posing, "keep", NOW)?.value, "keep");
  const hidden = { ...second, choices: [{ value: "close", label: "Clear\u200b It" }] };
  assert.deepEqual(asked(hidden, NOW)?.choices, [], "a choice hiding a space passed for Clear It");
  const asChoice = { ...posing, act: "Keep\u200b It" };
  assert.equal(asked(asChoice, NOW)?.act, OPEN, "an act reading as a choice offered showed");
  // "Ship\u202e" holds a turn, so it isn't shown, but it's still offered:
  // an act reading as it is refused, as For You tells the act apart from
  // the offers before any posing is dropped.
  const asDropped = {
    ...second,
    choices: [
      { value: "ship", label: "Ship\u202e" },
      { value: "keep", label: "Keep It" },
    ],
    act: "Ship",
  };
  assert.equal(asked(asDropped, NOW)?.act, OPEN, "an act reading as a dropped choice showed");
});

it("a look of the first form reads from its why, with its question and an act for its kind", () => {
  const ask = asked(first, NOW);
  assert.deepEqual(ask?.said, [first.why]);
  assert.equal(ask?.question, first.ask);
  assert.equal(ask?.ifNothing, first.ifNothing);
  assert.equal(ask?.act, OPEN);
  assert.equal(ask?.takesWords, false, "a look of the first form took his words");
  const asksBack = { ...first, wants: { words: true } };
  assert.equal(asked(asksBack, NOW)?.takesWords, false, "the first form's wants were read");
});

it("the second form's words lead only where it says what happened", () => {
  const unsaid = { ...second, happened: "  " };
  const ask = asked(unsaid, NOW);
  assert.deepEqual(ask?.said, [first.why], "why it matters led without what happened");
  assert.equal(ask?.act, OPEN, "the second form's act was taken without what happened");
});

it("only a web address is a link: a file, a session or a script is shown, never opened", () => {
  const file = { ...first, kind: "file", target: "/home/joel/notes.md" };
  assert.equal(asked(file, NOW)?.opens, null);
  assert.equal(asked(file, NOW)?.act, null);
  assert.equal(asked(file, NOW)?.target, "/home/joel/notes.md");
  const session = { ...first, kind: "terminal", target: "01a1fc95" };
  assert.equal(asked(session, NOW)?.act, null);
  for (const target of ["javascript:alert(1)", "file:///etc/passwd", "data:text/html,<b>x</b>"]) {
    assert.equal(asked({ ...first, kind: "page", target }, NOW)?.opens, null, `${target} linked`);
  }
});

it("a look about a session or a file links nowhere, even at a web address", () => {
  for (const kind of ["terminal", "file"]) {
    const ask = asked({ ...first, kind }, NOW);
    assert.equal(ask?.opens, null, `a ${kind} look was linked, where For You goes to its cell`);
    assert.equal(ask?.act, null);
  }
});

it("a plain http address links as https does", () => {
  const plain = { ...first, kind: "page", target: "http://example.com/" };
  assert.equal(asked(plain, NOW)?.opens, "http://example.com/");
});

it("a link opens the address as the browser parsed it", () => {
  const spaced = { ...first, target: "HTTPS://Example.com/a b" };
  assert.equal(asked(spaced, NOW)?.opens, "https://example.com/a%20b");
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

it("a look no longer waiting asks nothing, offers nothing and says nothing of doing nothing", () => {
  const ended = asked({ ...second, state: "resolved" }, NOW);
  assert.equal(ended?.question, "", "a resolved look still asked its question");
  assert.equal(ended?.ifNothing, "");
  assert.deepEqual(ended?.choices, []);
  assert.equal(ended?.lapses, null);
  assert.equal(ended?.takesWords, false, "a resolved look still took his words");
  assert.deepEqual(ended?.said, [`${second.happened}. ${second.matters}`], "its words went too");
});

it("a waiting look lapses at its expiresAt or its three days' quiet, whichever is sooner", () => {
  const came = Date.parse(first.at);
  const quiet = came + QUIET_AFTER + 1;
  assert.equal(asked(second, NOW)?.lapses, quiet);
  assert.equal(asked({ ...second, expiresAt: NOW + 5 }, NOW)?.lapses, NOW + 5);
  const later = { ...second, expiresAt: quiet + 86_400_000 };
  assert.equal(asked(later, NOW)?.lapses, quiet, "an expiry after the quiet set the lapse");
  assert.equal(asked({ ...second, at: undefined }, NOW)?.lapses, null);
  const untimed = { ...second, at: undefined, expiresAt: NOW + 5 };
  assert.equal(asked(untimed, NOW)?.lapses, NOW + 5, "an expiry alone set no lapse");
  for (const look of [second, later, { ...second, expiresAt: NOW + 5 }]) {
    const lapses = asked(look, NOW)?.lapses ?? Number.NaN;
    assert.equal(asked(look, lapses - 1)?.asks, true, "it stopped asking before it lapsed");
    assert.equal(asked(look, lapses)?.asks, false, "it still asked once it lapsed");
  }
});

it("a page's timer waits until a look lapses, but never past a browser timer's longest", () => {
  const soon = asked({ ...second, at: undefined, expiresAt: NOW + 5 }, NOW);
  assert.ok(soon !== null);
  assert.equal(waitFor(soon, NOW), 5);
  assert.equal(waitFor(soon, NOW + 9), 0, "a lapse already past waited");
  const month = 2_592_000_000;
  const far = asked({ ...second, at: undefined, expiresAt: NOW + month }, NOW);
  assert.ok(far !== null);
  assert.equal(far.lapses, NOW + month);
  // A browser's timer waits at most 2^31 - 1 ms, written here as the limit
  // itself, so the kit's constant can't drift with what it's held to.
  assert.equal(LONGEST_WAIT, 2_147_483_647);
  assert.equal(waitFor(far, NOW), 2_147_483_647, "a month's wait would fire at once");
  const never = asked({ ...second, at: undefined }, NOW);
  assert.ok(never !== null);
  assert.equal(waitFor(never, NOW), null);
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
  assert.deepEqual(
    answering(second, "keep", NOW),
    { act: "answer", look: "look-1", value: "keep" },
    "the page vouched for who pressed it",
  );
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
  const five = { ...first, choices: ["a", "b", "c", "d", "e"].map((value) => ({ value })) };
  assert.equal(answering(five, "d", NOW)?.value, "d");
  assert.equal(answering(five, "e", NOW), null, "a fifth value For You doesn't offer was sent");
  const came = Date.parse(first.at);
  assert.equal(answering(second, "keep", came + QUIET_AFTER + 1), null, "a quiet look took one");
  assert.equal(answering({ ...second, asks: false }, "keep", NOW), null, "a look to read took one");
  assert.equal(answering({ ...second, state: "opened" }, "keep", NOW), null);
  const expiring = { ...second, expiresAt: NOW + 60_000 };
  assert.ok(asked(expiring, NOW)?.asks, "a look before its expiry didn't ask");
  assert.equal(
    answering(expiring, "keep", NOW + 60_000),
    null,
    "a row left open answered a look past its expiry",
  );
});

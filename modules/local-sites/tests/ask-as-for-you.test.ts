// The kit's ask row reads a look's names and numbers as the launcher's For
// You does, held against the launcher's own code rather than against what
// ask.ts says it copied, so a rule changed on either side fails here. Each
// name LookForm.js reads as another, refuses as an act or finds posing as
// the launcher's, ask.ts reads and judges alike; the launcher's act names
// are those Now.qml's lookActNames names, as its words say them; and
// Looks.js's four choices offered and three days' quiet are the kit's. The
// check the-ask-row-reads-as-for-you hands it the launcher's files by the
// names `launcher` reads.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { runInNewContext } from "node:vm";
import { actOf, LAUNCHER_ACTS, OFFERED, poses, QUIET_AFTER, readAs } from "../kit/ask.ts";

/** What LookForm.js gives, run as Qt runs it: a script of functions. */
interface LookForm {
  readAs(name: string): string;
  poses(name: string, taken: readonly string[]): boolean;
  actOf(one: object, taken: readonly string[]): string;
}

/** A line of Qt's own, which no other engine reads, as `.pragma library`. */
const QT_LINE = /^\.(?:pragma|import)\b.*$/gmu;
/** Now.qml's list of the launcher's acts on a look, by their words' ids. */
const ACT_IDS = /lookActNames:\s*\[([^\]]*)\]/u;
const QUOTED = /"([^"]+)"/gu;
/** A constant's whole value in Looks.js, by its name. */
const CONSTANT = (name: string): RegExp => new RegExp(`^const ${name} = (.+);$`, "mu");

/** A file the check hands this, by the name it hands it under. */
function launcher(name: string): string {
  const path = process.env[name];
  assert.ok(path, `the check handed no ${name}`);
  return readFileSync(path, "utf8");
}

/** LookForm.js's functions, run apart from this test's own. */
function lookForm(): LookForm {
  const script = launcher("LOOK_FORM").replace(QT_LINE, "");
  return runInNewContext(`${script}\n;({ readAs, poses, actOf });`, {}) as LookForm;
}

/** Each word's name by its id, wherever the words file holds it. */
function namesIn(words: unknown, names: Map<string, string>): Map<string, string> {
  if (Array.isArray(words)) {
    for (const one of words) {
      namesIn(one, names);
    }
  } else if (words !== null && typeof words === "object") {
    const { id, name } = words as { id?: unknown; name?: unknown };
    if (typeof id === "string" && typeof name === "string") {
      names.set(id, name);
    }
    for (const one of Object.values(words)) {
      namesIn(one, names);
    }
  }
  return names;
}

/**
 * Names to tell apart: plain, spaced, stopped and cased; hiding what draws
 * as nothing, a space or a turn, the undrawn past U+FFFF among them; the
 * launcher's own; too long; and empty.
 */
const NAMES = [
  "Keep It",
  "keep it.",
  "KEEP IT!",
  "Keep  It",
  "Keep\u200b It",
  "Keep It\u200b",
  "Keep\u2060 It",
  "Keep\u00ad It",
  "Keep\u034f It",
  "Keep\ufe0f It",
  "Ke\u{1d173}ep It",
  "Keep\u{e0041} It",
  "Keep\u3164It",
  "\u2800Keep It",
  "\u202etI peeK",
  "Keep\u2066 It",
  "Open",
  "open.",
  "Answer",
  "Clear It",
  "clear  it.",
  "Go To",
  "Go\u3164To",
  "Explain It",
  "Talk",
  "Read the Diff",
  "One Two Three Four",
  "One Two Three Four Five",
  "  ",
  "",
];

it("reads every name as LookForm.js reads it", () => {
  const form = lookForm();
  for (const name of NAMES) {
    assert.equal(readAs(name), form.readAs(name), `${JSON.stringify(name)} read otherwise`);
  }
});

it("finds a name posing as the launcher's acts where LookForm.js does", () => {
  const form = lookForm();
  for (const name of NAMES) {
    for (const taken of [LAUNCHER_ACTS, []]) {
      assert.equal(
        poses(name, taken),
        form.poses(name, taken),
        `${JSON.stringify(name)} posed otherwise against ${JSON.stringify(taken)}`,
      );
    }
  }
});

it("takes the second form's act where LookForm.js takes it beside the same choices", () => {
  const form = lookForm();
  const offers = [
    { value: "keep", label: "Keep It", follows: "" },
    { value: "show", label: "Always Show It", follows: "" },
  ];
  for (const act of NAMES) {
    const told = form.actOf({ happened: "It happened.", act, offers }, LAUNCHER_ACTS);
    assert.equal(actOf(act.trim(), offers), told, `the act ${JSON.stringify(act)} was otherwise`);
  }
});

it("names the launcher's acts as Now.qml's lookActNames and its words do", () => {
  const ids = ACT_IDS.exec(launcher("NOW_QML"))?.[1];
  assert.ok(ids !== undefined, "Now.qml holds no lookActNames list to read");
  const names = namesIn(JSON.parse(launcher("AGENT_WORDS")), new Map());
  namesIn(JSON.parse(launcher("ON_WORDS")), names);
  const said = Array.from(ids.matchAll(QUOTED), ([, id]) => names.get(id ?? "") ?? `no ${id}`);
  assert.deepEqual([...LAUNCHER_ACTS], said, "the kit names the launcher's acts otherwise");
});

it("offers as many choices and goes quiet as soon as Looks.js does", () => {
  const looks = launcher("LOOKS");
  for (const [name, kit] of [
    ["OFFERED", OFFERED],
    ["QUIET_AFTER", QUIET_AFTER],
  ] as const) {
    const written = CONSTANT(name).exec(looks)?.[1];
    assert.ok(written !== undefined, `Looks.js holds no ${name} to read`);
    assert.equal(kit, runInNewContext(written, {}), `the kit's ${name} isn't Looks.js's`);
  }
});

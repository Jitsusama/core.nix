// The kit's ask row reads and judges every character in a name as the
// launcher's LookForm.js does, run in Node rather than in Qt's engine:
// within a name, leading and ending it, once and twice, and beside a space
// and a closing stop, so a range, a quantifier, a flag or the order of
// readAs's steps changed on either side fails here. It's a check of its
// own, the-ask-row-reads-each-character-as-for-you, since only LookForm.js
// and ask.ts decide it, and sweeping each code point takes a while.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { poses, readAs } from "../kit/ask.ts";
import { loaded, type Script } from "./launcher.ts";

/** The last code point, and those standing for half a pair, which are no character. */
const LAST_POINT = 0x10_ff_ff;
const HALVES = [0xd8_00, 0xdf_ff] as const;
/** How many characters read otherwise are named before the sweep stops. */
const NAMED = 10;
/** A code point's digits as it's written, U+ and hexadecimal. */
const HEX = 16;

/**
 * Whether `c` reads and is judged alike by both, within a name, leading
 * one and ending one, alone and twice over (so a pattern taking only one,
 * or only the first, shows), beside a space and after a stop (so steps run
 * in another order show), in a run after the first that ends nothing (so
 * a pattern rewriting only the first run shows), and against a name it
 * could pass for or by a turn pose as.
 */
function alike(form: Script, c: string): boolean {
  return [
    `Keep${c} It`,
    `${c}tI peeK`,
    `Keep It${c}`,
    `Keep${c}${c} It`,
    `${c}${c}tI peeK`,
    `Keep It${c}${c}`,
    `Keep ${c} It`,
    `Keep It.${c}`,
    `Keep It${c}Now`,
  ].every(
    (name) =>
      readAs(name) === form.readAs(name as never) &&
      poses(name, ["Keep It"]) === form.poses(name as never, ["Keep It"] as never),
  );
}

it("reads and judges every character as LookForm.js does, wherever it stands in a name", () => {
  const form = loaded("LOOK_FORM");
  const otherwise: string[] = [];
  for (const [from, to] of [
    [0, HALVES[0] - 1],
    [HALVES[1] + 1, LAST_POINT],
  ] as const) {
    for (let point = from; point <= to && otherwise.length < NAMED; point += 1) {
      if (!alike(form, String.fromCodePoint(point))) {
        otherwise.push(`U+${point.toString(HEX)}`);
      }
    }
  }
  assert.deepEqual(otherwise, [], "a character read or judged otherwise");
});

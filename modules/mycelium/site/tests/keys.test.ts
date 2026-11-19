// What each of the map's keys asks, decided apart from the drawing: every
// key its dialog names, the chords and fields that keep them from acting,
// and the order Escape closes things in.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  findKey,
  mapKey,
  type Pressed,
  pageKey,
  type Standing,
  stepped,
  switchedTo,
  treeKey,
} from "../keys.ts";

/** A key pressed alone, or with what `held` says. */
const press = (key: string, held: Partial<Pressed> = {}): Pressed => ({
  key,
  shiftKey: false,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  ...held,
});

/** The page with focus on the map and nothing open, but for what `then` says. */
const standing = (then: Partial<Standing> = {}): Standing => ({
  typing: false,
  keysOpen: false,
  onMap: true,
  findOpen: false,
  finding: false,
  chosen: false,
  nowOpen: false,
  ...then,
});

it("walks the map by arrows and by h, j, k and l alike", () => {
  const ways = [
    ["ArrowLeft", "h", [-1, 0]],
    ["ArrowRight", "l", [1, 0]],
    ["ArrowUp", "k", [0, -1]],
    ["ArrowDown", "j", [0, 1]],
  ] as const;
  for (const [arrow, letter, way] of ways) {
    assert.deepEqual(mapKey(press(arrow)), { act: "walk", way }, `${arrow} didn't walk`);
    assert.deepEqual(mapKey(press(letter)), { act: "walk", way }, `${letter} didn't walk`);
  }
});

it("moves the map rather than walking it while Shift is held, a capital as its letter", () => {
  assert.deepEqual(mapKey(press("ArrowUp", { shiftKey: true })), { act: "pan", way: [0, -1] });
  assert.deepEqual(mapKey(press("J", { shiftKey: true })), { act: "pan", way: [0, 1] });
});

it("opens what the walk reached with Return or Space, and leaves other keys alone", () => {
  assert.deepEqual(mapKey(press("Enter")), { act: "open" }, "Return didn't open");
  assert.deepEqual(mapKey(press(" ")), { act: "open" }, "Space didn't open");
  assert.equal(mapKey(press("x")), null, "a key the map doesn't name acted");
});

it("leaves every chord to the browser, on the map, the tree and the page", () => {
  for (const held of [{ ctrlKey: true }, { metaKey: true }, { altKey: true }]) {
    assert.equal(mapKey(press("j", held)), null, `the map took ${JSON.stringify(held)}`);
    assert.equal(treeKey(press("j", held)), null, `the tree took ${JSON.stringify(held)}`);
    assert.equal(
      pageKey(press("m", held), standing()),
      null,
      `the page took ${JSON.stringify(held)}`,
    );
  }
});

it("steps the tree as a tree does, by arrows and letters, Home and End, g and G", () => {
  const acts = {
    ArrowDown: "next",
    j: "next",
    ArrowUp: "prev",
    k: "prev",
    ArrowRight: "in",
    l: "in",
    ArrowLeft: "out",
    h: "out",
    Home: "first",
    g: "first",
    End: "last",
    Enter: "open",
    " ": "open",
  } as const;
  for (const [key, act] of Object.entries(acts)) {
    assert.equal(treeKey(press(key)), act, `${key} in the tree didn't ask ${act}`);
  }
  assert.equal(treeKey(press("G", { shiftKey: true })), "last", "G didn't go last");
});

it("answers Shift in the tree only as the G that goes last", () => {
  assert.equal(treeKey(press("J", { shiftKey: true })), null, "Shift+J stepped the tree");
  assert.equal(treeKey(press("ArrowDown", { shiftKey: true })), null, "Shift+Down stepped it");
});

it("says each of the page's letters asks what its keys dialog names", () => {
  const acts = {
    "?": "keys",
    F1: "keys",
    "/": "find",
    ".": "now",
    m: "machines",
    "0": "fit",
    "+": "zoom-in",
    "=": "zoom-in",
    i: "zoom-in",
    o: "zoom-out",
    "-": "out",
  } as const;
  for (const [key, act] of Object.entries(acts)) {
    assert.equal(pageKey(press(key), standing()), act, `${key} didn't ask ${act}`);
  }
});

it("lets a field keep every key typed into it, the keys dialog's too", () => {
  for (const key of ["?", "/", "m", "Escape"]) {
    assert.equal(pageKey(press(key), standing({ typing: true })), null, `${key} acted on a field`);
  }
});

it("answers only ? while the keys dialog is open, so it can close", () => {
  const open = standing({ keysOpen: true });
  assert.equal(pageKey(press("?"), open), "keys", "? didn't close the dialog");
  for (const key of ["/", "m", "0"]) {
    assert.equal(pageKey(press(key), open), null, `${key} acted behind the dialog`);
  }
});

it("answers find and the keys anywhere, and the map's own letters only on the map", () => {
  const away = standing({ onMap: false });
  assert.equal(pageKey(press("/"), away), "find", "find didn't open from off the map");
  assert.equal(pageKey(press("?"), away), "keys", "the keys didn't open from off the map");
  for (const key of ["m", ".", "0", "-", "Escape"]) {
    assert.equal(pageKey(press(key), away), null, `${key} acted from off the map`);
  }
});

it("closes with Escape what a mode opened: the find line, then what's chosen, then the now", () => {
  const all = standing({ findOpen: true, chosen: true, nowOpen: true });
  assert.equal(pageKey(press("Escape"), all), "close-find", "the find line didn't close first");
  const chosen = standing({ chosen: true, nowOpen: true });
  assert.equal(pageKey(press("Escape"), chosen), "let-go", "what's chosen didn't go next");
  assert.equal(pageKey(press("Escape"), standing({ nowOpen: true })), "close-now", "now stayed");
  assert.equal(pageKey(press("Escape"), standing()), null, "Escape acted with nothing open");
});

it("steps through what was found with n and N only while something is asked", () => {
  const finding = standing({ findOpen: true, finding: true });
  assert.equal(pageKey(press("n"), finding), "next-found", "n didn't go to the next");
  assert.equal(
    pageKey(press("N", { shiftKey: true }), finding),
    "found-before",
    "N didn't go back",
  );
  assert.equal(pageKey(press("n"), standing()), null, "n acted with nothing asked");
});

it("takes Return in the find line to the first match, and Escape to closing it", () => {
  assert.equal(findKey("Enter"), "first", "Return didn't go to the first match");
  assert.equal(findKey("Escape"), "close", "Escape didn't close the line");
  assert.equal(findKey("a"), null, "a letter typed was taken from the field");
});

it("steps round what was found, forward from the first and back from the last", () => {
  assert.equal(stepped(-1, 1, 3), 0, "the first step forward didn't land on the first");
  assert.equal(stepped(-1, -1, 3), 2, "the first step back didn't land on the last");
  assert.equal(stepped(2, 1, 3), 0, "a step past the last didn't come round");
  assert.equal(stepped(0, -1, 3), 2, "a step before the first didn't come round");
  assert.equal(stepped(-1, 1, 0), null, "a step went somewhere with nothing found");
});

it("moves the theme's switch by its arrows, round and round", () => {
  assert.equal(switchedTo("ArrowRight", 2, 3), 0, "right from the last didn't come round");
  assert.equal(switchedTo("ArrowUp", 0, 3), 2, "up from the first didn't come round");
  assert.equal(switchedTo("ArrowDown", 0, 3), 1, "down didn't move on");
  assert.equal(switchedTo("Enter", 0, 3), null, "a key that isn't an arrow moved it");
});

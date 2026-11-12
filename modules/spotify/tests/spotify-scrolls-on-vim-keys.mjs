// Spotify's scrolling extension as the client the account gets carries it
// (modules/spotify/scrolls.js with the key language written in, the first
// argument), run against a stand-in for Spotify's page: a view 800 pixels
// high over 4000 of page. Vim's keys for scrolling move it as far as Vim's
// do, gg only when its second g comes inside the language's wait and g then
// j as j, and Ctrl+f, a key with Alt, a key typed in a field and one with a
// dialog showing are left to Spotify.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

const source = readFileSync(process.argv[2], "utf8");

// A page with its view scrolled to the top, showing a dialog when asked.
const load = ({ dialog = false } = {}) => {
  const view = {
    scrollTop: 0,
    clientHeight: 800,
    scrollHeight: 4000,
    scrollBy({ top }) {
      this.scrollTop = Math.min(
        Math.max(this.scrollTop + top, 0),
        this.scrollHeight - this.clientHeight,
      );
    },
  };
  const page = { listeners: [], view, at: 0 };
  const context = {
    document: {
      querySelectorAll: (selectors) => {
        if (selectors.includes("main-view-container__scroll-node")) {
          return [view];
        }
        return dialog && selectors.includes('[role="dialog"]')
          ? [{ checkVisibility: () => true }]
          : [];
      },
    },
    addEventListener(type, listener, capture) {
      page.listeners.push({ type, listener, capture });
    },
  };
  runInNewContext(source, context);
  // A key pressed `after` milliseconds past the last, answering whether the
  // extension took it from Spotify.
  page.press = (key, how = {}) => {
    page.at += how.after ?? 1000;
    const event = {
      key,
      altKey: false,
      ctrlKey: false,
      metaKey: false,
      shiftKey: false,
      timeStamp: page.at,
      target: how.target ?? { closest: () => null },
      taken: false,
      preventDefault() {
        this.taken = true;
      },
      stopPropagation() {
        this.taken = true;
      },
      ...how,
    };
    for (const { type, listener } of page.listeners) {
      if (type === "keydown") {
        listener(event);
      }
    }
    return event.taken;
  };
  return page;
};

{
  const page = load();
  assert.equal(page.listeners.length, 1, "it listens once");
  assert.equal(page.listeners[0].capture, true, "it hears a key before Spotify does");
  const moves = [
    ["j", {}, 60, "j scrolls a line down"],
    ["k", {}, 0, "k scrolls a line up"],
    ["e", { ctrlKey: true }, 60, "Ctrl+e scrolls a line down"],
    ["y", { ctrlKey: true }, 0, "Ctrl+y scrolls a line up"],
    ["d", { ctrlKey: true }, 400, "Ctrl+d scrolls half the view down"],
    ["u", { ctrlKey: true }, 0, "Ctrl+u scrolls half the view up"],
    ["G", { shiftKey: true }, 3200, "G goes to the bottom"],
    ["b", { ctrlKey: true }, 2400, "Ctrl+b scrolls the whole view up"],
  ];
  for (const [key, how, top, what] of moves) {
    assert.equal(page.press(key, how), true, `${what}, taken from Spotify`);
    assert.equal(page.view.scrollTop, top, what);
  }
  assert.equal(page.press("g"), true, "a g waits, taken from Spotify");
  assert.equal(page.view.scrollTop, 2400, "a g alone moves nothing");
  page.press("g", { after: 100 });
  assert.equal(page.view.scrollTop, 0, "gg goes to the top");
  page.press("G", { shiftKey: true });
  page.press("g");
  page.press("g", { after: 1000 });
  assert.equal(page.view.scrollTop, 3200, "a g past the wait begins a pair again");
  page.press("g", { after: 100 });
  assert.equal(page.view.scrollTop, 0, "the pair it began ends on its second g");
  page.press("g");
  page.press("j", { after: 100 });
  assert.equal(page.view.scrollTop, 60, "g then j scrolls as j does");
}

{
  const page = load();
  const field = { closest: (selectors) => (selectors.includes("input") ? {} : null) };
  const left = [
    ["f", { ctrlKey: true }, "Ctrl+f, Spotify's filter"],
    ["j", { altKey: true }, "Alt+j, Spotify's menu for a row"],
    ["j", { target: field }, "j typed in a field"],
    ["x", {}, "a key the language gives no scrolling"],
  ];
  for (const [key, how, what] of left) {
    assert.equal(page.press(key, how), false, `${what} is left to Spotify`);
    assert.equal(page.view.scrollTop, 0, `${what} scrolls nothing`);
  }
}

{
  const page = load({ dialog: true });
  assert.equal(page.press("j"), false, "j with a dialog showing is left to Spotify");
  assert.equal(page.view.scrollTop, 0, "j with a dialog showing scrolls nothing");
}

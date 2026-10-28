// Spotify's Escape extension (modules/spotify/backs-out.js, the first
// argument), run against stand-ins for Spicetify and Spotify's page. Escape
// alone goes back a page; Escape with a modifier, held down, typed in a
// field, over a dialog, a menu or a popover, with a row chosen or in full
// screen leaves the page where it is, for Spotify to close or let go of
// that first; and the extension waits for Spicetify before it listens.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

const source = readFileSync(process.argv[2], "utf8");

// A page whose drawn-over parts are the selectors in `showing`, with a
// history that counts its steps back, and a Spicetify ready after `late`
// tries.
const load = ({ showing = [], fullscreen = false, late = 0 } = {}) => {
    const page = { back: 0, listeners: [], tries: 0 };
    const history = {
        goBack() {
            page.back += 1;
        },
    };
    const context = {
        document: {
            fullscreenElement: fullscreen ? {} : null,
            querySelector: (selectors) =>
                selectors.split(", ").some((selector) => showing.includes(selector)) ? {} : null,
        },
        addEventListener(type, listener, capture) {
            page.listeners.push({ type, listener, capture });
        },
        setTimeout(retry) {
            page.tries += 1;
            if (page.tries > late) {
                context.Spicetify = { Platform: { History: history } };
            }
            retry();
        },
    };
    if (late === 0) {
        context.Spicetify = { Platform: { History: history } };
    }
    runInNewContext(source, context);
    page.press = (key, how = {}) => {
        const event = { key, repeat: false, target: how.target ?? { closest: () => null }, ...how };
        for (const { type, listener } of page.listeners) {
            if (type === "keydown") listener(event);
        }
    };
    return page;
};

const field = { closest: (selectors) => (selectors.includes("input") ? {} : null) };

{
    const page = load();
    assert.equal(page.listeners.length, 1, "it listens once");
    assert.equal(page.listeners[0].capture, true, "it hears Escape before Spotify closes anything");
    page.press("Escape");
    assert.equal(page.back, 1, "Escape alone goes back a page");
    page.press("Enter");
    page.press("Escape", { shiftKey: true });
    page.press("Escape", { ctrlKey: true });
    page.press("Escape", { altKey: true });
    page.press("Escape", { repeat: true });
    page.press("Escape", { target: field });
    assert.equal(page.back, 1, "another key, a modifier, a held key or a field leaves the page where it is");
}

for (const showing of [
    '[role="dialog"]',
    '[role="menu"]',
    '[role="listbox"]',
    '[aria-modal="true"]',
    "[data-tippy-root]",
    '[role="row"][aria-selected="true"]',
]) {
    const page = load({ showing: [showing] });
    page.press("Escape");
    assert.equal(page.back, 0, `Escape with ${showing} showing is left to Spotify`);
}

{
    const page = load({ fullscreen: true });
    page.press("Escape");
    assert.equal(page.back, 0, "Escape in full screen is left to Spotify");
}

{
    const page = load({ late: 2 });
    assert.equal(page.tries, 3, "it waits for Spicetify");
    page.press("Escape");
    assert.equal(page.back, 1, "once Spicetify is there, Escape goes back");
}

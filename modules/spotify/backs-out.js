// Escape backs out a step in Spotify, as it does everywhere on the desktop.
// With nothing drawn over the page, no row chosen and the cursor in no field,
// it goes back a page, as Alt+Left does. Otherwise Escape is Spotify's, which
// closes what's drawn over the page or lets go of the chosen rows first, so
// the next Escape goes back. Spicetify loads it into Spotify's client
// (home.nix), and tests/spotify-backs-out-on-escape.mjs holds it to that.
(function backsOut() {
    // How long to wait for Spicetify, in milliseconds, before looking again.
    const awhile = 250;
    const history = globalThis.Spicetify?.Platform?.History;
    if (history === undefined) {
        globalThis.setTimeout(backsOut, awhile);
        return;
    }
    // What Spotify draws over a page: a dialog, a menu, a list to pick from,
    // a popover; and a row chosen in a list.
    const over = ["dialog", "menu", "listbox"]
        .map((role) => `[role="${role}"]`)
        .concat(['[aria-modal="true"]', "[data-tippy-root]"])
        .join(", ");
    const chosen = '[role="row"][aria-selected="true"]';
    const field = "input, textarea, select, [contenteditable]";
    const alone = (event) => !(event.ctrlKey || event.altKey || event.metaKey || event.shiftKey);
    globalThis.addEventListener(
        "keydown",
        (event) => {
            const inField = typeof event.target?.closest === "function" && event.target.closest(field) !== null;
            if (event.key !== "Escape" || event.repeat || !alone(event) || inField) {
                return;
            }
            const page = globalThis.document;
            if (page.fullscreenElement || page.querySelector(over) !== null || page.querySelector(chosen) !== null) {
                return;
            }
            history.goBack();
        },
        true,
    );
})();

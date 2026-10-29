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
    // What Spotify draws over a page: a dialog, a menu, a popover; and a row
    // chosen in a list. Each counts only while it shows, since Spotify keeps
    // its language and country dialogs in every page, hidden. A list to pick
    // from isn't one: Liked Songs' filter tags are a list on the page, and
    // one that pops up does so under a field, which leaves Escape to Spotify.
    const over = ["dialog", "menu"]
        .map((role) => `[role="${role}"]`)
        .concat(['[aria-modal="true"]', "[data-tippy-root]", '[role="row"][aria-selected="true"]'])
        .join(", ");
    const field = "input, textarea, select, [contenteditable]";
    const alone = (event) => !(event.ctrlKey || event.altKey || event.metaKey || event.shiftKey);
    const showing = (page) => Array.from(page.querySelectorAll(over)).some((part) => part.checkVisibility());
    globalThis.addEventListener(
        "keydown",
        (event) => {
            const inField = typeof event.target?.closest === "function" && event.target.closest(field) !== null;
            if (event.key !== "Escape" || event.repeat || !alone(event) || inField) {
                return;
            }
            const page = globalThis.document;
            if (page.fullscreenElement || showing(page)) {
                return;
            }
            history.goBack();
        },
        true,
    );
})();

// Spotify answers Vim's keys, as every page of ours does
// (modules/keys/language.toml). j and k scroll a line, as Ctrl+e and Ctrl+y
// do; Ctrl+d and Ctrl+u half the page, Ctrl+f and Ctrl+b a whole one, and gg
// and G go to the top and the bottom. Ctrl+o and Ctrl+i go back and forward a
// page, and / filters the list on the page. The keys come from the language,
// which home.nix writes in below as it builds the extension, with how long a
// g waits for its second. A key typed in a field, or with a dialog or a menu
// showing, is Spotify's, and so is Escape, which never steps out. Spicetify
// loads it into Spotify's client (home.nix), and
// tests/spotify-speaks-vims-keys.mjs holds it to that.
(function speaksVimsKeys() {
    const language = JSON.parse("@language@");
    // How far a line is, in pixels, as Vimium scrolls one in Chrome.
    const line = 60;
    // The part of Spotify's window that scrolls: the page beside the
    // library and above the player, its viewport where OverlayScrollbars
    // draws one.
    const scroller = [
        ".main-view-container__scroll-node [data-overlayscrollbars-viewport]",
        ".main-view-container__scroll-node",
    ].join(", ");
    // The page's filter: its field once it's open, else the button that
    // opens it.
    const filters = [".x-filterBox-filterInput", ".x-filterBox-expandButton"].map(
        (part) => `.main-view-container ${part}`,
    );
    const over = '[role="dialog"], [role="menu"], [aria-modal="true"], [data-tippy-root]';
    const field = "input, textarea, select, [contenteditable]";
    const page = globalThis.document;
    const view = () => {
        const views = Array.from(page.querySelectorAll(scroller));
        return views.find((each) => each.scrollHeight > each.clientHeight) ?? views[0];
    };
    // Each act the language gives Spotify, done, or false when there's
    // nothing on the page to do it to, which leaves the key to Spotify.
    const scrolls = (by) => () => {
        const shown = view();
        shown?.scrollBy({ top: by(shown) });
        return shown !== undefined;
    };
    const history = (step) => () => {
        const pages = globalThis.Spicetify?.Platform?.History;
        pages?.[step]();
        return pages !== undefined;
    };
    const half = 2;
    const acts = {
        down: scrolls(() => line),
        up: scrolls(() => -line),
        "scroll-down": scrolls(() => line),
        "scroll-up": scrolls(() => -line),
        "half-down": scrolls((shown) => shown.clientHeight / half),
        "half-up": scrolls((shown) => -shown.clientHeight / half),
        "screen-down": scrolls((shown) => shown.clientHeight),
        "screen-up": scrolls((shown) => -shown.clientHeight),
        first: scrolls((shown) => -shown.scrollHeight),
        last: scrolls((shown) => shown.scrollHeight),
        "jump-back": history("goBack"),
        "jump-forward": history("goForward"),
        "search-forward": () => {
            const [input, button] = filters.map((part) =>
                Array.from(page.querySelectorAll(part)).find((each) => each.checkVisibility()),
            );
            if (input !== undefined) {
                input.focus();
            } else {
                button?.click();
            }
            return input !== undefined || button !== undefined;
        },
    };
    // A key as the language writes it: Ctrl+d is <C-d>, and a letter is
    // itself, capital when shifted. Alt and the super key leave it to
    // Spotify.
    const named = (event) => {
        if (event.altKey || event.metaKey || event.key.length !== 1) {
            return null;
        }
        return event.ctrlKey ? `<C-${event.key.toLowerCase()}>` : event.key;
    };
    const spotifys = (event) => {
        const inField = typeof event.target?.closest === "function" && event.target.closest(field) !== null;
        return inField || Array.from(page.querySelectorAll(over)).some((part) => part.checkVisibility());
    };
    const take = (event) => {
        event.preventDefault();
        event.stopPropagation();
    };
    // The keys pressed as two, as gg is, where <C-d> is one.
    const pairs = Object.keys(language.keys).filter((each) => each.length > 1 && !each.startsWith("<"));
    // The key waiting for its second, and when it came.
    let waiting = null;
    globalThis.addEventListener(
        "keydown",
        (event) => {
            const key = named(event);
            if (key === null || spotifys(event)) {
                waiting = null;
                return;
            }
            // A pair's second key ends it; one that ends no pair counts
            // alone, as Vim takes g then j.
            const pair =
                waiting !== null && event.timeStamp - waiting.at < language.wait
                    ? language.keys[waiting.key + key]
                    : undefined;
            waiting = null;
            const act = pair ?? language.keys[key];
            if (act === undefined) {
                if (pairs.some((each) => each.startsWith(key))) {
                    waiting = { key, at: event.timeStamp };
                    take(event);
                }
                return;
            }
            if (acts[act]()) {
                take(event);
            }
        },
        true,
    );
})();

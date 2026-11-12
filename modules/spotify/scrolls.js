// Spotify's page scrolls on Vim's keys, as every page of ours does
// (modules/keys/language.toml): j and k a line, Ctrl+e and Ctrl+y a line,
// Ctrl+d and Ctrl+u half the page, Ctrl+b a whole one, and gg and G to the
// top and the bottom. The keys come from the language, which home.nix
// writes in below as it builds the extension, with how long a g waits for
// its second. A key typed in a field, or with a dialog or a menu showing, is
// Spotify's. Spicetify loads it into Spotify's client (home.nix), and
// tests/spotify-scrolls-on-vim-keys.mjs holds it to that.
(function scrolls() {
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
    const over = '[role="dialog"], [role="menu"], [aria-modal="true"], [data-tippy-root]';
    const field = "input, textarea, select, [contenteditable]";
    const page = globalThis.document;
    // How far each act scrolls a page, down from where it is.
    const half = 2;
    const by = {
        down: () => line,
        up: () => -line,
        "scroll-down": () => line,
        "scroll-up": () => -line,
        "half-down": (shown) => shown.clientHeight / half,
        "half-up": (shown) => -shown.clientHeight / half,
        "screen-down": (shown) => shown.clientHeight,
        "screen-up": (shown) => -shown.clientHeight,
        first: (shown) => -shown.scrollHeight,
        last: (shown) => shown.scrollHeight,
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
    const view = () => {
        const views = Array.from(page.querySelectorAll(scroller));
        return views.find((each) => each.scrollHeight > each.clientHeight) ?? views[0] ?? null;
    };
    const spotifys = (event) => {
        const inField = typeof event.target?.closest === "function" && event.target.closest(field) !== null;
        return inField || Array.from(page.querySelectorAll(over)).some((part) => part.checkVisibility());
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
                    event.preventDefault();
                    event.stopPropagation();
                }
                return;
            }
            const shown = view();
            if (shown === null) {
                return;
            }
            shown.scrollBy({ top: by[act](shown) });
            event.preventDefault();
            event.stopPropagation();
        },
        true,
    );
})();

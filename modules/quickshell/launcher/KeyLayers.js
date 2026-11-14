.pragma library
// Which layers' keys work in a window, its own first, read from its app id
// and its title as niri says them, so Shortcuts (KeyMap.qml) opens on the
// keys of the window Joel came from and a window's row wears the glyph of
// what runs in it (Windows.qml). Each layer's keys are jitsusama.keys';
// how they're grouped is KeyGroups.js's. tests/key-groups.qml holds it to
// real windows' app ids and titles, in Qt's own engine.

// Where a terminal's title says it runs pi, Neovim or btop: pi titles it by
// its letter and the session's name, Neovim by a memo and the file
// (modules/neovim/init.lua's titlestring) or by its own name, each read from
// the title's start, since a file, a folder or a quest's name may hold "vim";
// btop by its name alone (modules/btop/names-its-window.patch).
const TITLED = [["pi", /^(\u03C0|pi)( |$)/u], ["neovim", /^(\u{1F4DD}|n?vim\b)/iu], ["btop", /^btop$/u]];

// The program whose keys are each layer's, by its window's app id.
const TERMINAL_APP = "kitty";

// The layers of programs in windows of their own, by their windows' app
// ids, each with its own keys over the desktop's.
const WINDOW_APPS = new Map([["swayimg", "swayimg"], ["reader", "reader"], ["org.pwmt.zathura", "zathura"], ["mpv", "mpv"], ["google-chrome", "chrome"], ["slack", "slack"], ["com.onepassword.OnePassword", "onepassword"], ["gimp", "gimp"], ["org.inkscape.Inkscape", "inkscape"], ["spotify", "spotify"]]);
// A window Chrome opens for one site, as a program of its own: "chrome-", the site, its profile.
const CHROME_SITE = /^chrome-.+-[^-]+$/;

// Google's programs, each its site's own keys over Chrome's, by its desktop
// entry's id (modules/google/home.nix): by the page's title, without
// Chrome's own name after it, which says the page shown now in a tab or in
// a site window gone on to another of Google's, or else by the site and
// path the window was opened for, named in its app id, as
// chrome-docs.google.com__document_ does. Mail's title ends with Gmail, or
// with a Workspace's own name before Mail. Those that end a title with the
// program's name are tried first, since Calendar's and Meet's begin it and
// a document may be named for either: "Team - Calendar - Q4 - Google Docs"
// is a Docs page.
const GOOGLE_SITES = [
    ["google-mail", /^chrome-mail\.google\.com__/u, / - (Gmail|[^-]+ Mail)$/u],
    ["google-docs", /^chrome-docs\.google\.com__document/u, / - Google Docs$/u],
    ["google-sheets", /^chrome-docs\.google\.com__spreadsheets/u, / - Google Sheets$/u],
    ["google-slides", /^chrome-docs\.google\.com__presentation/u, / - Google Slides$/u],
    ["google-drive", /^chrome-drive\.google\.com__/u, / - Google Drive$/u],
    ["google-calendar", /^chrome-calendar\.google\.com__/u,
        /^(Google Calendar|.+ - Calendar)( - |$)/u],
    ["google-meet", /^chrome-meet\.google\.com__/u, /^Meet( - |: |$)/u]
];

// GitHub's pages, by the marks only GitHub's titles carry, since a forge
// that copies its "path \u00B7 owner/repo" may take other keys: a repository's
// code at a branch, a pull request, an issue or a discussion by its number,
// a commit by its hash, a repository's home with what it's for, and a page
// GitHub names itself, as "Notifications \u00B7 GitHub". A repository's home
// with nothing said of it, "owner/repo" alone, is left to Chrome's, as such
// a forge names its own so. Seven days of Joel's history had 631 of 769
// visits to GitHub named so, and two elsewhere.
const REPOSITORY = String.raw`[A-Za-z0-9][\w.-]*\/[\w.-]+`;
const GITHUB_TITLED = new RegExp([
    String.raw` \u00B7 GitHub$`,
    String.raw`^GitHub - ${REPOSITORY}`,
    String.raw` \u00B7 (Pull Request|Issue|Discussion) #\d+ \u00B7 ${REPOSITORY}$`,
    String.raw` at \S+ \u00B7 ${REPOSITORY}$`,
    String.raw`^${REPOSITORY}( at \S+|: [^\u00B7]+)$`,
    String.raw` \u00B7 ${REPOSITORY}@[0-9a-f]{5,}$`
].join("|"), "u");

// The sites with keys of their own, each over Chrome's: Google's, then
// GitHub's.
const SITES = [...GOOGLE_SITES, ["github", /^chrome-github\.com__/u, GITHUB_TITLED]];
const CHROME_NAMED = / - Google Chrome$/u;

// The site with keys of its own a Chrome window shows, by its app id or its
// title, or undefined for any other page. A Google program's own entry id
// is itself, so a program's row finds its layer (KeyMap.qml's ofProgram).
function site(app, title) {
    const page = title.replace(CHROME_NAMED, "");
    const found = SITES.find(([, , titled]) => titled.test(page))
        ?? SITES.find(([id, named]) => app === id || named.test(app));
    return found?.[0];
}

// The layers whose keys work in a window, its own first: a terminal's, or
// pi's, Neovim's or btop's and then the terminal's when its title says so,
// and then the desktop's, which work over any window. A window of a program
// with a layer of its own has that first, and any other, or none, has only
// the desktop's. Chrome's over one of Google's programs, or GitHub, has the
// site's first, then Chrome's.
function over(app, title) {
    if (app !== TERMINAL_APP) {
        const own = WINDOW_APPS.get(app) ?? (CHROME_SITE.test(app) ? "chrome" : undefined);
        // Only Chrome's own windows are read by their titles, since another
        // program's document may be named for a Google one.
        const read = own === "chrome" || own === undefined;
        const shown = read ? site(app, own === "chrome" ? title : "") : undefined;
        if (shown !== undefined) {
            return [shown, "chrome", "desktop"];
        }
        return own === undefined ? ["desktop"] : [own, "desktop"];
    }
    const titled = TITLED.find(([, pattern]) => pattern.test(title));
    return titled === undefined ? ["terminal", "desktop"] : [titled[0], "terminal", "desktop"];
}

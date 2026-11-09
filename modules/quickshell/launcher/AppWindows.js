// Which program a browser's app window belongs to when its app id names
// no desktop entry. Chrome names an app window after the page it opened
// at, its host, then its path with each slash an underscore, then its
// profile, as chrome-docs.google.com__document_create-Default. A program's
// desktop entry says the class of its own page's window, such as
// chrome-docs.google.com__document_-Default, so a window opened deeper in
// (a new document, a mail opened from the shell, a call) matched nothing,
// and counted for no program: no Go To, no count, no Quit, and Return
// opened another. Applications.qml asks this only after heuristicLookup
// finds nothing, and tests/app-windows.qml runs it in Qt's own engine.
.pragma library

// An app window's host, path and profile. The host holds no underscore and
// the profile no dash, while a path may hold dashes, as a Meet call's
// code does, so the path is all that lies between.
const CHROME_APP = /^chrome-([^_]+)__(.*)-([^-]+)$/;

function partsOf(name) {
    const found = CHROME_APP.exec(String(name ?? ""));
    return found ? { host: found[1], path: found[2], profile: found[3] } : null;
}

// The entry whose own page's window has the same host and profile and the
// longest path the window's path starts with, or null. Docs' document_
// never takes a spreadsheets_ window, and another profile's window is
// another person's, so it's never this one's.
function ownerOf(appId, entries) {
    const window = partsOf(appId);
    if (!window) {
        return null;
    }
    let owner = null;
    let longest = -1;
    for (const entry of entries ?? []) {
        const page = partsOf(entry?.startupClass);
        if (page && page.host === window.host && page.profile === window.profile
            && window.path.startsWith(page.path) && page.path.length > longest) {
            owner = entry;
            longest = page.path.length;
        }
    }
    return owner;
}

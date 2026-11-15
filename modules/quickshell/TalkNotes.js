// How a row of Super+S's talk line says a page's address or a document's
// file (TalkLine.js's attached): short on the line, whole where it's sent.
// tests/talk-line.qml holds it to its cases in Qt's own engine.
.pragma library

// What an address begins and ends with that its note leaves out.
const SCHEME = /^https?:\/\/(?:www\.)?/u;
const TRAILING_SLASH = /\/$/u;

// An address as a row's note says it, without the scheme every page has,
// so the note gives way later; it's sent whole.
function shortAddress(address) {
    return String(address ?? "").replace(SCHEME, "").replace(TRAILING_SLASH, "");
}

// The home folder a path begins with, which a note says as ~.
const HOME = /^\/home\/[^/]+(?=\/)/u;

// The file a document window shows, as the windows daemon reads it from
// its command line, or null where it shows none: on the line by its name,
// noted by its folder from home, and sent whole, since a title can repeat
// and a path can't (Joel, 2026-11-14 17:03: "we need the void to be more
// aware of what is on a window that i'm focused on when i launch it").
function fileOf(window) {
    const path = String(window?.file ?? "").trim();
    if (path === "") {
        return null;
    }
    const at = path.lastIndexOf("/");
    return { text: path.slice(at + 1), note: path.slice(0, Math.max(at, 0)).replace(HOME, "~"),
        sent: path };
}

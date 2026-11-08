// Where a picture the drawer shows is drawn from, in plain JavaScript so
// tests/media.qml holds it in Qt's own engine: a path on the machine, as a
// file's plate gives one, or a URL as given, as a player's art comes
// (launcher/Media.js), whose remote pictures the giver has already left out
// where the launcher may not ask the network. Drawer.qml draws both.
.pragma library

// A URL's scheme, as RFC 3986 spells one: file:, https:, data:.
const SCHEME = /^[a-z][a-z\d+.-]*:/i;

// The source an Image draws a picture from, or "" for none.
function source(picture) {
    const given = String(picture ?? "");
    if (given === "") {
        return "";
    }
    return SCHEME.test(given) ? given : `file://${given}`;
}

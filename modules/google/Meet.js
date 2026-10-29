// What the shell works out of Meet's calls: which call a window is, the
// code Joel types to join one, and whose meeting a call is, from the
// meetings the google program keeps (src/state.rs). Calendar.qml asks
// these, for itself and for Meet.qml, and the checks run them in Qt's own
// engine (tests/calendar.qml).
.pragma library

// A Meet call's code, as abc-defg-hij, from the app id Chrome gives the
// window it opens a call's link in as an app of its own: its host and
// path, the path's slash an underscore, then its profile, as
// chrome-meet.google.com__abc-defg-hij-Default. Any other window, Meet's
// own front page and a call in a browser tab among them, is no call: "".
const MEET_WINDOW = /^chrome-meet\.google\.com__([a-z]{3}-[a-z]{4}-[a-z]{3})-/;
// The code a Meet link joins by, before any ?authuser= that follows it.
const MEET_LINK = /^https:\/\/meet\.google\.com\/([a-z-]+)/;

function meetCall(appId) {
    const found = MEET_WINDOW.exec(appId ?? "");
    return found ? found[1] : "";
}

// A Meet code as Joel types or pastes it, three letters, four and three,
// with or without its dashes and spaces, or its whole link, written as
// Meet writes it: "abc-defg-hij"; or "" for anything else, which Meet
// wouldn't know.
const MEET_CODE = /^([a-z]{3})-?([a-z]{4})-?([a-z]{3})$/;
const SPACES = /\s+/g;

function meetCode(given) {
    const plain = String(given ?? "").trim().toLowerCase().replace(SPACES, "");
    const parts = MEET_CODE.exec(MEET_LINK.exec(plain)?.[1] ?? plain);
    return parts ? `${parts[1]}-${parts[2]}-${parts[3]}` : "";
}

// The meeting whose call has that code, or null for one nobody asked
// Joel to, such as a link someone pasted.
function calledBy(meetings, code) {
    return meetings.find(meeting => code !== ""
        && MEET_LINK.exec(meeting.link ?? "")?.[1] === code)
        ?? null;
}

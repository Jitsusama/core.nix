// Which window a notice's sender comes forward in, for Notices.qml, once
// what the notice is about has been opened. The shell hands the sender a
// token to raise itself with, but Slack and Chrome often ask niri for a
// token of their own, which niri refuses, so the conversation opens
// behind whatever was in front: of 48 clicks on Slack's notices on
// 2026-10-26, 19 left Slack where it was. Kept apart from Quickshell's
// types, so the `a-notice-comes-forward` check runs it in Qt's own engine
// (tests/notice-comes-forward.qml).
.pragma library

// Whether a window is one of the sender's, by the names its program goes
// by: its notice's app name, and its desktop entry's id.
function sends(window, names) {
    return typeof window?.app_id === "string" && names.includes(window.app_id.toLowerCase());
}

// The sender's window it was in last, or null when it has none open.
function senderWindow(windows, names) {
    let last = null;
    for (const window of Object.values(windows)) {
        if (sends(window, names) && (last === null || since(window) > since(last))) {
            last = window;
        }
    }
    return last;
}

// When a window was last focused, in seconds, or never.
function since(window) {
    const at = window.focus_timestamp;
    return at ? at.secs + at.nanos / 1e9 : -1;
}

// The window to focus once the sender has had its time to come forward:
// none when one of its own is focused already, or when it has none.
function toFocus(windows, focused, names) {
    if (sends(windows[focused], names)) {
        return null;
    }
    return senderWindow(windows, names)?.id ?? null;
}

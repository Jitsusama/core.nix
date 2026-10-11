pragma Singleton
// Every notification this session, newest first, kept in memory only, for
// the launcher's collection of them (launcher/PastNotices.qml). One leaves
// the screen in one of three ways: clicked, so it was seen; withdrawn by its
// program, as a chat app does once the message is read there; or by itself
// when its time is up, so it was missed. What's missed is counted until the
// collection is opened. One that arrives while notices are held (Quiet.qml)
// is never shown, so it's missed from the start.
//
// The notices on screen and the collection are one home: while the
// collection is open the screen's step aside (`gathered`), still there to
// be answered from the launcher, and come back if it closes before they go.
import Quickshell
import QtQuick

Singleton {
    id: root

    // How many are kept; the oldest go first.
    readonly property int keep: 200

    // { id, at, app, summary, body, glyph, hue, urgent, held, missed }
    property var history: []
    readonly property int missed: history.filter(notice => notice.missed).length
    // Those missed and gone from the screen, which wait for Joel.
    readonly property var waiting: history.filter(notice => notice.missed && !(notice.id in live))

    // How far down the notices make way for the margin's drawer while it's
    // out, so neither covers the other (Margin.qml).
    property real below: 0

    // The notifications still on screen, by id, whose own actions (Reply,
    // Mark as read) can be done from the launcher's keys while they last.
    property var live: ({})
    // Whether the launcher's collection of them is open.
    property bool gathered: false

    // A sender's glyph where Nerd Fonts has one, and the hue of what it
    // brings: a person's message wears the person's.
    readonly property var senders: ({
        "slack": { glyph: 0xf198, hue: "person" },
        "google chrome": { glyph: 0xf268, hue: "program" },
        "chromium": { glyph: 0xf268, hue: "program" },
        "niri": { glyph: 0xf0e51, hue: "machine" }
    })

    function sender(name) {
        const known = senders[name.toLowerCase()];
        return known ?? { glyph: 0xf0f3, hue: "program" };
    }

    // Keeps a notification as it arrives, missed until it's seen, and held
    // when it's never shown.
    function arrived(notification, urgent, held) {
        const from = sender(notification.appName);
        const notice = {
            id: notification.id,
            at: Date.now(),
            app: notification.appName,
            summary: notification.summary,
            // Its markup taken out, since the launcher's rows are plain.
            body: notification.body.replace(/<[^>]*>/g, "").replace(/&lt;/g, "<")
                .replace(/&gt;/g, ">").replace(/&amp;/g, "&"),
            glyph: from.glyph,
            hue: from.hue,
            urgent: urgent,
            held: held,
            missed: true
        };
        history = [notice].concat(history.filter(kept => kept.id !== notice.id)).slice(0, keep);
        if (!held)
            live = Object.assign({}, live, { [notice.id]: notification });
    }

    // How one left: only one that left by itself was missed.
    function left(id, expired) {
        const still = Object.assign({}, live);
        delete still[id];
        live = still;
        if (expired)
            return;
        history = history.map(notice => notice.id === id
            ? Object.assign({}, notice, { missed: false }) : notice);
    }

    // Everything kept has been seen.
    function seen() {
        if (missed > 0)
            history = history.map(notice => Object.assign({}, notice, { missed: false }));
    }
}

pragma Singleton
// Every notification, newest first, for the launcher's collection of them
// (launcher/PastNotices.qml), kept on disk in the shell's state so a restart
// loses none: a read one for a week, an unread one for a month, and never
// more than `keep`.
//
// One leaves the screen in one of three ways: clicked away; withdrawn by its
// program, as a chat app does once the message is read there; or by itself
// when its time is up. One that left by itself was missed, and what's missed
// is counted until the collection is opened. One that arrives while notices
// are held (Quiet.qml) is never shown, so it's missed from the start.
//
// Apart from that, each is unread until it's acted on: gone to, one of its
// actions done, marked read, or withdrawn by its program. A click only puts
// it away, so one clicked by accident still waits, unread, in the list.
//
// The notices on screen and the collection are one home: while the
// collection is open the screen's step aside (`gathered`), still there to
// be answered from the launcher, and come back if it closes before they go.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // How many are kept, and for how long; the oldest go first.
    readonly property int keep: 200
    readonly property real day: 24 * 60 * 60 * 1000
    readonly property real readFor: 7 * day
    readonly property real unreadFor: 30 * day

    // { id, at, app, summary, body, glyph, hue, urgent, held, missed, read }
    property var history: []
    readonly property int missed: history.filter(notice => notice.missed).length
    readonly property int unread: history.filter(notice => !notice.read).length
    // Those missed and gone from the screen, which wait for Joel.
    readonly property var waiting: history.filter(notice => notice.missed && !(notice.id in live))
    onWaitingChanged: Report.say(`waiting ${waiting.map(notice => `${notice.app}: ${notice.summary}`).join("; ")}`)

    // How far down the notices make way for the margin's drawer while it's
    // out, so neither covers the other (Margin.qml).
    property real below: 0

    // The notifications still on screen, by id, whose own actions (Reply,
    // Mark as read) can be done from the launcher's keys while they last.
    property var live: ({})
    // Whether the launcher's collection of them is open.
    property bool gathered: false

    // A sender's glyph where Nerd Fonts has one, and the hue of what it
    // brings: a person's message wears the person's. One that only answers
    // what Joel just did, as the recorder's saying where a file went, is
    // never missed.
    readonly property var senders: ({
        "slack": { glyph: 0xf198, hue: "person" },
        "tuple": { glyph: 0xf03f2, hue: "person" },
        "google chrome": { glyph: 0xf268, hue: "program" },
        "chromium": { glyph: 0xf268, hue: "program" },
        "niri": { glyph: 0xf0e51, hue: "machine" },
        // A pi session wanting Joel (Sessions.qml), in its mark, \u03c0.
        "pi": { glyph: 0x3c0, hue: "program" },
        "recorder": { glyph: 0xf0ec2, hue: "machine", answers: true }
    })

    function sender(name) {
        const known = senders[name.toLowerCase()];
        return known ?? { glyph: 0xf0f3, hue: "program" };
    }

    // Those still young enough to keep, at most `keep` of them.
    function fresh(notices) {
        const now = Date.now();
        return notices.filter(notice => now - notice.at < (notice.read ? readFor : unreadFor))
            .slice(0, keep);
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
            missed: !from.answers,
            read: false
        };
        // Live before it's kept, so it never reads as missed and gone from
        // the screen for the moment between, which would flash the margin.
        if (!held)
            live = Object.assign({}, live, { [notice.id]: notification });
        history = fresh([notice].concat(history.filter(kept => kept.id !== notice.id)));
    }

    // How one left, as "expired", "dismissed" or "withdrawn": only one that
    // went by itself was missed, and one its program withdrew was read there.
    // It's no longer missed before it leaves the screen, for the same reason.
    function left(id, how) {
        if (how !== "expired")
            history = history.map(notice => notice.id !== id ? notice
                : Object.assign({}, notice, { missed: false },
                    how === "withdrawn" ? { read: true } : {}));
        const still = Object.assign({}, live);
        delete still[id];
        live = still;
    }

    // Marks one read, or unread again.
    function mark(id, read) {
        history = history.map(notice => notice.id !== id ? notice
            : Object.assign({}, notice, { read: read }, read ? { missed: false } : {}));
    }

    // Everything kept has been seen, though not necessarily read.
    function seen() {
        if (missed > 0)
            history = history.map(notice => Object.assign({}, notice, { missed: false }));
    }

    // The history on disk. What's read back joins anything that arrived
    // first; its ids are made its own, since the server's start again from
    // one each time the shell starts. Nothing is written until it's read, so
    // an early arrival never overwrites what was kept.
    property bool restored: false

    FileView {
        id: kept
        path: Quickshell.statePath("notices.json")
        printErrors: false
        onLoaded: {
            let saved = [];
            try {
                saved = JSON.parse(text());
            } catch (error) {
                console.warn(`Notices: ${path} doesn't read as JSON, so it starts over`);
            }
            root.history = root.fresh(root.history.concat(saved.map((notice, index) =>
                Object.assign({}, notice, { id: `kept:${notice.at}:${index}` }))));
            root.restored = true;
        }
        onLoadFailed: root.restored = true
    }

    onHistoryChanged: {
        if (restored)
            saving.restart();
    }

    // A burst of changes is written once.
    Timer {
        id: saving
        interval: 1000
        onTriggered: kept.setText(JSON.stringify(root.history))
    }
}

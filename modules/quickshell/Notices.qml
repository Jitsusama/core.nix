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
// One that left by itself, or was never shown, rests: it's off the screen
// but still open, so its program keeps its actions (Reply, Open) for the
// launcher's keys, until the program withdraws it or it's let go. Closing
// it would tell the program it's done with, and the program would forget
// what it offered.
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
import "Forward.js" as Forward

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
    readonly property var waiting: history.filter(notice => notice.missed
        && (!(notice.id in live) || notice.id in resting))

    // The notifications still open, by id, on screen or resting, whose own
    // actions (Reply, Mark as read) can be done from the launcher's keys
    // while they last.
    property var live: ({})
    // Those of them resting off the screen, by id.
    property var resting: ({})
    // Whether the launcher's collection of them is open.
    property bool gathered: false

    // A sender's glyph where Nerd Fonts has one, and the hue of what it
    // brings: a person's message wears the person's. One that only answers
    // what Joel just did, as the recorder's saying where a file went, is
    // never missed.
    readonly property var senders: ({
        "slack": { glyph: 0xf198, hue: "person" },
        "tuple": { glyph: Theme.glyph.call.codePointAt(0), hue: "person" },
        "google chrome": { glyph: 0xf268, hue: "program" },
        "chromium": { glyph: 0xf268, hue: "program" },
        "niri": { glyph: 0xf0e51, hue: "machine" },
        // A pi session wanting Joel (Sessions.qml), in its mark. It's
        // never missed, since the margin's mark says it while it holds.
        "pi": { glyph: Theme.glyph.agent.codePointAt(0), hue: "program", answers: true },
        // Something an agent has ready for Joel to do (WaitingOnYou.qml).
        "waiting": { glyph: Theme.glyph.waitsOnJoel.codePointAt(0), hue: "program" },
        "recorder": { glyph: Theme.glyph.recording.codePointAt(0), hue: "machine", answers: true },
        // What arrives by itself (Arrivals.qml), in its margin mark, which
        // says it while it holds, so it's never missed either.
        "camera": { glyph: Theme.glyph.camera.codePointAt(0), hue: "machine", answers: true },
        // A meeting about to start (Starting.qml), with others in it.
        "calendar": { glyph: Theme.glyph.meeting.codePointAt(0), hue: "person", answers: true },
        "battery": { glyph: Theme.glyph.low.codePointAt(0), hue: "machine", answers: true }
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

    // Keeps a notification as it arrives, missed until it's seen unless
    // it's never counted, and held when it's never shown.
    function arrived(notification, urgent, held, counted) {
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
            missed: counted && !from.answers,
            read: false
        };
        // Live before it's kept, so it never reads as missed and gone from
        // the screen for the moment between, which would flash the margin.
        // One held rests from the start; one its program sends again with
        // the same id comes back to the screen.
        live = Object.assign({}, live, { [notice.id]: notification });
        const still = Object.assign({}, resting);
        if (held)
            still[notice.id] = true;
        else
            delete still[notice.id];
        resting = still;
        history = fresh([notice].concat(history.filter(kept => kept.id !== notice.id)));
    }

    // One whose time on screen is up rests off it, still open.
    function rest(id) {
        if (id in live)
            resting = Object.assign({}, resting, { [id]: true });
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
        if (id in resting) {
            const others = Object.assign({}, resting);
            delete others[id];
            resting = others;
        }
    }

    // Opens what a notice is about, by its sender's own action, and brings
    // the sender forward when it doesn't come by itself. The shell hands
    // it a token to raise its window with, but Slack and Chrome often ask
    // niri for one of their own, which niri refuses, so the conversation
    // would open behind what's in front (Forward.js). A second is given
    // for it to come, and then its window is focused, if it has one.
    function open(action, app) {
        action.invoke();
        const entry = DesktopEntries.heuristicLookup(app);
        forwarding = [app.toLowerCase()].concat(entry ? [entry.id.toLowerCase()] : []);
        forwarded.restart();
    }

    // The names the last opened notice's sender goes by, which its window
    // is found by.
    property var forwarding: []

    // The window of the program that sent a notice, as windows are now, or
    // null, found by its name and its desktop entry's, as its card and its
    // row in the launcher both go to it.
    function senderWindow(app) {
        const entry = DesktopEntries.heuristicLookup(app);
        const ids = [app.toLowerCase()].concat(entry ? [entry.id.toLowerCase()] : []);
        return Object.values(Niri.windows)
            .find(w => w.app_id && ids.includes(w.app_id.toLowerCase())) ?? null;
    }

    Timer {
        id: forwarded
        interval: 1000
        onTriggered: {
            const id = Forward.toFocus(Niri.windows, Niri.focused, root.forwarding);
            if (id !== null)
                Niri.act({ FocusWindow: { id: id } });
        }
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

pragma Singleton
// Do not disturb: while it's on, a notice is held rather than shown, kept in
// the session's history (Notices.qml) unseen, for the launcher's collection
// of them; a critical one still shows, since it needs Joel now. It's on for
// a while or until it's ended, and says so in the margin, since a mode
// should never hold unseen.
//
// It lasts through a restart of the shell, as a switch does, from a file in
// the runtime directory holding when it began and when it ends, in
// milliseconds since 1970, an end of 0 meaning until it's ended. The
// session's end clears it with the runtime directory.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    property bool on: false
    property real since: 0
    property real until: 0
    // The notices held since it began and not yet seen.
    readonly property var held: Notices.history
        .filter(notice => notice.held && notice.missed && notice.at >= since)

    // Holds notices for this many minutes, or until it's ended with none.
    function keep(minutes) {
        since = on ? since : Date.now();
        until = minutes > 0 ? Date.now() + minutes * 60000 : 0;
        on = true;
        save();
    }

    // Holds them this many minutes longer than it would have.
    function more(minutes) {
        if (until > 0)
            until = Math.max(until, Date.now()) + minutes * 60000;
        save();
    }

    function end() {
        on = false;
        save();
    }

    function save() {
        file.setText(on ? `${since} ${until}\n` : "");
    }

    // Ends when its time is up, however long that is.
    Timer {
        running: root.on && root.until > 0
        interval: Math.max(1, root.until - Date.now())
        onTriggered: root.end()
    }

    FileView {
        id: file
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/quiet`
        printErrors: false
        onLoaded: {
            const [since, until] = text().trim().split(" ").map(Number);
            const left = text().trim() !== "" && (until === 0 || until > Date.now());
            root.since = left ? since : 0;
            root.until = left ? until : 0;
            root.on = left;
        }
    }
}

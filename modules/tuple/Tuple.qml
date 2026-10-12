pragma Singleton
// Tuple's calls, as the shell sees them, for the margin's piece (Call.qml)
// and the launcher's rows (Calls.qml). Tuple's command starts, joins and
// ends a call and says nothing after, so whether Joel is in one comes from
// the daemon's log, which Tuple writes in its data, where the XDG spec puts
// it: a line when a call connects, and one when it ends, or when the daemon
// starts or stops, which ends any call it had. The log is followed as it's
// written, from its last lines when the shell starts, so a call already
// going counts. What a line means is Tuple.js's to say.
//
// Joel's room keeps one link, which tuple new prints as it opens the room.
// It's kept in the shell's state, to copy again without opening the room.
// A call's link on the clipboard, as one pasted in a chat arrives, is kept
// too, so the launcher can offer to join it; nothing else copied is kept.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Tuple.js" as Log

Singleton {
    id: root

    readonly property string log: `${Quickshell.env("XDG_DATA_HOME")
        || Quickshell.env("HOME") + "/.local/share"}/tuple/0/log.txt`

    property bool calling: false
    // When the call connected, in milliseconds since 1970.
    property real since: 0
    // Joel's room's link, once tuple new has printed it.
    property string room: ""
    // The call link last copied, until something else is.
    property string copied: ""

    // Opens Joel's room and copies its link, to hand to whoever's joining.
    function open() {
        opening.running = true;
    }

    function join(url) {
        Niri.spawn(["tuple", "join", url]);
    }

    function leave() {
        Niri.spawn(["tuple", "end"]);
    }

    // Tuple asks the portal, which asks Joel what to share.
    function share() {
        Niri.spawn(["tuple", "share"]);
    }

    function copyRoom() {
        if (room !== "")
            Niri.spawn(["wl-copy", room]);
    }

    Process {
        running: true
        command: ["tail", "-F", "-n", "400", root.log]
        stdout: SplitParser {
            onRead: line => {
                const call = Log.after({
                    calling: root.calling,
                    since: root.since
                }, line, Date.now());
                root.since = call.since;
                root.calling = call.calling;
            }
        }
    }

    // Each copy as one line, of which only a call's link is kept.
    Process {
        running: true
        command: ["wl-paste", "--no-newline", "--type", "text", "--watch",
            "sh", "-c", "tr '\\n' ' '; echo"]
        stdout: SplitParser {
            onRead: line => root.copied = Log.link(line)
        }
    }

    Process {
        id: opening
        command: ["tuple", "new"]
        stdout: SplitParser {
            onRead: line => {
                const url = Log.link(line);
                if (url === "")
                    return;
                Niri.spawn(["wl-copy", url]);
                if (url !== root.room) {
                    root.room = url;
                    kept.setText(url);
                }
            }
        }
    }

    FileView {
        id: kept
        path: Quickshell.statePath("tuple-room")
        blockLoading: true
        printErrors: false
    }

    Component.onCompleted: room = Log.link(kept.text())
}

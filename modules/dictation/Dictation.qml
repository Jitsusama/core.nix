pragma Singleton
pragma ComponentBehavior: Bound
// Dictation, as modules/dictation's service tells it: idle, listening, or
// transcribing, the moment between the second press and the typing. Asked
// to watch, the service says its state and then each change, so nothing is
// asked twice, and a moment's answer between them: still-writing for a
// press while it types, and heard-nothing, with why, just before an idle
// that typed nothing. Its socket starts it, and it loads no model until the
// first toggle, so watching costs nothing while it's unused.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    property string state: "idle"
    readonly property bool listening: state === "listening"
    readonly property bool transcribing: state === "transcribing"
    readonly property bool busy: listening || transcribing

    // A moment's answer: "still-writing", or why it heard nothing (muted,
    // short, silent or unclear).
    signal answered(string word)

    // What its chord does: listening starts, or stops and types what was said.
    function toggle() {
        Niri.spawn(["dictation", "toggle"]);
    }

    Socket {
        id: watching
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/dictation.sock`
        connected: true
        onConnectionStateChanged: {
            if (watching.connected) {
                watching.write("watch\n");
                watching.flush();
            } else {
                // A service that stopped hears nobody. Its socket stays with
                // systemd, so watching again starts it again. A socket that
                // won't connect only says so as an error, never as a change
                // here, so this is tried once each time and never loops.
                root.state = "idle";
                Qt.callLater(() => watching.connected = true);
            }
        }
        parser: SplitParser {
            onRead: line => {
                const said = line.trim();
                // A word it doesn't know is from a newer service, and
                // says nothing here.
                if (["idle", "listening", "transcribing"].includes(said))
                    root.state = said;
                else if (said === "still-writing")
                    root.answered(said);
                else if (said.startsWith("heard-nothing "))
                    root.answered(said.slice("heard-nothing ".length));
            }
        }
    }
}

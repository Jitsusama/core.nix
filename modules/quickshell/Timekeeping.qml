// How chrony keeps the clock, as timekeeping tells it over varlink on the
// socket systemd holds for it (timekeeping/src/main.rs): the clock's
// offset, frequency and reference, the hardware clock's drift, and every
// server chrony asks, each one's figures and why it's chosen or set aside.
// It's asked only while something shown watches it, home's Now or Time,
// since the reader asks chronyd once a second for as long as it's asked
// and nothing otherwise; the reading is dropped when it isn't, so nothing
// stale is ever shown. A machine without chrony has no socket, and Time
// says nothing keeps the time.
pragma Singleton
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // How many of the shell's surfaces are showing what it says: it's
    // asked while any is. Each adds itself while shown and takes itself
    // away after.
    property int watching: 0

    // The last reading, { synchronised, clock, servers, hardwareClock,
    // activity }, or null while it isn't asked or hasn't answered.
    property var reading: null

    // Whether the reader failed to answer since it was last heard: refused,
    // gone, or chronyd not answering it. It's cleared once nothing watches.
    property bool refused: false

    readonly property string path: `${Quickshell.env("XDG_RUNTIME_DIR")}/io.jitsusama.Timekeeping`

    // Whether a connection may be made now: false for a moment after one
    // was refused or dropped, so a fresh one is made.
    property bool trying: true

    function unanswered() {
        if (watching === 0)
            return;
        reading = null;
        refused = true;
    }

    // One reply from the reader. A line that isn't JSON is passed over, so
    // one bad line can't stop the rest being heard.
    function heard(data) {
        let reply;
        try {
            reply = JSON.parse(data);
        } catch (error) {
            console.warn("timekeeping said something unreadable:", error);
            return;
        }
        if (reply.error || !reply.parameters) {
            unanswered();
            return;
        }
        reading = reply.parameters;
        refused = false;
    }

    onWatchingChanged: {
        if (watching === 0) {
            reading = null;
            refused = false;
        }
    }

    // A fresh connection each time: a socket refused once never connects
    // again, so it's made anew while something still watches.
    LazyLoader {
        active: root.watching > 0 && root.trying

        Socket {
            path: root.path
            connected: true
            onConnectionStateChanged: {
                if (connected) {
                    write(JSON.stringify({
                        method: "io.jitsusama.Timekeeping.Watch",
                        more: true
                    }) + "\u0000");
                    flush();
                } else {
                    root.unanswered();
                    again.restart();
                }
            }
            onError: {
                root.unanswered();
                again.restart();
            }
            parser: SplitParser {
                splitMarker: "\u0000"
                onRead: data => root.heard(data)
            }
        }
    }

    Timer {
        id: again
        interval: 1000
        onTriggered: {
            root.trying = false;
            root.trying = true;
        }
    }
}

// How chrony keeps the clock, as timekeeping tells it over varlink on the
// socket systemd holds for it (timekeeping/src/main.rs): the clock's
// offset, frequency and reference, the hardware clock's drift, and every
// server chrony asks, each one's figures and why it's chosen or set aside.
// It's asked only while something shown watches it, home's Now or Time,
// since the reader asks chronyd once a second for as long as it's asked
// and nothing otherwise; the reading is kept no more than a minute past
// the reader's last answer. A machine without chrony has no socket, and
// Time says nothing keeps the time.
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
    // activity }, or null while it hasn't answered or was let go.
    property var reading: null

    // Whether the reader failed to answer since it was last heard: refused,
    // gone, or chronyd not answering it. It's cleared once nothing watches,
    // so a step back asks afresh rather than saying nothing keeps the time
    // while chrony keeps it (P9R1KI, 2026-11-15).
    property bool refused: false

    // When the reader last answered, by the wall clock, and how long past
    // it a reading is kept, watched or not: the reader answers each second
    // while watched, so one a minute old was left by a reader gone quiet.
    // The wall clock as well as `letGo`, as a Timer's minute stands still
    // across a suspend, and a reading from before one is the likeliest to
    // be wrong after it.
    property real heardAt: 0
    property int keptFor: 60000

    readonly property string path: `${Quickshell.env("XDG_RUNTIME_DIR")}/io.jitsusama.Timekeeping`

    // Whether a connection may be made now: false for a moment after one
    // was refused or dropped, so a fresh one is made.
    property bool trying: true

    function unanswered() {
        if (watching === 0)
            return;
        letGo.stop();
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
        heardAt = Date.now();
        letGo.interval = keptFor;
        letGo.restart();
    }

    // The reading is kept a while once nothing watches, not dropped at
    // once: dropped, a step out of home made Now's plate again inside the
    // step, for the row it was leaving (the keys lab, 2026-11-22). Watched
    // again, it's shown until the reader's first answer replaces it, which
    // comes at once, and is still let go in time if no answer comes, so a
    // reader that takes the connection and says nothing can't hold it up.
    // Its spans counted from when it was read, as a server's last sample
    // is, read that much low until that answer. Each change of watching
    // sets `letGo` again by the wall clock, so a suspend can't stretch the
    // minute past a step back; what's left is held to the minute, so a
    // wall clock stepped back can't stretch it either.
    onWatchingChanged: {
        const left = Math.min(keptFor, keptFor - (Date.now() - heardAt));
        if (watching === 0)
            refused = false;
        if (reading === null)
            return;
        if (left <= 0) {
            letGoOfReading();
        } else {
            letGo.interval = left;
            letGo.restart();
        }
    }
    // Only the reading: a refusal is said while watched and cleared once
    // nothing watches, and is never the timer's to clear.
    function letGoOfReading() {
        letGo.stop();
        reading = null;
    }

    Timer {
        id: letGo
        onTriggered: root.letGoOfReading()
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

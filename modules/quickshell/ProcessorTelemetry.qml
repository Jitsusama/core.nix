// The processor's own telemetry, as processor-telemetry tells it over
// varlink on the socket systemd holds for it: the temperatures no hwmon
// chip gives (its NPU's, image processor's and media and display
// engines'), for Heat. What held the processor back and what kept it
// from resting are the machine daemon's to say (modules/machine's
// telemetry_rows.rs), which asks the reader itself. It's asked only
// while something shown watches it, since the reader reads the
// processor's counters once a second for as long as it's asked and
// nothing otherwise; the reading is dropped when it isn't, so nothing
// stale is ever shown. A processor with no such telemetry answers
// NoRegion, and nothing is said.
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

    // The last second as the reader told it, { temperatures, heldBack,
    // clusters, npu, keptFromResting }, or null while it isn't asked or
    // hasn't answered.
    property var reading: null

    // Whether the reader has failed to answer since it was last heard:
    // refused, gone, or saying it has no telemetry. A row already shown
    // stays where it is, dimmed, rather than leaving and moving the rows
    // under it. It's cleared once nothing watches.
    property bool refused: false
    // Whether it has refused without ever being heard, for as long as the
    // shell runs: a machine with no reader. A place made then leaves out
    // what only it would say, rather than showing it dimmed for good.
    property bool missing: false
    property bool heardOnce: false

    readonly property string path: "/run/processor-telemetry/io.jitsusama.ProcessorTelemetry"

    // Whether a connection may be made now: false for a moment after one
    // was refused or dropped, so a fresh one is made.
    property bool trying: true

    function dropped() {
        reading = null;
    }

    // The reader didn't answer: it refused, went, or has no telemetry. A
    // connection closed because nothing watches any more says nothing.
    function unanswered() {
        if (watching === 0)
            return;
        refused = true;
        if (!heardOnce)
            missing = true;
    }

    // One reply from the reader. A line that isn't JSON is passed over, so
    // one bad line can't stop the rest being heard.
    function heard(data) {
        let reply;
        try {
            reply = JSON.parse(data);
        } catch (error) {
            console.warn("processor-telemetry said something unreadable:", error);
            return;
        }
        reading = reply.error ? null : reply.parameters ?? null;
        if (reading === null) {
            unanswered();
            return;
        }
        refused = false;
        missing = false;
        heardOnce = true;
    }

    onWatchingChanged: {
        if (watching === 0) {
            dropped();
            refused = false;
        }
    }

    // A fresh connection each time, as Copies.qml makes its own: a socket
    // refused once, as when the reader's isn't there yet, never connects
    // again, so it's made anew while something still watches.
    LazyLoader {
        active: root.watching > 0 && root.trying

        Socket {
            path: root.path
            connected: true
            onConnectionStateChanged: {
                if (connected) {
                    // zlink closes the connection on empty parameters
                    // given to a method that takes none, as Wifi.qml says
                    // of wifid.
                    write(JSON.stringify({
                        method: "io.jitsusama.ProcessorTelemetry.Watch",
                        more: true
                    }) + "\u0000");
                    flush();
                } else {
                    root.dropped();
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

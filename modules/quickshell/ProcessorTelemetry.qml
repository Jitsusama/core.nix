// The processor's own telemetry, as processor-telemetry tells it over
// varlink on the socket systemd holds for it: the temperatures no hwmon
// chip gives (its NPU's, image processor's and media and display
// engines'), and what held the processor, each core cluster and the NPU
// back over the last second, and what kept the package from resting, kept
// over the last ten seconds so a share near a twentieth doesn't come and
// go each second (launcher/Machine.js says how). It's asked only while something shown watches
// it, since the reader reads the processor's counters once a second for as
// long as it's asked and nothing otherwise; the reading is dropped when it
// isn't, so nothing stale is ever shown. A processor with no such telemetry
// answers NoRegion, and nothing is said.
pragma Singleton
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher/Machine.js" as Machine

Singleton {
    id: root

    // How many of the shell's surfaces are showing what it says: it's
    // asked while any is. Each adds itself while shown and takes itself
    // away after.
    property int watching: 0

    // The last second as the reader told it, { temperatures, heldBack,
    // clusters, npu, keptFromResting }, or null while it isn't asked or hasn't answered.
    property var reading: null

    // The readings of what kept the package from resting, oldest first,
    // the last ten seconds' at most, and what they say together, as
    // Machine.kept says it ({ share, by, shown }), or null while nothing
    // has said it. Both go with the reading.
    property var rested: []
    property var kept: null
    readonly property int remembered: 10

    readonly property string path: "/run/processor-telemetry/io.jitsusama.ProcessorTelemetry"

    // Whether a connection may be made now: false for a moment after one
    // was refused or dropped, so a fresh one is made.
    property bool trying: true

    function dropped() {
        reading = null;
        rested = [];
        kept = null;
    }

    onWatchingChanged: {
        if (watching === 0)
            dropped();
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
                    again.restart();
                }
            }
            onError: again.restart()
            parser: SplitParser {
                splitMarker: "\u0000"
                onRead: data => {
                    const reply = JSON.parse(data);
                    root.reading = reply.error ? null : reply.parameters ?? null;
                    const resting = root.reading?.keptFromResting ?? null;
                    if (resting !== null) {
                        root.rested = root.rested.concat([resting]).slice(-root.remembered);
                        root.kept = Machine.kept(root.rested, root.kept?.shown ?? false);
                    }
                }
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

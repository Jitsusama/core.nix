// The processor's own telemetry, as processor-telemetry tells it over
// varlink on the socket systemd holds for it: the temperatures no hwmon
// chip gives (its NPU's, image processor's and media and display
// engines'), and what held the processor, each core cluster and the NPU
// back over the last second. It's asked only while something shown watches
// it, since the reader reads the processor's counters once a second for as
// long as it's asked and nothing otherwise; the reading is dropped when it
// isn't, so nothing stale is ever shown. A processor with no such telemetry
// answers NoRegion, and nothing is said.
pragma Singleton
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
    // clusters, npu }, or null while it isn't asked or hasn't answered.
    property var reading: null

    readonly property string path: "/run/processor-telemetry/io.jitsusama.ProcessorTelemetry"

    Socket {
        path: root.path
        connected: root.watching > 0
        onConnectionStateChanged: {
            if (connected) {
                // zlink closes the connection on empty parameters given to
                // a method that takes none, as Wifi.qml says of wifid.
                write(JSON.stringify({
                    method: "io.jitsusama.ProcessorTelemetry.Watch",
                    more: true
                }) + "\u0000");
                flush();
            } else {
                root.reading = null;
            }
        }
        parser: SplitParser {
            splitMarker: "\u0000"
            onRead: data => {
                const reply = JSON.parse(data);
                root.reading = reply.error ? null : reply.parameters ?? null;
            }
        }
    }
}

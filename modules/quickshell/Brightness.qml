// Shows the screen's brightness for a moment whenever it changes, from the
// keys or from anything else, as the same bar the volume uses. The kernel
// sends a udev event every time a backlight's brightness is written, so this
// listens for those rather than asking every so often.
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: root

    // The sysfs directory of the backlight that changed last.
    property string backlight: ""
    property real level: 0
    property bool shown: false

    Process {
        running: true
        command: ["udevadm", "monitor", "--udev", "--subsystem-match=backlight"]

        stdout: SplitParser {
            // An event reads like this, after a header:
            // UDEV  [160084.525157] change   /devices/…/intel_backlight (backlight)
            onRead: line => {
                const event = line.match(/ change +(\S+) \(backlight\)$/);
                if (!event)
                    return;
                root.backlight = "/sys" + event[1];
                brightness.reload();
                maximum.reload();
                root.level = parseInt(brightness.text()) / parseInt(maximum.text());
                root.shown = true;
                hide.restart();
            }
        }
    }

    FileView {
        id: brightness
        path: root.backlight ? root.backlight + "/brightness" : ""
        blockLoading: true
    }

    FileView {
        id: maximum
        path: root.backlight ? root.backlight + "/max_brightness" : ""
        blockLoading: true
    }

    Timer {
        id: hide
        interval: 1500
        onTriggered: root.shown = false
    }

    Meter {
        name: "brightness"
        visible: root.shown
        level: root.level
    }
}

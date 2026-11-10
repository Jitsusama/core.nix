// Drives the launcher's Screens (modules/quickshell/launcher/Displays.qml)
// through Quickshell itself, with niri stood in, answering what it has
// of each screen anew each time it's asked, as niri does. Screens asks
// at every workspace change and as the launcher opens, and niri's answer
// reads the same while no screen changed, so those make no screen's row
// again; a screen that did change is drawn as it now stands. It exits 0
// once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    function screens() {
        return displays.all.filter(row => String(row.key).startsWith("display:"));
    }

    QtObject {
        id: theLauncher

        property bool open: false
        readonly property var sources: []
        readonly property var terminalSees: ({ walked: [] })
    }

    Launcher.Displays {
        id: displays

        launcher: theLauncher
    }

    Component.onCompleted: Qt.callLater(() => {
        try {
            const first = screens();
            expect("the panel and the monitor are listed", first.length === 2,
                first.map(row => row.key).join(", "));
            // niri's workspaces said anew, and the launcher opening, each ask
            // again; the answer reads as it did.
            const madeBefore = displays.tally.rows;
            Niri.workspaces = { said: "anew" };
            theLauncher.open = true;
            expect("niri's screens said again as they were make no screen's row again",
                displays.tally.rows === madeBefore, displays.tally.rows - madeBefore);
            expect("and each row is the one it was", screens().every((row, at) => row === first[at]),
                "");
            // The monitor turned off, so its row says so.
            Niri.monitorOn = false;
            Niri.workspaces = { said: "again" };
            const was = first.find(row => row.key === "display:DP-1");
            const monitor = screens().find(row => row.key === "display:DP-1");
            expect("a screen that changed is drawn as it now stands", monitor !== was
                && monitor?.verb !== was?.verb
                && displays.tally.rows > madeBefore, displays.tally.rows - madeBefore);
            Qt.exit(0);
        } catch (error) {
            root.fail(error.message);
        }
    })
}

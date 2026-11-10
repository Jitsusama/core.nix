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
import "Expect.js" as Expect

ShellRoot {
    id: root

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
            Expect.expect("the panel and the monitor are listed", first.length === 2,
                first.map(row => row.key).join(", "));
            // niri's workspaces said anew, and the launcher opening, each ask
            // again; the answer reads as it did.
            const madeBefore = displays.tally.rows;
            Niri.workspaces = { said: "anew" };
            theLauncher.open = true;
            Expect.expect("niri's screens said again as they were make no screen's row again",
                displays.tally.rows === madeBefore, displays.tally.rows - madeBefore);
            Expect.expect("and each row is the one it was", screens().every((row, at) => row === first[at]),
                "");
            // The same screens said in the other order, as niri's map may
            // give them at any ask.
            Niri.turned = true;
            Niri.workspaces = { said: "turned" };
            Expect.expect("the same screens in another order make no screen's row again",
                displays.tally.rows === madeBefore, displays.tally.rows - madeBefore);
            // The monitor turned off, so its row says so.
            Niri.monitorOn = false;
            Niri.workspaces = { said: "again" };
            const was = first.find(row => row.key === "display:DP-1");
            const monitor = screens().find(row => row.key === "display:DP-1");
            Expect.expect("a screen that changed is drawn as it now stands", monitor !== was
                && monitor?.verb !== was?.verb
                && displays.tally.rows > madeBefore, displays.tally.rows - madeBefore);
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    })
}

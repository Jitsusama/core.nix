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
import "launcher/Catalogue.js" as Catalogue

ShellRoot {
    id: root

    function screens() {
        return displays.all.filter(row => String(row.key).startsWith("display:"));
    }

    QtObject {
        id: theLauncher

        property bool open: false
        readonly property var sources: []
        readonly property var terminalSees: ({ walked: [], listed: [] })
        // What Seen.js reads of the open launcher: nothing answered,
        // stepped into, found or placed, so the lights stay unwatched.
        readonly property var answer: null
        readonly property var held: []
        readonly property var matches: []
        readonly property var placed: []
        readonly property string typed: ""
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
            // The panel's Room Light, its sensor known but what it hears
            // not read yet, as a walk of the catalogue finds it with no
            // watch running: it says it's being read, so Return has a
            // reason to only close, and copies nothing.
            displays.light = { connector: "eDP-1", share: 50, now: 100, most: 200 };
            displays.roomSensor = { device: "acpi-als", lux: null, raw: null };
            const panel = displays.shown.find(output => output.name === "eDP-1");
            const roomLight = displays.lightRows(panel)
                .find(row => String(row.key).endsWith(":room"));
            Expect.expect("a Room Light still being read does something on Return",
                !!roomLight && Catalogue.acts(roomLight),
                JSON.stringify({ side: roomLight?.side, verb: roomLight?.verb }));
            Expect.expect("and copies nothing while it's read", roomLight?.copies() === "",
                roomLight?.copies());
            // What its spots are made of holds while a level or a lux moves,
            // so a closing remakes none of them (Provider.spotsStamp), and
            // moves once a part comes or goes, or a screen changes.
            const made = displays.spotsStamp;
            displays.light = { connector: "eDP-1", share: 80, now: 160, most: 200 };
            displays.roomSensor = { device: "acpi-als", lux: 312, raw: 40 };
            Expect.expect("a level and a lux moving leave the spots' make as it was",
                displays.spotsStamp === made, "");
            displays.roomSensor = null;
            Expect.expect("the room's sensor going changes it", displays.spotsStamp !== made, "");
            displays.roomSensor = { device: "acpi-als", lux: 5, raw: 1 };
            Expect.expect("and coming back puts it back", displays.spotsStamp === made, "");
            Niri.monitorOn = true;
            Niri.workspaces = { said: "lit again" };
            Expect.expect("a screen turned on changes it", displays.spotsStamp !== made, "");
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    })
}

pragma ComponentBehavior: Bound
// Drives the launcher's Screens (modules/quickshell/launcher/Displays.qml)
// through Quickshell itself, over a stand-in for the windows daemon
// serving a panel's and a monitor's rows ready, each holding niri's record
// of its screen, as modules/windows' screens.rs does. Screens draws each
// row as it's served; a change to the monitor's row makes its row again
// and keeps the panel's; a change Screens makes to a screen asks niri and
// then asks the daemon to look again, since niri's stream says nothing of
// it; and what its spots are made of holds while a level or a lux moves.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect
import "launcher/Catalogue.js" as Catalogue

ShellRoot {
    id: root

    property string waiting: "the screens served"

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

    // A screen as niri says one, laid out unless `lit` is false.
    function niri(name, make, model, lit) {
        const mode = (width, height) => ({
            width: width, height: height, refresh_rate: 60000, is_preferred: true
        });
        return {
            name: name, make: make, model: model, serial: null, physical_size: [300, 190],
            modes: [mode(2880, 1800), mode(1920, 1080)], current_mode: 0,
            is_custom_mode: false, vrr_supported: false, vrr_enabled: false,
            logical: lit ? { x: 0, y: 0, width: 1440, height: 900, scale: 2,
                transform: "Normal" } : null
        };
    }

    // A screen's row as the windows daemon serves it, wrapped as the
    // ready-rows crate's Kept wraps each.
    function screen(name, said, order, lit) {
        const key = `row:displays:display:${name}`;
        return { key: key, kind: "row", row: {
            key: key, thing: `display:${name}`, place: "displays", kind: "screen",
            order: order, name: said, glyph: "display", hue: "machine", depth: 0,
            side: lit ? [] : [{ said: "off" }],
            note: lit ? "2880 \u00d7 1800 \u00b7 60 Hz \u00b7 scale 2\u00d7" : "off",
            held: { output: name, standing: lit ? "lit" : "off", builtIn: name === "eDP-1",
                niri: root.niri(name, name === "eDP-1" ? "LG Display" : "Dell Inc.",
                    name === "eDP-1" ? "0x07C6" : "U2723QE", lit) }
        } };
    }

    readonly property var sleep: ({ key: "row:displays:sleep", kind: "row", row: {
            key: "row:displays:sleep", thing: "displays:sleep", place: "displays",
            kind: "action", order: 3072, name: "Turn Every Screen Off", glyph: "displayOff",
            hue: "action", depth: 0, side: [], note: "a key wakes them"
        } })

    // The shell's watch, so the stand-in can tell it more, and the looks
    // it was asked.
    property var watch: null
    property int looks: 0

    function tell(records) {
        root.watch.socket.write(`${JSON.stringify({ id: root.watch.id, changed: records })}\n`);
        root.watch.socket.flush();
    }

    // A stand-in for the windows daemon: hello, a watch of the displays'
    // rows answered whole, and a look answered once it's counted.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "windows",
                            state: "answering" };
                    } else if (ask.ask === "watch") {
                        root.watch = { socket: asked, id: ask.id };
                        answer = { id: ask.id, records: [
                            root.screen("eDP-1", "Built-in Screen", 1024, true),
                            root.screen("DP-1", "Dell Inc. U2723QE", 2048, true), root.sleep] };
                    } else if (ask.ask === "look") {
                        root.looks += 1;
                        answer = { id: ask.id, looked: true };
                    } else {
                        answer = { id: ask.id, ok: true };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    Launcher.Displays {
        id: displays

        launcher: theLauncher
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: Expect.fail(`never came: ${root.waiting}`)
    }

    property var first: []
    property string made: ""

    // Each case waits for what it needs to have been heard.
    Timer {
        running: true
        repeat: true
        interval: 20
        onTriggered: {
            try {
                root.step();
            } catch (error) {
                root.waiting = "a failure";
                Expect.fail(error.message);
            }
        }
    }

    function step() {
        if (waiting === "the screens served" && screens().length === 2)
            served();
        else if (waiting === "the monitor's change" && monitorChanged())
            changedHere();
        else if (waiting === "a look" && looks === 1)
            looked();
        else if (waiting === "the monitor lit again" && displays.spotsStamp !== made) {
            waiting = "done";
            Qt.exit(0);
        }
    }

    // Each row is drawn as it's served, the panel first, and Turn Every
    // Screen Off after them.
    function served() {
        first = screens();
        Expect.expect("each screen named as it's served",
            first.map(row => row.name).join(", ") === "Built-in Screen, Dell Inc. U2723QE",
            first.map(row => row.name).join(", "));
        Expect.expect("and what it runs as its note",
            first[0].note === "2880 \u00d7 1800 \u00b7 60 Hz \u00b7 scale 2\u00d7", first[0].note);
        Expect.expect("Turn Every Screen Off follows them",
            displays.all[2]?.key === "displays:sleep", displays.all.map(row => row.key).join(", "));
        waiting = "the monitor's change";
        tell([screen("DP-1", "Dell Inc. U2723QE", 2048, false)]);
    }

    function monitorChanged() {
        const monitor = screens().find(row => row.key === "display:DP-1");
        return monitor !== undefined && monitor !== first[1];
    }

    // The monitor's row told off makes its row again and keeps the panel's;
    // then turning it on here asks niri, then the daemon to look.
    function changedHere() {
        const now = screens();
        Expect.expect("the panel's row, unchanged, is the one it was", now[0] === first[0], "");
        Expect.expect("the monitor told off says so and offers to turn it on",
            now[1].side === "off" && now[1].verb !== first[1].verb,
            JSON.stringify({ side: now[1].side, verb: now[1].verb }));
        waiting = "a look";
        now[1].run();
    }

    function looked() {
        Expect.expect("turning it on asked niri", Niri.asked.length === 1,
            JSON.stringify(Niri.asked));
        // The panel's Room Light, its sensor known but what it hears not
        // read yet, as a walk of the catalogue finds it with no watch
        // running: it says it's being read, so Return has a reason to only
        // close, and copies nothing.
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
        // What its spots are made of holds while a level or a lux moves, so
        // a closing remakes none of them (Provider.spotsStamp), and moves
        // once a part comes or goes, or a screen changes.
        made = displays.spotsStamp;
        displays.light = { connector: "eDP-1", share: 80, now: 160, most: 200 };
        displays.roomSensor = { device: "acpi-als", lux: 312, raw: 40 };
        Expect.expect("a level and a lux moving leave the spots' make as it was",
            displays.spotsStamp === made, "");
        displays.roomSensor = null;
        Expect.expect("the room's sensor going changes it", displays.spotsStamp !== made, "");
        displays.roomSensor = { device: "acpi-als", lux: 5, raw: 1 };
        Expect.expect("and coming back puts it back", displays.spotsStamp === made, "");
        waiting = "the monitor lit again";
        tell([screen("DP-1", "Dell Inc. U2723QE", 2048, true)]);
    }
}

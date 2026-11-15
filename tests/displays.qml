pragma ComponentBehavior: Bound
// Drives the launcher's Screens (modules/quickshell/launcher/Displays.qml)
// through Quickshell itself, over a stand-in for the windows daemon
// serving a panel's and a monitor's rows ready, each holding niri's record
// of its screen, as modules/windows' screens.rs does. Screens draws each
// row as it's served; a change to the monitor's row makes its row again
// and keeps the panel's; a screen's Return asks the daemon the act it
// serves, and niri nothing; :set brightness and "brightness 40" say
// the share the served Brightness holds; a share typed asks the daemon as a Brightness level does;
// and :set nightlight asks what the Warmth's Night Light asks, only to
// turn it the other way.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

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
        const act = lit ? { key: "displays:focus", ask: { ask: "screen", do: "focus",
            output: name } } : { key: "verb:display-on", ask: { ask: "screen", do: "on",
            output: name } };
        return { key: key, kind: "row", row: {
            key: key, thing: `display:${name}`, place: "displays", kind: "screen",
            order: order, name: said, glyph: "display", hue: "machine", depth: 0,
            side: lit ? [] : [{ said: "off" }],
            note: lit ? "2880 \u00d7 1800 \u00b7 60 Hz \u00b7 scale 2\u00d7" : "off",
            held: { output: name, standing: lit ? "lit" : "off", builtIn: name === "eDP-1",
                niri: root.niri(name, name === "eDP-1" ? "LG Display" : "Dell Inc.",
                    name === "eDP-1" ? "0x07C6" : "U2723QE", lit) },
            acts: [act]
        } };
    }

    readonly property var sleep: ({ key: "row:displays:sleep", kind: "row", row: {
            key: "row:displays:sleep", thing: "displays:sleep", place: "displays",
            kind: "action", order: 3072, name: "Turn Every Screen Off", glyph: "displayOff",
            hue: "action", depth: 0, side: [], note: "a key wakes them",
            acts: [{ key: "displays:sleep", ask: { ask: "screen", do: "sleep" } }]
        } })

    // The panel's Brightness at `share`, a level inside it and its Room
    // Light, as light/brightness.rs and light/room.rs serve them, each
    // ordered as the ready-rows crate's Kept orders every row.
    function brightness(share) {
        return { key: "row:displays:setting:eDP-1:brightness", kind: "row", row: {
            key: "row:displays:setting:eDP-1:brightness",
            thing: "displays:eDP-1:brightness", place: "displays", kind: "setting",
            order: 1025, name: "Brightness", glyph: "brightness", hue: "machine",
            parent: "row:displays:display:eDP-1", depth: 1,
            side: [{ said: `${share}%` }], note: "", held: { share: share },
            graph: { shape: "bar", values: [share / 100], warn: false },
            acts: [{ key: "verb:more", step: "rows" }]
        } };
    }
    readonly property var room: ({ key: "row:displays:setting:eDP-1:room", kind: "row",
        row: {
            key: "row:displays:setting:eDP-1:room", thing: "displays:eDP-1:room",
            place: "displays", kind: "fact", order: 1027, name: "Room Light",
            glyph: "room", hue: "machine", parent: "row:displays:display:eDP-1", depth: 1,
            side: [{ said: "312 lx" }], note: "", acts: []
        } })
    readonly property var level: ({ key: "row:displays:choice:eDP-1:brightness-50",
        kind: "row", row: {
            key: "row:displays:choice:eDP-1:brightness-50",
            thing: "verb:display-brightness-50", place: "displays", kind: "choice",
            order: 1026, name: "50%", glyph: "brightness", hue: "action",
            parent: "row:displays:setting:eDP-1:brightness", depth: 2, side: [], note: "",
            held: { fill: { n: 50 } },
            acts: [{ key: "displays:brightness:level",
                ask: { ask: "light", do: "brightness", share: 50 } }]
        } })

    // The Warmth, Screens' own beside the screens, as light/warmth.rs
    // serves it, with its Night Light inside it, off, and no Follow the
    // Room, as with the lid shut.
    readonly property var warmth: ({ key: "row:displays:warmth", kind: "row", row: {
            key: "row:displays:warmth", thing: "displays:warmth", place: "displays",
            kind: "warmth", order: 2560, name: "Warmth", glyph: "warmth", hue: "machine",
            depth: 0, side: [{ said: "6,500 K" }], note: "the screen's own white",
            acts: [{ key: "verb:more", step: "rows" }]
        } })
    readonly property var nightLight: ({ key: "row:displays:switch:night-light", kind: "row",
        row: {
            key: "row:displays:switch:night-light", thing: "displays:night-light",
            place: "displays", kind: "switch", order: 2561, name: "Night Light",
            glyph: "nightLight", hue: "machine", parent: "row:displays:warmth", depth: 1,
            side: [{ said: "off" }], held: { on: false },
            acts: [{ key: "displays:night-light",
                ask: { ask: "light", do: "night-light", on: true } }]
        } })

    // The shell's watch, so the stand-in can tell it more, and the acts it
    // was asked of a screen and of its light.
    property var watch: null
    property var acted: []
    property var lit: []

    function tell(records) {
        root.watch.socket.write(`${JSON.stringify({ id: root.watch.id, changed: records })}\n`);
        root.watch.socket.flush();
    }

    // A stand-in for the windows daemon: hello, a watch of the displays'
    // rows answered whole, and an act on a screen done once it's kept.
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
                            root.screen("DP-1", "Dell Inc. U2723QE", 2048, true), root.sleep,
                            root.brightness(50), root.level, root.room, root.warmth,
                            root.nightLight] };
                    } else if (ask.ask === "screen") {
                        root.acted = root.acted.concat([ask]);
                        answer = { id: ask.id, done: true };
                    } else if (ask.ask === "light") {
                        root.lit = root.lit.concat([ask]);
                        answer = { id: ask.id, done: true };
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
        else if (waiting === "the act" && acted.length === 1)
            done();
        else if (waiting === "the share moved" && displays.servedBrightness?.share === 80)
            shareMoved();
        else if (waiting === "the monitor lit again" && displays.spotsStamp !== made)
            brightened();
        else if (waiting === "the shares asked" && lit.length === 3)
            asked();
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
        // Its connector is among the words it's found by, made ready as the
        // launcher's search reads them (Launcher.qml's hiddenWords).
        Expect.expect("each screen is found by its connector",
            first[1].keywords?.text === "DP-1", JSON.stringify(first[1].keywords));
        Expect.expect("the Warmth stands beside them, Turn Every Screen Off after it",
            displays.all[2]?.key === "displays:warmth" && displays.all[3]?.key === "displays:sleep",
            displays.all.map(row => row.key).join(", "));
        Expect.expect("\"screen warmth\" says the Warmth beside the screens",
            displays.warmthAnswer()?.headline === "6,500 K",
            JSON.stringify(displays.warmthAnswer()?.headline));
        waiting = "the monitor's change";
        tell([screen("DP-1", "Dell Inc. U2723QE", 2048, false)]);
    }

    function monitorChanged() {
        const monitor = screens().find(row => row.key === "display:DP-1");
        return monitor !== undefined && monitor !== first[1];
    }

    // The monitor's row told off makes its row again and keeps the panel's;
    // then its Return asks the daemon to turn it on.
    function changedHere() {
        const now = screens();
        Expect.expect("the panel's row, unchanged, is the one it was", now[0] === first[0], "");
        Expect.expect("the monitor told off says so and offers to turn it on",
            now[1].side === "off" && now[1].verb !== first[1].verb,
            JSON.stringify({ side: now[1].side, verb: now[1].verb }));
        waiting = "the act";
        now[1].run();
    }

    function done() {
        Expect.expect("turning it on asked the daemon to turn that one on",
            JSON.stringify(acted.map(ask => [ask.do, ask.output])) === '[["on","DP-1"]]',
            JSON.stringify(acted));
        Expect.expect("and niri nothing", Niri.asked.length === 0, JSON.stringify(Niri.asked));
        // The panel holds the Brightness the daemon serves inside it, at
        // the share it read.
        Expect.expect("the panel holds the Brightness served inside it, at its share",
            displays.backlitKey === "row:displays:display:eDP-1"
                && displays.servedBrightness?.share === 50,
            JSON.stringify([displays.backlitKey, displays.servedBrightness]));
        waiting = "the share moved";
        tell([brightness(80)]);
    }

    // The share it moves to is what :set brightness and "brightness 40"
    // say it is now; then the monitor lights again.
    function shareMoved() {
        Expect.expect(":set brightness says the share served",
            displays.setOptions.brightness?.now() === 80,
            JSON.stringify(displays.setOptions.brightness?.now()));
        Expect.expect("and \"brightness 40\" says it's at that share now",
            displays.setTo(40)?.note.includes("80") === true,
            JSON.stringify(displays.setTo(40)?.note));
        made = displays.spotsStamp;
        waiting = "the monitor lit again";
        tell([screen("DP-1", "Dell Inc. U2723QE", 2048, true)]);
    }

    // "brightness 40" typed and :set brightness=45 each ask the windows
    // daemon what a Brightness level asks, at the share typed; :set
    // nonightlight, the Night Light off already, asks nothing, and :set
    // nightlight what its Return asks. With no Follow the Room served,
    // :set warmth isn't offered.
    function brightened() {
        Expect.expect(":set nightlight says the Night Light stands off",
            displays.setOptions.nightlight?.now() === false,
            JSON.stringify(Object.keys(displays.setOptions)));
        Expect.expect("and :set warmth isn't offered with no room to follow",
            displays.setOptions.warmth === undefined, "");
        waiting = "the shares asked";
        displays.setTo(40).run();
        displays.setOptions.brightness.set(45);
        displays.setOptions.nightlight.set(false);
        displays.setOptions.nightlight.set(true);
    }

    function asked() {
        Expect.expect("each share typed and the night light asked the daemon as its row does",
            JSON.stringify(lit.map(ask => [ask.do, ask.share ?? ask.on]))
                === '[["brightness",40],["brightness",45],["night-light",true]]',
            JSON.stringify(lit));
        Expect.expect("and no level was drawn asking", acted.length === 1, JSON.stringify(acted));
        waiting = "done";
        Qt.exit(0);
    }
}

// Drives the launcher's Keyboard collection (modules/quickshell/launcher/
// Keyboard.qml) and the layout it reads (KeyboardLayout.qml) through
// Quickshell itself, over the ErgoDox's own description and a board locked
// on Numb, with niri stood in for: a symbol is found by its name with its
// layer and where it sits, an action by what it does, a layer by its name,
// the locked layer offers to be let go rather than locked again, the
// board's Layer and Light choices are found by their words though they're
// listed only in the board's own parts (Keyboards.qml), and the overlay
// shows what the launcher chose. Among the keyboards plugged in, the
// board's parts hold its Layer, saying where it's locked, and its Light,
// each a setting whose choices a step in run the board's own command.
// Then the board is unplugged, as its watcher says with "-", and the open
// layout goes, with every key, act and choice of the board, until it's
// plugged back in. It exits 0 once every case holds, and otherwise 1,
// saying which case didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher"

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function first(typed) {
        return keyboard.search(typed).rows[0] ?? { name: "nothing" };
    }

    Keyboard {
        id: keyboard
    }

    Keyboards {
        id: keyboards
    }

    // The ErgoDox as the kernel's list of input devices has it, without the
    // lock lights or a USB folder the sandbox has no files for.
    readonly property var board: ({
        name: "ZSA Technology Labs ErgoDox EZ",
        phys: "usb-0000:00:14.0-2/input0",
        sysfs: "/devices/virtual/input/input9",
        bus: "0003",
        vendor: "3297",
        product: "1307",
        leds: 0
    })

    // What each of a setting's choices runs, by its name.
    function runs(choices) {
        return choices.map(choice => {
            choice.run();
            return `${choice.name}: ${Niri.spawned.join(" ")}`;
        }).join("; ");
    }

    // Where the board's watcher says its layer, written here as the watcher
    // would as the board is unplugged and plugged back in.
    FileView {
        id: layerFile
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/keyboard-layer`
        printErrors: false
    }

    // The board's rows the launcher would list or find.
    function boardRows() {
        return keyboard.everything().concat(keyboard.search("backslash").rows,
            keyboard.search("lock punk").rows, keyboard.search("flash").rows);
    }

    // Waits for the watcher's word to reach the shell, for five seconds at
    // most, then runs the next cases.
    Timer {
        id: plugging
        interval: 50
        repeat: true
        property int left: 100
        property bool plugged: false
        property var next: () => {}
        onTriggered: {
            if (KeyboardLayer.present === plugged) {
                stop();
                root.guarded(next);
            } else if (--left === 0) {
                stop();
                console.warn(`FAIL the watcher's "${plugged ? "Numb locked" : "-"}" never came`);
                Qt.exit(1);
            }
        }
    }

    function plug(plugged, next) {
        layerFile.setText(plugged ? "Numb locked\n" : "-\n");
        plugging.plugged = plugged;
        plugging.next = next;
        plugging.left = 100;
        plugging.start();
    }

    function guarded(cases) {
        try {
            cases();
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }

    // Unplugged, the open layout goes and nothing of the board is listed,
    // found or shown, though its layout file stays; plugged back in, it all
    // comes back.
    function unplugged() {
        expect("the layout put away as the board goes", !KeyboardLayout.shown,
            `${KeyboardLayout.shown}`);
        expect("no board while it's unplugged", !KeyboardLayout.present,
            `${KeyboardLayout.layers.length} layers`);
        expect("none of its rows", boardRows().length === 0,
            boardRows().map(row => row.name).join(", "));
        KeyboardLayout.toggle();
        expect("no layout to show", !KeyboardLayout.shown, `${KeyboardLayout.shown}`);
        plug(true, pluggedBack);
    }

    function pluggedBack() {
        expect("the board back", KeyboardLayout.present && first("backslash").name
            === "\\ backslash", first("backslash").name);
        KeyboardLayout.toggle();
        expect("its layout shown again", KeyboardLayout.shown && KeyboardLayout.layer === 2,
            `${KeyboardLayout.shown} ${KeyboardLayout.layer}`);
        Qt.callLater(() => Qt.exit(0));
    }

    // Quickshell reads the files in the background, so the cases wait for
    // both, for five seconds at most.
    Timer {
        id: waiting
        interval: 50
        repeat: true
        running: true
        property int left: 100
        onTriggered: {
            if (KeyboardLayout.present && KeyboardLayer.present) {
                stop();
                root.run();
            } else if (--left === 0) {
                stop();
                console.warn("FAIL the files never loaded");
                Qt.exit(1);
            }
        }
    }

    function run() {
        try {
            expect("the layout", KeyboardLayout.present && KeyboardLayout.layers.length === 3,
                `${KeyboardLayout.layers.length} layers`);
            expect("the board's layer", KeyboardLayout.live === 2, `${KeyboardLayout.live}`);

            const slash = first("backslash");
            expect("a symbol by its name", slash.name === "\\ backslash" && slash.side === "Punk"
                && slash.note === "left hand, lower row, little finger",
                `${slash.name}, ${slash.side}, ${slash.note}`);
            const seven = first("keypad 7");
            expect("a keypad key", seven.side === "Numb" && seven.name === "7 keypad 7",
                `${seven.name}, ${seven.side}`);
            const lock = first("lock the screen");
            expect("a key named by its words", lock.name === "Lock the screen"
                && lock.side === "Numb", `${lock.name}, ${lock.side}`);
            const ctrl = first("a");
            expect("a letter with its hold", ctrl.name === "A" && ctrl.side === "Coal, held Ctrl",
                `${ctrl.name}, ${ctrl.side}`);

            expect("an action by what it does", first("flash").name === "Flash the Keyboard",
                first("flash").name);
            // Only showing the layout is listed; locking a layer and its
            // light are the board's Layer's and Light's, and flashing it the
            // ZSA board's own (Keyboards.qml).
            const names = keyboard.everything().map(row => row.name);
            expect("the board's own acts listed", names[0] === "Show the Keyboard's Layout"
                && !names.includes("Flash the Keyboard") && !names.includes("Light")
                && !names.some(name => /^Lock .* on$|^Let .* go$|keyboard's light/.test(name)),
                names.filter(name => /^Lock |^Let |light/.test(name)).join(", "));
            const numbActs = keyboard.search("numb").rows.map(row => row.name);
            const punkActs = keyboard.search("punk").rows.map(row => row.name);
            expect("the locked layer let go", numbActs.includes("Let Numb Go")
                && !numbActs.includes("Lock Numb On") && punkActs.includes("Lock Punk On"),
                numbActs.concat(punkActs).join(", "));
            const lit = ["turn off keyboard", "turn on keyboard", "brighten keyboard",
                "dim keyboard"].map(typed => first(typed).name);
            expect("each light's choice found", lit.join(", ") === "Turn Off the "
                + "Keyboard's Light, Turn on the Keyboard's Light, Brighten the "
                + "Keyboard's Light, Dim the Keyboard's Light", lit.join(", "));
            const brighter = first("brighten keyboard");
            expect("a choice found by its words",
                brighter.name === "Brighten the Keyboard's Light"
                && brighter.verb === "brighten", `${brighter.name}, ${brighter.verb}`);
            // One word each: an old word for the light finds nothing, the
            // glossary's own old one among them.
            const old = keyboard.search("leds dark").rows
                .concat(keyboard.search("backlight").rows);
            expect("an old word finds nothing", old.length === 0,
                old.map(row => row.name).join(", "));
            brighter.run();
            expect("brightening runs ergodox", Niri.spawned.join(" ") === "ergodox brighter",
                Niri.spawned.join(" "));
            const flash = first("flash");
            expect("an act's verb", flash.verb === "flash" && first("keypad 7").verb === "show",
                `${flash.verb}, ${first("keypad 7").verb}`);

            const punk = keyboard.search("punk").rows;
            expect("a layer's keys by its name", punk.length > 20, `${punk.length} rows`);

            // Each carries its layer, so the : line fills :set lock={layer}.
            expect("a layer's act carries its layer", first("lock punk").fill?.layer === "Punk"
                && first("let numb").fill?.layer === "Numb",
                `${first("lock punk").fill?.layer}, ${first("let numb").fill?.layer}`);
            first("lock punk").run();
            expect("locking runs ergodox", Niri.spawned.join(" ") === "ergodox layer 1",
                Niri.spawned.join(" "));

            const parts = keyboards.parts(board, null);
            const layer = parts.find(part => part.name === "Layer");
            expect("the board's Layer, a setting", layer !== undefined
                && layer.side === "Numb, locked" && layer.verb === "more",
                parts.map(part => `${part.name} ${part.side}`).join(", "));
            expect("the Layer's choices run ergodox", runs(layer.verbs())
                === "Let Numb Go: ergodox layer 0; Lock Punk On: ergodox layer 1",
                runs(layer.verbs()));
            expect("the Layer's choices carry their layer", layer.verbs()
                .map(choice => choice.fill?.layer).join(", ") === "Numb, Punk",
                layer.verbs().map(choice => choice.fill?.layer).join(", "));
            const lights = parts.find(part => part.name === "Light");
            expect("the board's Light, a setting", lights !== undefined
                && lights.verb === "more", parts.map(part => part.name).join(", "));
            expect("the Light's choices run ergodox", runs(lights.verbs())
                === "Light Off: ergodox lights off; Light On: ergodox lights on; "
                + "Brighter: ergodox brighter; Dimmer: ergodox dimmer", runs(lights.verbs()));
            // Flashing is the board's own act, its last part, so stepping
            // in never lands on it.
            const flashed = parts[parts.length - 1];
            expect("the board's Flash, its last part", flashed.name === "Flash the Keyboard"
                && flashed.verb === "flash", parts.map(part => part.name).join(", "));
            flashed.run();
            expect("flashing runs ergodox", Niri.spawned.join(" ").includes("ergodox flash"),
                Niri.spawned.join(" "));

            slash.run();
            expect("a key shown on its layer", KeyboardLayout.shown && KeyboardLayout.layer === 1
                && KeyboardLayout.marked === 41,
                `${KeyboardLayout.shown} ${KeyboardLayout.layer} ${KeyboardLayout.marked}`);
            KeyboardLayout.toggle();
            expect("put away", !KeyboardLayout.shown && KeyboardLayout.marked === -1,
                `${KeyboardLayout.shown} ${KeyboardLayout.marked}`);
            KeyboardLayout.toggle();
            expect("the board's layer shown", KeyboardLayout.shown && KeyboardLayout.layer === 2,
                `${KeyboardLayout.shown} ${KeyboardLayout.layer}`);
            plug(false, unplugged);
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }
}
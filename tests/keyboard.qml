// Drives the launcher's Keyboard collection (modules/quickshell/launcher/
// Keyboard.qml) and the layout it reads (KeyboardLayout.qml) through
// Quickshell itself, over the ErgoDox's own description and a board locked
// on Numb, with niri stood in for: a symbol is found by its name with its
// layer and where it sits, an action by what it does, a layer by its name,
// the locked layer offers to be let go rather than locked again, the
// board's Layer and Lights choices are found by their words though they're
// listed only in the board's own parts (Keyboards.qml), and the overlay
// shows what the launcher chose. It exits 0 once every case holds,
// and otherwise 1, saying which case didn't.
import qs
import Quickshell
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

            expect("an action by what it does", first("flash").name === "Flash the keyboard",
                first("flash").name);
            // Only what's done to the board itself is listed; locking a
            // layer and its lights are the board's Layer's and Lights'.
            const names = keyboard.everything().map(row => row.name);
            expect("the board's own acts listed", names[0] === "Show the keyboard's layout"
                && names[1] === "Flash the keyboard" && !names.includes("Lights")
                && !names.some(name => /^Lock .* on$|^Let .* go$|keyboard's lights/.test(name)),
                names.filter(name => /^Lock |^Let |lights/.test(name)).join(", "));
            const layered = keyboard.search("layer").rows.map(row => row.name);
            expect("the locked layer let go", layered.includes("Let Numb go")
                && !layered.includes("Lock Numb on") && layered.includes("Lock Punk on"),
                layered.join(", "));
            const lit = ["turn off keyboard", "turn on keyboard", "brighten keyboard",
                "dim keyboard"].map(typed => first(typed).name);
            expect("each light's choice found", lit.join(", ") === "Turn off the "
                + "keyboard's lights, Turn on the keyboard's lights, Brighten the keyboard's "
                + "lights, Dim the keyboard's lights", lit.join(", "));
            const brighter = first("brighten keyboard");
            expect("a choice found by its words", brighter.name === "Brighten the keyboard's lights"
                && brighter.verb === "brighten", `${brighter.name}, ${brighter.verb}`);
            expect("a choice found by the lights' words",
                first("backlight dark").name === "Turn off the keyboard's lights",
                first("backlight dark").name);
            brighter.run();
            expect("brightening runs ergodox", Niri.spawned.join(" ") === "ergodox brighter",
                Niri.spawned.join(" "));
            const flash = first("flash");
            expect("an act's verb", flash.verb === "flash" && first("keypad 7").verb === "show",
                `${flash.verb}, ${first("keypad 7").verb}`);

            const punk = keyboard.search("punk").rows;
            expect("a layer's keys by its name", punk.length > 20, `${punk.length} rows`);

            first("lock punk").run();
            expect("locking runs ergodox", Niri.spawned.join(" ") === "ergodox layer 1",
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
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }
}

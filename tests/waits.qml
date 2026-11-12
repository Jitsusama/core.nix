// Drives the launcher's Lag (modules/quickshell/launcher/Waits.qml) through
// Quickshell itself, over lag-watch's status file as it writes one, which
// the machine daemon tells the shell over a machine of the check's own
// (MACHINE_ROOT): while
// the processor held the desktop up, "lag" and "slow" say so, the cause
// first, as Machine shows it, then lag-watch's words, then
// offers to show what's using the processor, which goes there from the top,
// then says how fast the desktop answers in plain words, and how hard heavy
// work runs, which goes to the queue; with nothing in the way, it says
// nothing's slowing the desktop and offers nothing to look into. It exits 0
// once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    // The launcher as Lag's acts reach it: where they leave it.
    QtObject {
        id: theLauncher

        property var held: [{ row: {} }]
        property string typed: ""

        function restart(text) {
            typed = text;
        }
    }

    Launcher.Waits {
        id: waits

        launcher: theLauncher
        settings: ({ name: "waits" })
    }

    // Writes lag-watch's status as it does, whole, then goes on with what
    // follows once the daemon has told it.
    Process {
        id: writer
    }

    // What follows a write, and the end of the window it waits to be told.
    property var then: null
    property real ended: 0

    Connections {
        target: Watchers

        function onLagChanged() {
            if (root.then !== null && Watchers.lag?.ended === root.ended) {
                const then = root.then;
                root.then = null;
                Qt.callLater(() => root.run(then));
            }
        }
    }

    function write(window, then) {
        const text = said(window);
        root.ended = JSON.parse(text).ended;
        root.then = then;
        writer.command = ["sh", "-c", 'mkdir -p "${2%/*}" && printf %s "$1" > "$2.tmp"'
            + ' && mv "$2.tmp" "$2"', "write", text,
            `${Quickshell.env("MACHINE_ROOT")}/run/user/1000/lag-watch/status.json`];
        writer.running = true;
    }

    function run(step) {
        try {
            step();
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }

    function said(window) {
        // Each window ends later than the one before, so each is told.
        return JSON.stringify(Object.assign({
            ended: Math.max(Date.now() / 1000, root.ended + 1),
            seconds: 10,
            band: "busy",
            niri: { p50_ms: 0.3, p99_ms: 0.4 },
            trouble: [],
            worst: "",
            who: ""
        }, window));
    }

    readonly property string line: "the desktop waited for a CPU 3.0% of the time"

    function slowed() {
        const slow = waits.answer("lag");
        expect("lag is answered", slow !== null, "nothing");
        expect("what the worst wait was on leads", slow.headline === "slowed by the processor",
            slow.headline);
        expect("slow asks it too", waits.answer("slow")?.headline === slow.headline,
            String(waits.answer("slow")?.headline));
        expect("then what to do about it, how fast the desktop answers, and heavy work",
            names(slow.rows) === `${line}, Show What's Using the Processor, Desktop Answers, `
                + "Heavy Work", names(slow.rows));
        expect("the desktop's answers in plain words",
            slow.rows[2].side === "in 0.3 ms, at worst 0.4 ms", slow.rows[2].side);
        expect("heavy work says how hard it runs", slow.rows[3].side === "busy",
            slow.rows[3].side);
        slow.rows[1].run();
        expect("showing goes to the processor from the top",
            theLauncher.held.length === 0 && theLauncher.typed === "processor",
            `${theLauncher.held.length} ${theLauncher.typed}`);
        expect("and keeps the launcher there", slow.rows[1].stays === true,
            String(slow.rows[1].stays));
        slow.rows[3].run();
        expect("heavy work goes to the queue", theLauncher.typed === "queue", theLauncher.typed);
        write({}, calmed);
    }

    function calmed() {
        const calm = waits.answer("lag");
        expect("nothing in the way says so", calm.headline === "nothing's slowing the desktop",
            calm.headline);
        expect("and offers nothing to look into",
            names(calm.rows) === "Desktop Answers, Heavy Work", names(calm.rows));
        expect("an old word asks nothing", waits.answer("jank") === null, "it answered");
        Qt.callLater(() => Qt.exit(0));
    }

    Component.onCompleted: write({ trouble: [line], worst: "cpu", who: "desktop" }, slowed)

    // Nothing here waits forever: a case that never comes fails.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.run(() => root.expect("the cases ran", false, "they never finished"))
    }
}
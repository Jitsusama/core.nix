// Drives the launcher's Lag (modules/quickshell/launcher/Waits.qml) through
// Quickshell itself, over lag-watch's status file as it writes one: while
// the processor held the desktop up, "lag" says so in lag-watch's words, then
// offers to see what's using the processor, which goes there from the top,
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
        id: launcher

        property var held: [{ row: {} }]
        property string typed: ""

        function restart(text) {
            typed = text;
        }
    }

    Launcher.Waits {
        id: waits

        launcher: launcher
        settings: ({ name: "waits" })
    }

    // Writes lag-watch's status as it does, whole, then goes on with what
    // follows once it's on the disk.
    Process {
        id: writer

        property var then: null

        onExited: Qt.callLater(() => root.run(writer.then))
    }

    function write(window, then) {
        writer.then = then;
        writer.command = ["sh", "-c", 'printf %s "$1" > "$2.tmp" && mv "$2.tmp" "$2"', "write",
            said(window), `${Quickshell.env("XDG_RUNTIME_DIR")}/lag-watch/status.json`];
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
        return JSON.stringify(Object.assign({
            ended: Date.now() / 1000,
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
        expect("the worst of what waited leads", slow.headline === line, slow.headline);
        expect("then what to do about it, how fast the desktop answers, and heavy work",
            names(slow.rows) === `${line}, See What's Using the Processor, Desktop Answers, `
                + "Heavy Work", names(slow.rows));
        expect("the desktop's answers in plain words",
            slow.rows[2].side === "in 0.3 ms, at worst 0.4 ms", slow.rows[2].side);
        expect("heavy work says how hard it runs", slow.rows[3].side === "busy",
            slow.rows[3].side);
        slow.rows[1].run();
        expect("seeing goes to the processor from the top",
            launcher.held.length === 0 && launcher.typed === "processor",
            `${launcher.held.length} ${launcher.typed}`);
        expect("and keeps the launcher there", slow.rows[1].stays === true,
            String(slow.rows[1].stays));
        slow.rows[3].run();
        expect("heavy work goes to the queue", launcher.typed === "queue", launcher.typed);
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
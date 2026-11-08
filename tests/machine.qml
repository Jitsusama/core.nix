// Drives the launcher's Machine (modules/quickshell/launcher/Machine.qml)
// through Quickshell itself, with the machine daemon (modules/machine)
// running over the sandbox's own /proc: Machine's place holds the
// processors' place before the daemon answers, and asks it; then the
// processors', memory's and uptime's headlines are the daemon's, and
// make none of their parts; the programs using the machine aren't asked
// for until the processor's parts are stepped into, and then are. It exits 0
// once every case holds, and otherwise 1, saying which case didn't.
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    // The launcher as Machine reads it: open, showing what `answer` is.
    QtObject {
        id: theLauncher

        property bool open: true
        property var answer: null
        property var held: []
        property var matches: []
        property var placed: []
    }

    Launcher.Machine {
        id: machine

        launcher: theLauncher
    }

    // Machine's place stepped into, before the daemon has said anything,
    // since Machine asks it only while one of its answers is shown, and a
    // place is made once: Processor holds its place there with the parts
    // to come, and holding it asks the daemon. Power's Kept From Resting
    // is the telemetry's, so a place holding it alone asks nothing.
    Component.onCompleted: {
        try {
            theLauncher.placed = ["energy", "machine:cpu:kept-from-resting"];
            expect("Power's place asks the daemon nothing", !machine.shown, "it did");
            const before = machine.answer("processor");
            expect("before the daemon says, Processor waits with its parts to come",
                before.parted === true && before.asking === true,
                `${before.parted} ${before.asking}`);
            theLauncher.placed = ["energy", "machine:cpu", "machine:memory", "heat"];
            expect("Machine's place asks the daemon", machine.shown, "it didn't");
            waiting.start();
        } catch (error) {
            fail(error);
        }
    }

    function fail(error) {
        console.warn(`FAIL ${error.message ?? error}`);
        Qt.callLater(() => Qt.exit(1));
    }

    property var processors: null

    // Asks again every tenth of a second, for at most ten seconds, until
    // `ready` holds, then runs `then`.
    Timer {
        id: waiting

        property int left: 100

        interval: 100
        repeat: true
        onTriggered: {
            if (machine.processor() === null || machine.record("memory:main") === null
                || machine.bootRecord() === null) {
                if (--left > 0)
                    return;
                stop();
                root.fail(new Error("the daemon never said the processor, memory and boot"));
                return;
            }
            stop();
            root.headlines();
        }
    }

    function headlines() {
        try {
            processors = machine.answer("processor");
            const memory = machine.answer("memory");
            const uptime = machine.answer("uptime");
            expect("the processors' headline", /^\d+% busy$/.test(processors.headline),
                processors.headline);
            expect("the memory's headline", / used, \d+%$/.test(memory.headline), memory.headline);
            expect("the uptime's headline", /^up /.test(uptime.headline), uptime.headline);
            expect("both say they have parts", processors.parted && memory.parted,
                `${processors.parted} ${memory.parted}`);
            expect("the headlines asked for no programs", !machine.usingWanted, "they did");
            // Stepped into, its parts ask for the programs.
            theLauncher.held = [{ row: processors }];
            const parts = processors.rows.map(row => row.name);
            expect("the processors' parts", parts.includes("Cores") && parts.includes("Time Spent"),
                parts.join(", "));
            programs.start();
        } catch (error) {
            fail(error);
        }
    }

    Timer {
        id: programs

        property int left: 100

        interval: 100
        repeat: true
        onTriggered: {
            const users = machine.using("cpu");
            if (users === null || users.length === 0) {
                if (--left > 0)
                    return;
                stop();
                root.fail(new Error(`the programs were never asked for: ${machine.usingWanted}`));
                return;
            }
            stop();
            try {
                const parts = machine.answer("processor").rows;
                const using = parts.find(row => row.name === "Using It");
                root.expect("the programs using it", using !== undefined && using.side !== "\u2026",
                    parts.map(row => `${row.name} ${row.side}`).join(", "));
                Qt.callLater(() => Qt.exit(0));
            } catch (error) {
                root.fail(error);
            }
        }
    }
}

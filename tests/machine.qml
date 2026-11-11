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
        // What a terminal's `launcher show` sees, as Launcher.qml starts it,
        // so Machine's bindings read it rather than throwing on its absence.
        property var terminalSees: ({ walked: [], listed: [] })

        // No frame is drawn here, so what waits on one runs at once.
        function afterFrame(then) {
            then();
        }
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
            // While the daemon is down, its place says so plainly, never
            // naming a daemon (VKCF3Z's words, Y2YBTS's ruling).
            expect("a daemon that's down is said plainly",
                machine.says("cpu", "down") === "not answering", machine.says("cpu", "down"));
            // A time is a measure, by the launcher's one rule: three
            // significant digits, zeros kept, grouped past three whole
            // digits, in the machine's own grouping and point.
            const times = [1234, 0.25, 12.34].map(value => machine.ms(value));
            expect("a time in milliseconds", times.join("; ") === "1,230 ms; 0.250 ms; 12.3 ms",
                times.join("; "));
            machine.settings = { group: ".", point: "," };
            expect("a time as the settings group and point it", machine.ms(1234.5) === "1.230 ms"
                && machine.ms(5.2) === "5,20 ms", `${machine.ms(1234.5)} ${machine.ms(5.2)}`);
            machine.settings = {};
            // A short disk's drawer doesn't say its room again, since it's
            // beside it, and says its size and how long it has; a healthy
            // one, its room beside it alone, has no drawer.
            const guard = { shown: bytes => `${bytes} B`, duration: () => "1 h",
                banded: band => band };
            const short = machine.disked(guard, { path: "/nix", band: "filling", free: 7,
                total: 9, full_at: Date.now() / 1000 + 3600, says: "" }, { name: "/nix" });
            const facts = (short.drawer?.facts ?? []).map(fact => `${fact.label}=${fact.value}`);
            expect("a short disk's drawer says its size and time, its room beside it",
                short.side === "7 B free \u00b7 clearing old builds"
                && facts.join("; ") === "Size=9 B; Full In=1 h", `${short.side}; ${facts}`);
            const healthy = machine.disked(guard, { path: "/", band: "healthy", free: 7,
                total: 9 }, { name: "/" });
            expect("a healthy disk says its room beside it, with no drawer",
                healthy.side === "7 B free" && !healthy.drawer, JSON.stringify(healthy.drawer));
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

// Drives the launcher's Machine (modules/quickshell/launcher/Machine.qml)
// through Quickshell itself, with the machine daemon (modules/machine)
// running over the sandbox's own /proc: Machine's place holds the
// processors' place before the daemon answers, and asks it; then the
// processors', memory's and uptime's headlines are the daemon's, and
// make none of their parts, each from the rows it serves ready; the
// programs using the machine aren't asked for until the processor's
// parts are stepped into, and then are, the processor's and the
// memory's served Using It naming them once they're read; a place
// holding Disk asks for the disk's own place, and nothing else does (its
// rows are proven in modules/machine/tests/daemon.rs, since the sandbox
// has no disk to read). It exits 0
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
        // Quests as Machine reads it, saying which ids it was asked to title.
        property var sources: [theQuests]

        // No frame is drawn here, so what waits on one runs at once.
        function afterFrame(then) {
            then();
        }
    }

    QtObject {
        id: theQuests

        readonly property var settings: ({ name: "quests" })
        property var asked: []
        signal titlesHeard

        function titleOf(id) {
            asked = asked.concat([id]);
            return undefined;
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
    // is listed whole while it waits, and asked once it's listed, so a
    // place holding it alone asks nothing.
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
            // Disk's Space speaks for / while every disk is in one band, as
            // the disk piece does (#3768): on 12 November it said the 47 GB
            // build disk's 10.9 GB free while / had over 300. A disk in a
            // worse band still leads.
            const bands = ["healthy", "filling"];
            const guarding = disks => Object.assign({}, guard, { disks, holders: [],
                worse: (one, other) => bands.indexOf(one.band) > bands.indexOf(other.band) });
            const build = band => ({ path: "/var/cache/cargo-builds", band, free: 10, total: 47 });
            const root = { path: "/", band: "healthy", free: 300, total: 1000 };
            const space = disks => machine.guardParts(guarding(disks))[0];
            const even = space([build("healthy"), root]);
            expect("Space says the room on / while every disk has room",
                even.side === "300 B free" && even.note === "on /, one of 2",
                `${even.side}; ${even.note}`);
            const filling = space([root, build("filling")]);
            expect("a disk in a worse band speaks for the rest",
                filling.side === "10 B free"
                && filling.note === "on /var/cache/cargo-builds, the least of 2",
                `${filling.side}; ${filling.note}`);
            // A quest filling the disk is named to Quests as the guard's
            // holders arrive, so its title is known before Machine opens
            // (VKCF3Z, 2026-11-13); a worktree holder is no quest.
            theQuests.asked = [];
            machine.titleHolders({ holders: [
                { owner: { kind: "quest", name: "QEST-20250105-BANNER" } },
                { owner: { kind: "worktree", name: "worktree-a" } }] });
            expect("a quest holding the disk is titled ahead",
                JSON.stringify(theQuests.asked) === '["QEST-20250105-BANNER"]',
                JSON.stringify(theQuests.asked));
            theLauncher.placed = ["energy", "machine:cpu", "machine:memory", "heat"];
            expect("Machine's place asks the daemon", machine.shown, "it didn't");
            expect("Machine's place asks nothing of the disks", !machine.disksWanted, "it did");
            theLauncher.placed = theLauncher.placed.concat(["machine:disk"]);
            expect("a place holding Disk asks for the disks", machine.disksWanted, "it didn't");
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
            if (machine.cpu().asking || machine.memory().asking || machine.uptime().asking) {
                if (--left > 0)
                    return;
                stop();
                root.fail(new Error("the daemon never said the processor, memory and uptime"));
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
            expect("the uptime's headline", /^up \d+ (d|h|min)\b/.test(uptime.headline),
                uptime.headline);
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
            // The daemon makes the processor's Using It again once the
            // programs are read, naming them rather than still reading.
            const parts = machine.answer("processor").rows;
            const using = parts.find(row => row.name === "Using It");
            if (using === undefined || using.side === "\u2026") {
                if (--left > 0)
                    return;
                stop();
                root.fail(new Error(`the programs were never asked for: ${machine.usingWanted}; `
                    + parts.map(row => `${row.name} ${row.side}`).join(", ")));
                return;
            }
            stop();
            try {
                const load = parts.find(row => row.name === "Load");
                root.expect("the processor's load, served",
                    load !== undefined && /^\d/.test(load.side),
                    parts.map(row => `${row.name} ${row.side}`).join(", "));
                root.servedParts();
            } catch (error) {
                root.fail(error);
            }
        }
    }

    // The memory's and the uptime's parts are the rows the daemon serves
    // inside theirs, in their words' order, the uptime's start a moment
    // worded here.
    function servedParts() {
        try {
            const memory = machine.memory().rows.map(row => row.name);
            expect("the memory's parts, used first", memory[0] === "Used"
                && memory.includes("Every Count"), memory.join(", "));
            const uptime = machine.uptime().rows;
            const started = uptime.find(row => row.name === "Started");
            expect("the uptime's start, said as a moment",
                started !== undefined && /\d\d:\d\d$/.test(started.side),
                uptime.map(row => `${row.name} ${row.side}`).join(", "));
            servedUsing.start();
        } catch (error) {
            fail(error);
        }
    }

    // The programs read, the daemon makes the memory's Using It again
    // naming them; it's waited for, as the place is remade as they're
    // read.
    Timer {
        id: servedUsing

        property int left: 100

        interval: 100
        repeat: true
        onTriggered: {
            const using = machine.memory().rows.find(row => row.name === "Using It");
            if (using === undefined || using.side === "\u2026") {
                if (--left > 0)
                    return;
                stop();
                root.fail(new Error(`the memory's Using It never named them: ${using?.side}`));
                return;
            }
            stop();
            Qt.callLater(() => Qt.exit(0));
        }
    }
}

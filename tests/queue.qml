// Drives the launcher's queue (modules/quickshell/launcher/Queue.qml)
// through Quickshell itself, with the machine's queue, pi's sessions, niri,
// Quests and a slice's memory stood in. Its row says how many run and wait,
// and the machine's room below; the running jobs come first, then those in
// line; each is named in words rather than a check's identifier, wears the
// stage it's at, says how long it's run against its last run or what it
// waits for, and below it its quest, kind, memory held or needed and
// marks; Return goes to its session's window, or copies its name with
// none; a step in lists its quest, session, command and the rest, the
// quest opening as Quests opens it; typing finds a job by its name or its
// quest; and a queue that stopped saying says so. It exits 0 once every
// case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property real now: Date.now()
    property bool questOpened: false

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    function ago(minutes) {
        return new Date(root.now - minutes * 60000 - 5000).toISOString();
    }

    // Quests' own row for the quest, whose Return the queue's reuses.
    QtObject {
        id: quests

        readonly property var settings: ({ name: "quests" })
        readonly property var all: [{
            key: "quest:QEST-1",
            name: "Make Tea",
            verb: "open",
            run: () => {
                root.questOpened = true;
            }
        }]
    }

    QtObject {
        id: launcher

        property bool open: true
        readonly property var sources: [quests]
    }

    Launcher.Queue {
        id: queue

        launcher: launcher
        settings: ({ name: "jobs" })
        cgroups: Quickshell.env("CGROUPS")
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail("never came: the cases")
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    Component.onCompleted: {
        Jobs.said = {
            v: 1,
            machine: { availableGib: 3, pressure: { cpu: 12.5 } },
            wasted: { today: 4 },
            jobs: [
                { id: "w2", kind: "vm-test", state: "waiting", place: 2, owner: { session: "gone" },
                    since: ago(1), what: "nix build .#checks.x86_64-linux.vm-desktop-launcher-answers",
                    name: "vm-desktop-launcher-answers" },
                { id: "r1", kind: "build", class: "bulk", state: "running", owner: { session: "s1" },
                    since: ago(21), lastRanS: 1500, memoryGib: 6, slice: "/work.scope",
                    quest: "QEST-1", capped: true, what: "nix build .#lighthouse", name: "lighthouse" },
                { id: "w1", kind: "vm-test", state: "waiting", place: 1, owner: { session: "s1" },
                    since: ago(2), memoryGib: 6, waitingOn: "memory", quest: "QEST-1", first: true,
                    what: "nix build .#checks.x86_64-linux.vm-zathura-opens-a-pdf",
                    name: "vm-zathura-opens-a-pdf" },
                { id: "r2", kind: "vm-check", state: "running", owner: { session: "gone" },
                    since: ago(30), lastRanS: 1500, what: "changed-checks",
                    name: "a-theme-changes-everything and 57 more" },
                { id: "r3", kind: "build", state: "running", owner: { session: "gone" },
                    since: ago(6), what: "changed-checks --no-vm", name: "x86_64-linux" },
                { id: "w3", kind: "vm-test", state: "waiting", place: 3, owner: { session: "gone" },
                    since: ago(1), waitingOn: "behind", what: "nix build .#checks.x86_64-linux.vm-x" },
                { id: "w4", kind: "benchmark", state: "waiting", place: 4, owner: { session: "gone" },
                    since: ago(1), waitingOn: "vms", what: "bench", name: "changed-checks" }
            ]
        };
        // As the minute turns over, once the queue's written.
        queue.tick();
        Qt.callLater(root.cases);
    }

    function cases() {
        try {
            expect("its row says how many run and wait", queue.counted() === "3 running, 4 waiting",
                queue.counted());
            expect("and below it, the machine's room",
                queue.noted() === "3 GiB to spare \u00b7 the processors 13% held up"
                + " \u00b7 4 min waited with room today", queue.noted());

            const all = queue.everything();
            expect("running first, then in line, each named in words",
                names(all) === "Lighthouse, A Theme Changes Everything, Checks, Zathura Opens a PDF,"
                + " Desktop Launcher Answers, VM Test, Changed Checks", names(all));
            const [house, theme, checks, zathura, answers, unnamed, bench] = all;

            expect("a running job wears the active stage", house.glyph === Theme.glyph.active,
                house.glyph);
            expect("a waiting one wears queued", zathura.glyph === Theme.glyph.queued, zathura.glyph);
            expect("how long it's run against its last run", house.side === "21 of 25 min", house.side);
            expect("past its last run, by how much", theme.side === "5 min over", theme.side);
            expect("with no run before, for how long", checks.side === "for 6 min", checks.side);
            expect("what a waiting one waits for", zathura.side === "waits for memory", zathura.side);
            expect("behind a VM test", unnamed.side === "behind a VM test", unnamed.side);
            expect("for a VM", bench.side === "waits for a VM", bench.side);
            expect("its place, waiting only its turn", answers.side === "2nd in line", answers.side);

            expect("below a running one: its quest, kind, memory held and marks",
                house.note === "Make Tea \u00b7 build \u00b7 holds 5.8 GiB \u00b7 capped", house.note);
            expect("below a waiting one: what it needs, and put first",
                zathura.note === "Make Tea \u00b7 VM test \u00b7 needs 6 GiB \u00b7 put first",
                zathura.note);
            expect("how many more a list runs with",
                theme.note === "and 57 more \u00b7 VM check", theme.note);

            expect("Return goes to its session's window", house.verb === "go", house.verb);
            house.run();
            expect("that window", JSON.stringify(Niri.acted) === "{\"FocusWindow\":{\"id\":7}}",
                JSON.stringify(Niri.acted));
            expect("with no window, Return copies its name", answers.verb === "copy", answers.verb);

            const parts = house.verbs();
            expect("a step in says what the queue knows of it",
                names(parts) === "Quest, Session, Command, Class, Needs, Holds, Since, Last Run",
                names(parts));
            expect("its quest by its title", parts[0].side === "Make Tea", parts[0].side);
            parts[0].run();
            expect("opened as Quests opens it", root.questOpened, "not opened");
            expect("its session by its address", parts[1].side === "QUEUE1", parts[1].side);
            expect("the command, copied", parts[2].side === "nix build .#lighthouse"
                && parts[2].verb === "copy", `${parts[2].side} ${parts[2].verb}`);
            expect("what it needs and holds", parts[4].side === "6 GiB" && parts[5].side === "5.8 GiB",
                `${parts[4].side} ${parts[5].side}`);
            expect("its last run", parts[7].side === "25 min", parts[7].side);
            expect("a waiting one holds nothing yet",
                !names(zathura.verbs()).includes("Holds"), names(zathura.verbs()));

            const found = queue.search("zathura").rows;
            expect("found by its name", found.length === 1 && found[0] === zathura, names(found));
            const byQuest = queue.search("make tea").rows;
            expect("found by its quest", names(byQuest) === "Lighthouse, Zathura Opens a PDF",
                names(byQuest));

            Jobs.said = null;
            expect("a queue that stopped saying says so",
                queue.counted() === "the queue has stopped saying", queue.counted());
        } catch (error) {
            root.fail(error.message);
            return;
        }
        console.log("the queue says what each job is");
        Qt.callLater(() => Qt.exit(0));
    }
}

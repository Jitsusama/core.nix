pragma ComponentBehavior: Bound
// Drives the launcher's Heat (modules/quickshell/launcher/Heat.qml)
// through Quickshell itself, over a stand-in for the machine daemon
// serving a processor's chip and a fan's, with the launcher stood in,
// open, "heat" typed. Heat's answer asked again before the daemon reads
// again, as each key typed asks it, makes no row; a change to the fan's
// chip alone makes the fan's chip and its row again and keeps the
// processor's; and a chip let go is gone. It exits 0 once every case
// holds, and otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the first reading"

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        root.waiting = "a failure";
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    // The launcher as Heat reads it: open, "heat" typed, so Heat's rows
    // can be seen and its daemon is watched.
    QtObject {
        id: theLauncher

        property bool open: true
        property string typed: "heat"
        property var answer: null
        property var held: []
        property var matches: []
        property var placed: []

        // No frame is drawn here, so what waits on one runs at once.
        function afterFrame(then) {
            then();
        }
    }

    // A chip's record as the machine daemon says it: a processor's
    // package, and a fan's chip turning at `rpm`.
    readonly property string processorKey: "sensor:coretemp-isa-0000"
    readonly property string fanKey: "sensor:dell_smm-isa-0000"
    readonly property var processor: ({
            key: root.processorKey,
            kind: "sensor",
            part: "processor",
            chip: "coretemp",
            temperatures: [{ n: 1, label: "Package id 0", temperature_c: 61, high_c: 100,
                    critical_c: 100 }],
            fans: []
        })
    function fan(rpm) {
        return { key: root.fanKey, kind: "sensor", part: "", chip: "dell_smm", temperatures: [],
            fans: [{ n: 1, label: "CPU Fan", rpm: rpm }] };
    }

    // The shell's watch, so the daemon can tell it more.
    property var watch: null

    function tell(change) {
        root.watch.socket.write(`${JSON.stringify(Object.assign({ id: root.watch.id },
                change))}\n`);
        root.watch.socket.flush();
    }

    // A stand-in for the machine daemon, speaking the daemon interface:
    // hello, and a watch of Heat's kinds answered whole.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "machine",
                            state: "running" };
                    } else if (ask.ask === "watch") {
                        root.watch = { socket: asked, id: ask.id };
                        answer = { id: ask.id, records: [root.processor, root.fan(2400)] };
                    } else {
                        answer = { id: ask.id, ok: true };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    property var before: []
    property var chipsBefore: ({})
    property int counted: 0

    // Heat says it answered each time the daemon tells it something while
    // it shows, and a moment after a first read; each case waits for what
    // it needs to have been heard.
    Launcher.Heat {
        id: theHeat

        launcher: theLauncher
        settings: ({ name: "heat" })
        onAnswered: {
            try {
                if (root.waiting === "the first reading")
                    root.first();
                else if (root.waiting === "the fan's change")
                    root.fanChanged();
                else if (root.waiting === "the fan let go")
                    root.fanGone();
            } catch (error) {
                root.fail(error.message);
            }
        }
    }

    // Asked again with nothing read since, Heat makes no row, where each
    // key typed while it shows made every row again.
    function first() {
        const answer = theHeat.answer("heat");
        if (answer === null || answer.rows.length < 2)
            return;
        root.waiting = "the fan's change";
        const made = theHeat.tally.rows;
        const again = theHeat.answer("heat");
        expect("asked again with nothing read, no row made", theHeat.tally.rows === made,
            `${theHeat.tally.rows - made} rows`);
        expect("asked again, the same rows", again.rows.every((row, at) => row
            === answer.rows[at]), "made anew");
        root.before = answer.rows;
        root.chipsBefore = Object.assign({}, theHeat.chipsMade);
        expect("each chip made by its key", Object.keys(root.chipsBefore).length === 2,
            Object.keys(root.chipsBefore).join(", "));
        root.counted = made;
        // Shown, so what the daemon tells is answered at once.
        theLauncher.answer = answer;
        root.tell({ changed: [root.fan(3100)] });
    }

    // The daemon tells the fan's chip anew: the processor's chip and row
    // are kept and the fan's made again.
    function fanChanged() {
        const fan = theHeat.chipsMade[root.fanKey];
        if (!fan || fan.fans[0].value !== 3100)
            return;
        root.waiting = "the fan let go";
        expect("the processor's chip kept", theHeat.chipsMade[root.processorKey]
            === root.chipsBefore[root.processorKey], "made anew");
        theHeat.reads = null;
        const answer = theHeat.answer("heat");
        expect("the processor's row kept", answer.rows[0] === root.before[0],
            answer.rows[0]?.name);
        const fanRow = answer.rows[1];
        expect("the fan's row made again, saying its speed", fanRow !== root.before[1]
            && fanRow.side.includes("3,100"), fanRow?.side);
        expect("one row made", theHeat.tally.rows - root.counted === 1,
            `${theHeat.tally.rows - root.counted} rows`);
        root.tell({ gone: [root.fanKey] });
    }

    // The fan's chip let go, its row goes with it.
    function fanGone() {
        if (root.fanKey in theHeat.chipsMade)
            return;
        root.waiting = "the end";
        theHeat.reads = null;
        const answer = theHeat.answer("heat");
        expect("the fan gone", answer.rows.length === 1 && answer.rows[0] === root.before[0],
            answer.rows.map(row => row.name));
        Qt.callLater(() => Qt.exit(0));
    }
}

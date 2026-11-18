pragma ComponentBehavior: Bound
// Drives the launcher's Heat (modules/quickshell/launcher/Heat.qml)
// through Quickshell itself, over a stand-in for the machine daemon
// serving Heat's rows and headline ready (the ready-row form), with the
// launcher stood in, open, "heat" typed. Heat answers the headline and its
// own rows as served, in their order, each holding the rows inside it a
// step in; its answer asked again, as each key typed asks it, makes no
// row; a change to the fan's row makes it alone again and keeps the
// processor's; the processor's row told again with only its order moved
// moves and is made no more; Throttling asked alone answers its own rows;
// and a row let go is gone. It exits 0 once every case holds, and otherwise 1, saying
// which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect
import "Watch.js" as Watch
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the first rows"

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
        property var entered: null

        function enter(row) {
            entered = row;
        }

        // No frame is drawn here, so what waits on one runs at once.
        function afterFrame(then) {
            then();
        }
    }

    // A row as the machine daemon serves it, in the crate's wrap.
    function served(id, order, fields) {
        const key = `row:heat:${id}`;
        return { key: key, kind: "row", row: Object.assign({ key: key, thing: `heat:${id}`,
                place: "heat", kind: "figure", order: order, depth: 0, side: [], acts: [] },
            fields) };
    }
    readonly property var processor: served("processor:0", 1, {
            kind: "part",
            name: "Processor",
            glyph: "heat",
            side: [{ said: "71.0 \u00b0C", keep: 3 },
                { said: "29.0 \u00b0C below high", keep: 1, hue: "warning" }],
            graph: { shape: "bar", values: [0.7], warn: false },
            acts: [{ key: "verb:copy", copy: "71.0 \u00b0C" }, { key: "verb:more", step: "rows" }]
        })
    readonly property var lately: served("processor:0/lately", 1, {
            name: "Last Reads",
            parent: "row:heat:processor:0",
            depth: 1,
            side: [{ said: "71.0 \u00b0C" }],
            graph: { shape: "spark", values: [0.5], warn: false },
            acts: [{ key: "verb:copy", copy: "71.0 \u00b0C" }]
        })
    readonly property var throttled: served("throttled", 2, {
            kind: "throttled",
            name: "Throttled",
            side: [{ said: "3,202 times" }],
            acts: [{ key: "verb:copy", copy: "3,202 times" }, { key: "verb:more", step: "rows" }]
        })
    readonly property var cores: served("throttled/cores", 1, {
            name: "Cores",
            parent: "row:heat:throttled",
            depth: 1,
            side: [{ said: "32" }],
            acts: [{ key: "verb:copy", copy: "32" }]
        })
    function fan(rpm) {
        return served("fan:fan:1", 3, {
            kind: "fan",
            name: "Fan 1",
            glyph: "fan",
            side: [{ said: `${rpm} rpm` }],
            acts: [{ key: "verb:copy", copy: `${rpm} rpm` }]
        });
    }
    readonly property var head: ({ key: "head:heat", kind: "head", head: {
                said: "Processor at 71.0 \u00b0C, 1 fan turning",
                graph: { shape: "bar", values: [0.7], warn: false } } })

    // The shell's watch, so the daemon can tell it more.
    property var watch: null
    property var watched: null

    // A stand-in for the machine daemon, speaking the daemon interface:
    // hello, and a watch of Heat's rows and headline answered whole.
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
                        root.watched = ask;
                        answer = { id: ask.id, records: [root.fan(2400), root.lately,
                                root.processor, root.head, root.cores, root.throttled] };
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
        onTriggered: Expect.fail(`never came: ${root.waiting}`)
    }

    property var before: []
    property int counted: 0

    // Heat says it answered as its first rows come, and each time the
    // daemon tells it something while it shows; each case waits for what
    // it needs to have been heard.
    Launcher.Heat {
        id: theHeat

        launcher: theLauncher
        settings: ({ name: "heat" })
        onAnswered: {
            try {
                if (root.waiting === "the first rows")
                    root.first();
                else if (root.waiting === "the fan's change")
                    root.fanChanged();
                else if (root.waiting === "the processor moved")
                    root.processorMoved();
                else if (root.waiting === "the fan let go")
                    root.fanGone();
            } catch (error) {
                root.waiting = "a failure";
                Expect.fail(error.message);
            }
        }
    }

    // Heat's own rows as served, in their order, under the served headline;
    // asked again with nothing told since, it makes no row.
    function first() {
        const answer = theHeat.answer("heat");
        if (answer === null || answer.rows.length < 3)
            return;
        root.waiting = "the fan's change";
        Expect.expect("Heat's rows and headline watched", root.watched.kind.includes("row")
            && root.watched.kind.includes("head") && root.watched.place === "heat",
            JSON.stringify(root.watched));
        Expect.expect("the headline as served", answer.headline
            === "Processor at 71.0 \u00b0C, 1 fan turning" && answer.graph?.shape === "bar",
            answer.headline);
        Expect.expect("its own rows, in the daemon's order", answer.rows.map(row => row.name)
            .join(", ") === "Processor, Throttled, Fan 1", answer.rows.map(row => row.name));
        const part = answer.rows[0];
        Expect.expect("a part says its figure and its limit, the limit warning",
            part.side === "71.0 \u00b0C \u00b7 29.0 \u00b0C below high"
            && part.sideItems?.[1]?.hue === "warning", JSON.stringify(part.sideItems));
        Expect.expect("a part drawn against its most", part.graph?.values?.[0] === 0.7,
            JSON.stringify(part.graph));
        Expect.expect("a part's Return copies and closes", part.copies?.() === "71.0 \u00b0C"
            && part.stays !== true, `${part.copies?.()} ${part.stays}`);
        Expect.expect("a part holds its Last Reads a step in", part.verbs().map(row => row.name)
            .join() === "Last Reads", part.verbs().map(row => row.name));
        const made = theHeat.tally.rows;
        const again = theHeat.answer("heat");
        Expect.expect("asked again with nothing told, no row made", theHeat.tally.rows === made,
            `${theHeat.tally.rows - made} rows`);
        Expect.expect("asked again, the same rows", again.rows.every((row, at) => row
            === answer.rows[at]), "made anew");
        const alone = theHeat.alone("heat:throttled");
        Expect.expect("Throttled alone, its figure the headline and its rows inside",
            alone?.headline === "3,202 times" && alone.rows.map(row => row.name).join()
            === "Cores", JSON.stringify(alone?.headline));
        root.before = answer.rows;
        root.counted = theHeat.tally.rows;
        // Shown, so what the daemon tells is answered at once.
        theLauncher.answer = answer;
        Watch.tell(root.watch, { changed: [root.fan(3100)] });
    }

    // The daemon tells the fan's row anew: the processor's row is kept and
    // the fan's made again, saying its speed.
    function fanChanged() {
        const answer = theHeat.answer("heat");
        const fanRow = answer.rows[2];
        if (!fanRow || !fanRow.side.includes("3100"))
            return;
        root.waiting = "the processor moved";
        Expect.expect("the processor's row kept", answer.rows[0] === root.before[0],
            answer.rows[0]?.name);
        Expect.expect("the fan's row made again", fanRow !== root.before[2], fanRow.side);
        Expect.expect("one row made", theHeat.tally.rows - root.counted === 1,
            `${theHeat.tally.rows - root.counted} rows`);
        root.counted = theHeat.tally.rows;
        Watch.tell(root.watch, { changed: [{ key: root.processor.key, kind: "row",
                    row: Object.assign({}, root.processor.row, { order: 4 }) }] });
    }

    // The daemon makes room and tells the processor's row again with only
    // its order moved, as ready-rows' spacing does: it moves, and what was
    // made of it is kept (Sound's 103 rows made again, 2026-11-11).
    function processorMoved() {
        const answer = theHeat.answer("heat");
        if (answer.rows[2]?.name !== "Processor")
            return;
        root.waiting = "the fan let go";
        Expect.expect("moved after the fan", answer.rows.map(row => row.name).join(", ")
            === "Throttled, Fan 1, Processor", answer.rows.map(row => row.name));
        Expect.expect("the processor's row kept as it moved", answer.rows[2] === root.before[0],
            "made anew");
        Expect.expect("and is made no more", theHeat.tally.rows === root.counted,
            `${theHeat.tally.rows - root.counted} rows`);
        Watch.tell(root.watch, { gone: [root.fan(3100).key] });
    }

    // The fan's row let go, it's gone.
    function fanGone() {
        const answer = theHeat.answer("heat");
        if (answer.rows.length !== 2)
            return;
        root.waiting = "the end";
        Expect.expect("the fan gone, the rest kept", answer.rows[0] === root.before[1]
            && answer.rows[1] === root.before[0], answer.rows.map(row => row.name));
        Qt.callLater(() => Qt.exit(0));
    }
}

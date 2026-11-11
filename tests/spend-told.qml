// Drives the launcher's collection of AI spend (launcher/Spend.qml) told by
// spend-watch's socket rather than read from its file, through Quickshell
// itself, over records cut from the collection spend-watch wrote beside
// this, with a stand-in serving them as the daemon does. The socket's first
// answer, saying what the collection and budgets files said with its keys
// in another order, makes no row and leaves the budgets as they were, and
// while the daemon is starting its figures are still being
// read, said as its last run's, until it says it's answering; the file
// isn't watched while the socket answers; a change to a session's
// figure makes none of the rows and a change to one row makes that row
// alone; going to a finding's session and marking it expected are asked
// of the socket, spawning nothing; and a watch that never answers is drawn from what's held while
// it's still reading, with no row made twice. It exits 0 once every case
// holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "SpendWatch.js" as Records
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string collected: realCollection.text()

    Launcher.Spend {
        id: spend
        settings: ({ name: "spend" })
    }

    // The launcher as a source asks it whether its rows are wanted and
    // whether it's open, which decides what the shell watches (Provider.qml's
    // wanted).
    QtObject {
        id: launcher

        property bool open: true

        function wants(source) {
            return true;
        }

        function afterFrame(then) {
            then();
        }
    }

    Launcher.Spend {
        id: wanted
        settings: ({ name: "spend" })
        launcher: launcher
    }

    FileView {
        id: realCollection
        path: `${root.runtime}/written/collection.json`
    }

    FileView {
        id: realBudgets
        path: `${root.runtime}/written/budgets.json`
    }

    // spend-watch's socket as its daemon serves it (its src/interface.rs):
    // each watch told its kinds whole, and then only what changed, as
    // `tell` says it. A kind in `silent` is never answered, as a daemon too
    // slow to answer leaves it.
    property var served: ({})
    property var silent: []
    // The acts asked, each as its act and its row's key.
    property var acts: []
    readonly property var watchers: ({})
    // What hello says, as the daemon says "starting" until its first pass.
    property string saying: "answering"

    // A value with every object's keys in the other order, as serde's
    // sorted keys are against the order the file was written in.
    function turned(value) {
        if (typeof value !== "object" || value === null)
            return value;
        if (Array.isArray(value))
            return value.map(turned);
        const now = {};
        for (const key of Object.keys(value).reverse())
            now[key] = turned(value[key]);
        return now;
    }

    // Says the daemon is answering now, as its first pass tells each
    // connection watching.
    function passed() {
        saying = "answering";
        const told = [];
        for (const kind of Object.keys(watchers)) {
            const watcher = watchers[kind];
            if (told.includes(watcher))
                continue;
            told.push(watcher);
            watcher.write(`${JSON.stringify({ id: "hello", state: "answering" })}\n`);
            watcher.flush();
        }
    }

    function recordsOf(kinds) {
        return Object.values(served).filter(record => kinds.includes(Records.kindOf(record.key)));
    }

    // Tells each watch of a record's kind that it changed.
    function tell(record) {
        served[record.key] = record;
        const watcher = watchers[Records.kindOf(record.key)];
        watcher.write(`${JSON.stringify({ id: "all", changed: [record], gone: [] })}\n`);
        watcher.flush();
    }

    SocketServer {
        id: daemon

        active: false
        path: `${root.runtime}/spend-watch/socket`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    const kinds = [].concat(ask.kind ?? []);
                    let answer = { id: ask.id, unwatched: true };
                    if (ask.ask === "hello")
                        answer = { id: ask.id, version: 1, domain: "spend", state: root.saying };
                    else if (ask.ask === "watch") {
                        for (const kind of kinds)
                            root.watchers[kind] = asked;
                        if (kinds.some(kind => root.silent.includes(kind)))
                            return;
                        answer = { id: ask.id, records: root.recordsOf(kinds) };
                    } else if (ask.ask === "act") {
                        root.acts.push(`${ask.act} ${ask.line}`);
                        answer = { id: ask.id, done: true };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    // Waits for `holds`, five seconds at most, then goes on to `then`, or
    // fails saying `what`; `then` failing fails too.
    function until(what, holds, then) {
        waits.what = what;
        waits.holds = holds;
        waits.then = then;
        waits.left = 100;
        waits.start();
    }

    Timer {
        id: waits

        property string what: ""
        property var holds: null
        property var then: null
        property int left: 0

        interval: 50
        repeat: true
        onTriggered: {
            try {
                if (holds()) {
                    stop();
                    then();
                } else if (--left === 0) {
                    stop();
                    throw new Error(`${what} never held`);
                }
            } catch (error) {
                stop();
                console.warn(`FAIL ${error.message}`);
                Qt.callLater(() => Qt.exit(1));
            }
        }
    }

    // First what the file says, as the shell reads it while the socket
    // isn't answering, so the socket's first answer can be held against it.
    // The check puts what spend-watch wrote where it writes it.
    function begin() {
        until("the collection read from the file",
            () => root.collected !== "" && spend.known && SpendWatch.fromFile
                && Object.keys(SpendWatch.budgets).length > 0, stream);
    }

    // Told by the socket rather than read from the file: what it says
    // anew remakes only the rows it changed, every pass having made every
    // row again before (2026-11-10).
    property int streamed: 0

    function stream() {
        const records = Object.assign(Records.records(JSON.parse(root.collected)),
            Records.budgetRecords(JSON.parse(realBudgets.text())));
        const budgets = SpendWatch.budgets;
        served = {};
        for (const key of Object.keys(records))
            served[key] = turned(records[key]);
        streamed = spend.tally.rows;
        const first = spend.everything()[0];
        Expect.expect("the file watched while it's read", SpendWatch.watchingFile, "unwatched");
        saying = "starting";
        daemon.active = true;
        until("the socket answering, starting", () => !SpendWatch.fromFile
            && SpendWatch.lastRun && !SpendWatch.rowsReading && !SpendWatch.reportsReading, () => {
            Expect.expect("the socket's answer, as the file's in another order, makes no row",
                spend.everything()[0] === first && spend.tally.rows === streamed,
                spend.tally.rows - streamed);
            Expect.expect("the file unwatched while the socket answers", !SpendWatch.watchingFile,
                "watched");
            Expect.expect("the budgets told as the file said them", Object.keys(budgets).length > 0
                && SpendWatch.budgets === budgets, Object.keys(SpendWatch.budgets).join(" "));
            Expect.expect("starting, its figures are still being read, said as its last run's",
                spend.stillReading && spend.answer("spend").note
                === "the watch is still reading; these are its last run's figures",
                `${spend.stillReading} ${spend.answer("spend").note}`);
            passed();
            until("the daemon answering", () => !spend.stillReading, current);
        });
    }

    function current() {
        Expect.expect("its figures pass as current once it's answering",
            spend.answer("spend").note === "Return copies the summary",
            spend.answer("spend").note);
        // Counted from here, since an answer asked makes its own rows.
        streamed = spend.tally.rows;
        // Agents' figure, as each pass tells some session's.
        const session = Object.keys(served).find(key => Records.kindOf(key) === "session");
        tell(Object.assign({}, served[session], { spent: "$999" }));
        until("a session's figure told",
            () => SpendWatch.sessions[Records.idOf(session)] === "$999", sessionTold);
    }

    function sessionTold() {
        spend.everything();
        Expect.expect("a session's figure changed makes none of the rows",
            spend.tally.rows === streamed, spend.tally.rows - streamed);
        const row = Object.keys(served).find(key => key.startsWith("row:now:"));
        const line = Object.assign({}, served[row].line, { side: "$1 \u00b7 told" });
        tell(Object.assign({}, served[row], { line: line }));
        until("a row told",
            () => SpendWatch.rows.now.some(one => one.line.side === "$1 \u00b7 told"), rowTold);
    }

    function rowTold() {
        const rows = spend.everything();
        Expect.expect("a row changed makes that row alone",
            rows.some(one => one.side === "$1 \u00b7 told")
            && spend.tally.rows === streamed + 1, spend.tally.rows - streamed);
        acted();
    }

    // A finding told by the socket is acted on through it, as the shell
    // spawns nothing for an act (2026-11-10).
    function acted() {
        const burn = "A burn in Aquifer";
        tell({
            key: "row:findings:finding:burn:s1", kind: "row", list: "findings", order: 0,
            line: {
                key: "finding:burn:s1", name: burn, side: "alert", copy: `${burn}.`,
                role: "alert", run: ["spend-watch", "go", "s1"], verb: "go to",
                expect: ["spend-watch", "findings", "expect", "burn:s1"]
            }
        });
        until("a finding told", () => spend.everything().some(row => row.name === burn), () => {
            const found = spend.everything().find(row => row.name === burn);
            found.run();
            found.verbs().find(row => row.name === "It's Expected").run();
            until("its acts asked", () => root.acts.length === 2, () => {
                Expect.expect("going to its session and marking it expected ask the watch",
                    root.acts.join(", ") === "run finding:burn:s1, expect finding:burn:s1",
                    root.acts.join(", "));
                Expect.expect("acting spawns nothing", Niri.spawned.length === 0,
                    Niri.spawned.join(" "));
                unanswered();
            });
        });
    }

    // A watch the daemon never answers is still reading, drawn from what's
    // held, and remakes nothing.
    function unanswered() {
        const made = spend.tally.rows;
        const rows = spend.everything();
        launcher.open = false;
        silent = ["row"];
        launcher.open = true;
        let ticks = 0;
        until("a watch left unanswered a while", () => ++ticks === 10, () => {
            Expect.expect("a watch never answered is still reading", spend.stillReading
                && !SpendWatch.fromFile, `${spend.stillReading} ${SpendWatch.fromFile}`);
            const still = spend.everything();
            Expect.expect("drawn from what's held, no row made twice",
                still.length === rows.length && spend.tally.rows === made,
                spend.tally.rows - made);
            Qt.callLater(() => Qt.exit(0));
        });
    }

    Component.onCompleted: begin()
}

// Drives the launcher's collection of AI spend (launcher/Spend.qml) told by
// spend-watch's socket rather than read from its file, through Quickshell
// itself, over records cut from the collection spend-watch wrote beside
// this, with a stand-in serving them as the daemon does. The socket's first
// answer, saying what the file said, makes no row; a change to a session's
// figure makes none of the rows and a change to one row makes that row
// alone; and a watch that never answers is drawn from what's held while
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

    // spend-watch's socket as its daemon serves it (its src/interface.rs):
    // each watch told its kinds whole, and then only what changed, as
    // `tell` says it. A kind in `silent` is never answered, as a daemon too
    // slow to answer leaves it.
    property var served: ({})
    property var silent: []
    readonly property var watchers: ({})

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
                        answer = { id: ask.id, version: 1, domain: "spend", state: "answering" };
                    else if (ask.ask === "watch") {
                        for (const kind of kinds)
                            root.watchers[kind] = asked;
                        if (kinds.some(kind => root.silent.includes(kind)))
                            return;
                        answer = { id: ask.id, records: root.recordsOf(kinds) };
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
            () => root.collected !== "" && spend.known && SpendWatch.fromFile, stream);
    }

    // Told by the socket rather than read from the file: what it says
    // anew remakes only the rows it changed, every pass having made every
    // row again before (2026-11-10).
    property int streamed: 0

    function stream() {
        served = Records.records(JSON.parse(root.collected));
        streamed = spend.tally.rows;
        const first = spend.everything()[0];
        daemon.active = true;
        until("the socket answering", () => !SpendWatch.fromFile && !spend.stillReading, () => {
            Expect.expect("the socket's answer, the same as the file's, makes no row",
                spend.everything()[0] === first && spend.tally.rows === streamed,
                spend.tally.rows - streamed);
            Expect.expect("its figures pass as current",
                spend.answer("spend").note === "Return copies the summary",
                spend.answer("spend").note);
            // Counted from here, since an answer asked makes its own rows.
            streamed = spend.tally.rows;
            // Agents' figure, as each pass tells some session's.
            const session = Object.keys(served).find(key => Records.kindOf(key) === "session");
            tell(Object.assign({}, served[session], { spent: "$999" }));
            until("a session's figure told",
                () => SpendWatch.sessions[Records.idOf(session)] === "$999", sessionTold);
        });
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
        unanswered();
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

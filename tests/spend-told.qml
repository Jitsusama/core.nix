// Drives the launcher's collection of AI spend (launcher/Spend.qml) told by
// spend-watch's socket rather than read from its file, through Quickshell
// itself, over records cut from the collection spend-watch wrote beside
// this and a few rows served ready, with a stand-in serving them as the
// daemon does, each place's rows to that place's watch. The socket's first
// answer leaves the budgets as the files said them, and while the daemon
// is starting its figures are still being read, said as its last run's,
// until it says it's answering; the file isn't watched while the socket
// answers; a change to a session's figure makes none of the rows and a
// change to one row makes that row alone; going to a finding's session and
// marking it expected are asked of the socket, spawning nothing, from the
// launcher's rows and the bar's piece alike; and a watch that never answers
// is drawn from what's held while it's still reading, with no row made
// twice. (tests/spend-served.qml holds the rows against the real daemon.)
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
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
        property bool wanting: true

        function wants(source) {
            return wanting;
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

    // The bar's piece, over a status this writes.
    Spend {
        id: bar
        settings: ({ status: `${root.runtime}/spend-watch/bar.json` })
    }

    FileView {
        id: barStatus
        path: `${root.runtime}/spend-watch/bar.json`
        printErrors: false
    }

    // spend-watch's socket as its daemon serves it (its src/interface.rs):
    // each watch told its kinds whole, a row or a summary only of the place
    // it names, and then only what changed, as `tell` says it. A kind in
    // `silent` is never answered, as a daemon too slow to answer leaves it.
    property var served: ({})
    property var silent: []
    // The acts asked, each as its act and its row's key.
    property var acts: []
    // Each connection watching, by what it hears: a kind, or a kind and
    // the place it names, "row@spend".
    readonly property var watchers: ({})
    // What hello says, as the daemon says "starting" until its first pass.
    property string saying: "answering"

    // A row served ready in Spend's place, as spend-watch's ready.rs lays
    // one: `fields` over a row of `id` saying `side`.
    function ready(id, side, fields) {
        const key = `row:spend:${id}`;
        return {
            key: key, kind: "row",
            row: Object.assign({
                key: key, thing: `spend:${id}`, place: "spend", kind: id.split(":")[0],
                order: 1024, name: "Today", glyph: "cost", hue: "voice", depth: 0,
                side: [{ said: side }], acts: [{ key: "verb:copy", copy: `Today: ${side}` }],
                held: {}
            }, fields ?? {})
        };
    }

    // What a record is heard by: its kind, and a row's or a summary's place.
    function heardBy(record) {
        const kind = Records.kindOf(record.key);
        const place = record.row?.place ?? record.summary?.place;
        return place === undefined ? kind : `${kind}@${place}`;
    }

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

    // Every connection watching anything, once each.
    function connections() {
        const all = [];
        for (const heard of Object.keys(watchers)) {
            for (const watcher of watchers[heard]) {
                if (!all.includes(watcher))
                    all.push(watcher);
            }
        }
        return all;
    }

    // Says the daemon is answering now, as its first pass tells each
    // connection watching.
    function passed() {
        saying = "answering";
        for (const watcher of connections()) {
            watcher.write(`${JSON.stringify({ id: "hello", state: "answering" })}\n`);
            watcher.flush();
        }
    }

    // Tells each watch hearing a record that it changed.
    function tell(record) {
        served[record.key] = record;
        for (const watcher of watchers[heardBy(record)] ?? []) {
            watcher.write(`${JSON.stringify({ id: "all", changed: [record], gone: [] })}\n`);
            watcher.flush();
        }
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
                    const hears = [].concat(ask.kind ?? []).map(kind => ["row", "summary"]
                        .includes(kind) && ask.place ? `${kind}@${ask.place}` : kind);
                    let answer = { id: ask.id, unwatched: true };
                    if (ask.ask === "hello")
                        answer = { id: ask.id, version: 1, domain: "spend", state: root.saying };
                    else if (ask.ask === "watch") {
                        for (const heard of hears)
                            root.watchers[heard] = (root.watchers[heard] ?? []).concat([asked]);
                        if (hears.some(heard => root.silent.includes(heard)))
                            return;
                        answer = { id: ask.id, records: Object.values(root.served)
                            .filter(record => hears.includes(root.heardBy(record))) };
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

    // First what the files say, as the shell reads them while the socket
    // isn't answering, so the socket's first answer can be held against
    // them. The check puts what spend-watch wrote where it writes it.
    function begin() {
        until("the collection read from the file",
            () => root.collected !== "" && SpendWatch.known && SpendWatch.fromFile
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
        served["row:spend:today"] = ready("today", "$4 \u00b7 usual");
        served["summary:spend"] = { key: "summary:spend", kind: "summary", summary: {
            place: "spend", said: "$4 today", headline: "Today: $4", rows: ["row:spend:today"]
        } };
        Expect.expect("the file watched while it's read", SpendWatch.watchingFile, "unwatched");
        saying = "starting";
        daemon.active = true;
        until("the socket answering, starting", () => !SpendWatch.fromFile
            && SpendWatch.lastRun && !SpendWatch.reportsReading && spend.known, () => {
            Expect.expect("its rows as served", spend.everything()[0]?.name === "Today",
                spend.everything().map(row => row.name).join(", "));
            Expect.expect("the file unwatched while the socket answers", !SpendWatch.watchingFile,
                "watched");
            Expect.expect("the budgets told as the file said them", Object.keys(budgets).length > 0
                && SpendWatch.budgets === budgets, Object.keys(SpendWatch.budgets).join(" "));
            Expect.expect("starting, its figures are still being read, said as its last run's",
                spend.stillReading && spend.answer("spend").note
                === "still reading · its last figures",
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
        spend.everything();
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
        tell(ready("today", "$1 \u00b7 told"));
        until("a row told",
            () => spend.served.served("row:spend:today")?.side[0].said === "$1 \u00b7 told",
            rowTold);
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
        const act = what => ({ ask: "act", act: what, line: "finding:burn:s1" });
        // Listed first, as the daemon lists a finding that wants a look,
        // and marking it expected behind it.
        tell(ready("finding:burn:s1", "alert", {
            order: 512, name: burn, glyph: "urgent", hue: "alert", held: { verb: "go to" },
            acts: [{ key: "verb:spend:run", ask: act("run") }, { key: "verb:more", step: "rows" }]
        }));
        tell(ready("finding:burn:s1:expect", "", {
            parent: "row:spend:finding:burn:s1", depth: 1, name: "It's Expected", glyph: "fine",
            hue: "action", side: [], held: { verb: "expect" },
            acts: [{ key: "verb:spend:expect", ask: act("expect") }]
        }));
        until("a finding told", () => spend.everything()[0]?.name === burn, () => {
            const found = spend.everything()[0];
            found.run();
            found.verbs().find(row => row.name === "It's Expected").run();
            until("its acts asked", () => root.acts.length === 2, () => {
                Expect.expect("going to its session and marking it expected ask the watch",
                    root.acts.join(", ") === "run finding:burn:s1, expect finding:burn:s1",
                    root.acts.join(", "));
                Expect.expect("acting spawns nothing", Niri.spawned.length === 0,
                    Niri.spawned.join(" "));
                barActed();
            });
        });
    }

    // The bar's piece asks the socket for an action the watch offers, by
    // its key, as the launcher's rows do.
    function barActed() {
        const said = (value, hint) => ({ value: value, badge: "91%", hint: hint, context: "" });
        barStatus.setText(JSON.stringify({
            v: 1, heartbeat: Date.now() / 1000, active: true, role: "alert", wants: true,
            title: "AI spend",
            open: said("$3,642 of $4,000 this week", "The weekly cap is 91% spent"),
            quiet: said("the weekly cap is close", "The weekly cap is close"),
            actions: [{
                kind: "go", key: "status:finding:burn:s1", name: "Go to the session",
                run: ["spend-watch", "go", "s1"]
            }, {
                kind: "mark", key: "status:mark:burn:s1", name: "It's expected",
                run: ["spend-watch", "findings", "expect", "burn:s1"]
            }]
        }));
        until("the bar's actions offered", () => bar.actions.length === 3, () => {
            Expect.expect("the bar offers the watch's actions first, each with its mark",
                bar.actions.map(action => action.name).join(", ")
                === "Go to the session, It's expected, Show where it went"
                && bar.actions[0].glyph === Theme.glyph.windows
                && bar.actions[1].glyph === Theme.glyph.fine,
                bar.actions.map(action => action.name).join(", "));
            root.acts = [];
            bar.actions[0].run();
            bar.actions[1].run();
            until("the bar's acts asked", () => root.acts.length === 2, () => {
                Expect.expect("the bar's actions ask the watch by their keys, spawning nothing",
                    root.acts.join(", ") === "run status:finding:burn:s1, run status:mark:burn:s1"
                    && Niri.spawned.length === 0,
                    [Niri.spawned.join(" ")].concat(root.acts).join(", "));
                unanswered();
            });
        });
    }

    // A watch the daemon never answers is still reading, drawn from what's
    // held, and remakes nothing.
    function unanswered() {
        const made = spend.tally.rows;
        const rows = spend.everything();
        launcher.wanting = false;
        silent = ["report"];
        launcher.wanting = true;
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

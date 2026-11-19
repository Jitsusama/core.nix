pragma ComponentBehavior: Bound
// Drives the launcher's Power (modules/quickshell/launcher/Power.qml)
// through Quickshell itself, over a stand-in for the machine daemon
// serving Power's rows and headline ready (the ready-row form), with the
// launcher stood in, open, "battery" typed. Power answers the headline and
// its own rows as served, in their order, each holding the rows inside it
// a step in; says how it stands from the headline; lends a row by its key,
// as Now's Battery borrows the Energy's; and asked again, as each key
// typed asks it, makes no row. The charge told anew makes its row alone
// again and keeps the Charger's. A charge limit's stop chosen draws the
// limit as it will read at once, the launcher left open, and one the
// daemon refuses is taken back in the daemon's words. It exits 0 once
// every case holds, and otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect
import "Watch.js" as Watch
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the first rows"

    // The launcher as Power reads it: open, "battery" typed, so Power's
    // rows can be seen and its daemon is watched.
    QtObject {
        id: theLauncher

        property bool open: true
        property string typed: "battery"
        property var answer: null
        property var held: []
        property var matches: []
        property var placed: []

        // No frame is drawn here, so what waits on one runs at once.
        function afterFrame(then) {
            then();
        }

        // What an act says its rows become, and why one was taken back,
        // as the launcher hears them.
        property var becoming: ({})
        property var takenBack: null
        function becomesAll(provider, holds, how) {
            for (const one of holds)
                becoming[one.key] = Object.assign({ row: one.row }, how);
        }
        function becameNot(key, why) {
            takenBack = { key: key, why: why };
            root.limitRefused();
        }
    }

    // A row as the machine daemon serves it, in the crate's wrap.
    function served(id, order, fields) {
        const key = `row:power:${id}`;
        return { key: key, kind: "row", row: Object.assign({ key: key, thing: `power:${id}`,
                place: "power", kind: "figure", order: order, depth: 0, side: [], acts: [] },
            fields) };
    }
    readonly property var charger: served("charger", 1, {
            kind: "charger",
            name: "Charger",
            glyph: "charging",
            side: [{ said: "none plugged in" }],
            acts: [{ key: "verb:copy", copy: "none plugged in" }, { key: "verb:more",
                    step: "rows" }]
        })
    readonly property var port: served("port:1", 1, {
            kind: "port",
            name: "Port 1",
            parent: "row:power:charger",
            depth: 1,
            side: [{ said: "empty" }],
            acts: [{ key: "verb:copy", copy: "empty" }]
        })
    function charge(pct) {
        return served("charge", 2, {
            name: "Charge",
            side: [{ said: `${pct}%` }],
            graph: { shape: "bar", values: [pct / 100], warn: false },
            acts: [{ key: "verb:copy", copy: `${pct}%` }]
        });
    }
    readonly property var energy: served("energy", 3, {
            name: "Energy",
            side: [{ said: "40.0 of 50.0 Wh" }],
            drawer: { eyebrow: "on this machine \u00b7 battery", eyebrowNames: true,
                title: "Energy", facts: [{ name: "Battery", said: "BAT0", identifies: true }] },
            acts: [{ key: "verb:copy", copy: "40.0 of 50.0 Wh" }]
        })
    function limit(stop, start) {
        return served("limit", 4, {
            kind: "holder",
            name: "Charge Limit",
            glyph: "charge",
            side: [{ said: `stops at ${stop}%, starts below ${start}%` }],
            acts: [{ key: "verb:more", step: "rows" }]
        });
    }
    readonly property var stop: served("limit:stop:70", 1, {
            kind: "choice",
            name: "Stop at 70%",
            parent: "row:power:limit",
            depth: 1,
            glyph: "charge",
            hue: "action",
            acts: [{ key: "verb:set", stays: true, ask: { ask: "act", act: "charge-limit",
                        supply: "BAT0", stop_pct: 70 }, becomes: root.limit(70, 65).row }]
        })
    readonly property var head: ({ key: "head:power", kind: "head", head: {
                said: "80% \u00b7 on battery",
                graph: { shape: "bar", values: [0.8], warn: false } } })

    // The shell's watch, so the daemon can tell it more.
    property var watch: null
    property var watched: null
    property var asked: null

    // A stand-in for the machine daemon, speaking the daemon interface:
    // hello, a watch of Power's rows and headline answered whole, and a
    // charge limit refused in the daemon's own words.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        handler: Socket {
            id: socket

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "machine",
                            state: "running" };
                    } else if (ask.ask === "watch") {
                        root.watch = { socket: socket, id: ask.id };
                        root.watched = ask;
                        answer = { id: ask.id, records: [root.stop, root.limit(80, 75),
                                root.energy, root.port, root.charge(80), root.head,
                                root.charger] };
                    } else if (ask.ask === "act" && ask.act === "charge-limit") {
                        root.asked = ask;
                        answer = { id: ask.id, error: { code: "refused",
                                said: "the battery wouldn't take it" } };
                    } else {
                        answer = { id: ask.id, ok: true };
                    }
                    socket.write(`${JSON.stringify(answer)}\n`);
                    socket.flush();
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

    // Power says it answered as its first rows come, and each time the
    // daemon tells it something while it shows; each case waits for what
    // it needs to have been heard.
    Launcher.Power {
        id: thePower

        launcher: theLauncher
        settings: ({ name: "power" })
        onAnswered: {
            try {
                if (root.waiting === "the first rows")
                    root.first();
                else if (root.waiting === "the charge told anew")
                    root.chargeTold();
            } catch (error) {
                root.waiting = "a failure";
                Expect.fail(error.message);
            }
        }
    }

    // Power's own rows as served, in their order, under the served
    // headline; asked again with nothing told since, it makes no row.
    function first() {
        const answer = thePower.answer("battery");
        if (answer === null || answer.rows.length < 4)
            return;
        root.waiting = "the charge told anew";
        Expect.expect("Power's rows and headline watched", root.watched.kind.includes("row")
            && root.watched.kind.includes("head") && root.watched.place === "power",
            JSON.stringify(root.watched));
        Expect.expect("the headline as served", answer.headline === "80% \u00b7 on battery"
            && answer.graph?.values?.[0] === 0.8, answer.headline);
        Expect.expect("its own rows, in the daemon's order", answer.rows.map(row => row.name)
            .join(", ") === "Charger, Charge, Energy, Charge Limit",
            answer.rows.map(row => row.name));
        Expect.expect("how it stands, as the headline says it",
            thePower.stands() === "80% \u00b7 on battery", thePower.stands());
        const [held, value] = [answer.rows[0], answer.rows[1]];
        Expect.expect("a part holding more in the machine's ink, its ports a step in",
            held.hue === "machine" && held.verbs().map(row => row.name).join() === "Port 1",
            `${held.hue} ${held.verbs().map(row => row.name)}`);
        Expect.expect("a value in the voice's ink, its Return copying and closing",
            value.hue === "voice" && value.copies?.() === "80%" && value.stays !== true,
            `${value.hue} ${value.copies?.()} ${value.stays}`);
        const lent = thePower.rowNow("power:energy");
        Expect.expect("the Energy lent by its key, its drawer as served",
            lent?.side === "40.0 of 50.0 Wh" && lent.drawer()?.title === "Energy",
            JSON.stringify(lent?.side));
        Expect.expect("a row it doesn't serve lent as unknown",
            thePower.rowNow("power:nothing") === undefined, "lent");
        const made = thePower.tally.rows;
        const again = thePower.answer("battery");
        Expect.expect("asked again with nothing told, no row made", thePower.tally.rows === made,
            `${thePower.tally.rows - made} rows`);
        Expect.expect("asked again, the same rows", again.rows.every((row, at) => row
            === answer.rows[at]), "made anew");
        root.before = answer.rows;
        root.counted = thePower.tally.rows;
        // Shown, so what the daemon tells is answered at once.
        theLauncher.answer = answer;
        Watch.tell(root.watch, { changed: [root.charge(79)] });
    }

    // The daemon tells the charge anew: the Charger's row is kept and the
    // charge's made again, saying its new figure. Then a stop is chosen.
    function chargeTold() {
        const answer = thePower.answer("battery");
        if (answer.rows[1]?.side !== "79%")
            return;
        root.waiting = "the charge limit refused";
        Expect.expect("the Charger's row kept", answer.rows[0] === root.before[0],
            answer.rows[0]?.name);
        Expect.expect("one row made", thePower.tally.rows - root.counted === 1,
            `${thePower.tally.rows - root.counted} rows`);
        const stop = answer.rows[3].verbs()[0];
        Expect.expect("a stop's Return asks and leaves the launcher open",
            stop?.name === "Stop at 70%" && stop.stays === true, `${stop?.name} ${stop?.stays}`);
        stop.run();
        const becomes = theLauncher.becoming["power:limit"];
        Expect.expect("the limit says where it will stop at once",
            becomes?.row?.side === "stops at 70%, starts below 65%", becomes?.row?.side);
        Expect.expect("as a set Charge Limit", becomes.name === "Stop at 70%"
            && becomes.act === "charge-limit", `${becomes.name} ${becomes.act}`);
    }

    // The daemon refused it: the row goes back, and the foot says why in
    // the daemon's own words.
    function limitRefused() {
        try {
            Expect.expect("the daemon asked for that battery's stop", root.asked?.supply
                === "BAT0" && root.asked?.stop_pct === 70, JSON.stringify(root.asked));
            Expect.expect("taken back, saying why", theLauncher.takenBack.key === "power:limit"
                && theLauncher.takenBack.why === "the battery wouldn't take it",
                JSON.stringify(theLauncher.takenBack));
        } catch (error) {
            root.waiting = "a failure";
            Expect.fail(error.message);
            return;
        }
        root.waiting = "the end";
        Qt.callLater(() => Qt.exit(0));
    }
}

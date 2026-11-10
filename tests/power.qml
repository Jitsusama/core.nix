pragma ComponentBehavior: Bound
// Drives the launcher's Power (modules/quickshell/launcher/Power.qml)
// through Quickshell itself, over a stand-in for the machine daemon
// serving a battery and a USB-C port, with the launcher stood in, open,
// "battery" typed and Power's answer shown. Power's answer asked again
// before anything is read, as each key typed asks it, makes no row; the
// battery told anew makes its own rows again and keeps the Charger's; and
// the port told anew makes the Charger's again and keeps the battery's.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect
import "Watch.js" as Watch
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the first reading"

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
    }

    // The machine's battery at `pct`, and its USB-C port, `online` or
    // not, as the machine daemon says them.
    function battery(pct) {
        return { key: "supply:BAT0", kind: "supply", type: "Battery", scope: "System",
            name: "BAT0", status: "Discharging", capacity_pct: pct, energy_wh: pct / 2,
            energy_full_wh: 50 };
    }
    function port(online) {
        return { key: "supply:ucsi-source-psy-USBC000:001", kind: "supply", type: "USB",
            name: "ucsi-source-psy-USBC000:001", online: online };
    }

    // The shell's watch, so the daemon can tell it more.
    property var watch: null

    // A stand-in for the machine daemon, speaking the daemon interface:
    // hello, and a watch of the supplies answered whole.
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
                        answer = { id: ask.id, records: [root.battery(80), root.port(false)] };
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

    // Power says it answered as the battery first comes, and each time the
    // daemon tells it something while its answer shows; each case waits
    // for what it needs to have been heard.
    Launcher.Power {
        id: thePower

        launcher: theLauncher
        settings: ({ name: "power" })
        onAnswered: {
            try {
                if (root.waiting === "the first reading")
                    root.first();
                else if (root.waiting === "the battery told anew")
                    root.batteryTold();
                else if (root.waiting === "the port told anew")
                    root.portTold();
            } catch (error) {
                root.waiting = "a failure";
                Expect.fail(error.message);
            }
        }
    }

    function charge(answer) {
        return answer.rows.find(row => row.key === "power:charge");
    }

    // Asked again with nothing read since, Power makes no row, where each
    // key typed while it shows made every row again.
    function first() {
        const answer = thePower.answer("battery");
        if (answer === null || !charge(answer))
            return;
        root.waiting = "the battery told anew";
        const made = thePower.tally.rows;
        const again = thePower.answer("battery");
        Expect.expect("asked again with nothing read, no row made", thePower.tally.rows === made,
            `${thePower.tally.rows - made} rows`);
        Expect.expect("asked again, the same rows", again.rows.length === answer.rows.length
            && again.rows.every((row, at) => row === answer.rows[at]), "made anew");
        Expect.expect("the Charger first", answer.rows[0].key === "power:charger", answer.rows[0].key);
        root.before = answer.rows;
        theLauncher.answer = answer;
        Watch.tell(root.watch, { changed: [root.battery(79)] });
    }

    // The battery told anew: its rows are made again, saying its charge,
    // and the Charger's is kept.
    function batteryTold() {
        const answer = thePower.answer("battery");
        if (charge(answer).side !== "79%")
            return;
        root.waiting = "the port told anew";
        Expect.expect("the Charger's row kept", answer.rows[0] === root.before[0], "made anew");
        root.before = answer.rows;
        // The sparks read again once a second; none falls due before the
        // port's case, however slow the machine running it, so what moves
        // is only what the daemon tells.
        thePower.series = Object.assign({}, thePower.series, { at: Date.now() + 3600000 });
        Watch.tell(root.watch, { changed: [root.port(true)] });
    }

    // The port told anew: the Charger's row is made again, and the
    // battery's are kept.
    function portTold() {
        const answer = thePower.answer("battery");
        if (answer.rows[0] === root.before[0])
            return;
        root.waiting = "the end";
        Expect.expect("the battery's rows kept", answer.rows.slice(1).every((row, at) => row
            === root.before[at + 1]), "made anew");
        Qt.callLater(() => Qt.exit(0));
    }
}

pragma ComponentBehavior: Bound
// Drives the launcher's Machines (modules/quickshell/launcher/Machines.qml)
// through Quickshell itself, over a stand-in for the realm's mycelium node
// serving its machines (kind `machine`, changes saying only what differs),
// with the launcher and pi's sessions stood in. Machines lists them this
// machine first, then those reached and those out of reach; a change to
// one machine makes its row alone again and keeps the others; a change
// while the launcher doesn't want Machines makes no row; and wanted again,
// only the machine that changed is made again, saying what it was told.
// Shut on Joel's shell, every node change made all four machines' rows
// again: 100 rows in four minutes, for nobody (2026-11-15). It exits 0
// once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect
import "Watch.js" as Watch
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the first rows"

    // The launcher as Machines reads it: open, and wanting Machines' rows
    // while `wanting` says so, as Launcher.qml's wants does.
    QtObject {
        id: theLauncher

        property bool open: true
        property string typed: ""
        property var held: []
        property bool wanting: true

        function wants(source) {
            return wanting;
        }

        function afterFrame(then) {
            then();
        }
    }

    // A machine as the node serves it, by its name.
    function machine(name, fields) {
        return Object.assign({ key: `machine:${name}`, kind: "machine", name: name,
            state: "ready", system: "linux", os: "NixOS 26.11",
            measured_at: new Date().toISOString() }, fields);
    }
    readonly property var served: [
        machine("anchor", { here: true, model_name: "XPS 14" }),
        machine("skiff", { power: { on_battery: true, charge: 62 } }),
        machine("lantern", { system: "macos", os: "macOS 26.6.2",
            power: { on_battery: true, charge: 80 } }),
        machine("hermit", { state: "unreachable" })
    ]

    // The shell's watch, so the node can tell it more.
    property var watch: null

    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/personal.sock`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "mycelium",
                            state: "running" };
                    } else if (ask.ask === "watch" && ask.kind === "machine") {
                        root.watch = { socket: asked, id: ask.id };
                        answer = { id: ask.id, records: root.served };
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

    Launcher.Machines {
        id: theMachines

        launcher: theLauncher
        settings: ({ name: "machines", realms: ["personal"] })
    }

    property var before: []
    property int counted: 0
    // When the change Machines wasn't wanted for was heard, so the case
    // gives it a moment to make what it would before counting.
    property real heardAt: 0

    function rows() {
        return theMachines.all;
    }
    function named(name) {
        return rows().find(row => row.name === name);
    }
    function made() {
        return theMachines.tally.rows - root.counted;
    }

    // Each case looks again as anything moves, until what it waits for
    // has come.
    Timer {
        running: true
        repeat: true
        interval: 20
        onTriggered: {
            try {
                root.step();
            } catch (error) {
                root.waiting = "a failure";
                Expect.fail(error.message);
            }
        }
    }

    function step() {
        if (root.waiting === "the first rows")
            first();
        else if (root.waiting === "skiff's change")
            skiffChanged();
        else if (root.waiting === "lantern's change, unwanted")
            unwanted();
        else if (root.waiting === "lantern's row, wanted again")
            wantedAgain();
    }

    // Every machine, this one first, then those reached by name, then the
    // one out of reach.
    function first() {
        if (root.watch === null || rows().length < 4)
            return;
        root.waiting = "skiff's change";
        Expect.expect("this machine first, then reached, then out of reach",
            rows().map(row => row.name).join(" ") === "anchor lantern skiff hermit",
            rows().map(row => row.name).join(" "));
        root.before = rows().slice();
        root.counted = theMachines.tally.rows;
        Watch.tell(root.watch, { changed: [{ key: "machine:skiff", kind: "machine",
                    power: { on_battery: true, charge: 55 } }] });
    }

    // The node tells skiff's charge anew: its row alone is made again,
    // saying it, and every other row is kept as it was.
    function skiffChanged() {
        const skiff = named("skiff");
        if (!skiff || !skiff.side.includes("55"))
            return;
        root.waiting = "lantern's change, unwanted";
        Expect.expect("the order kept", rows().map(row => row.name).join(" ")
            === "anchor lantern skiff hermit", rows().map(row => row.name).join(" "));
        Expect.expect("skiff's row made again", skiff !== root.before[2], skiff.side);
        for (const name of ["anchor", "lantern", "hermit"]) {
            Expect.expect(`${name}'s row kept as skiff changed`,
                named(name) === root.before.find(row => row.name === name), "made anew");
        }
        Expect.expect("one row made as one machine changed", made() === 1, `${made()} rows`);
        root.before = rows().slice();
        root.counted = theMachines.tally.rows;
        theLauncher.wanting = false;
        Watch.tell(root.watch, { changed: [{ key: "machine:lantern", kind: "machine",
                    power: { on_battery: true, charge: 41 } }] });
    }

    Connections {
        target: theMachines

        function onHeldByChanged() {
            if (root.waiting === "lantern's change, unwanted" && root.heardAt === 0)
                root.heardAt = Date.now();
        }
    }

    // Unwanted, lantern's change makes no row, however long it's given.
    function unwanted() {
        if (root.heardAt === 0 || Date.now() - root.heardAt < 300)
            return;
        root.waiting = "lantern's row, wanted again";
        Expect.expect("no row made while Machines isn't wanted", made() === 0,
            `${made()} rows`);
        theLauncher.wanting = true;
    }

    // Wanted again, lantern's row is made again, saying its new charge,
    // and the rest are kept.
    function wantedAgain() {
        const lantern = named("lantern");
        if (!lantern || !lantern.side.includes("41"))
            return;
        root.waiting = "the end";
        Expect.expect("lantern's row made again", lantern !== root.before[1], lantern.side);
        for (const name of ["anchor", "skiff", "hermit"]) {
            Expect.expect(`${name}'s row kept as Machines was wanted again`,
                named(name) === root.before.find(row => row.name === name), "made anew");
        }
        Expect.expect("one row made as it was wanted again", made() === 1, `${made()} rows`);
        Qt.callLater(() => Qt.exit(0));
    }
}

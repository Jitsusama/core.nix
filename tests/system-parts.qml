pragma ComponentBehavior: Bound
// Drives the shell's SystemParts (modules/quickshell/SystemParts.qml)
// through Quickshell itself, over a stand-in for the system daemon
// (modules/system) serving its part records and its summary, as Network's
// home row reads them (launcher/SystemPart.js). The shell watches the
// summary, part and machine kinds; holds each part by its name, the
// summary aside, and this machine's own record as `machine`, as Machine's
// home row reads it, the other machines aside; follows a part told anew
// and one let go; and, the daemon gone, holds no part, so no old
// judgement reads as current. It exits 0 once every case holds,
// and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect
import "Watch.js" as Watch

ShellRoot {
    id: root

    property string waiting: "the first parts"

    // A part as the system daemon serves it.
    function part(name, health, side, says) {
        return { key: `part:${name}`, kind: "part", part: name, health, side, says };
    }
    readonly property string lost: "the modem's downstream channel 16 has lost 878"
    readonly property var network: part("network", "watch", [root.lost],
        "The modem's downstream channel 16 has lost 878")
    readonly property var power: part("power", "good", [], "on the charger")
    readonly property var summary: ({ key: "summary", kind: "summary", health: "watch",
            machine: "laptop", side: [root.lost] })
    // A machine as the daemon serves it: this one, and another whose
    // trouble isn't this machine's.
    function machine(name, health, side) {
        return { key: `machine:${name}`, kind: "machine", name, part: "machines", health, side,
            says: `${name} says` };
    }
    readonly property var here: machine("laptop", "watch", ["filling disk, 40 GB free"])
    readonly property var studio: machine("studio", "bad", ["out of reach"])

    property var watch: null
    property var watched: null

    // A stand-in for the system daemon, speaking the daemon interface:
    // hello, and a watch of its parts answered whole.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/system.sock`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "system", state: "running" };
                    } else if (ask.ask === "watch") {
                        root.watch = { socket: asked, id: ask.id };
                        root.watched = ask;
                        answer = { id: ask.id, records: [root.network, root.power,
                                root.summary, root.here, root.studio] };
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

    Connections {
        target: SystemParts

        function onPartsChanged() {
            root.step();
        }

        function onHeardChanged() {
            root.step();
        }
    }

    function step() {
        try {
            if (root.waiting === "the first parts")
                root.first();
            else if (root.waiting === "the network told anew")
                root.networkTold();
            else if (root.waiting === "power let go")
                root.powerGone();
            else if (root.waiting === "the daemon gone")
                root.daemonGone();
        } catch (error) {
            root.waiting = "a failure";
            Expect.fail(error.message);
        }
    }

    // Each part held by its name as served, the summary aside, and the
    // daemon heard.
    function first() {
        const parts = SystemParts.parts;
        if (!SystemParts.heard || parts.network === undefined)
            return;
        root.waiting = "the network told anew";
        Expect.same("the summary, parts and machines watched", root.watched.kind,
            ["summary", "part", "machine"]);
        Expect.same("each part by its name, the summary and other machines aside",
            Object.keys(parts).sort(), ["machine", "network", "power"]);
        Expect.same("this machine's own record as its machine", parts.machine.side,
            ["filling disk, 40 GB free"]);
        Expect.same("a part as the daemon said it, its side whole", parts.network.side,
            [root.lost]);
        Expect.same("and its health", parts.network.health, "watch");
        Watch.tell(root.watch, { changed: [root.part("network", "good", [], "15 devices")] });
    }

    // A part told anew replaces what was held.
    function networkTold() {
        if (SystemParts.parts.network?.health !== "good")
            return;
        root.waiting = "power let go";
        Expect.same("the part told anew, whole", SystemParts.parts.network.side, []);
        Expect.same("the other kept", SystemParts.parts.power?.health, "good");
        Watch.tell(root.watch, { gone: ["part:power"] });
    }

    // A part let go is gone.
    function powerGone() {
        if (SystemParts.parts.power !== undefined)
            return;
        root.waiting = "the daemon gone";
        Expect.same("the part let go is gone, the rest kept",
            Object.keys(SystemParts.parts).sort(), ["machine", "network"]);
        root.watch.socket.connected = false;
    }

    // The daemon gone, nothing it said stands.
    function daemonGone() {
        if (SystemParts.heard || Object.keys(SystemParts.parts).length > 0)
            return;
        root.waiting = "the end";
        Qt.callLater(() => Qt.exit(0));
    }
}

// Drives the shell's Tuple (modules/tuple/Tuple.qml) through Quickshell
// itself, over a stand-in tuple-people that says a call's lines as the
// real one does, one at a time as this writes them, and a stand-in
// notify-send that keeps what it was asked to say. A call connecting says
// who's in it by name and since when; muting counts only once connected;
// whose screen and camera are shown, and Joel's own, are said; someone
// coming and going is told once the call has settled; the call ending
// ends it; and when the stream stops, the shell waits for Tuple's log to
// start it again. It exits 0 once every case holds, and otherwise 1,
// saying which didn't.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    property string waiting: "the start"
    property var next: null

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    // Each case, in order: what it waits for, then what it checks and does.
    function step(name, test, then) {
        root.waiting = name;
        root.next = { test: test, then: then };
        Qt.callLater(root.heard);
    }

    function heard() {
        const now = root.next;
        if (now === null || !now.test())
            return;
        root.next = null;
        try {
            now.then();
        } catch (error) {
            root.fail(`${root.waiting}: ${error.message}`);
        }
    }

    // Appends to a file of the stand-ins', one line.
    function write(file, line) {
        writer.createObject(root, { command: ["sh", "-c", "printf '%s\\n' \"$1\" >> \"$0\"", file, line] });
    }

    Component {
        id: writer

        Process {
            id: writing
            running: true
            onExited: writing.destroy()
        }
    }

    function says(call) {
        write(`${root.runtime}/call-feed`, JSON.stringify({ call: call }));
    }

    // What notify-send was asked to say, as each line its stand-in kept.
    property string told: ""

    Process {
        id: reading
        command: ["cat", `${root.runtime}/told`]
        stdout: StdioCollector {
            onStreamFinished: {
                root.told = this.text;
                Qt.callLater(root.heard);
            }
        }
    }

    Timer {
        // Reads again a while after each case asks, as notices are sent.
        id: reread
        interval: 200
        repeat: true
        running: root.waiting.startsWith("told")
        onTriggered: reading.running = true
    }

    readonly property real started: Date.now() - 60 * 1000
    function call(connection, since, own, people) {
        return {
            since: since,
            connection: connection,
            muted: own[0],
            sharing: own[1],
            camera: own[2],
            people: people
        };
    }
    readonly property var ada: [7, "Ada Brightwater", "connected", false, false, false]
    readonly property var cal: [9, "Cal Dunmore", "connected", false, false, false]

    Connections {
        target: Tuple

        function onLiveChanged() {
            Qt.callLater(root.heard);
        }

        function onStreamedChanged() {
            Qt.callLater(root.heard);
        }
    }

    Component.onCompleted: root.step("the stream", () => Tuple.streamed, root.first)

    function first() {
        expect("no call yet", Tuple.live === false && !Tuple.calling, JSON.stringify(Tuple.live));
        root.says(root.call("signaling", null, [true, false, false],
            [[7, "Ada Brightwater", "signaling", true, false, false]]));
        root.step("signaling", () => Tuple.calling, root.signaling);
    }

    function signaling() {
        expect("connecting", Tuple.connecting, Tuple.connecting);
        expect("not muted before it connects", Tuple.muted === null, Tuple.muted);
        expect("named by Tuple", Tuple.name("7") === "Ada Brightwater", Tuple.name("7"));
        expect("since it was heard", Math.abs(Tuple.since - Date.now()) < 5000, Tuple.since);
        root.says(root.call("connected", root.started, [true, false, false], [root.ada]));
        root.step("connected", () => !Tuple.connecting, root.connected);
    }

    function connected() {
        expect("since it connected", Tuple.since === root.started, Tuple.since);
        expect("muted, once connected", Tuple.muted === true, Tuple.muted);
        expect("who's in it", JSON.stringify(Tuple.others) === "[\"7\"]", JSON.stringify(Tuple.others));
        expect("those already there aren't news", Tuple.telling.length === 0, JSON.stringify(Tuple.telling));
        root.says(root.call("connected", root.started, [false, true, false],
            [[7, "Ada Brightwater", "connected", false, false, true]]));
        root.step("sharing", () => Tuple.sending, root.sharing);
    }

    function sharing() {
        expect("heard again", Tuple.muted === false, Tuple.muted);
        expect("her camera", JSON.stringify(Tuple.watching) === "[\"7\"]", JSON.stringify(Tuple.watching));
        expect("no one else's screen", Tuple.showing.length === 0, JSON.stringify(Tuple.showing));
        root.says(root.call("connected", root.started, [false, false, false],
            [[7, "Ada Brightwater", "connected", false, true, false], root.cal]));
        root.step("told Cal came", () => /Cal Dunmore joined the call/.test(root.told), root.came);
    }

    function came() {
        expect("her screen", JSON.stringify(Tuple.showing) === "[\"7\"]", JSON.stringify(Tuple.showing));
        expect("mine stopped", !Tuple.sending, Tuple.sending);
        root.says(root.call("connected", root.started, [false, false, false], []));
        root.step("told everyone left", () => /You're the only one still in it/.test(root.told), root.alone);
    }

    function alone() {
        root.says(null);
        root.step("the call ending", () => !Tuple.calling, root.ended);
    }

    function ended() {
        expect("no call, while the stream runs", Tuple.live === false && Tuple.muted === null,
            `${JSON.stringify(Tuple.live)} ${Tuple.muted}`);
        // Tuple stops: the stream ends, and starts again only with the log.
        write(`${root.runtime}/call-feed`, "END");
        root.step("the stream ending", () => !Tuple.streamed, root.stopped);
    }

    function stopped() {
        expect("no call without the stream", !Tuple.calling, Tuple.calling);
        write(`${root.runtime}/call-feed.next`, "");
        root.step("the log waking it", () => Tuple.streamed, root.again);
        write(Tuple.log, "12:00:00.000|1|1|info|daemon loop started, waiting for events...");
    }

    function again() {
        Qt.exit(0);
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }
}

// Drives the shell's Tuple (modules/tuple/Tuple.qml) through Quickshell
// itself, over the real tuple-call daemon following a stand-in tuple: its
// stream says each line this writes to it, and any other command is kept,
// as a stand-in notify-send keeps what it was asked to say. Joel's room is
// the one Tuple lists until he opens one; a call connecting says who's in
// it by name and since when; muting counts only once connected; whose
// screen and camera are shown, and Joel's own, are said; someone coming and
// going is told once the call has settled; muting and opening the room are
// asked of the daemon, which runs Tuple for each; the call ending ends it;
// and a ring and a missed call, from Tuple's log, are told. It exits 0 once
// every case holds, and otherwise 1, saying which didn't.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string log: `${Quickshell.env("XDG_DATA_HOME")}/tuple/0/log.txt`
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

    // Appends a line to a file.
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

    // Tuple's stream saying the call, or that there's none.
    function says(call) {
        write(`${root.runtime}/stream`, JSON.stringify({ type: "current_call", data: call }));
    }

    // Tuple's log saying something now, at the time of day it writes.
    function logs(message) {
        write(root.log, `${new Date().toISOString().slice(11, 23)}|1|2|info|calls|${message}`);
    }

    // What notify-send was asked to say, and what was asked of Tuple, as
    // each line their stand-ins kept.
    property string told: ""
    property string asked: ""

    Process {
        id: reading
        command: ["sh", "-c", "cat \"$0/told\"; echo ---; cat \"$0/asked\" 2>/dev/null", root.runtime]
        stdout: StdioCollector {
            onStreamFinished: {
                const [told, asked] = this.text.split("---\n");
                root.told = told;
                root.asked = asked ?? "";
                Qt.callLater(root.heard);
            }
        }
    }

    Timer {
        // Reads again a while after each case asks, as notices are sent
        // and Tuple is run.
        interval: 200
        repeat: true
        running: /^(told|asked)/.test(root.waiting)
        onTriggered: reading.running = true
    }

    readonly property real started: Date.now() - 60 * 1000
    // Someone as Tuple's stream says them: whether their voice is sent,
    // their screen shared and their camera on.
    function person(fields, connection, muted, sharing, camera) {
        return Object.assign({
            connection_state: connection,
            audio_enabled: !muted,
            share_state: sharing ? "screen" : "audio",
            webcam_enabled: camera
        }, fields);
    }
    function call(connection, since, own, people) {
        return Object.assign({
            id: "c-1",
            local: root.person({ is_current_user: true }, connection, ...own),
            participants: people
        }, since === null ? {} : { started_at: since });
    }
    function ada(sharing, camera) {
        return root.person({ id: 7, full_name: "Ada Brightwater", email: "ada@example.com" },
            "connected", false, sharing, camera);
    }
    readonly property var cal: root.person({ id: 9, full_name: "Cal Dunmore" }, "connected",
        false, false, false)

    Connections {
        target: Tuple

        function onCallChanged() {
            Qt.callLater(root.heard);
        }

        function onAnsweringChanged() {
            Qt.callLater(root.heard);
        }
    }

    Connections {
        target: Niri

        function onSpawnedChanged() {
            Qt.callLater(root.heard);
        }
    }

    Component.onCompleted: root.step("the daemon", () => Tuple.answering, root.first)

    function first() {
        expect("no call yet", !Tuple.calling && Tuple.ringing === "", JSON.stringify(Tuple.call));
        expect("the room Tuple lists", Tuple.room === "https://tuple.app/c/listed", Tuple.room);
        root.says(root.call("signaling", null, [true, false, false], [root.person(
            { id: 7, full_name: "Ada Brightwater", email: "ada@example.com" },
            "signaling", true, false, false)]));
        root.step("signaling", () => Tuple.calling, root.signaling);
    }

    function signaling() {
        expect("connecting", Tuple.connecting, Tuple.connecting);
        expect("not muted before it connects", Tuple.muted === null, Tuple.muted);
        expect("named by Tuple", Tuple.name("7") === "Ada Brightwater", Tuple.name("7"));
        expect("since it was heard", Math.abs(Tuple.since - Date.now()) < 5000, Tuple.since);
        root.says(root.call("connected", root.started, [true, false, false], [root.ada(false, false)]));
        root.step("connected", () => !Tuple.connecting, root.connected);
    }

    function connected() {
        expect("since it connected", Tuple.since === root.started, Tuple.since);
        expect("muted, once connected", Tuple.muted === true, Tuple.muted);
        expect("who's in it", JSON.stringify(Tuple.others) === "[\"7\"]", JSON.stringify(Tuple.others));
        expect("said as company", Tuple.company === "with Ada Brightwater", Tuple.company);
        root.says(root.call("connected", root.started, [false, true, false], [root.ada(false, true)]));
        root.step("sharing", () => Tuple.sending, root.sharing);
    }

    function sharing() {
        expect("heard again", Tuple.muted === false, Tuple.muted);
        expect("her camera", JSON.stringify(Tuple.watching) === "[\"7\"]", JSON.stringify(Tuple.watching));
        expect("no one else's screen", Tuple.showing.length === 0, JSON.stringify(Tuple.showing));
        root.says(root.call("connected", root.started, [false, false, false],
            [root.ada(true, false), root.cal]));
        root.step("told Cal came", () => /Cal Dunmore joined the call/.test(root.told), root.came);
    }

    function came() {
        expect("those already there weren't news", !/Ada Brightwater joined/.test(root.told), root.told);
        expect("her screen", JSON.stringify(Tuple.showing) === "[\"7\"]", JSON.stringify(Tuple.showing));
        expect("mine stopped", !Tuple.sending, Tuple.sending);
        root.says(root.call("connected", root.started, [false, false, false], []));
        root.step("told everyone left", () => /You're the only one still in it/.test(root.told), root.alone);
    }

    function alone() {
        Tuple.mute(true);
        root.step("asked to mute", () => /^call mute$/m.test(root.asked), root.muting);
    }

    function muting() {
        root.says(null);
        root.step("the call ending", () => !Tuple.calling, root.ended);
    }

    function ended() {
        expect("no mute without a call", Tuple.muted === null, Tuple.muted);
        Tuple.open();
        root.step("the room's link copied",
            () => Niri.spawned.some(command => command.join(" ") === "wl-copy https://tuple.app/c/opened"),
            root.opened);
    }

    function opened() {
        root.step("the room opened kept", () => Tuple.room === "https://tuple.app/c/opened", root.kept);
    }

    function kept() {
        root.logs("received incoming call: 7");
        root.step("ringing", () => Tuple.ringing === "7", root.ringing);
    }

    function ringing() {
        expect("not a call yet", !Tuple.calling, Tuple.calling);
        expect("the caller named", Tuple.name(Tuple.ringing) === "Ada Brightwater", Tuple.name("7"));
        root.step("told Ada's calling", () => /Ada Brightwater is calling/.test(root.told), root.rang);
    }

    function rang() {
        root.logs("call ended");
        root.step("told it was missed", () => /Missed a call from Ada Brightwater/.test(root.told),
            root.missed);
    }

    function missed() {
        expect("no longer ringing", Tuple.ringing === "", Tuple.ringing);
        Qt.exit(0);
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }
}

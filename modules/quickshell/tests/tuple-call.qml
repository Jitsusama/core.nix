// Drives the shell's Tuple (modules/tuple/Tuple.qml) through Quickshell
// itself, over the real tuple-call daemon following a stand-in tuple: its
// stream says each line this writes to it, and any other command is kept,
// as a stand-in notify-send keeps what it was asked to say and a stand-in
// busctl the notices it was asked to close. Joel's room is
// the one Tuple lists until he opens one; a call connecting says who's in
// it by name and since when; muting counts only once connected; whose
// screen and camera are shown, and Joel's own, are said; someone coming and
// going is told once the call has settled; muting, opening the room and
// Joel's camera are asked of the daemon, which runs Tuple for each, a
// camera stopped turning the row back to the picker before Tuple's stream
// says so, a stop Tuple refuses told as a kept notice, its reason as
// plain text, offering the call's window if Tuple has one as it answers,
// and putting away the notice before it, and a picker it refuses told as
// a passing one, though neither once the camera is said off or the call
// is over by the time Tuple answers; the call ending ends it and is
// kept in Recent with everyone who was in it, though they'd left; and a
// ring and a missed call, from Tuple's log, are told, the missed call kept
// in Recent ahead of the call; and a call in the room Joel opened is kept
// as his, where the first was kept with nobody seen to ring. It exits 0
// once every case holds, and otherwise 1, saying which didn't.
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

    // What notify-send was asked to say, what was asked of Tuple, and the
    // notices busctl was asked to close, as each line their stand-ins kept.
    property string told: ""
    property string asked: ""
    property string closed: ""

    Process {
        id: reading
        command: ["sh", "-c", "cat \"$0/told\"; echo ---; cat \"$0/asked\" 2>/dev/null; "
            + "echo ---; cat \"$0/closed\" 2>/dev/null", root.runtime]
        stdout: StdioCollector {
            onStreamFinished: {
                const [told, asked, closed] = this.text.split("---\n");
                root.told = told;
                root.asked = asked ?? "";
                root.closed = closed ?? "";
                Qt.callLater(root.heard);
            }
        }
    }

    Timer {
        // Reads again a while after each case asks, as notices are sent
        // and Tuple is run.
        interval: 200
        repeat: true
        running: /^(told|asked|closed)/.test(root.waiting)
        onTriggered: reading.running = true
    }

    readonly property real started: Date.now() - 60 * 1000
    // When the call in the room Joel opened connected, set as it does.
    property real inRoom: 0
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

    // Hello says the daemon answers before its watch's answer brings the
    // call, so the first step waits for the room as well.
    Component.onCompleted: root.step("the daemon", () => Tuple.answering && Tuple.room !== "",
        root.first)

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

    // The camera's row in a call: Tuple's picker while Joel's camera is
    // off, and stopping it while it's on, each as Tuple's webcam command.
    function camera() {
        return Tuple.controls.find(one => one.key === "tuple:camera") ?? null;
    }

    function alone() {
        const row = root.camera();
        expect("the camera's picker offered in a call",
            row?.name === "Show My Camera in the Call", JSON.stringify(row));
        row?.run();
        root.step("asked for the camera's picker", () => /^webcam start$/m.test(root.asked),
            root.picked);
    }

    function picked() {
        root.says(root.call("connected", root.started, [false, false, true], []));
        root.step("my camera on", () => Tuple.filming, root.filming);
    }

    function filming() {
        const row = root.camera();
        expect("my camera stopped from the call",
            row?.name === "Stop My Camera in the Call", JSON.stringify(row));
        row?.run();
        root.step("asked to stop the camera", () => /^webcam stop$/m.test(root.asked),
            root.stopped);
    }

    // Tuple's stream still says the camera's on, so only the stop done
    // can say it's off. The stand-in refuses each camera command the
    // second time it's asked.
    function stopped() {
        root.step("my camera off, ahead of Tuple's stream", () => !Tuple.filming, root.off);
    }

    function off() {
        const row = root.camera();
        expect("the camera's picker offered again once it's off",
            row?.name === "Show My Camera in the Call", JSON.stringify(row));
        root.says(root.call("connected", root.started, [false, false, true], []));
        root.step("my camera on again", () => Tuple.filming, root.refusing);
    }

    function refusing() {
        const row = root.camera();
        expect("my camera stopped again from the call",
            row?.name === "Stop My Camera in the Call", JSON.stringify(row));
        row?.run();
        root.step("told my camera may still be in the call",
            () => /Your camera may still be in the call/.test(root.told), root.refused);
    }

    // The last line a notice of Tuple's was told in, saying what's given.
    function noticeSaying(said) {
        return root.told.split("\n").filter(line => line.includes(said)).pop() ?? "";
    }

    // How many notices of Tuple's said what's given.
    function noticesSaying(said) {
        return root.told.split("\n").filter(line => line.includes(said)).length;
    }

    // How many times Tuple was asked what's given.
    function timesAsked(command) {
        return root.asked.split("\n").filter(line => line === command).length;
    }

    // Tells Tuple's stand-in to answer the camera command asked the time
    // given (people.nix says how).
    function letTupleAnswer(command, time) {
        root.write(`${root.runtime}/go-${command}-${time}`, "");
    }

    // A stop Tuple refused is told with its reason as plain text after
    // notify-send's flags, kept and waited on, and leaves the camera said
    // on; with no window of Tuple's, it offers none.
    function refused() {
        const told = root.noticeSaying("may still be in the call");
        expect("told why as plain text, kept and waited on, offering nothing",
            told.includes("-- Your camera may still be in the call no webcam to stop &lt;&amp;&gt;")
            && !told.includes("--transient") && told.includes("--wait")
            && !told.includes("--action"), told);
        expect("a refused stop leaves the camera on", Tuple.filming, Tuple.filming);
        root.camera()?.run();
        root.step("asked a third stop", () => root.timesAsked("webcam stop") === 3,
            root.stoppingBeforeItsWindow);
    }

    // Tuple's window opens while Tuple has yet to answer the stop, so it's
    // offered once Tuple refuses.
    function stoppingBeforeItsWindow() {
        Niri.windows = { "9": { id: 9, app_id: "Tuple" } };
        root.letTupleAnswer("stop", 3);
        root.step("told again my camera may still be in the call",
            () => root.noticesSaying("may still be in the call") === 2,
            root.refusedBesideItsWindow);
    }

    // A new notice of a stop refused puts away the one before it, closed by
    // its id, the line it was told on.
    function refusedBesideItsWindow() {
        const told = root.noticeSaying("may still be in the call");
        expect("told why, kept, offering the call's window",
            told.includes("no webcam to stop") && !told.includes("--transient")
            && !told.includes("--wait") && told.includes("--action=camera=Go To the Call"), told);
        const first = root.told.split("\n")
            .findIndex(line => line.includes("may still be in the call")) + 1;
        root.step("closed the first notice of the stop",
            () => root.closed.split("\n")
                .some(line => line.endsWith(` CloseNotification u ${first}`)),
            root.withdrawnOnce);
    }

    // A fourth stop, which Tuple refuses only once its stream has said the
    // camera off, so it's no news.
    function withdrawnOnce() {
        root.camera()?.run();
        root.step("asked a fourth stop", () => root.timesAsked("webcam stop") === 4,
            root.stoppingAStaleOne);
    }

    function stoppingAStaleOne() {
        Niri.windows = {};
        root.says(root.call("connected", root.started, [false, false, false], []));
        root.step("my camera off, as Tuple's stream says", () => !Tuple.filming, root.saidOff);
    }

    function saidOff() {
        root.letTupleAnswer("stop", 4);
        root.picking();
    }

    function picking() {
        const row = root.camera();
        expect("the camera's picker offered once Tuple says it's off",
            row?.name === "Show My Camera in the Call", JSON.stringify(row));
        row?.run();
        root.step("told the camera's picker didn't open",
            () => /camera picker didn't open/.test(root.told), root.unpicked);
    }

    // A picker Tuple didn't open is told with its reason and goes, since
    // nothing was shared, offering nothing.
    function unpicked() {
        const told = root.noticeSaying("camera picker didn't open");
        expect("told why, not kept, offering nothing",
            told.includes("no webcam to start") && told.includes("--transient")
            && !told.includes("--action"), told);
        root.camera()?.run();
        root.step("asked for a third picker", () => root.timesAsked("webcam start") === 3,
            root.pickingAcrossTheEnd);
    }

    // A picker Tuple refuses only once the call is over, so it's no news.
    function pickingAcrossTheEnd() {
        Tuple.mute(true);
        root.step("asked to mute", () => /^call mute$/m.test(root.asked), root.muting);
    }

    function muting() {
        root.says(null);
        root.step("the call ending", () => !Tuple.calling, root.ended);
    }

    function ended() {
        root.letTupleAnswer("start", 3);
        expect("no mute without a call", Tuple.muted === null, Tuple.muted);
        expect("no camera without a call", root.camera() === null, JSON.stringify(root.camera()));
        const kept = Tuple.recent[0] ?? {};
        const had = (kept.people ?? []).map(one => `${one.id} ${one.name} ${one.email}`);
        expect("kept in Recent with everyone who was in it", Tuple.recent.length === 1
            && kept.since === root.started && kept.missed === false
            && kept.ended >= Date.now() - 5000
            && had.join(", ") === "7 Ada Brightwater ada@example.com, 9 Cal Dunmore ",
            JSON.stringify(Tuple.recent));
        // Nothing tuple-call heard began it, no ring nor the room opened.
        expect("kept with nobody seen to ring", kept.caller === null, JSON.stringify(kept));
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
        expect("kept in Recent, newest first", Tuple.recent.length === 2 && Tuple.recent[0].missed
            && Tuple.recent[0].people[0]?.name === "Ada Brightwater" && !Tuple.recent[1].missed,
            JSON.stringify(Tuple.recent));
        // Ada comes to the room Joel opened a moment ago, and so to his call.
        root.inRoom = Date.now();
        root.says(root.call("connected", root.inRoom, [false, false, false],
            [root.ada(false, false)]));
        root.step("in Joel's room", () => Tuple.calling && !Tuple.connecting, root.inHisRoom);
    }

    function inHisRoom() {
        root.says(null);
        root.step("Joel's call kept", () => Tuple.recent.length === 3, root.his);
    }

    function his() {
        const kept = Tuple.recent[0];
        expect("kept as Joel's", kept.since === root.inRoom && kept.caller === "",
            JSON.stringify(Tuple.recent));
        // Tuple refused the stale stop and picker long since.
        expect("a stop refused once the camera was said off told nothing",
            root.timesAsked("webcam stop") === 4
            && root.noticesSaying("may still be in the call") === 2, root.told);
        expect("a picker refused once the call was over told nothing",
            root.timesAsked("webcam start") === 3
            && root.noticesSaying("picker") === 1, root.told);
        Qt.exit(0);
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }
}

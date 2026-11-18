// Drives a microphone's Noise Cancelling row in the launcher's Sound
// (modules/quickshell/launcher/Sound.qml) through noise-cancelled going
// away after it answered, through Quickshell itself, with a stand-in
// serving the trial's record on its socket as the daemon does. While it
// answers, the row says the cleaner it runs; once its socket's gone, the
// row offers to start it, rather than saying the cleaner it ran last as
// though it still ran, as it did on Joel's machine with the trial stopped
// (2026-11-13). It exits 0 once every case holds, and otherwise 1, saying
// which didn't.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")

    // A microphone as the sound server holds one.
    QtObject {
        id: microphone

        property string name: "usb-desk-microphone"
        property string nickname: ""
        property string description: name
        property bool isSink: false
        property bool isStream: false
        property int type: PwNodeType.AudioSource
        property var properties: ({ "device.bus": "usb" })
        property var audio: ({ volume: 0.5, muted: false })
    }

    // The launcher as Sound asks it, Sound's rows unwanted.
    QtObject {
        id: asking

        property bool open: false
        readonly property var held: []
        property var selected: null
        property bool drawerOut: false

        function wants(source) {
            return false;
        }

        function becomesAll() {
        }

        function becameNot() {
        }
    }

    Launcher.Sound {
        id: sound

        launcher: asking
        serverNodes: [microphone]
    }

    // The trial as the daemon sends it idle, cleaning with DeepFilterNet.
    readonly property var trial: ({
            key: "trial:now", kind: "trial", heard: 1000, stage: "idle", why: null,
            microphone: null, seconds: 6, until: null, clips: [], playing: null,
            picked: null, revealed: null, costs: null, marks: null, running: "deepfilternet"
        })

    // The connection watching the trial, closed once the daemon goes.
    property var watcher: null

    SocketServer {
        id: server

        active: true
        path: `${root.runtime}/daemons/noise-cancelled.sock`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer = { id: ask.id, unwatched: true };
                    if (ask.ask === "hello")
                        answer = { id: ask.id, version: 1, domain: "noise cancelling",
                            state: "answering" };
                    else if (ask.ask === "watch") {
                        root.watcher = asked;
                        answer = { id: ask.id, records: [root.trial] };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    readonly property string startVerb: Words.verb("sound:by-ear:down")

    // Each case waits on what the daemon's connection says next, looked at
    // every tenth of a second; five seconds without, it fails saying what.
    property var pending: null
    property int waited: 0

    function when(what, holds, step) {
        pending = { what: what, holds: holds, step: step };
        waited = 0;
    }

    Timer {
        interval: 100
        repeat: true
        running: root.pending !== null
        onTriggered: {
            const waiting = root.pending;
            try {
                if (waiting.holds()) {
                    root.pending = null;
                    waiting.step();
                } else if (++root.waited > 50)
                    Expect.fail(`${waiting.what}: never held`);
            } catch (error) {
                Expect.fail(error.message ?? String(error));
            }
        }
    }

    Component.onCompleted: when("the trial answering", () => NoiseCancelling.record !== null
        && root.watcher !== null, answering)

    function answering() {
        const row = sound.noisePart(microphone);
        Expect.expect("answering, the row says the cleaner it runs",
            row.verb !== root.startVerb && row.side === "DeepFilterNet, full",
            `${row.verb}: ${row.side}`);
        // Its socket goes, as stopping the trial's socket takes it away,
        // and the connection with it.
        server.active = false;
        root.watcher.connected = false;
        when("the trial not answering", () => NoiseCancelling.state === "not answering",
            gone);
    }

    function gone() {
        const row = sound.noisePart(microphone);
        Expect.expect("its socket gone, the row offers to start it",
            NoiseCancelling.record === null && row.verb === root.startVerb,
            `${JSON.stringify(NoiseCancelling.record?.running)}, ${row.verb}: ${row.side}`);
        Qt.exit(0);
    }
}

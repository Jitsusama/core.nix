// Drives pi's sessions (modules/quickshell/Sessions.qml) through Quickshell
// itself, over the records and processes tests/sessions-state.sh lays out,
// with niri, kitty's kitten and the notices stood in. Every window niri
// shows carries the same title, so nothing can be found by one. A session
// runs while the process its record names does, on this boot, started when
// the record says; its window is found through the kitty it runs in, by its
// record's terminal or its process's environment, and among a kitty's
// several windows by kitty's own list; Go To focuses that window, or asks
// kitty when niri's isn't known; a record that goes, or a process that
// ends, takes its session with it. A machine's node that isn't listening
// as the shell starts, as when a switch restarts it, is watched once it
// listens, however long after. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string state: Quickshell.env("XDG_STATE_HOME")
    property string stage: "listed"

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    function addresses() {
        return Sessions.running.map(session => session.address).sort().join(" ");
    }

    function find(address) {
        return Sessions.running.find(session => session.address === address);
    }

    Connections {
        target: Sessions
        function onRunningChanged() {
            Qt.callLater(root.check);
        }
    }

    Component.onCompleted: Qt.callLater(check)

    function check() {
        try {
            if (stage === "listed")
                listed();
            else if (stage === "forgotten")
                forgotten();
            else if (stage === "ended")
                ended();
            else if (stage === "node")
                watched();
        } catch (error) {
            fail(error.message);
        }
    }

    // Waits until every running session is listed and the kitty with two
    // windows has said which holds its session.
    function listed() {
        const other = find("OTHER2");
        if (addresses() !== "LIVE01 NOWIN6 OTHER2" || !other || !other.window)
            return;
        const live = find("LIVE01");
        expect("a kitty with one window has it", live.window && live.window.id === 10,
            JSON.stringify(live.window));
        expect("its kitty from its process's environment",
            JSON.stringify(live.kitty) === '{"socket":"unix:@kitty-500","window":3}',
            JSON.stringify(live.kitty));
        expect("its label shown, cut at its folder",
            live.title === "Live One" && live.folder === "core.nix", `${live.title} ${live.folder}`);
        expect("the second of kitty's windows is niri's second", other.window.id === 21,
            other.window.id);
        expect("its kitty from its record's terminal",
            JSON.stringify(other.kitty) === '{"socket":"unix:@kitty-600","window":4}',
            JSON.stringify(other.kitty));
        expect("one ended, one from another boot and one whose pid was used again aren't there",
            Object.keys(Sessions.present).sort().join(" ") === "LIVE01 NOWIN6 OTHER2",
            Object.keys(Sessions.present).sort().join(" "));
        expect("working, as its record says", Sessions.working === 1, Sessions.working);
        expect("a session is its window", Sessions.isOf(other) === "window:21", Sessions.isOf(other));

        Sessions.goTo(other);
        expect("Go To focuses its window", JSON.stringify(Niri.acted) === '{"FocusWindow":{"id":21}}',
            JSON.stringify(Niri.acted));
        const unseen = find("NOWIN6");
        expect("with no window niri shows, none", unseen.window === null,
            JSON.stringify(unseen.window));
        expect("a session with no window is itself", Sessions.isOf(unseen) === "agent:NOWIN6",
            Sessions.isOf(unseen));
        Sessions.goTo(unseen);
        expect("Go To asks its kitty", Niri.spawned.join(" ")
            === "kitten @ --to unix:@kitty-700 focus-window --match id:2", Niri.spawned.join(" "));

        stage = "forgotten";
        removing.running = true;
    }

    // A record that goes takes its session.
    function forgotten() {
        if (addresses() !== "NOWIN6 OTHER2")
            return;
        stage = "ended";
        ending.running = true;
    }

    // A process that ends takes its session once a window goes, as its
    // window does when kitty's program ends.
    function ended() {
        if (addresses() !== "NOWIN6")
            return;
        // Refused since the shell started, the node listens now.
        stage = "node";
        node.active = true;
    }

    // The node's sessions are heard once it listens: asked again, the
    // shell watches it, and what it tells is held.
    function watched() {
        if (Sessions.node === null)
            return;
        console.log("a session is known by its own record");
        Qt.callLater(() => Qt.exit(0));
    }

    Connections {
        target: Sessions
        function onNodeChanged() {
            Qt.callLater(root.check);
        }
    }

    // A stand-in for the machine's node, listening only once asked, which
    // answers a watch with no sessions.
    SocketServer {
        id: node
        active: false
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/harbour.sock`
        handler: Socket {
            id: asked
            parser: SplitParser {
                onRead: line => {
                    if (!line.includes("Watch"))
                        return;
                    asked.write('{"SessionsChanged":{"sessions":[]}}\n');
                    asked.flush();
                }
            }
        }
    }

    Process {
        id: removing
        command: ["rm", `${root.state}/pi/agentic-harness.pi/mesh-workflow/presence/live.json`]
    }

    Process {
        id: ending
        command: ["kill", Quickshell.env("OTHER_PID")]
        onExited: {
            const windows = Object.assign({}, Niri.windows);
            delete windows[21];
            Niri.windows = windows;
            Niri.revision += 1;
        }
    }

    Timer {
        interval: 30000
        running: true
        onTriggered: root.fail(`still at ${root.stage}: ${root.addresses()}`)
    }
}

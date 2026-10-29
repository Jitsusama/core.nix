// Drives pi's sessions (modules/quickshell/Sessions.qml) through Quickshell
// itself, over the records and processes tests/sessions-state.sh lays out,
// with niri, kitty's kitten and the notices stood in. Every window niri
// shows carries the same title, so nothing can be found by one. A session
// runs while the process its record names does, on this boot, started when
// the record says; its window is found through the kitty it runs in, by its
// record's terminal or its process's environment, and among a kitty's
// several windows by kitty's own list; Go To focuses that window, or asks
// kitty when niri's isn't known; a record that goes, or a process that
// ends, takes its session with it. What an earlier boot's end took away
// is offered whole, reopened held in its own folder, until put off. A
// machine's node that isn't listening as the shell starts, as when a
// switch restarts it, is watched once it listens, however long after.
// What it tells of sessions on the realm's other machines is held too:
// each says its machine, after this machine's; Go To joins one by the
// words its node gives; one out of reach stays, saying so, and isn't
// joined; one last heard of before this boot is passed over; and a steer
// to one goes through the node, its refusal said. It exits 0 once every
// case holds, and otherwise 1, saying which didn't.
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
        lastBoot();

        stage = "forgotten";
        removing.running = true;
    }

    // The sessions another boot's end took away are offered, but for one
    // with no folder or an id pi's command line can't take; each reopens
    // in its own window and folder, held; once put off they're gone, and
    // stay gone however often the shell reads it again.
    function lastBoot() {
        expect("only the whole ones of another boot",
            Sessions.lastBoot.map(one => one.id).join(" ") === "s-before",
            JSON.stringify(Sessions.lastBoot));
        Sessions.reopen(Sessions.lastBoot[0]);
        // Held as every start is, by the realm's node where there is one.
        const reopening = Niri.spawned.join(" ");
        expect("reopened in its own window and folder",
            reopening.startsWith("kitty -1 --instance-group pi-sessions --directory /tmp zsh -lic ")
            && reopening.endsWith(" pi --session s-before"), reopening);
        Sessions.putOffLastBoot();
        expect("put off, none", Sessions.lastBoot.length === 0, JSON.stringify(Sessions.lastBoot));
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
        if (Sessions.node === null || addresses() !== "FARAWY OUTRCH")
            return;
        const far = find("FARAWY");
        const out = find("OUTRCH");
        expect("one elsewhere says its machine", !far.here && far.machine === "lighthouse"
            && far.reachable, JSON.stringify(far));
        expect("one out of reach says so", !out.reachable && out.attach === null,
            JSON.stringify(out));
        Sessions.goTo(far);
        expect("Go To joins it by its node's words", Niri.spawned.join(" ")
            === "kitty -1 --instance-group pi-sessions zsh -lic mycelium attach r-far",
            Niri.spawned.join(" "));
        Sessions.goTo(out);
        expect("one out of reach isn't joined", root.unjoined === "OUTRCH", root.unjoined);
        stage = "steered";
        Sessions.steer("farawy", { act: "pause" });
    }

    property string unjoined: ""

    // A steer elsewhere goes through the node, whose refusal is said.
    Connections {
        target: Sessions

        function onUnjoined(session) {
            root.unjoined = session.address;
        }

        function onUnsteered(session, why) {
            try {
                root.expect("a steer elsewhere is refused by the node, saying why",
                    root.stage === "steered" && session.address === "FARAWY"
                    && why === "lighthouse is out of reach, so FARAWY can't be told now", why);
                root.steeredBy();
            } catch (error) {
                root.fail(error.message);
            }
        }
    }
    Process {
        id: steerSaid
        command: ["cat", `${root.state}/steered`]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    root.expect("the node is asked with the steer",
                        text.startsWith("--realm harbour steer FARAWY\n{")
                        && text.includes('"act":"pause"'), text);
                    console.log("a session is known by its own record");
                    Qt.callLater(() => Qt.exit(0));
                } catch (error) {
                    root.fail(error.message);
                }
            }
        }
    }
    function steeredBy() {
        steerSaid.running = true;
    }

    Connections {
        target: Sessions
        function onNodeChanged() {
            Qt.callLater(root.check);
        }
    }

    // A stand-in for the machine's node, listening only once asked, which
    // answers a watch with three sessions on another machine: one held
    // there, one out of reach, and one last heard of before this boot.
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
                    asked.write(`${JSON.stringify({ SessionsChanged: { sessions: root.elsewhere() } })}\n`);
                    asked.flush();
                }
            }
        }
    }

    function elsewhere() {
        const now = new Date().toISOString();
        const one = (id, address, state, at, attach) => ({
            id: id, address: address, machine: "lighthouse", state: state, at: at,
            attach: attach, window: null, groups: [], title: "",
            presence: { addresses: [address], title: `\u03c0 - ${address} - /far`, busy: true }
        });
        return [
            one("r-far", "FARAWY", "busy", now, "mycelium attach r-far"),
            one("r-out", "OUTRCH", "unreachable", now, null),
            one("r-old", "OLDONE", "busy", "2000-01-01T00:00:00Z", "mycelium attach r-old")
        ];
    }

    Process {
        id: removing
        command: ["rm", `${root.state}/pi/agentic-harness.pi/mesh-workflow/presence/live.json`]
    }

    Process {
        id: ending
        // Until it has ended, not only been told to: a window gone before
        // its process is would leave the session listed, as nothing else
        // asks again, which a busy machine's checks hit.
        command: ["sh", "-c", 'kill "$1"; while kill -0 "$1" 2>/dev/null'
            + ' && [ "$(cut -d " " -f 3 "/proc/$1/stat")" != Z ]; do sleep 0.05; done',
            "end", Quickshell.env("OTHER_PID")]
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

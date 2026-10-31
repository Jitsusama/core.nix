pragma ComponentBehavior: Bound
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
// is offered whole, reopened held in its own folder, until put off, what
// another machine carried here is offered after it, the newest first, and
// never put off with it, and a
// window it had joined to a session another machine holds is joined again
// by its words, once; a closed session resumes in its own folder, and one whose folder is gone
// opens nowhere but its quest's start folder, never the home folder. A
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
            else if (stage === "reopening")
                reopening();
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
        stage = "reopening";
    }

    // Why each that didn't open as asked didn't, as its caller is told:
    // the id, the folder gone and where it started afresh instead.
    property var unopened: []
    function told(id) {
        return (cwd, startedIn) => {
            unopened = unopened.concat([`${id} ${cwd} ${startedIn}`]);
            Qt.callLater(root.check);
        };
    }

    // The sessions another boot's end took away are offered, but for one
    // with no folder or an id pi's command line can't take; each reopens
    // in its own window and folder, held, and one whose folder is gone
    // since opens nowhere, never in the home folder. A closed session
    // resumes in the folder its own record says, which needn't be the
    // home folder; one whose folder is gone starts its quest afresh in
    // the quest's start folder, or nowhere when it has none.
    function lastBoot() {
        expect("only the whole ones of another boot",
            Sessions.lastBoot.map(one => one.id).join(" ") === "s-before s-given-back s-far",
            JSON.stringify(Sessions.lastBoot));
        // Those another machine carried here apart, the newest first, and
        // one saying where from badly left out.
        expect("the carried, newest first",
            Sessions.carried.map(one => one.id).join(" ") === "s-ferry s-ferried",
            JSON.stringify(Sessions.carried));
        expect("the last boot's, then the carried", Sessions.toReopen.map(one => one.id).join(" ")
            === "s-before s-given-back s-far s-ferry s-ferried", JSON.stringify(Sessions.toReopen));
        Sessions.lastBoot.forEach(one => Sessions.reopen(one, told(one.id)));
        // One held elsewhere is joined again once, however often asked.
        Sessions.reopen(Sessions.lastBoot[2], told("s-far"));
        Sessions.resume("s-kept", "QEST-KEPT", "/tmp", told("s-kept"));
        Sessions.resume("s-gone", "QEST-GONE", root.state, told("s-gone"));
        Sessions.resume("s-gone", "QEST-NONE", "", told("s-none"));
    }

    readonly property string kitty: "kitty -1 --instance-group pi-sessions --directory"
    function reopening() {
        const kitties = Niri.spawns.filter(command => command.startsWith("kitty "));
        if (kitties.length < 4 || unopened.length < 3)
            return;
        // Held as every start is, by the realm's node where there is one.
        const opened = command => kitties.some(one => one.startsWith(command[0])
            && one.endsWith(command[1]));
        expect("reopened in its own window and folder",
            opened([`${kitty} /tmp zsh -lic `, " pi --session s-before"]), kitties.join("\n"));
        expect("resumed in the folder its record says",
            opened([`${kitty} ${state}/trees/karst zsh -lic `, " pi --session s-kept"]),
            kitties.join("\n"));
        expect("its folder gone, its quest started afresh in the quest's",
            opened([`${kitty} ${state} env QUEST_WORKFLOW_AUTOLOAD_ID=QEST-GONE zsh -lic `, " pi"]),
            kitties.join("\n"));
        expect("one held elsewhere joined again by its words, in its folder here",
            kitties.includes(`${kitty} /tmp ssh -t lighthouse mycelium attach s-far`),
            kitties.join("\n"));
        expect("four opened, none in the home folder nor a gone one", kitties.length === 4
            && !kitties.some(one => one.includes(`--directory ${Quickshell.env("HOME")} `)
                || one.includes("given-back") || one.includes("QEST-NONE")),
            kitties.join("\n"));
        expect("each not opened as asked told why", JSON.stringify(unopened.slice().sort())
            === JSON.stringify(["s-given-back /given-back ",
                `s-gone ${state}/trees/given-back ${state}`, `s-none ${state}/trees/given-back `]),
            JSON.stringify(unopened));
        Sessions.putOffLastBoot();
        expect("put off, none", Sessions.lastBoot.length === 0, JSON.stringify(Sessions.lastBoot));
        // The carried are never put off with the last boot.
        expect("put off, the carried still", Sessions.toReopen.map(one => one.id).join(" ")
            === "s-ferry s-ferried", JSON.stringify(Sessions.toReopen));
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
                    // Asked as mycelium's node takes a watch, its groups, if
                    // any, one pattern of group names, or refused as it
                    // refuses one. The shell hears the sessions alone, so a
                    // watch naming groups, whose messages it never reads, is
                    // a failure too.
                    const watch = JSON.parse(line).Watch;
                    if (!watch || !root.takes(watch)) {
                        root.fail(`a watch the node would refuse: ${line}`);
                        asked.write('{"Err":"not a request"}\n');
                        asked.flush();
                        return;
                    }
                    asked.write(`${JSON.stringify({ SessionsChanged: { sessions: root.elsewhere() } })}\n`);
                    asked.flush();
                }
            }
        }
    }

    // A pattern of groups as mycelium takes one (its group.rs): tokens of
    // a-z, 0-9, `-` and `_` between dots, any one `*`, and `>` last only.
    function pattern(groups) {
        return typeof groups === "string"
            && /^([a-z0-9_-]+|\*)(\.([a-z0-9_-]+|\*))*(\.>)?$|^>$/.test(groups);
    }

    // Whether the node takes this watch, and it asks for the sessions
    // alone: no groups (absent or null, as the node reads both), and a
    // `recent`, if given, a number.
    function takes(watch) {
        const groups = watch.groups ?? null;
        if (groups !== null && !root.pattern(groups)) {
            return false;
        }
        if (watch.recent !== undefined && typeof watch.recent !== "number") {
            return false;
        }
        if (groups !== null) {
            root.fail(`a watch asking for the messages of ${groups}, which the shell never reads`);
        }
        return true;
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

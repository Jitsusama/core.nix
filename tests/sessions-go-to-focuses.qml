pragma ComponentBehavior: Bound
// Drives Go To on sessions with no window here (modules/quickshell/Sessions.qml),
// with the machine's node and niri stood in: one pi's host runs here, which
// a viewer opens for, one running on another machine, which a join opens
// for, and one held here, which an attach opens for. Each window opens on
// the workspace Go To picks, away from the one Joel is on, and comes to the
// front once niri lists it, focused by the shell, since the sessions kitty
// that draws it never had niri's token to take the focus itself (Joel,
// 2026-11-18 10:44: "it isn't coming to the forefront"). One that comes too
// late, after Joel moved the focus himself, or after a later Go To, stays
// where it opened. It exits 0 once that holds, and otherwise 1, saying what
// didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    // Each Go To in turn: the session, the window niri lists for it, and
    // whether it should come to the front. `before` runs between Go To and
    // the listing, as what happens meanwhile.
    readonly property var steps: [
        { address: "HOSTED", window: 60, forward: true, path: "a hosted session's view" },
        { address: "AWAY", window: 61, forward: true, path: "a far session's join" },
        { address: "HELD", window: 62, forward: true, path: "a held session's attach" },
        { address: "LATE", window: 63, forward: false, path: "a window past patience",
            before: () => { Sessions.goToPatience = 0; } },
        { address: "MOVED", window: 64, forward: false, path: "a window after Joel moved",
            before: () => {
                Sessions.goToPatience = 30000;
                Niri.focused = 99;
            } },
        { address: "FIRST", window: 65, forward: false, path: "a window a later Go To replaced",
            before: () => {
                Niri.focused = null;
                root.goingTo("SECOND");
            } }
    ]
    property int step: 0
    property bool listed: false
    // When the window being waited on was listed, for its focus to follow.
    property real listedAt: 0

    function find(address) {
        return Sessions.running.find(session => session.address === address);
    }

    function record(id, fields) {
        return Object.assign({ key: `session:${id}`, kind: "session", heard: Date.now(), id,
            state: "idle", standing: "running", here: true, machine: Sessions.machine,
            at: new Date().toISOString(), window: null, held: null }, fields);
    }
    // A session pi's host runs here, as the node tells of it.
    function hosted(id, address) {
        return record(id, {
            attach: ["mycelium", "--realm", "harbour", "attach", "--host",
                "harbour-background", "--session", id],
            presence: { session: id, addresses: [address],
                owner: { pid: 4100, startedAt: 17, bootId: "b1", startTicks: 7,
                    session: id, host: "harbour-background" },
                hosted: { host: "harbour-background", conversation: id } } });
    }
    // A session running on another machine, joined by the node's words.
    function away(id, address) {
        return record(id, { state: "busy", here: false, machine: "elsewhere",
            reachable: true, attach: ["ssh", "-t", "--", "elsewhere", "mycelium", "attach", id],
            presence: { session: id, addresses: [address] } });
    }
    // A session here that the realm's holder keeps, with no window.
    function held(id, address) {
        return record(id, { held: id, presence: { session: id, addresses: [address] } });
    }
    readonly property var sessions: [hosted("s-hosted", "HOSTED"), away("s-away", "AWAY"),
        held("s-held", "HELD"), hosted("s-late", "LATE"), away("s-moved", "MOVED"),
        hosted("s-first", "FIRST"), away("s-second", "SECOND")]

    // The window niri was last told to open for a spawn, by app id.
    function openedFor(spawned) {
        const opens = Niri.acts.filter(act => act.OpenNextWindowOn)
            .map(act => act.OpenNextWindowOn);
        return opens.find(one => spawned.includes(`/pi-sessions/kitty.sock ${one.app_id} `))
            ?? null;
    }
    function focused(id) {
        return Niri.acts.some(act => act.FocusWindow?.id === id);
    }

    // Goes to one, and says what niri was told to open for it.
    function goingTo(address) {
        const before = Niri.spawns.length;
        Sessions.goTo(find(address));
        const spawned = Niri.spawns.slice(before);
        Expect.expect(`Go To on ${address} opens one window in the sessions kitty`,
            spawned.length === 1, Niri.spawns.join("\n"));
        const open = openedFor(spawned[0]);
        Expect.expect(`niri is told where ${address}'s window opens, away from Joel's`,
            open !== null && open.reference.Id !== 40, JSON.stringify(Niri.acts));
        return open;
    }
    function list(id, opened, workspace) {
        const window = { id, pid: id * 10, workspace_id: workspace, session: true, opened,
            layout: { pos_in_scrolling_layout: [1, 1] } };
        Niri.windows = Object.assign({}, Niri.windows, { [id]: window });
        Niri.revision += 1;
        listed = true;
        listedAt = Date.now();
        Niri.windowOpenedOrChanged(window);
    }

    // A window that should come forward does within a second; one that
    // shouldn't is still unfocused a second on.
    function judged(one) {
        const waited = Date.now() - listedAt;
        if (one.forward && focused(one.window))
            return true;
        if (one.forward)
            Expect.expect(`${one.path} came to the front once niri listed it`, waited < 1000,
                JSON.stringify(Niri.acts));
        else
            Expect.expect(`${one.path} stayed where it opened`, !focused(one.window),
                JSON.stringify(Niri.acts));
        return !one.forward && waited >= 1000;
    }

    function check() {
        try {
            if (step === steps.length) {
                done();
                return;
            }
            const one = steps[step];
            if (!listed) {
                if (!find(one.address))
                    return;
                const open = goingTo(one.address);
                (one.before ?? (() => {}))();
                list(one.window, open.app_id, open.reference.Id);
            } else if (judged(one)) {
                listed = false;
                step += 1;
            }
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    // A window of the same app id listed again, as niri tells of a title
    // changing, takes the focus no more.
    function done() {
        const asked = Niri.acts.filter(act => act.FocusWindow).length;
        Niri.windowOpenedOrChanged(Niri.windows["60"]);
        Expect.expect("a window already brought forward is never focused again",
            Niri.acts.filter(act => act.FocusWindow).length === asked,
            JSON.stringify(Niri.acts));
        console.log("go to brings a windowless session forward");
        Qt.exit(0);
    }

    Timer {
        running: true
        repeat: true
        interval: 100
        onTriggered: root.check()
    }

    // A stand-in for the machine's node, over the daemon interface, holding
    // the sessions with no window here.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/harbour.sock`
        handler: Socket {
            id: asked
            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer = { id: ask.id, version: 1, domain: "node", state: "running" };
                    if (ask.ask === "watch" && ask.kind === "session")
                        answer = { id: ask.id, records: root.sessions };
                    else if (ask.ask === "watch")
                        answer = { id: ask.id, records: [] };
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    Timer {
        running: true
        interval: 50000
        onTriggered: Expect.fail(`still at step ${root.step}: ${JSON.stringify(Niri.acts)}`)
    }
}

pragma ComponentBehavior: Bound
// Drives Go To on sessions with no window here (modules/quickshell/Sessions.qml),
// with the machine's node and niri stood in: one pi's host runs here, which
// a viewer opens for, and one running on another machine, which a join
// opens for. Each window opens on the workspace Go To picks, away from the
// one Joel is on, and comes to the front once niri lists it, focused by the
// shell, since the sessions kitty that draws it never had niri's token to
// take the focus itself (Joel, 2026-11-18 10:44: "it isn't coming to the
// forefront"). It exits 0 once that holds, and otherwise 1, saying what
// didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    property string stage: "listed"
    // When the window being waited on was listed, for its focus to follow.
    property real listedAt: 0

    function find(address) {
        return Sessions.running.find(session => session.address === address);
    }

    // A session pi's host runs here, as the node tells of it.
    function hosted(id, address) {
        return { key: `session:${id}`, kind: "session", heard: Date.now(), id,
            state: "idle", standing: "running", here: true, machine: Sessions.machine,
            at: new Date().toISOString(), window: null, held: null,
            attach: ["mycelium", "--realm", "harbour", "attach", "--host",
                "harbour-background", "--session", id],
            presence: { session: id, addresses: [address],
                owner: { pid: 4100, startedAt: 17, bootId: "b1", startTicks: 7,
                    session: id, host: "harbour-background" },
                hosted: { host: "harbour-background", conversation: id } } };
    }
    // A session running on another machine, joined by the node's words.
    function away(id, address) {
        return { key: `session:${id}`, kind: "session", heard: Date.now(), id,
            state: "busy", standing: "running", here: false, machine: "elsewhere",
            reachable: true, at: new Date().toISOString(), window: null, held: null,
            attach: ["ssh", "-t", "--", "elsewhere", "mycelium", "attach", id],
            presence: { session: id, addresses: [address] } };
    }

    // What niri was asked to open, by app id, and the window it lists then.
    function openedFor(spawned) {
        const opens = Niri.acts.filter(act => act.OpenNextWindowOn)
            .map(act => act.OpenNextWindowOn);
        return opens.find(one => spawned.includes(`/pi-sessions/kitty.sock ${one.app_id} `))
            ?? null;
    }
    function list(id, opened, workspace) {
        const window = { id, pid: id * 10, workspace_id: workspace, session: true, opened,
            layout: { pos_in_scrolling_layout: [1, 1] } };
        Niri.windows = Object.assign({}, Niri.windows, { [id]: window });
        Niri.revision += 1;
        root.listedAt = Date.now();
        Niri.windowOpenedOrChanged(window);
    }
    function focusedOn(id) {
        return Niri.acts.some(act => act.FocusWindow?.id === id);
    }

    // Goes to one, then lists the window niri opens for it.
    function goingTo(address, id, next) {
        const session = find(address);
        if (!session)
            return;
        const before = Niri.spawns.length;
        Sessions.goTo(session);
        const spawned = Niri.spawns.slice(before);
        Expect.expect(`Go To on ${address} opens one window in the sessions kitty`,
            spawned.length === 1, Niri.spawns.join("\n"));
        const open = openedFor(spawned[0]);
        Expect.expect(`niri is told where ${address}'s window opens, away from Joel's`,
            open !== null && open.reference.Id !== 40, JSON.stringify(Niri.acts));
        stage = next;
        list(id, open.app_id, open.reference.Id);
    }

    // The window comes to the front, by the shell's own ask of niri.
    function cameForward(address, id, next) {
        if (!focusedOn(id)) {
            Expect.expect(`${address}'s window came to the front once niri listed it`,
                Date.now() - root.listedAt < 1000, JSON.stringify(Niri.acts));
            return;
        }
        stage = next;
    }

    function check() {
        try {
            if (stage === "listed")
                goingTo("HOSTED", 60, "hosted");
            else if (stage === "hosted")
                cameForward("HOSTED", 60, "away");
            else if (stage === "away")
                goingTo("AWAY", 61, "joined");
            else if (stage === "joined")
                cameForward("AWAY", 61, "done");
            else if (stage === "done")
                done();
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
    // the two sessions with no window here.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/harbour.sock`
        handler: Socket {
            id: asked
            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer = { id: ask.id, version: 1, domain: "node", state: "running" };
                    if (ask.ask === "watch" && ask.kind === "session") {
                        answer = { id: ask.id, records: [root.hosted("s-hosted", "HOSTED"),
                            root.away("s-away", "AWAY")] };
                    } else if (ask.ask === "watch") {
                        answer = { id: ask.id, records: [] };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    Timer {
        running: true
        interval: 50000
        onTriggered: Expect.fail(`still at ${root.stage}: ${JSON.stringify(Niri.acts)}`)
    }
}

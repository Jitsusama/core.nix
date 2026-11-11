pragma ComponentBehavior: Bound
// Drives Reopen Them All over sessions pi's host runs
// (modules/quickshell/Sessions.qml), with the machine's node and niri stood
// in, and the windows daemon told as its watch is. Their host brought each
// conversation back as the machine started, so only a viewer is opened,
// never a pi: one the host serves opens quietly where the last boot kept
// it, on the screen's empty workspace, and holds there; one no host serves
// yet has its place kept, for its viewer to open once one does; one the
// boot kept no place of opens none. One bound for the workspace Joel is on
// waits until he leaves it, then opens there. It exits 0 once that holds,
// and otherwise 1, saying what didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect
import "Watch.js" as Watch

ShellRoot {
    id: root

    property string stage: "listed"

    function find(address) {
        return Sessions.running.find(session => session.address === address);
    }

    // The node's words to view one, as it gives them for a hosted session.
    function viewing(id) {
        return ["mycelium", "--realm", "harbour", "attach", "--host", "harbour-background",
            "--session", id];
    }

    // A session pi's host runs here, as the node tells of it.
    function hosted(id, address) {
        return { key: `session:${id}`, kind: "session", heard: Date.now(), id,
            state: "idle", standing: "running", here: true, machine: Sessions.machine,
            at: new Date().toISOString(), window: null, held: null, attach: viewing(id),
            presence: { session: id, addresses: [address],
                hosted: { host: "harbour-background", conversation: id } } };
    }

    // Where the last boot kept each window, by session id, on the monitor.
    function kept(places) {
        return id => places[id] ?? null;
    }
    function keptOn(workspace) {
        return { screen: "HP Z27k", output: "DP-4", workspace, column: 1, tile: 1 };
    }

    function viewerSpawns(id) {
        return Niri.spawns.filter(command => command.includes(` --class pi-hosted.${id} `));
    }
    function opensOn(id) {
        return Niri.acts.filter(act => act.OpenNextWindowOn?.app_id === `pi-hosted.${id}`)
            .map(act => act.OpenNextWindowOn);
    }

    function windowRecord(id, opened, workspace) {
        return { key: `window:${id}`, id, pid: id * 10, opened, workspace, column: null,
            down: null, order: 0 };
    }
    function windowsTell(said) {
        Sessions.windowsFrom.said(JSON.stringify(Object.assign({ id: "all" }, said)));
    }
    // niri's workspaces with Joel on `focused`.
    function joelOn(focused) {
        const now = {};
        for (const one of Object.values(Niri.workspaces))
            now[one.id] = Object.assign({}, one, { is_focused: one.id === focused });
        Niri.workspaces = now;
        Niri.revision += 1;
    }

    Component.onCompleted: windowsTell({ records: [windowRecord(90, "firefox", 40)] })

    function never(one, cwd) {
        Expect.fail(`${one.id} was looked for in ${cwd}, as a pi to start`);
    }

    function check() {
        try {
            if (stage === "listed")
                listed();
            else if (stage === "opening")
                opening();
            else if (stage === "walked")
                walked();
            else if (stage === "joel")
                joel();
            else if (stage === "waiting")
                waiting();
            else if (stage === "left")
                left();
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    // The session its host serves is listed with the words to view it,
    // and the last boot's three are reopened.
    function listed() {
        const viewed = find("VIEWED");
        if (!viewed)
            return;
        Expect.expect("one hosted here keeps the node's words to view it",
            JSON.stringify(viewed.attach) === JSON.stringify(viewing("s-viewed")),
            JSON.stringify(viewed));
        stage = "opening";
        Sessions.reopenAll([{ id: "s-viewed", cwd: "/tmp", hosted: true },
            { id: "s-unserved", cwd: "/tmp", hosted: true },
            { id: "s-placeless", cwd: "/tmp", hosted: true }], root.never,
            kept({ "s-viewed": keptOn(2), "s-unserved": keptOn(3) }), "");
    }

    // Its viewer opens alone, held, quietly on the monitor's empty
    // workspace, where its kept workspace isn't yet; then niri opens it.
    function opening() {
        const spawned = viewerSpawns("s-viewed");
        if (spawned.length === 0)
            return;
        Expect.expect("its viewer opens by the node's words, held, without the focus",
            spawned[0].startsWith("env -u XDG_ACTIVATION_TOKEN -u DESKTOP_STARTUP_ID ")
            && spawned[0].includes("MYCELIUM_APP_ID=pi-hosted.s-viewed ")
            && spawned[0].includes("mycelium --realm harbour hold -- "
                + viewing("s-viewed").join(" ")), spawned[0]);
        Expect.expect("niri is told where it opens: the monitor's empty workspace",
            JSON.stringify(opensOn("s-viewed")) === JSON.stringify([{
                app_id: "pi-hosted.s-viewed", reference: { Id: 41 }, column: 1 }]),
            JSON.stringify(Niri.acts));
        const window = { id: 50, pid: 500, workspace_id: 41, session: true,
            opened: "pi-hosted.s-viewed", layout: { pos_in_scrolling_layout: [1, 1] } };
        Niri.windows = Object.assign({}, Niri.windows, { "50": window });
        Niri.workspaces = Object.assign({}, Niri.workspaces, {
            "41": Object.assign({}, Niri.workspaces["41"], { active_window_id: 50 }),
            "42": { id: 42, idx: 3, output: "DP-4", is_focused: false, active_window_id: null }
        });
        Niri.revision += 1;
        windowsTell({ changed: [windowRecord(50, "pi-hosted.s-viewed", 41)] });
        stage = "walked";
        Niri.windowOpenedOrChanged(window);
    }

    // The walk goes on past it: the one no host serves has its place kept,
    // the one kept nowhere opens nothing, and no pi is started for any.
    function walked() {
        const unserved = Sessions.keptPlaces["s-unserved"];
        if (!unserved || (Sessions.reopening?.queue.length ?? 0) > 0)
            return;
        Expect.expect("one no host serves yet has its place kept, on the next empty workspace",
            unserved.workspace === 42 && unserved.column === 1, JSON.stringify(unserved));
        Expect.expect("one kept nowhere opens nothing and keeps nothing",
            viewerSpawns("s-placeless").length === 0
            && Sessions.keptPlaces["s-placeless"] === undefined,
            JSON.stringify(Sessions.keptPlaces));
        Expect.expect("no pi started, one viewer opened", viewerSpawns("s-viewed").length === 1
            && !Niri.spawns.some(command => / pi( |$)/.test(command)
                || command.includes("s-unserved")), Niri.spawns.join("\n"));
        Expect.expect("its viewer holds where it opened, and nothing took the focus",
            !Niri.acts.some(act => act.MoveWindowToWorkspace || act.MoveColumnToIndex
                || act.FocusWindow || act.FocusWorkspace || act.FocusMonitor
                || act.FocusColumnLast), JSON.stringify(Niri.acts));
        // Joel goes to the empty workspace, and another host serves one,
        // told with a new digest, since the launcher is shut.
        joelOn(42);
        stage = "joel";
        Watch.tell(root.watch, { changed: [Object.assign(hosted("s-second", "SECOND"),
            { told: "0000000000000001" })] });
    }

    function joel() {
        if (!find("SECOND"))
            return;
        stage = "waiting";
        Sessions.reopenAll([{ id: "s-second", cwd: "/tmp", hosted: true }], root.never,
            kept({ "s-second": keptOn(5) }), "");
    }

    // Bound for the workspace Joel is on, it waits, kept, until he leaves.
    function waiting() {
        const second = Sessions.keptPlaces["s-second"];
        if (!second)
            return;
        Expect.expect("one bound for Joel's workspace is kept there", second.workspace === 42,
            JSON.stringify(second));
        Expect.expect("and opens nothing while he's there", viewerSpawns("s-second").length === 0,
            Niri.spawns.join("\n"));
        stage = "left";
        joelOn(40);
    }

    function left() {
        if (viewerSpawns("s-second").length === 0)
            return;
        Expect.expect("once he leaves, it opens where it was kept",
            JSON.stringify(opensOn("s-second")) === JSON.stringify([{
                app_id: "pi-hosted.s-second", reference: { Id: 42 }, column: 1 }]),
            JSON.stringify(Niri.acts));
        Expect.expect("and is kept no more", Sessions.keptPlaces["s-second"] === undefined,
            JSON.stringify(Sessions.keptPlaces));
        console.log("a hosted viewer reopens where it was");
        Qt.exit(0);
    }

    // Every step waits on niri's stand-in, the node or a timer of Sessions'
    // own, so each is looked at often rather than heard.
    Timer {
        running: true
        repeat: true
        interval: 100
        onTriggered: root.check()
    }

    // The shell's watch of sessions, as it asked the node, so it can be
    // told more.
    property var watch: null

    // A stand-in for the machine's node, over the daemon interface, holding
    // the one session its host serves at first.
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
                        root.watch = { id: ask.id, socket: asked };
                        answer = { id: ask.id, records: [root.hosted("s-viewed", "VIEWED")] };
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
        onTriggered: Expect.fail(`still at ${root.stage}: ${JSON.stringify(Niri.acts)} `
            + JSON.stringify(Sessions.keptPlaces))
    }
}

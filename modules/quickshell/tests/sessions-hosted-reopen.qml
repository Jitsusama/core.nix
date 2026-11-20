pragma ComponentBehavior: Bound
// Drives Reopen Them All over sessions pi's host runs
// (modules/quickshell/Sessions.qml), with the machine's node and niri stood
// in, and the windows daemon told as its watch is. Their host brought each
// conversation back as the machine started, so only a viewer is opened,
// never a pi: one the host serves opens quietly where the last boot kept
// it, on the screen's empty workspace, and holds there; one no host serves
// yet has its place kept, for its viewer to open once one does; one the
// boot kept no place of opens none. One bound for the workspace Joel is on
// waits until he leaves it, then opens there. Asked to reopen as a plain pi
// one its host's own list names, it begins none; one whose hosted presence
// outlived its host's list is begun; and one parked in its holder here is
// woken, told to carry on, its place kept for its window to open there as it
// wakes. One whose folder is gone starts its quest afresh, under an id of its
// own, in the folder the harness's quest index says a new one starts in. It
// exits 0 once that holds, and otherwise 1, saying what didn't.
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

    // A session pi's host runs here, as the node tells of it, its owner the
    // host's process, whose own list the state script lays out naming
    // s-viewed alone.
    function hosted(id, address) {
        return { key: `session:${id}`, kind: "session", heard: Date.now(), id,
            state: "idle", standing: "running", here: true, machine: Sessions.machine,
            at: new Date().toISOString(), window: null, held: null, attach: viewing(id),
            presence: { session: id, addresses: [address],
                owner: { pid: 4100, startedAt: 17, bootId: "b1", startTicks: 7,
                    session: id, host: "harbour-background" },
                hosted: { host: "harbour-background", conversation: id } } };
    }
    // A session parked in its holder here, its window closed as it slept.
    function napping(id, address) {
        return { key: `session:${id}`, kind: "session", heard: Date.now(), id,
            state: "ended", standing: "parked", here: true, machine: Sessions.machine,
            at: new Date().toISOString(), window: null, held: "hold-n", attach: null,
            presence: { session: id, addresses: [address], state: "parked" } };
    }

    // Where the last boot kept each window, by session id, on the monitor.
    function kept(places) {
        return id => places[id] ?? null;
    }
    function keptOn(workspace) {
        return { screen: "HP Z27k", output: "DP-4", workspace, column: 1, tile: 1 };
    }

    function viewerSpawns(id) {
        return Niri.spawns.filter(command => command.includes(`/pi-sessions/kitty.sock pi-hosted.${id} `));
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
            else if (stage === "present")
                present();
            else if (stage === "back")
                back();
            else if (stage === "told")
                told();
            else if (stage === "asked")
                asked();
            else if (stage === "adrift")
                adrift();
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
            && spawned[0].includes("/pi-sessions/kitty.sock pi-hosted.s-viewed ")
            && spawned[0].includes("mycelium --realm harbour holds add -- "
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
        // Joel comes back to the machine with three places kept on a
        // workspace he isn't on: one now, one by a shell before this one,
        // as a switch's restart finds, and one now for a session parked in
        // its host. Then the host serves all three.
        Away.now = false;
        const place = at => ({ workspace: 41, column: 2, at });
        Sessions.keepPlaces(Object.assign({}, Sessions.keptPlaces, {
            "s-third": place(Date.now()), "s-old": place(Sessions.startedAt - 1),
            "s-parked": place(Date.now()) }));
        const parked = hosted("s-parked", "PARKED");
        parked.presence.parked = { since: new Date().toISOString() };
        stage = "present";
        Watch.tell(root.watch, { changed: [
            Object.assign(hosted("s-third", "THIRD"), { told: "0000000000000002" }),
            Object.assign(hosted("s-old", "OLD"), { told: "0000000000000003" }),
            Object.assign(parked, { told: "0000000000000004" })] });
    }

    // While Joel is at the machine none opens by itself (OSPHT4
    // [0762284c], 13:41:38 on 11 November): each is still kept.
    function present() {
        if (!find("THIRD") || !find("OLD") || !find("PARKED"))
            return;
        Expect.expect("with Joel there, none opens", ["s-third", "s-old", "s-parked"]
            .every(id => viewerSpawns(id).length === 0
                && Sessions.keptPlaces[id] !== undefined),
            `${Niri.spawns.join("\n")} ${JSON.stringify(Sessions.keptPlaces)}`);
        stage = "back";
        Away.now = true;
    }

    // Once he's away, the one kept this shell opens where it was kept; the
    // one kept before it and the parked one, which viewing would wake, wait.
    function back() {
        if (viewerSpawns("s-third").length === 0)
            return;
        Expect.expect("it opens where it was kept",
            opensOn("s-third").some(one => one.reference.Id === 41 && one.column === 2),
            JSON.stringify(Niri.acts));
        Expect.expect("the one kept before this shell and the parked one still wait",
            ["s-old", "s-parked"].every(id => viewerSpawns(id).length === 0
                && Sessions.keptPlaces[id] !== undefined),
            `${Niri.spawns.join("\n")} ${JSON.stringify(Sessions.keptPlaces)}`);
        // The node tells of one whose hosted presence its host no longer
        // lists, as XUBK2E's after the drill of 15 November (OSPHT4
        // [58d7fa8e]), and one parked in its holder, as 2RCBOJ was.
        stage = "told";
        Watch.tell(root.watch, { changed: [
            Object.assign(hosted("s-stale", "STALE"), { told: "0000000000000005" }),
            Object.assign(napping("s-napped", "NAPPED"), { told: "0000000000000006" })] });
    }

    // Each is asked to reopen as a plain pi, as Reopen Them All asks the last
    // boot's, telling each to carry on; with them, one carried here from
    // another machine, which no boot here kept.
    function told() {
        if (!find("STALE") || !Sessions.parkedAs("NAPPED"))
            return;
        stage = "asked";
        Sessions.reopenAll([{ id: "s-viewed", cwd: "/tmp", way: "boot" },
            { id: "s-stale", cwd: "/tmp", way: "boot" },
            { id: "s-napped", cwd: "/tmp", way: "boot" },
            { id: "s-carried", cwd: "/tmp", way: "carried" }],
            (one, cwd) => Expect.fail(`${one.id}'s ${cwd} gone`),
            kept({ "s-napped": keptOn(2) }), Sessions.carryOn);
    }

    function piFor(id) {
        return Niri.spawns.filter(command => command.includes(` pi --session ${id}`));
    }
    function asked() {
        const woken = steers.find(one => one.address === "NAPPED");
        if (piFor("s-stale").length === 0 || piFor("s-carried").length === 0 || !woken)
            return;
        Expect.expect("one its host lists is begun no more", piFor("s-viewed").length === 0,
            Niri.spawns.join("\n"));
        Expect.expect("one parked is woken, told to carry on, never begun",
            woken.steer.text === Sessions.carryOn && piFor("s-napped").length === 0,
            JSON.stringify(woken));
        const place = Sessions.keptPlaces["s-napped"];
        Expect.expect("its place kept for its window as it wakes", place?.slept === true
            && place?.asleep === true && typeof place?.workspace === "number",
            JSON.stringify(Sessions.keptPlaces));
        // Kept nowhere as the boot ended, it had no window, so it comes back
        // held without one, and the walk goes on without waiting for one.
        // Its folder and its carry-on go as the shell's own `$1` and `$0`,
        // after its words, so neither is read as the shell's.
        const held = piFor("s-stale")[0] ?? "";
        Expect.expect("one kept nowhere comes back held, with no window",
            piFor("s-stale").length === 1
            && held.includes("holds add --detached --folder \"$1\" -- pi --session s-stale")
            && (held.endsWith(` -- "$0" ${Sessions.carryOn} /tmp`)
                || held.endsWith(" pi --session s-stale zsh /tmp"))
            && !held.includes("/pi-sessions/kitty.sock"),
            Niri.spawns.join("\n"));
        Expect.expect("one carried here, though kept nowhere, still opens in a window",
            piFor("s-carried").length === 1
            && piFor("s-carried")[0].includes("/pi-sessions/kitty.sock")
            && !piFor("s-carried")[0].includes("holds add --detached"),
            Niri.spawns.join("\n"));
        stage = "adrift";
        Sessions.reopenAll([{ id: "s-adrift", cwd: "/given-back", quest: "QEST-ADRIFT" }],
            (one, cwd) => Expect.fail(`${one.id}'s ${cwd} gone, and nothing started afresh`),
            kept({ "s-adrift": keptOn(3) }), Sessions.carryOn);
    }

    // Its folder given back, its quest starts afresh where the harness says,
    // under an id of its own, since pi resumes a session only in its own
    // folder; the one it stands in for is never begun.
    function adrift() {
        const started = Niri.spawns.filter(command =>
            command.includes(" env QUEST_WORKFLOW_AUTOLOAD_ID=QEST-ADRIFT "));
        if (started.length === 0)
            return;
        const start = `${Quickshell.env("XDG_STATE_HOME")}/trees/keel`;
        Expect.expect("its quest started afresh in its start, under an id of its own",
            started.length === 1 && started[0].includes(`--directory ${start} env `)
            && /pi --session-id [0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab]/u.test(started[0])
            && !Niri.spawns.some(command => command.includes("s-adrift")), Niri.spawns.join("\n"));
        console.log("a hosted viewer reopens where it was");
        Qt.exit(0);
    }
    // Each steer the node was asked.
    property var steers: []

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
                    } else if (ask.ask === "steer") {
                        root.steers = root.steers.concat([ask]);
                        answer = { id: ask.id, steered: { machine: Sessions.machine } };
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

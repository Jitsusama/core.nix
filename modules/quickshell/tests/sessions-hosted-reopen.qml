pragma ComponentBehavior: Bound
// Drives Reopen Them All over sessions pi's host runs
// (modules/quickshell/Sessions.qml), with the machine's node and niri stood
// in, and the windows daemon told as its watch is. Their host brought each
// conversation back as the machine started, so only a viewer is opened,
// never a pi: one the host serves opens quietly where the last boot kept
// it, on the screen's empty workspace, and holds there; one no host serves
// yet has its place kept, for its viewer to open once one does, unless its
// restore entry says the words its viewer runs, when that viewer opens at
// once to wait there for its host, once however often he asks, a place
// kept for it before let go only once its window comes; one the boot kept
// no place of opens none. One bound for the workspace Joel is on waits
// until he leaves it, then opens there, by those words while its host is
// still away. One whose viewer was asked for moments ago and hasn't come
// is asked for no second time, the walk going on, and its place, kept
// with words, stays kept; one kept with words whose viewer is open opens
// no second. Asked to reopen as a plain pi
// one its host's own list names, it begins none; one whose hosted presence
// outlived its host's list is begun; and one parked in its holder here is
// woken, told to carry on, its place kept for its window to open there as it
// wakes. One whose folder is gone starts its quest afresh, under an id of its
// own, in the folder the harness's quest index says a new one starts in. One
// kept on a workspace named for its orchestrator goes back to the one named
// so now, whatever its number, and one whose name no workspace has names the
// empty one it opens on. It exits 0 once that holds, and otherwise 1, saying
// what didn't.
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
    // The words a hosted session's restore entry says its viewer runs, the
    // harness's join, which looks again until the host its roster names
    // shows the conversation.
    function waitingView(id) {
        return ["node", "/lasting/scripts/host-join.ts", "--view", "--session", id, "--",
            "pi", "--session", id];
    }
    // What titles a viewer's window before its words run, as niri's
    // stand-in records a spawn.
    readonly property string titling: 'sh -c printf "\\033]2;%s\\007" "$0"; exec "$@"'

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
            else if (stage === "unserved")
                unserved();
            else if (stage === "settled")
                settled();
            else if (stage === "refused")
                refused();
            else if (stage === "rested")
                rested();
            else if (stage === "again")
                again();
            else if (stage === "leaves")
                leaves();
            else if (stage === "asking")
                asking();
            else if (stage === "raised")
                raised();
            else if (stage === "guarded")
                guarded();
            else if (stage === "naming")
                naming();
            else if (stage === "named")
                named();
            else if (stage === "renamed")
                renamed();
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
        Expect.expect("its viewer opens by the node's words, held, titled, without the focus",
            spawned[0].startsWith("env -u XDG_ACTIVATION_TOKEN -u DESKTOP_STARTUP_ID ")
            && spawned[0].includes("/pi-sessions/kitty.sock pi-hosted.s-viewed ")
            && spawned[0].includes("mycelium --realm harbour holds add -- "
                + `${titling} \u03c0 - VIEWED ` + viewing("s-viewed").join(" ")), spawned[0]);
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
        // He leaves it for another at the machine, not going away, which
        // is what opens it: kept for him to go away, it would open hours
        // later, on a workspace he moved on from.
        Away.now = false;
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
        // niri opens the one carried here as its kitty does, beside Joel,
        // since no place was kept for it, so the walk goes on without
        // waiting out its patience, and once the rest are back it is done,
        // with no window it gave up on left to come.
        const carried = Sessions.reopening?.waiting;
        if (carried?.id === "s-carried" && carried.classed && carried.opened === null) {
            const window = { id: 59, pid: 590, workspace_id: 40, session: true,
                opened: carried.classed, layout: { pos_in_scrolling_layout: [1, 1] } };
            Niri.windows = Object.assign({}, Niri.windows, { "59": window });
            Niri.revision += 1;
            Niri.windowOpenedOrChanged(window);
        }
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
            kept({ "s-adrift": keptOn(2) }), Sessions.carryOn);
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
        // niri opens it, so the walk goes on without waiting it out.
        const classed = Sessions.reopening?.waiting?.classed ?? "";
        if (classed !== "") {
            // Kept on a workspace of its own, it leaves the next its place.
            const window = { id: 60, pid: 600, workspace_id: 41, session: true,
                opened: classed, layout: { pos_in_scrolling_layout: [1, 1] } };
            Niri.windows = Object.assign({}, Niri.windows, { "60": window });
            Niri.revision += 1;
            Niri.windowOpenedOrChanged(window);
        }
        // After a crash its host is still coming back, so it serves nothing
        // yet, but the restore entry says the words its viewer runs.
        // A place kept for it before, as the cutover keeps one, is let go.
        Sessions.keepPlaces(Object.assign({}, Sessions.keptPlaces,
            { "s-waiting": { workspace: 41, column: 2, at: Date.now() } }));
        stage = "unserved";
        Sessions.reopenAll([waitingEntry("s-waiting")], root.never,
            kept({ "s-waiting": keptOn(3) }), "");
    }
    function waitingEntry(id) {
        return { id, cwd: "/home/skiff/harbour", title: "Reef", hosted: true, way: "boot",
            view: waitingView(id) };
    }

    // Its viewer opens at once, where its window was, quietly, held and
    // titled as its host will title it, to wait there for its host; its
    // place isn't kept for a viewer to open later by itself, and no pi is
    // started for it (20 November: three viewers opened unasked at 15:03,
    // nearly two hours after the walk, titled "mycelium").
    function unserved() {
        const spawned = viewerSpawns("s-waiting");
        if (spawned.length === 0)
            return;
        Expect.expect("it opens by its entry's words, held and titled, without the focus",
            spawned.length === 1
            && spawned[0].startsWith("env -u XDG_ACTIVATION_TOKEN -u DESKTOP_STARTUP_ID ")
            && spawned[0].includes("mycelium --realm harbour holds add -- "
                + `${titling} \u03c0 - Reef - harbour ` + waitingView("s-waiting").join(" ")),
            spawned[0]);
        Expect.expect("niri is told where it opens", opensOn("s-waiting").length === 1
            && typeof opensOn("s-waiting")[0].reference.Id === "number"
            && opensOn("s-waiting")[0].column === 1, JSON.stringify(Niri.acts));
        const on = opensOn("s-waiting")[0].reference.Id;
        Expect.expect("it opens off the workspace Joel is on",
            Niri.workspaces[on]?.is_focused !== true, JSON.stringify(Niri.workspaces));
        Expect.expect("a place kept for it before stays until its window comes",
            Sessions.keptPlaces["s-waiting"]?.workspace === 41,
            JSON.stringify(Sessions.keptPlaces));
        Expect.expect("nothing but its viewer is started for it",
            Niri.spawns.filter(command => command.includes("s-waiting")).length === 1,
            Niri.spawns.join("\n"));
        // niri opens it, and the place kept before is let go, so viewKept
        // never opens a second one by itself once its host serves it.
        const window = { id: 61, pid: 610, workspace_id: on, session: true,
            opened: "pi-hosted.s-waiting", layout: { pos_in_scrolling_layout: [1, 1] } };
        Niri.windows = Object.assign({}, Niri.windows, { "61": window });
        Niri.revision += 1;
        Niri.windowOpenedOrChanged(window);
        Expect.expect("its window come, the place kept before is let go",
            Sessions.keptPlaces["s-waiting"] === undefined, JSON.stringify(Sessions.keptPlaces));
        waitedOn = on;
        stage = "settled";
    }
    property int waitedOn: -1
    property real walkedAt: 0

    // Asked again once that walk is done, before the windows daemon lists
    // its window, the walk finds it open by niri's own list, and goes
    // straight on to the next rather than waiting out its patience for it.
    // Each walk here begins once the one before is done, since a list
    // handed over meanwhile passes over those it began.
    function settled() {
        if (Sessions.reopening !== null)
            return;
        walkedAt = Date.now();
        stage = "refused";
        Sessions.reopenAll([waitingEntry("s-waiting"), waitingEntry("s-after")], root.never,
            kept({ "s-waiting": keptOn(3), "s-after": keptOn(3) }), "");
    }

    function refused() {
        if (viewerSpawns("s-after").length === 0)
            return;
        Expect.expect("one open, waiting for its host, isn't asked for again",
            viewerSpawns("s-waiting").length === 1, Niri.spawns.join("\n"));
        Expect.expect("the next opens without waiting out the walk's patience",
            Date.now() - walkedAt < 10000, `${Date.now() - walkedAt} ms`);
        Expect.expect("nor is a place kept for it, for a second to open later",
            Sessions.keptPlaces["s-waiting"] === undefined, JSON.stringify(Sessions.keptPlaces));
        const window = { id: 62, pid: 620, workspace_id: waitedOn, session: true,
            opened: "pi-hosted.s-after", layout: { pos_in_scrolling_layout: [2, 1] } };
        Niri.windows = Object.assign({}, Niri.windows, { "62": window });
        Niri.revision += 1;
        windowsTell({ changed: [windowRecord(61, "pi-hosted.s-waiting", waitedOn),
            windowRecord(62, "pi-hosted.s-after", waitedOn)] });
        stage = "rested";
        Niri.windowOpenedOrChanged(window);
    }

    function rested() {
        if (Sessions.reopening !== null)
            return;
        // The daemon lists them, Joel goes to their workspace, and the
        // moments a viewer is waited for pass; asked again while the host
        // is still coming back, as its row is still offered, the walk finds
        // the one waiting open by its app id alone, and keeps another bound
        // there for him to leave.
        joelOn(waitedOn);
        Sessions.attaching = ({});
        stage = "again";
        Sessions.reopenAll([waitingEntry("s-waiting"), waitingEntry("s-watched")], root.never,
            kept({ "s-waiting": keptOn(3), "s-watched": keptOn(3) }), "");
    }

    function again() {
        const watched = Sessions.keptPlaces["s-watched"];
        if (!watched || (Sessions.reopening?.queue.length ?? 0) > 0)
            return;
        Expect.expect("one open, waiting for its host, opens no second viewer",
            viewerSpawns("s-waiting").length === 1, Niri.spawns.join("\n"));
        Expect.expect("nor is a place kept for it, for a second to open later",
            Sessions.keptPlaces["s-waiting"] === undefined, JSON.stringify(Sessions.keptPlaces));
        Expect.expect("one bound for Joel's workspace is kept there, with its words, not opened",
            watched.workspace === waitedOn && viewerSpawns("s-watched").length === 0
            && JSON.stringify(watched.waiting) === JSON.stringify(waitingView("s-watched"))
            && watched.named === "pi-hosted.s-watched"
            && watched.title === "\u03c0 - Reef - harbour",
            `${JSON.stringify(watched)} ${Niri.spawns.join("\n")}`);
        // Joel leaves for another workspace, its host still away.
        const other = Object.values(Niri.workspaces).find(one => one.id !== waitedOn);
        stage = "leaves";
        joelOn(other.id);
    }

    // It opens by its words where it was kept, without the focus, and its
    // place is done with, rather than waiting for its host to serve it.
    function leaves() {
        const spawned = viewerSpawns("s-watched");
        if (spawned.length === 0)
            return;
        Expect.expect("it opens once, by its words, titled", spawned.length === 1
            && spawned[0].includes(`${titling} \u03c0 - Reef - harbour `
                + waitingView("s-watched").join(" ")), spawned[0]);
        Expect.expect("where it was kept", opensOn("s-watched").length === 1
            && opensOn("s-watched")[0].reference.Id === waitedOn, JSON.stringify(Niri.acts));
        Expect.expect("its place done with", Sessions.keptPlaces["s-watched"] === undefined,
            JSON.stringify(Sessions.keptPlaces));
        // One whose viewer was asked for moments ago, by Go To or viewKept,
        // and hasn't opened yet, is opening: the walk asks for no second
        // and goes on.
        Sessions.attaching = ({ "pi-hosted.s-asked": Date.now() });
        walkedAt = Date.now();
        stage = "asking";
        Sessions.reopenAll([waitingEntry("s-asked"), waitingEntry("s-next")], root.never,
            kept({ "s-asked": keptOn(3), "s-next": keptOn(3) }), "");
    }

    function asking() {
        if (viewerSpawns("s-next").length === 0)
            return;
        Expect.expect("one asked for moments ago isn't asked for again",
            viewerSpawns("s-asked").length === 0, Niri.spawns.join("\n"));
        Expect.expect("and the walk goes on without waiting out its patience for it",
            Date.now() - walkedAt < 10000, `${Date.now() - walkedAt} ms`);
        // Two kept with words to wait by on a workspace Joel isn't on: one
        // whose viewer is open already, which viewKept leaves be, and one
        // whose viewer was asked for moments ago, which stays kept, so a
        // viewer that never opens still leaves its place to open by.
        const place = id => ({ workspace: waitedOn, column: 1, at: Date.now(),
            waiting: waitingView(id), named: `pi-hosted.${id}`, title: "\u03c0 - Reef" });
        Sessions.attaching = Object.assign({}, Sessions.attaching,
            { "pi-hosted.s-held": Date.now() });
        stage = "guarded";
        Sessions.keepPlaces(Object.assign({}, Sessions.keptPlaces,
            { "s-waiting": place("s-waiting"), "s-held": place("s-held") }));
        Niri.revision += 1;
    }

    function guarded() {
        if (Sessions.keptPlaces["s-waiting"] !== undefined)
            return;
        Expect.expect("one kept whose viewer is open opens no second",
            viewerSpawns("s-waiting").length === 1, Niri.spawns.join("\n"));
        Expect.expect("one kept whose viewer was asked for moments ago stays kept, unopened",
            Sessions.keptPlaces["s-held"]?.named === "pi-hosted.s-held"
            && viewerSpawns("s-held").length === 0,
            `${JSON.stringify(Sessions.keptPlaces)} ${Niri.spawns.join("\n")}`);
        // A session its host serves whose viewer niri lists, as just after
        // a crash, before the windows daemon does.
        const window = { id: 63, pid: 630, workspace_id: waitedOn, session: true,
            opened: "pi-hosted.s-goto", layout: { pos_in_scrolling_layout: [3, 1] } };
        Niri.windows = Object.assign({}, Niri.windows, { "63": window });
        Niri.revision += 1;
        stage = "raised";
        Watch.tell(root.watch, { changed: [Object.assign(hosted("s-goto", "GOTO"),
            { told: "0000000000000009" })] });
    }

    // Go To raises the viewer niri lists, rather than opening a second or
    // doing nothing.
    function raised() {
        const session = find("GOTO");
        if (!session)
            return;
        Sessions.goTo(session);
        Expect.expect("Go To raises the viewer niri lists, opening no second",
            Niri.acts.some(act => act.FocusWindow?.id === 63)
            && viewerSpawns("s-goto").length === 0,
            `${JSON.stringify(Niri.acts)} ${Niri.spawns.join("\n")}`);
        // KELP's workspace, named for its orchestrator, comes back fifth
        // this boot, with a window of its own, and niri's empty one after.
        Niri.workspaces = Object.assign({}, Niri.workspaces, {
            "44": { id: 44, idx: 5, name: "KELP", output: "DP-4", is_focused: false,
                active_window_id: 92 },
            "45": { id: 45, idx: 6, output: "DP-4", is_focused: false, active_window_id: null }
        });
        Niri.windows = Object.assign({}, Niri.windows, { "92": { id: 92, pid: 920,
            workspace_id: 44, opened: "pi-hosted.s-lead",
            layout: { pos_in_scrolling_layout: [1, 1] } } });
        Niri.revision += 1;
        stage = "naming";
        Watch.tell(root.watch, { changed: [
            Object.assign(hosted("s-kelp", "KELPER"), { told: "0000000000000010" }),
            Object.assign(hosted("s-reef", "REEFER"), { told: "0000000000000011" })] });
    }

    // One kept on a workspace named for its orchestrator goes back to the
    // one named so now, though niri numbers it otherwise this boot
    // (2026-11-21 10:08: WM4KSZ's sessions came back on workspace 3 by
    // number), naming nothing.
    function naming() {
        if (!find("KELPER") || !find("REEFER") || (Sessions.reopening?.queue.length ?? 0) > 0)
            return;
        stage = "named";
        Sessions.reopenAll([{ id: "s-kelp", cwd: "/tmp", hosted: true }], root.never,
            kept({ "s-kelp": Object.assign(keptOn(3), { named: "KELP" }) }), "");
    }
    function named() {
        const opens = opensOn("s-kelp");
        if (opens.length === 0)
            return;
        Expect.expect("one kept on a named workspace goes back to the one named so now",
            opens[0].reference?.Id === 44 && opens[0].column === 2, JSON.stringify(opens));
        Expect.expect("naming nothing, since one has the name",
            !Niri.acts.some(act => act.SetWorkspaceName), JSON.stringify(Niri.acts));
        const window = { id: 93, pid: 930, workspace_id: 44, session: true,
            opened: "pi-hosted.s-kelp", layout: { pos_in_scrolling_layout: [2, 1] } };
        Niri.windows = Object.assign({}, Niri.windows, { "93": window });
        Niri.revision += 1;
        windowsTell({ changed: [windowRecord(93, "pi-hosted.s-kelp", 44)] });
        stage = "renamed";
        Niri.windowOpenedOrChanged(window);
        Sessions.reopenAll([{ id: "s-reef", cwd: "/tmp", hosted: true }], root.never,
            kept({ "s-reef": Object.assign(keptOn(4), { named: "REEF" }) }), "");
    }

    // One whose name no workspace has now opens on the screen's empty
    // workspace and names it, once.
    function renamed() {
        const opens = opensOn("s-reef");
        if (opens.length === 0)
            return;
        const naming = Niri.acts.filter(act => act.SetWorkspaceName)
            .map(act => act.SetWorkspaceName);
        Expect.expect("one whose name no workspace has names the empty one it opens on, once",
            naming.length === 1 && naming[0].name === "REEF"
            && naming[0].workspace?.Id === opens[0].reference?.Id
            && Niri.workspaces[opens[0].reference?.Id]?.active_window_id === null,
            `${JSON.stringify(naming)} ${JSON.stringify(opens)}`);
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

    // Long enough for each walk to be done before the next begins, and
    // short of the check's own 120 s timeout, so a stage that never ends
    // says which it is rather than the check ending with no word.
    Timer {
        running: true
        interval: 110000
        onTriggered: Expect.fail(`still at ${root.stage}: ${JSON.stringify(Niri.acts)} `
            + JSON.stringify(Sessions.keptPlaces))
    }
}

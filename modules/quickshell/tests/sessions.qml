pragma ComponentBehavior: Bound
// Drives pi's sessions (modules/quickshell/Sessions.qml) through Quickshell
// itself, with the machine's mycelium node stood in for over the daemon
// interface it speaks, the windows daemon told as its watch is, and niri,
// kitty's kitten and the notices stood in too, over what sessions-state.sh lays out. The sessions are the
// node's and nothing else's: a node not listening as the shell starts, as
// when a switch restarts it, is said as not answering, with no session
// guessed in its place, and is watched once it listens. Every window niri
// shows carries the same title, so nothing can be found by one. A
// session's window is the one niri lists by the app id the node says its
// window was opened with, or for one opened without, through the kitty its
// record's terminal names, by kitty's own list of its several windows. Go
// To focuses that window, or asks kitty when niri's isn't known. One the
// node says is parked is held apart from those running, and one it says
// closed apart again, however each record reads; Wake, or talking to one
// parked, asks the node, which wakes it. What an earlier boot's end took away is
// offered as the agents daemon serves it, stood in for too, one that ran
// hosted among it, whose host brings it back, so asking reopens no pi for
// it (modules/quickshell/tests/sessions-hosted-reopen.qml), the rest reopened
// held in its own folder, one pi never saved started under its own id with its
// quest, and Reopen Them All reopens even when niri never answers its ask
// for the screens, counting the lists given before it does as one walk,
// and by the daemon's places when given none; what
// another machine carried here, or pi's host gave back, is offered after
// it; forgetting one and putting the last boot off are asked of the
// daemon; and a window it had joined to a session another machine holds
// is joined again by its words, once; a closed session
// resumes in its own folder, and one whose folder is gone opens nowhere
// but its quest's start folder, never the home folder. What the node tells
// of sessions on the realm's other machines is held too: each says its
// machine, after this machine's; Go To joins one by the words its node
// gives; one out of reach stays, saying so, and isn't joined; one pi's
// host runs with no window is said and never opened; one the node says
// nothing of how it stands is passed over; and a steer to one is asked
// of the node, its refusal said. One the node says is gone goes; a
// change saying only what differs keeps the rest of what the node said,
// and one to a running session tells nothing of the parked or closed;
// the one whose window the windows daemon says was used last goes first,
// and a window retitled makes no session's window again;
// one the node says has ended goes; and a node lost leaves what it said
// shown. It exits 0 once every case holds, and otherwise 1, saying which
// didn't. Long because each stage hears what the one before set going,
// over one shell's sessions and the stand-ins for the node and the agents
// daemon, so split, each part would lay all of it out again.
// Its stand-ins are in modules/quickshell/tests/stand-ins.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Reopening.js" as Reopening

ShellRoot {
    id: root

    readonly property string state: Quickshell.env("XDG_STATE_HOME")
    property string stage: "unanswered"

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

    // How often what reads parked and closed sessions was told they moved,
    // so a change to a running one alone is seen to tell neither.
    property var retold: ({ parked: 0, closed: 0 })
    property var retoldBefore: null

    Connections {
        target: Sessions
        function onRunningChanged() {
            Qt.callLater(root.check);
        }
        function onParkedChanged() {
            root.retold.parked += 1;
            Qt.callLater(root.check);
        }
        function onClosedChanged() {
            root.retold.closed += 1;
        }
        function onToReopenChanged() {
            Qt.callLater(root.check);
        }
        function onKeptPlacesChanged() {
            Qt.callLater(root.check);
        }
    }

    Connections {
        target: Sessions.node
        function onStateChanged() {
            Qt.callLater(root.check);
        }
    }

    function parkedAs() {
        return Sessions.parked.map(session => session.address).sort().join(" ");
    }

    // Reopen Them All opens each on a timer of its own, which nothing else
    // here hears.
    Connections {
        target: Niri
        function onSpawnsChanged() {
            Qt.callLater(root.check);
        }
    }

    Component.onCompleted: Qt.callLater(check)

    // A window as the windows daemon serves it (modules/windows), with only
    // what Sessions reads of it, and the title a row would say.
    function windowRecord(id, pid, opened, fields) {
        return Object.assign({ key: `window:${id}`, id, pid, opened,
            title: "\u03c0 - Lighthouse", workspace: null, column: null, down: null, order: 0 },
            fields ?? {});
    }
    // Tells Sessions' watch of the windows daemon, as the daemon does: its
    // whole answer, or a change.
    function windowsTell(said) {
        Sessions.windowsFrom.said(JSON.stringify(Object.assign({ id: "all" }, said)));
    }
    // How often Sessions made its windows by process and app id again.
    property int windowsMade: 0
    Connections {
        target: Sessions
        function onNiriWindowsChanged() {
            root.windowsMade += 1;
        }
    }

    function check() {
        try {
            if (stage === "unanswered")
                unanswered();
            else if (stage === "listed")
                listed();
            else if (stage === "reopening")
                reopening();
            else if (stage === "forgetting")
                forgetting();
            else if (stage === "forgotten")
                forgotten();
            else if (stage === "differing")
                differing();
            else if (stage === "resting")
                resting();
            else if (stage === "ended")
                ended();
            else if (stage === "lost")
                lost();
        } catch (error) {
            fail(error.message);
        }
    }

    // With the node not listening, it's said so, and no session is
    // guessed from a folder or a process; then it listens.
    function unanswered() {
        if (Sessions.node?.state !== "not answering")
            return;
        expect("no session while the node doesn't answer", addresses() === "", addresses());
        // The same three windows niri's stand-in lists (checks.nix), and
        // the one a parked session's holder keeps. Older windows of
        // another kitty were opened with the same app ids, as old spawns
        // share small numbers (2QZBX9, 2026-11-11), and are neither's.
        windowsTell({ records: [windowRecord(10, 500, "kitty"), windowRecord(20, 600, "kitty"),
            windowRecord(17, 900, "pi-session.7", { workspace: 11, column: 1, down: 1 }),
            windowRecord(18, 900, "pi-spawned.9", { workspace: 11, column: 2, down: 1 }),
            windowRecord(21, 600, "pi-session.7", { workspace: 3, column: 2, down: 1 }),
            windowRecord(22, 800, "pi-spawned.9", { workspace: 5, column: 1, down: 1 }),
            windowRecord(23, 950, "kitty", { workspace: 8, column: 3, down: 1 })] });
        stage = "listed";
        node.active = true;
    }

    // Waits until every running session is listed, the one in a kitty
    // drawing two windows found by its window's app id.
    function listed() {
        const other = find("OTHER2");
        if (addresses() !== "FARAWY LIVE01 NOWIN6 OTHER2 OUTRCH" || !other || !other.window
            || Sessions.toReopen.length === 0 || Sessions.keptPlaces["s-moving"] === undefined)
            return;
        const live = find("LIVE01");
        expect("a kitty with one window has it", live.window && live.window.id === 10,
            JSON.stringify(live.window));
        // The bell rings for what the daemon says rings, known as it says,
        // whatever each one's presence holds.
        expect("only a want the daemon says rings stops its session",
            Sessions.stopped.map(session => session.address).join(" ") === "LIVE01",
            Sessions.stopped.map(session => session.address).join(" "));
        // One whose want he put off waits on him no longer, though it asks.
        expect("a want the daemon says is put off leaves what waits on Joel",
            JSON.stringify(Sessions.putOff) === '{"NOWIN6":true}'
                && Sessions.asks(find("NOWIN6"))
                && !Sessions.wanting.some(session => session.address === "NOWIN6"),
            JSON.stringify([Sessions.putOff, Sessions.wanting.map(session => session.address)]));
        expect("Put It Off can name the ask the daemon knows, whatever the address's case",
            Sessions.wantKnown("nowin6") === "NOWIN6 question@now"
                && Sessions.wantKnown("NOBODY") === "",
            Sessions.wantKnown("nowin6"));
        expect("known as the daemon knows it, and rung for, being new",
            Sessions.key(live) === "LIVE01 question-1" && Sessions.ringing === live,
            `${Sessions.key(live)} ${Sessions.ringing?.address}`);
        expect("its label shown, cut at its folder",
            live.title === "Live One" && live.folder === "core.nix", `${live.title} ${live.folder}`);
        expect("its quest's title and its folder as its record says them, not its label",
            other.title === "Lighthouse - Keep It Lit" && other.folder === "lamp",
            `${other.title} ${other.folder}`);
        expect("of a kitty's several windows, the one opened with its app id",
            other.window.id === 21, other.window.id);
        expect("its kitty from its record's terminal",
            JSON.stringify(other.kitty) === '{"socket":"unix:@kitty-600","window":4}',
            JSON.stringify(other.kitty));
        expect("one the node says has ended, and one it passes over elsewhere, aren't there",
            Object.keys(Sessions.present).sort().join(" ") === "FARAWY LIVE01 NOWIN6 OTHER2 OUTRCH",
            Object.keys(Sessions.present).sort().join(" "));
        // LIVE01 here, and both elsewhere.
        expect("working, as each record says", Sessions.working === 3, Sessions.working);
        expect("a session is its window", Sessions.isOf(other) === "window:21", Sessions.isOf(other));
        // Asked to keep where a session's window is, as the move onto pi's
        // host asks before it closes it, the shell says whether it could,
        // beside the place the daemon kept from before it started, and
        // the daemon is told them all to keep.
        expect("a session's window kept", Sessions.keepPlace("s-OTHER2") === "kept"
            && JSON.stringify(Sessions.keptPlaces["s-OTHER2"] ?? null).startsWith(
                '{"workspace":3,"column":2,'), JSON.stringify(Sessions.keptPlaces));
        const keptNot = ["s-NOWIN6", "r-far", "s-nobody"].map(id => Sessions.keepPlace(id));
        expect("one with no window, or not here, keeps nothing", keptNot.every(answer =>
            answer === "no window to keep")
            && Object.keys(Sessions.keptPlaces).sort().join(" ") === "s-OTHER2 s-moving",
            keptNot.join(", "));
        Sessions.keepPlaces({});

        Sessions.goTo(other);
        expect("Go To focuses its window", JSON.stringify(Niri.acted) === '{"FocusWindow":{"id":21}}',
            JSON.stringify(Niri.acted));
        const unseen = find("NOWIN6");
        expect("with no window niri shows, none", unseen.window === null,
            JSON.stringify(unseen.window));
        expect("a session with no window is itself", Sessions.isOf(unseen) === "agent:NOWIN6",
            Sessions.isOf(unseen));
        const unseenWent = Sessions.goTo(unseen);
        expect("Go To asks its kitty, saying it went", unseenWent === true
            && Niri.spawned.join(" ")
            === "kitten @ --to unix:@kitty-700 focus-window --match id:2", Niri.spawned.join(" "));
        elsewhere();
        parks();
        lastBoot();
        stage = "reopening";
    }

    // Those on the realm's other machines, as the node tells of them.
    function elsewhere() {
        const far = find("FARAWY");
        const out = find("OUTRCH");
        expect("one elsewhere says its machine", !far.here && far.machine === "lighthouse"
            && far.reachable, JSON.stringify(far));
        expect("one out of reach says so", !out.reachable && out.attach === null,
            JSON.stringify(out));
        expect("this machine's first", Sessions.running.slice(0, 3).every(session => session.here),
            Sessions.running.map(session => session.address).join(" "));
        const farWent = Sessions.goTo(far);
        const joined = Niri.spawned.join(" ");
        expect("and says it went", farWent === true, String(farWent));
        expect("Go To joins it by its node's words", inSessions(joined)
            === "sessions zsh -lic mycelium attach r-far", joined);
        // Its window opens on its orchestrator's workspace, never on the
        // one Joel is on, keeping niri's token, since he asked for it (Joel,
        // 2026-11-15 18:45: one "materialize[d] on my main chrome workspace").
        // The token alone never took him there; the shell focuses the window
        // once niri lists it
        // (modules/quickshell/tests/sessions-go-to-focuses.qml).
        const opened = Niri.acted?.OpenNextWindowOn;
        expect("one elsewhere opens on its orchestrator's workspace",
            opened?.reference?.Id === 8 && opened?.app_id === asked(joined)?.classed,
            JSON.stringify(Niri.acted));
        expect("and keeps niri's token", !joined.includes("XDG_ACTIVATION_TOKEN"), joined);
        const outWent = Sessions.goTo(out);
        expect("one out of reach isn't joined", root.unjoined === "OUTRCH", root.unjoined);
        // Said false, so a Go To from a link with the launcher shut opens it to say why, rather
        // than telling a launcher nobody opens (KP0L6A, 2026-11-24 [00902e94]).
        expect("and says it didn't go", outWent === false, String(outWent));
        // Parked on another machine, woken there, its window opens there, which takes Joel
        // nowhere, so a Go To that can't reach it still says it didn't go, rather than the wake
        // hiding it (#6519's review).
        const outParked = Object.assign({}, out, { presence: Object.assign({}, out.presence,
            { state: "parked" }) });
        root.unjoined = "";
        const outWoke = Sessions.wakeToGo(outParked, out);
        expect("one parked elsewhere and out of reach, woken, still says it didn't go",
            outWoke === false && root.unjoined === "OUTRCH",
            JSON.stringify([outWoke, root.unjoined]));
        hosts();
        Sessions.steer("farawy", { act: "pause" }, (put, answer, sent) => {
            root.farawyTold = [put, answer?.error?.message ?? null, sent];
        });
    }
    // What the refused steer's caller was told: not put, the node's
    // refusal, and sent, so a caller tells it from one never sent.
    property var farawyTold: null

    // One pi's host runs, with no window: Go To views it by its node's
    // words, in a window named for it, once; niri's window by that name is
    // its own after; and one the node gives no words for is said.
    function hosts() {
        const hosted = {
            address: "HOSTED", here: true, window: null, kitty: null, held: null,
            attach: ["mycelium", "--realm", "harbour", "attach", "--host", "harbour-attended",
                "--session", "s-hosted"],
            presence: { hosted: { host: "harbour-attended", conversation: "s-hosted" },
                session: "s-hosted", addresses: ["HOSTED"] }
        };
        const before = Niri.spawns.length;
        Sessions.goTo(hosted);
        const viewed = Niri.spawned.join(" ");
        const hostedAsk = asked(viewed);
        // Titled before its words run, as pi's would be with no name or folder said.
        expect("a hosted one is viewed by its node's words, held, titled, in a window named for it",
            hostedAsk !== null && hostedAsk.classed === "pi-hosted.s-hosted"
            && hostedAsk.words === "mycelium --realm harbour holds add -- "
                + 'sh -c printf "\\033]2;%s\\007" "$0"; exec "$@" \u03c0 '
                + "mycelium --realm harbour attach --host harbour-attended --session s-hosted",
            viewed);
        Sessions.goTo(hosted);
        expect("viewed only once while its window comes, and asked again it says so",
            Niri.spawns.length === before + 1 && root.unhosted === "HOSTED"
            && root.unhostedWhy === "opening",
            `${root.unhosted} ${root.unhostedWhy} ${Niri.spawns.slice(before).join("\n")}`);
        const record = { opened: "", kitty: null, presence: hosted.presence };
        const window = { id: 40, opened: "pi-hosted.s-hosted" };
        expect("its viewer is its window, by the name it opened with",
            Sessions.windowOf(record, {}, { "pi-hosted.s-hosted": [window] }) === window,
            JSON.stringify(Sessions.windowOf(record, {}, { "pi-hosted.s-hosted": [window] })));
        const listing = Niri.windows;
        Niri.windows = Object.assign({}, listing, { "40": window });
        Sessions.goTo(Object.assign({}, hosted, { window: window }));
        Niri.windows = listing;
        const raised = JSON.stringify(Niri.acted);
        expect("Go To raises its viewer", raised === '{"FocusWindow":{"id":40}}', raised);
        // No words to view it by is said as that, even while a viewer
        // asked for moments ago comes, as a row's Reopen says it.
        Sessions.goTo(Object.assign({}, hosted, { address: "WORDLESS", attach: null }));
        expect("one with no words is said, never opened", root.unhosted === "WORDLESS"
            && root.unhostedWhy === "hosted" && Niri.spawns.length === before + 1,
            `${root.unhosted} ${root.unhostedWhy} ${Niri.spawned.join(" ")}`);
        // One whose id no app id could carry has no viewer to raise, even
        // while niri lists a window that opened with no app id.
        const windows = Niri.windows;
        Niri.windows = Object.assign({}, windows, { "99": { id: 99, pid: 990, opened: "" } });
        Niri.acted = null;
        Sessions.goTo(Object.assign({}, hosted, { address: "UNNAMED",
            presence: Object.assign({}, hosted.presence, { session: "s unnamed" }) }));
        Niri.windows = windows;
        expect("one whose viewer can't be named raises no window, but is said",
            Niri.acted === null && root.unhosted === "UNNAMED" && root.unhostedWhy === "unnamed"
            && Niri.spawns.length === before + 1,
            `${JSON.stringify(Niri.acted)} ${root.unhosted} ${root.unhostedWhy}`);
        // pi run here in another terminal: no window, kitty or holder.
        Sessions.goTo({ address: "ELSETERM", window: null, kitty: null, held: null,
            here: true, presence: { session: "s-elsewhere-term" } });
        expect("one in a terminal it can't reach is said, never a press that does nothing",
            root.unreached === "ELSETERM" && Niri.spawns.length === before + 1,
            `${root.unreached} ${Niri.spawned.join(" ")}`);
        // Each Go To says whom it was asked for, and when, so its refusal can open the
        // launcher to answer that press alone (Agents' refusedGoing).
        expect("a Go To says whom it was asked for, just now",
            Sessions.goneTo.address === "ELSETERM" && Date.now() - Sessions.goneTo.at < 5000,
            JSON.stringify(Sessions.goneTo));
        // Parked here, woken by Go To, it reopens by its wake, so what goTo can't open
        // meanwhile is said while it wakes, which Agents leaves unsaid, and it went
        // (#6519's second review).
        const asleep = { address: "ASLEEPTERM", window: null, kitty: null, held: null,
            here: true, presence: { session: "s-asleep-term", state: "parked" } };
        root.wakingSaid = "-";
        const sleptWent = Sessions.wakeToGo(asleep, asleep);
        expect("one parked here, woken, went, its refusal said while it wakes",
            sleptWent === true && root.unreached === "ASLEEPTERM"
            && root.wakingSaid === "ASLEEPTERM" && Sessions.waking === "",
            JSON.stringify([sleptWent, root.unreached, root.wakingSaid, Sessions.waking]));
        root.wakingSaid = "-";
        Sessions.goTo(asleep);
        expect("and gone to with no wake, its refusal is said as one",
            root.wakingSaid === "", root.wakingSaid);
        // A row's own Reopen for one no host serves yet, as just after a
        // crash, views it by its entry's words to wait for its host, once
        // while that window comes; one whose entry gives none is said.
        const view = ["mycelium", "--realm", "harbour", "attach", "--host", "harbour-attended",
            "--session", "s-row-waiting"];
        // Reopen Them All kept a place for its viewer, Joel being on its
        // workspace then, as reopenViewer keeps one.
        // Workspace 5, which Joel isn't on, so viewKept would open its viewer
        // there by itself for as long as the place stays kept.
        Sessions.keepPlaces({ "s-row-waiting": { workspace: 5, column: 1, at: Date.now(),
            asked: true, there: true, waiting: view, named: "pi-hosted.s-row-waiting",
            title: "Reef" } });
        Sessions.reopen({ id: "s-row-waiting", cwd: "/tmp", title: "Reef", hosted: true, view });
        const rowAsk = asked(Niri.spawned.join(" "));
        expect("the walk's place for it stays while its viewer comes, so one never come "
            + "leaves a place to open it by", Sessions.keptPlaces["s-row-waiting"] !== undefined,
            JSON.stringify(Sessions.keptPlaces));
        expect("a row's Reopen with no host views it by its entry's words, to wait",
            Niri.spawns.length === before + 2 && rowAsk !== null
            && rowAsk.classed === "pi-hosted.s-row-waiting"
            && rowAsk.words === "mycelium --realm harbour holds add -- "
                + 'sh -c printf "\\033]2;%s\\007" "$0"; exec "$@" \u03c0 - Reef - tmp '
                + view.join(" "),
            Niri.spawns.slice(before).join("\n"));
        Sessions.reopen({ id: "s-row-waiting", cwd: "/tmp", title: "Reef", hosted: true, view });
        Sessions.reopen({ id: "s-row-wordless", cwd: "/tmp", hosted: true });
        expect("viewed once while it comes, and one with no words is said",
            Niri.spawns.length === before + 2 && root.unhosted === "s-row-wordless"
            && root.unhostedWhy === "hostless",
            `${root.unhosted} ${root.unhostedWhy} ${Niri.spawns.slice(before).join("\n")}`);
        // A press must never do nothing: asked again while its viewer
        // comes, it says so.
        Sessions.reopen({ id: "s-row-waiting", cwd: "/tmp", title: "Reef", hosted: true, view });
        expect("asked again while its viewer comes, it says it's opening",
            Niri.spawns.length === before + 2 && root.unhosted === "s-row-waiting"
            && root.unhostedWhy === "opening", `${root.unhosted} ${root.unhostedWhy}`);
        // Its viewer comes, and Joel closes it: the walk's place went as
        // it came, so none opens by itself once he has.
        const reef = { id: 97, pid: 970, opened: "pi-hosted.s-row-waiting" };
        const unreefed = Niri.windows;
        Niri.windows = Object.assign({}, unreefed, { "97": reef });
        Niri.windowOpenedOrChanged(reef);
        const reefCame = Sessions.keptPlaces["s-row-waiting"] === undefined;
        Niri.windows = unreefed;
        Sessions.viewKept();
        expect("the walk's place goes as its viewer comes, so none opens by itself after "
            + "Joel closes it", reefCame && Niri.spawns.length === before + 2,
            `${JSON.stringify(Sessions.keptPlaces)} ${Niri.spawns.slice(before).join("\n")}`);
        // One with words but an id no app id could carry has no viewer to
        // open, and is said so, not as one with no words.
        Sessions.reopen({ id: "s row spaced", cwd: "/tmp", hosted: true, view });
        expect("one whose id can't name a viewer is said so, never opened",
            Niri.spawns.length === before + 2 && root.unhosted === "s row spaced"
            && root.unhostedWhy === "unnamed", `${root.unhosted} ${root.unhostedWhy}`);
        // A viewer niri lists is raised, never opened again.
        const shown = Niri.windows;
        Niri.windows = Object.assign({}, shown,
            { "77": { id: 77, pid: 770, opened: "pi-hosted.s-row-shown" } });
        Niri.acted = null;
        Sessions.reopen({ id: "s-row-shown", cwd: "/tmp", hosted: true, view });
        expect("a viewer niri lists is raised, not opened again",
            JSON.stringify(Niri.acted) === '{"FocusWindow":{"id":77}}'
            && Niri.spawns.length === before + 2,
            `${JSON.stringify(Niri.acted)} ${Niri.spawns.slice(before).join("\n")}`);
        // niri's list is the windows daemon's own, so one only this
        // file's index of the daemon lists while niri lists others is gone:
        // it's viewed afresh, never raised.
        const index = Sessions.niriWindows;
        Sessions.niriWindows = { pids: index.pids, opened: Object.assign({}, index.opened,
            { "pi-hosted.s-row-listed": [{ id: 78, opened: "pi-hosted.s-row-listed" }],
                "pi-hosted.s-row-stale": [{ id: 79, opened: "pi-hosted.s-row-stale" }] }) };
        Niri.acted = null;
        Sessions.reopen({ id: "s-row-listed", cwd: "/tmp", hosted: true, view });
        expect("one only the index lists while niri lists others is viewed afresh, not raised",
            Niri.acted === null && Niri.spawns.length === before + 3
            && asked(Niri.spawned.join(" "))?.classed === "pi-hosted.s-row-listed",
            `${JSON.stringify(Niri.acted)} ${Niri.spawns.slice(before).join("\n")}`);
        // While niri lists none, as before it has heard the daemon, the
        // index's is raised.
        Niri.windows = {};
        Sessions.reopen({ id: "s-row-listed", cwd: "/tmp", hosted: true, view });
        const fromIndex = JSON.stringify(Niri.acted);
        // But not while niri's own stream stands in for a daemon that
        // isn't answering, which leaves the index as it last stood.
        Niri.acted = null;
        Niri.standIn = true;
        Sessions.reopen({ id: "s-row-stale", cwd: "/tmp", hosted: true, view });
        Niri.standIn = false;
        Niri.windows = shown;
        Sessions.niriWindows = index;
        expect("while niri lists none, a viewer the windows daemon lists is raised",
            fromIndex === '{"FocusWindow":{"id":78}}' && Niri.spawns.length === before + 4,
            `${fromIndex} ${Niri.spawns.slice(before).join("\n")}`);
        expect("unless niri stands in for the daemon: the index's is viewed afresh, not raised",
            Niri.acted === null
            && asked(Niri.spawned.join(" "))?.classed === "pi-hosted.s-row-stale",
            `${JSON.stringify(Niri.acted)} ${Niri.spawns.slice(before).join("\n")}`);
        // Go To on one whose window the index names but niri doesn't list
        // views it afresh, rather than raising a window that's gone.
        const stale = { id: 41, opened: "pi-hosted.s-stale" };
        Sessions.goTo(Object.assign({}, hosted, { address: "STALE", window: stale,
            attach: view.slice(0, -1).concat(["s-stale"]),
            presence: Object.assign({}, hosted.presence, { session: "s-stale" }) }));
        expect("Go To on one whose window niri doesn't list views it afresh",
            Niri.acted === null && asked(Niri.spawned.join(" "))?.classed === "pi-hosted.s-stale",
            `${JSON.stringify(Niri.acted)} ${Niri.spawns.slice(before).join("\n")}`);
        // A row whose id begins with a dash, which mycelium's --session
        // could take for a flag, names no viewer.
        Sessions.reopen({ id: "-row", cwd: "/tmp", hosted: true, view });
        expect("a row whose id begins with a dash is said unnamed, never opened",
            root.unhosted === "-row" && root.unhostedWhy === "unnamed"
            && !Niri.spawns.some(one => one.includes("pi-hosted.-row")),
            `${root.unhosted} ${root.unhostedWhy} ${Niri.spawns.slice(before).join("\n")}`);
        // A viewer no sessions kitty took, as opens-in-sessions.sh tells
        // the shell (Sessions' untaken), is no longer opening: the next Go
        // To asks for it again, rather than saying it's opening for 30 s.
        const untaken = Object.assign({}, hosted, { address: "UNTAKEN",
            attach: hosted.attach.slice(0, -1).concat(["s-untaken"]),
            presence: Object.assign({}, hosted.presence, { session: "s-untaken" }) });
        const untakenFrom = Niri.spawns.length;
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.goTo(untaken);
        Sessions.untaken("pi-hosted.s-untaken", String(Date.now()));
        // The press that asked saw no window, so the give-up is said.
        const untakenSaid = `${root.unhosted} ${root.unhostedWhy}`;
        expect("one no kitty took is said so, its press having opened nothing",
            untakenSaid === "UNTAKEN untaken", untakenSaid);
        Sessions.goTo(untaken);
        expect("one no kitty took is asked for again, not said as opening",
            Niri.spawns.length === untakenFrom + 2
            && Niri.spawns.slice(untakenFrom).every(one => one.includes("pi-hosted.s-untaken")),
            `${root.unhosted} ${root.unhostedWhy} ${Niri.spawns.slice(untakenFrom).join("\n")}`);
        // A script that gave up late, started before the press that asked
        // again, leaves that newer ask coming, so a third press opens no
        // second viewer beside it.
        Sessions.untaken("pi-hosted.s-untaken", String(Date.now() - 60000));
        expect("a give-up older than the ask leaves it coming",
            Sessions.viewerComing("pi-hosted.s-untaken"),
            JSON.stringify(Sessions.attaching));
        // Any program of Joel's can call the handler, so only a time in ms
        // is a give-up's, and only a viewer's app id one the script asks
        // under: a join's ask, say, is left coming as it was.
        Sessions.untaken("pi-hosted.s-untaken", "1700000000%3N");
        const joining = "harbour:JOIN";
        Sessions.attaching = Object.assign({}, Sessions.attaching, { [joining]: Date.now() });
        Sessions.untaken(joining, String(Date.now()));
        expect("a give-up of no ms, or of no viewer's app id, takes nothing",
            Sessions.viewerComing("pi-hosted.s-untaken") && joining in Sessions.attaching,
            JSON.stringify(Sessions.attaching));
        const joined = Object.assign({}, Sessions.attaching);
        delete joined[joining];
        Sessions.attaching = joined;
        // One asked for by itself into a kept place, unfocused, as
        // viewKept asks, then pressed for while it comes: the press is
        // said opening, and the viewer takes the focus once it's listed,
        // since Joel asked for it.
        const pressed = Object.assign({}, hosted, { address: "PRESSED",
            attach: hosted.attach.slice(0, -1).concat(["s-pressed"]),
            presence: Object.assign({}, hosted.presence, { session: "s-pressed" }) });
        Sessions.viewWith(pressed.attach, "pi-hosted.s-pressed", { workspace: 5, column: 1 },
            "Pressed");
        Sessions.goTo(pressed);
        const heldPress = root.unhosted === "PRESSED" && root.unhostedWhy === "opening";
        const came = { id: 96, pid: 960, opened: "pi-hosted.s-pressed" };
        const uncome = Niri.windows;
        Niri.windows = Object.assign({}, uncome, { "96": came });
        Niri.acted = null;
        Niri.windowOpenedOrChanged(came);
        Niri.windows = uncome;
        expect("one pressed for while it comes takes the focus as it's listed",
            heldPress && JSON.stringify(Niri.acted) === '{"FocusWindow":{"id":96}}',
            `${heldPress} ${JSON.stringify(Niri.acted)}`);
        // A row's Reopen, being Joel's press, opens its viewer where Go To
        // would, home on its orchestrator's workspace, and focused once
        // it comes.
        Niri.acted = null;
        Sessions.reopen({ id: "s-row-led", cwd: "/tmp", hosted: true, orchestratedBy: "FARLED",
            view: view.slice(0, -1).concat(["s-row-led"]) });
        const ledOpened = Niri.acted?.OpenNextWindowOn;
        expect("a row's Reopen opens its viewer home, to be focused as it comes",
            ledOpened?.reference?.Id === 8 && ledOpened?.app_id === "pi-hosted.s-row-led"
            && Sessions.goingTo?.app === "pi-hosted.s-row-led",
            `${JSON.stringify(Niri.acted)} ${JSON.stringify(Sessions.goingTo)}`);
        holdsAlone();
    }

    // One the realm's node holds here with no window, as the void is once
    // it leaves pi's host (OSPHT4, 2026-11-12): Go To attaches its hold in
    // a new window of the sessions' kitty, where niri opens one, focused,
    // and only once while that window comes.
    function holdsAlone() {
        const alone = {
            address: "ALONE7", here: true, window: null, kitty: null, held: "hold-7",
            presence: { session: "s-alone", addresses: ["ALONE7"] }
        };
        const before = Niri.spawns.length;
        Sessions.goTo(alone);
        const attached = asked(Niri.spawned.join(" "));
        expect("Go To attaches one held alone in a window of its own, where niri opens one",
            attached !== null && /^pi-session\.\d+$/.test(attached.classed)
            && attached.words === 'zsh -lic exec mycelium --realm "$0" attach "$1" harbour hold-7'
            && !Niri.spawned.includes("XDG_ACTIVATION_TOKEN"), Niri.spawned.join(" "));
        Sessions.goTo(alone);
        expect("attached only once while its window comes", Niri.spawns.length === before + 1,
            Niri.spawns.slice(before).join("\n"));
    }

    // One the node says is parked is listed apart, never running, however
    // its record reads; Wake asks the node to wake it, saying nothing, and
    // talking to it asks the node to steer its inbox, which wakes it too,
    // so the shell starts no program for either.
    function parks() {
        expect("only those the node says are parked are parked",
            parkedAs() === "DOZER4 NAPPER", parkedAs());
        // Those the node says are closed, one whose record says parked
        // among them, the latest first, by id when they ended at once, each
        // saying when it ended as the node's number, never a time written
        // out for the launcher to read back.
        const closed = Sessions.closed;
        expect("closed, as the node says each is",
            closed.map(one => one.id).join(" ") === "s-ENDED3 s-STALE7 s-WOKEN8",
            JSON.stringify(closed.map(one => one.id)));
        expect("each saying when it ended, and its address",
            closed.every(one => typeof one.ended === "number" && one.ended > 0
                && one.endedAt === undefined) && closed[0].address === "ENDED3",
            JSON.stringify(closed[0]));
        const napper = Sessions.parkedAs("NAPPER");
        expect("one parked is named by its record, in the window its holder keeps",
            napper.title === "Napper" && napper.window?.id === 22, JSON.stringify(napper));
        // The cutover wakes a parked one to move it, so where its window
        // sits is kept first, as a running one's is.
        expect("a parked session's window kept", Sessions.keepPlace("s-NAPPER") === "kept"
            && JSON.stringify(Sessions.keptPlaces["s-NAPPER"] ?? null).startsWith(
                '{"workspace":5,"column":1,'), JSON.stringify(Sessions.keptPlaces));
        // And one whose kitty opened its window with no app id, as the
        // oldest sessions' did, by the one window niri shows of its kitty.
        expect("a parked session's window kept by its kitty",
            Sessions.keepPlace("s-DOZER4") === "kept"
            && JSON.stringify(Sessions.keptPlaces["s-DOZER4"] ?? null).startsWith(
                '{"workspace":8,"column":3,'), JSON.stringify(Sessions.keptPlaces));
        // A sleep keeps its place marked so, for its window to open there
        // again as it wakes.
        expect("a sleeper's window kept as it goes to sleep",
            Sessions.keepPlaceAsleep("s-NAPPER") === "kept"
            && Sessions.keptPlaces["s-NAPPER"]?.slept === true,
            JSON.stringify(Sessions.keptPlaces));
        sleepsHosted();
        // Woken by Joel, as a quest's Return or a Go To wakes it, its kept
        // place is marked asked, so its window opens there as it wakes; a
        // wake nobody asked for opens none (Viewers.viewersDue).
        Sessions.keepPlaces({ "s-NAPPER": { workspace: 5, column: 1, at: 0, slept: true,
            asleep: true } });
        wokeAt = Niri.spawns.length;
        Sessions.wake(napper);
        const marked = Sessions.keptPlaces["s-NAPPER"] ?? null;
        expect("a holder's sleeper Joel wakes has its place asked for, from now",
            marked?.asked === true && marked.at > 0 && marked.workspace === 5,
            JSON.stringify(marked));
        Sessions.keepPlaces({});
        Sessions.talk("napper", "what now?");
        // One parked here that the node can't wake, its holder gone, is
        // reopened where it left off, since Joel asked for it and nothing
        // else will wake it; one parked on another machine never is, as
        // that would run it twice, nor one fenced by a word no fence goes by,
        // which could come back unfenced, nor one running here already. One
        // fenced strict comes back by its id under its own fence.
        const orphan = { address: "ORPHAN", here: true, window: null, held: null,
            folder: "/tmp", presence: { session: "s-orphan", state: "parked",
                quest: "QEST-ORPHAN", addresses: ["ORPHAN"], fence: "none" } };
        const like = (address, session, more) => Object.assign({}, orphan, { address,
            presence: Object.assign({}, orphan.presence, { session, addresses: [address] },
                more) });
        Sessions.wake(like("FENCED", "s-fenced", { fence: "sandbox" }));
        Sessions.wake(like("WARDED", "s-warded", { fence: "strict" }));
        // And one fenced by the fence's own word, which comes back under it,
        // never under strict; and one whose presence names no fence, which
        // says nothing of how it ran, so it stays asleep, said so.
        Sessions.wake(like("HEDGED", "s-hedged", { fence: "fence" }));
        Sessions.wake(like("UNSAID", "s-unsaid", { fence: undefined }));
        Sessions.wake(like("ALIVE", "s-LIVE01"));
        Sessions.wake(orphan);
        // Woken again before pi is up, it opens no second.
        Sessions.wake(orphan);
        // One with no session file is never started afresh in its place,
        // though its folder is there.
        Sessions.wake(Object.assign({}, orphan, { address: "NOFILE",
            folder: `${root.state}/trees/karst`,
            presence: Object.assign({}, orphan.presence, { session: "s-nofile",
                quest: "QEST-NOFILE", addresses: ["NOFILE"] }) }));
        Sessions.wake(Object.assign(like("STRAY", "s-stray"), { here: false }));
    }

    // One asleep in pi's host with no window wakes as it's viewed, which the
    // node's wake can't do: Wake views it where its window was as it went
    // to sleep, asking the node nothing, and lets that place go.
    function sleepsHosted() {
        const sleeper = {
            address: "SLEEPER", here: true, window: null, kitty: null, held: null,
            attach: ["mycelium", "--realm", "harbour", "attach", "--host", "harbour-attended",
                "--session", "s-sleeper"],
            presence: { hosted: { host: "harbour-attended", conversation: "s-sleeper" },
                session: "s-sleeper", state: "parked", addresses: ["SLEEPER"] }
        };
        Sessions.keepPlaces({ "s-sleeper": { workspace: 5, column: 2, at: 0, slept: true,
            asleep: true } });
        const before = Niri.spawns.length;
        const steered = steers.length;
        Sessions.wake(sleeper);
        const viewed = asked(Niri.spawns.slice(before).join(" "));
        expect("Wake views one asleep in pi's host, in a window named for it",
            viewed !== null && viewed.classed === "pi-hosted.s-sleeper",
            Niri.spawns.slice(before).join(" | "));
        const where = JSON.stringify(Niri.acted);
        expect("where its window was as it went to sleep", where
            === '{"OpenNextWindowOn":{"app_id":"pi-hosted.s-sleeper","reference":{"Id":5},'
            + '"column":2}}', where);
        expect("asking the node nothing", steers.length === steered,
            JSON.stringify(steers.slice(steered)));
        expect("and letting its place go", Sessions.keptPlaces["s-sleeper"] === undefined,
            JSON.stringify(Sessions.keptPlaces));
        // A wake is Joel's press, so a give-up is said to it (untaken),
        // and one pressed again while its viewer comes is said opening,
        // as Go To's is, rather than met with nothing.
        expect("a wake is told if no kitty takes its window",
            Sessions.pressedFor["pi-hosted.s-sleeper"]?.address === "SLEEPER",
            JSON.stringify(Sessions.pressedFor["pi-hosted.s-sleeper"] ?? null));
        const goingBefore = Sessions.goingTo;
        root.unhosted = "";
        root.unhostedWhy = "";
        const wokeAgain = Niri.spawns.length;
        Sessions.wake(sleeper);
        expect("and woken again while it comes, is said opening, asking none",
            root.unhosted === "SLEEPER" && root.unhostedWhy === "opening"
            && Niri.spawns.length === wokeAgain,
            `${root.unhosted} ${root.unhostedWhy} ${Niri.spawns.slice(wokeAgain).join(" | ")}`);
        Sessions.goingTo = goingBefore;

        // Go To on one (Agents' and For You's Deal With It alike) leaves
        // the waking to goTo's viewer, which opens where Joel is, focused,
        // rather than where it slept, holding goTo's off while it came.
        const dreamer = Object.assign({}, sleeper, {
            address: "DREAMER",
            attach: sleeper.attach.slice(0, -1).concat(["s-dreamer"]),
            presence: Object.assign({}, sleeper.presence, {
                hosted: { host: "harbour-attended", conversation: "s-dreamer" },
                session: "s-dreamer", addresses: ["DREAMER"] })
        });
        Sessions.keepPlaces({ "s-dreamer": { workspace: 5, column: 2, at: 0, slept: true,
            asleep: true } });
        const goneAt = Niri.spawns.length;
        Niri.acted = null;
        Sessions.wakeToGo(dreamer, dreamer);
        const opened = Niri.spawns.slice(goneAt);
        expect("Go To on one asleep in pi's host opens one viewer where Joel is",
            opened.length === 1 && asked(opened[0])?.classed === "pi-hosted.s-dreamer"
            && !JSON.stringify(Niri.acted).includes('"reference":{"Id":5}'),
            `${opened.join(" | ")} ${JSON.stringify(Niri.acted)}`);
        // Its closed window still named by the windows daemon's index alone,
        // which niri, its list trusted, no longer lists, it's as windowless.
        const unlisted = Object.assign({}, dreamer, { window: { id: 4040, pid: 500 } });
        expect("one whose window only the index names is still woken by viewing alone",
            Sessions.trustsNiri() && !("4040" in Niri.windows)
            && Sessions.wokenByViewing(unlisted), JSON.stringify(Object.keys(Niri.windows)));
        Sessions.keepPlaces({});
    }

    // Whether the node was asked to wake the one parked, and then to steer
    // it the words talked to it, with nothing run by the shell for either.
    property int wokeAt: -1
    function talked() {
        const told = steers.filter(one => one.ask.address === "NAPPER")
            .map(one => one.ask.steer);
        if (wokeAt < 0 || told.length < 2)
            return false;
        expect("Wake asks the node to wake the one parked, saying nothing",
            told[0].act === "wake" && told[0].from === "joel" && told[0].text === undefined
            && told[0].id === undefined, JSON.stringify(told[0]));
        expect("talking to one parked asks the node to steer it",
            told[1].text === "what now?" && told[1].from === "joel"
            && told[1].act === undefined, JSON.stringify(told[1]));
        // Later stages start sessions, so only what would write a file is
        // looked for.
        expect("and the shell writes nothing itself for either",
            !Niri.spawns.slice(wokeAt).some(one => one.includes("printf")
                || one.includes(".wake") || one.includes("/steer/")),
            Niri.spawns.slice(wokeAt).join(" | "));
        return true;
    }

    // Why each that didn't open as asked didn't, as its caller is told:
    // the id, the folder gone and where it started afresh instead.
    property var unopened: []
    // What Reopen Them All's walk said it found, reading by reading.
    property var tallied: []
    // Whether s-counted, its folder gone, was found so, and s-uncounted,
    // whose walk counts nothing.
    property bool countedGone: false
    property bool uncountedGone: false
    function tally(now) {
        tallied = tallied.concat([`${now.gone} gone, ${now.reopened} reopened`]);
        Qt.callLater(root.check);
    }
    function told(id) {
        return (cwd, startedIn) => {
            unopened = unopened.concat([`${id} ${cwd} ${startedIn}`]);
            Qt.callLater(root.check);
        };
    }

    // The sessions another boot's end took away are offered as the agents
    // daemon serves them, in its order, the last boot's apart from those
    // carried or given back, and the boot they're of known; each reopens
    // in its own window and folder, held, and one whose folder is gone
    // since opens nowhere, never in the home folder. A closed session
    // resumes in the folder its own record says, which needn't be the
    // home folder; one whose folder is gone starts its quest afresh in
    // the quest's start folder, or nowhere when it has none.
    function lastBoot() {
        expect("the last boot's, one hosted among them, in the daemon's order",
            Sessions.lastBoot.map(one => one.id).join(" ")
                === "s-before s-fresh s-given-back s-hosted s-far",
            JSON.stringify(Sessions.lastBoot));
        expect("the carried and given back apart, in its order",
            Sessions.apart.map(one => one.id).join(" ") === "s-ferry s-freed s-ferried",
            JSON.stringify(Sessions.apart));
        expect("the last boot's, then those apart", Sessions.toReopen.map(one => one.id).join(" ")
            === "s-before s-fresh s-given-back s-hosted s-far s-ferry s-freed s-ferried",
            JSON.stringify(Sessions.toReopen));
        // Each says the way it came, as the walk reads it (Reopening.satHere),
        // which decides whether one kept nowhere comes back without a window
        // (Reopening.broughtBack). On 2026-11-21 it was dropped, so two kept
        // nowhere got windows.
        expect("each says the way it came",
            Sessions.lastBoot.every(Reopening.satHere)
            && Sessions.apart.every(one => one.way !== undefined && !Reopening.satHere(one)),
            JSON.stringify(Sessions.toReopen.map(one => [one.id, one.way])));
        expect("the boot they're of, which places are kept by",
            Sessions.restoring.boot === "an-earlier-boot", JSON.stringify(Sessions.restoring));
        // Where that boot's windows sat, as the daemon keeps it, never
        // kept here.
        expect("the places kept are the daemon's",
            Sessions.arrangement["an-earlier-boot"]?.["s-elsewhere"]?.workspace === 2
            && Sessions.arrangedOthers["an-earlier-boot"]?.[0]?.app === "slack"
            && Sessions.arrangedBefore() === "an-earlier-boot",
            JSON.stringify(Sessions.arrangement));
        Sessions.lastBoot.forEach(one => Sessions.reopen(one, told(one.id)));
        // One held elsewhere is joined again once, however often asked.
        Sessions.reopen(Sessions.lastBoot.find(one => one.id === "s-far"), told("s-far"));
        Sessions.resume("s-kept", "QEST-KEPT", "/tmp", told("s-kept"));
        Sessions.resume("s-gone", "QEST-GONE", root.state, told("s-gone"));
        Sessions.resume("s-gone", "QEST-NONE", "", told("s-none"));
        // A walk begun with nothing to count, as the harness begins one,
        // holds s-uncounted, its folder gone, so Reopen Them All's lists
        // below join a walk with no tally of their own, and their count
        // is the one they bring; s-uncounted counts in none.
        Sessions.reopenAll([{ id: "s-uncounted", cwd: "/gone/s-uncounted" }], () => {
            root.uncountedGone = true;
            Qt.callLater(root.check);
        }, id => null, undefined);
        // Asked as the launcher asks, with no places of its own, the walk
        // takes the daemon's: the boot being restored kept places, so one
        // that sat here kept nowhere had no window, and comes back held.
        // Asked first, since niri here opens no window for the next to
        // wait on.
        Sessions.reopenAll([{ id: "s-unkept", cwd: "/tmp", way: "boot" }], told("s-unkept"),
            undefined, Sessions.carryOn, root.tally);
        // Reopen Them All asks niri for its screens first, and niri here
        // never answers, as it doesn't a request it refused. Given before
        // it would, Sessions joins each list to the one walk after, and
        // they count as one. s-counted, its folder gone, is found so as
        // s-unkept is begun, before s-patient, whose window niri never
        // opens; s-patient follows by the walk's pace.
        Sessions.reopenAll([{ id: "s-counted", cwd: "/gone/s-counted" }], () => {
            root.countedGone = true;
            Qt.callLater(root.check);
        }, id => null, undefined, root.tally);
        Sessions.reopenAll([{ id: "s-patient", cwd: "/tmp" }], told("s-patient"), id => null,
            undefined, root.tally);
        patience.start();
    }
    // Saying what the walk still waits on, since s-unkept opened in a
    // window would hold s-patient up as niri's answer would.
    Timer {
        id: patience
        interval: 12000
        onTriggered: {
            if (!Niri.spawns.some(one => one.endsWith(" pi --session s-patient")))
                root.fail("Reopen Them All waited on niri's answer for good, or on "
                    + `${Sessions.reopening?.waiting?.id ?? "nothing"}'s window`);
        }
    }

    readonly property string kitty: "sessions --directory"
    // Every window of the sessions' kitty is asked through the shell's one
    // opening script, which tells its kitty and its process the same class,
    // so a spawn reads as that script, the socket, the class and the words.
    readonly property string scope: `sh ${Sessions.opening} `
    function asked(command) {
        const at = command.indexOf(scope);
        if (at < 0)
            return null;
        const [socket, classed, ...words] = command.slice(at + scope.length).split(" ");
        return { socket: socket, classed: classed, words: words.join(" ") };
    }
    // The words the sessions' kitty is asked to open, or "" for anything
    // not asked of it.
    function inSessions(command) {
        const one = asked(command);
        return one === null ? "" : `sessions ${one.words}`;
    }
    function reopening() {
        const kitties = Niri.spawns.map(command => inSessions(command))
            .filter(command => command.startsWith("sessions ") && !command.includes("attach r-far")
                && !command.includes("attach --host") && !command.includes('attach "$1"'));
        // Six reopened at once, and the three woken that no holder keeps.
        if (kitties.length < 9 || unopened.length < 3 || !steered || !talked()
            || !kitties.some(one => one.endsWith(" pi --session s-orphan"))
            || root.unfiled === "" && !Niri.spawns.some(one => one.includes("QEST-NOFILE"))
            || !("FENCED" in root.asleep) && !Niri.spawns.some(one => one.includes("s-fenced"))
            || !kitties.some(one => one.endsWith(" pi --session s-warded"))
            || !kitties.some(one => one.endsWith(" pi --session s-hedged"))
            || !("UNSAID" in root.asleep) && !Niri.spawns.some(one => one.includes("s-unsaid"))
            || !Niri.spawns.some(one => one.includes("s-unkept")) || !countedGone
            || !uncountedGone)
            return;
        // Chosen as each list was given, before niri said its screens,
        // the tally would count each list alone, "1 gone, 0 reopened"
        // (core.nix#6126's round 3); and left with the walk they join,
        // which counts nothing, it would tell none.
        expect("lists given before the walk begins count as one walk, its own",
            tallied.join("; ") === "1 gone, 2 reopened", tallied.join("; "));
        expect("one the daemon's boot kept nowhere comes back held, with no window",
            Niri.spawns.filter(one => one.includes(" pi --session s-unkept ")).length === 1
            && Niri.spawns.some(one => one.includes(" pi --session s-unkept ")
                && one.includes('holds add --detached --folder "$1" -- '))
            && !kitties.some(one => one.includes("s-unkept")), Niri.spawns.join("\n"));
        expect("one parked that no holder keeps is reopened where it left off, once",
            kitties.filter(one => one.startsWith(`${kitty} ${state}/trees/karst zsh -lic `)
                && one.endsWith(" pi --session s-orphan")).length === 1, kitties.join("\n"));
        expect("and one with no session file is never started afresh, said as refused",
            !Niri.spawns.some(one => one.includes("s-nofile") || one.includes("QEST-NOFILE"))
            && root.unfiled.startsWith("NOFILE isn't parked"),
            `${root.unfiled}: ${Niri.spawns.join("\n")}`);
        expect("and one parked elsewhere never is, said as the node refused it",
            !Niri.spawns.some(one => one.includes("s-stray"))
            && root.strayed === "STRAY isn't parked with a holder to wake it this boot",
            `${root.strayed}: ${Niri.spawns.join("\n")}`);
        // Said as the Wake refused, so only a row becoming woken is taken
        // back for it (Agents' onUnsteered), never one a talk refused.
        expect("and its refusal names the act refused, a wake", root.strayedAct === "wake",
            root.strayedAct);
        expect("as one that couldn't be reopened does", root.unfiledAct === "wake",
            root.unfiledAct);
        expect("one fenced strict reopens by its id, PI_SANDBOX its own fence",
            kitties.filter(one => one.startsWith(
                `${kitty} ${state}/trees/karst env PI_SANDBOX=strict zsh -lic `)
                && one.endsWith(" pi --session s-warded")).length === 1, kitties.join("\n"));
        expect("one fenced by the fence's word reopens by its id under that word",
            kitties.filter(one => one.startsWith(
                `${kitty} ${state}/trees/karst env PI_SANDBOX=fence zsh -lic `)
                && one.endsWith(" pi --session s-hedged")).length === 1, kitties.join("\n"));
        // Each said as one that might come back unfenced, once, never as the
        // node's refusal, which blames only the holder it lacks.
        expect("nor one whose presence names no fence, said as one it might bring back unfenced",
            !Niri.spawns.some(one => one.includes("s-unsaid"))
            && root.asleep.UNSAID === "unfenced", `${JSON.stringify(root.asleep)}: `
                + Niri.spawns.join("\n"));
        expect("nor one fenced by a word no fence goes by, said so too",
            !Niri.spawns.some(one => one.includes("s-fenced"))
            && root.asleep.FENCED === "unfenced", `${JSON.stringify(root.asleep)}: `
                + Niri.spawns.join("\n"));
        expect("nor one running here already",
            !Niri.spawns.some(one => one.endsWith(" pi --session s-LIVE01")),
            Niri.spawns.join("\n"));
        // Each window is opened with an app id of its own, which its process
        // is told, so the node's record of what it holds names it as niri
        // lists it; a hosted one's viewer, a sleeper's, a waiting row's and
        // one whose viewer niri no longer lists too, by its session's id.
        // One no kitty took is asked for again once, by the id it was first.
        const asks = Niri.spawns.map(command => asked(command)).filter(one => one !== null);
        const untaken = asks.filter(one => one.classed === "pi-hosted.s-untaken").length;
        const named = asks.filter(one => one.classed !== "pi-hosted.s-untaken")
            .map(one => [one.classed]);
        const socket = `${Quickshell.env("XDG_RUNTIME_DIR")}/pi-sessions/kitty.sock`;
        const viewed = "hosted|sleeper|dreamer|stale|row-waiting|row-listed|row-stale|untaken"
            + "|pressed|row-led";
        const own = new RegExp(`^pi-((session|spawned)\\.\\d+|hosted\\.s-(${viewed}))$`);
        expect("each window is asked of the sessions' socket with an app id of its own",
            asks.every(one => one.socket === socket && own.test(one.classed))
            && new Set(named.map(([told]) => told)).size === named.length && untaken === 2,
            `${untaken} ${JSON.stringify(asks.map(one => [one.socket, one.classed]))}`);
        // What the script does with them is its own case's
        // (the-sessions-kitty-is-asked-only-its-own-way).
        expect("each is opened by the script beside the shell's files",
            Sessions.opening.endsWith("/opens-in-sessions.sh")
            && !Sessions.opening.startsWith("file:"), Sessions.opening);
        // Numbered on from when this shell started, so a shell started again,
        // as every switch does, names none as a window still open from before.
        const numbers = named.map(([told]) => /\.(\d+)$/.exec(told ?? ""))
            .filter(number => number !== null).map(number => Number(number[1]));
        expect("each window numbered past the moment its shell started",
            numbers.length > 0 && Math.abs(Date.now() - Sessions.startedAt) < 10 * 60 * 1000
                && numbers.every(number => number > Sessions.startedAt),
            `${Sessions.startedAt}: ${numbers.join(" ")}`);
        // Held as every start is, by the realm's node where there is one.
        const opened = command => kitties.some(one => one.startsWith(command[0])
            && one.endsWith(command[1]));
        // A start its record says that isn't the fence, or isn't a word,
        // is never passed on.
        expect("reopened in its own window and folder, started with nothing it can't be",
            opened([`${kitty} /tmp zsh -lic `, " pi --session s-before"]), kitties.join("\n"));
        // Started again with its fence, so it never comes back unfenced.
        expect("one pi never saved started under its own id, its quest loaded, fenced", opened([
            `${kitty} /tmp env QUEST_WORKFLOW_AUTOLOAD_ID=QEST-FRESH PI_SANDBOX=fence `
                + "PI_SPAWNER=on zsh -lic ", " pi --session-id s-fresh"]), kitties.join("\n"));
        expect("reopened though niri never said its screens",
            opened([`${kitty} /tmp zsh -lic `, " pi --session s-patient"]), kitties.join("\n"));
        expect("resumed in the folder its record says",
            opened([`${kitty} ${state}/trees/karst zsh -lic `, " pi --session s-kept"]),
            kitties.join("\n"));
        expect("its folder gone, its quest started afresh in the quest's",
            opened([`${kitty} ${state} env QUEST_WORKFLOW_AUTOLOAD_ID=QEST-GONE zsh -lic `, " pi"]),
            kitties.join("\n"));
        expect("one held elsewhere joined again by its words, in its folder here",
            kitties.includes(`${kitty} /tmp ssh -t lighthouse mycelium attach s-far`),
            kitties.join("\n"));
        expect("nine opened, none in the home folder nor a gone one", kitties.length === 9
            && !kitties.some(one => one.includes(`--directory ${Quickshell.env("HOME")} `)
                || one.includes("given-back") || one.includes("QEST-NONE")),
            kitties.join("\n"));
        expect("each not opened as asked told why", JSON.stringify(unopened.slice().sort())
            === JSON.stringify(["s-given-back /given-back ",
                `s-gone ${state}/trees/given-back ${state}`, `s-none ${state}/trees/given-back `]),
            JSON.stringify(unopened));
        // Forgetting one, the daemon is asked, which keeps it, by pi's id
        // for it, since the ask's own id is the interface's.
        stage = "forgetting";
        Sessions.forgetReopen({ id: "s-ferried" });
    }

    // Forgotten, one carried is offered no more, as the daemon says; put
    // off, the last boot's go, as it says too, and those apart stay.
    function forgetting() {
        const asked = agentsAsked;
        const ids = Sessions.toReopen.map(one => one.id).join(" ");
        if (asked.length === 1 && ids === "s-before s-fresh s-given-back s-hosted s-far "
            + "s-ferry s-freed") {
            expect("forgetting asks the daemon, naming the session",
                asked[0].session === "s-ferried" && asked[0].id.startsWith("ask-"),
                JSON.stringify(asked[0]));
            // Asked before it, on the same socket, so heard by now; a
            // viewer reopened since may have kept another after them.
            const told = placesAsked.slice(0, 2).map(one => Object.keys(one.places ?? {}).sort().join(" "));
            expect("the daemon is told the places kept, the one from before too, then none",
                JSON.stringify(told) === '["s-OTHER2 s-moving",""]', JSON.stringify(placesAsked));
            Sessions.putOffLastBoot();
        } else if (asked.length === 2 && ids === "s-ferry s-freed") {
            expect("Not Now asks the daemon to put the last boot off",
                asked[1].ask === "put-off" && Sessions.lastBoot.length === 0,
                JSON.stringify(asked[1]));
            stage = "forgotten";
            tell({ gone: ["session:s-LIVE01"] });
        }
    }

    // One the node says is gone goes.
    function forgotten() {
        if (addresses() !== "FARAWY NOWIN6 OTHER2 OUTRCH")
            return;
        // Told only that it's working now, it keeps the rest of what the
        // node said of it: running here, now in kitty 600's first window,
        // which nothing named.
        // Its terminal is read shut, so the node tells a new digest.
        stage = "differing";
        tell({
            changed: [{
                key: "session:s-NOWIN6",
                told: "0000000000000001",
                presence: { session: "s-NOWIN6", addresses: ["NOWIN6"], busy: true,
                    terminal: root.terminal("unix:@kitty-600", "1") }
            }]
        });
    }

    function differing() {
        const unseen = find("NOWIN6");
        if (!unseen || !unseen.presence.busy || !unseen.window)
            return;
        expect("a change saying only what differs keeps the rest", unseen.here
            && JSON.stringify(unseen.kitty) === '{"socket":"unix:@kitty-600","window":1}',
            JSON.stringify(unseen));
        // A window opened with no app id, as every one was before the shell
        // named them, is found by kitty's list: its first is niri's first.
        expect("of a kitty's several windows, one unnamed by kitty's own list",
            unseen.window.id === 20, unseen.window.id);
        // Then it rests, as a session's own work moves it, its window
        // the same, with the launcher showing it, since a shut one holds
        // a turn between working and listening back (PiSource.js).
        stage = "resting";
        retoldBefore = Object.assign({}, retold);
        PiSource.showing = true;
        tell({
            changed: [{
                key: "session:s-NOWIN6",
                presence: { session: "s-NOWIN6", addresses: ["NOWIN6"], busy: false,
                    terminal: root.terminal("unix:@kitty-600", "1") }
            }]
        });
    }

    // Only a running session moved, so what reads the parked and the
    // closed isn't told they did.
    function resting() {
        if (find("NOWIN6")?.presence.busy !== false || !whens.includes(null))
            return;
        // Shut, it watched only what moves a session's `told`, and shown,
        // it asked again under the same watch for every change.
        expect("a shut launcher's watch asks for what it reads, a shown one's for all",
            JSON.stringify(whens[0]) === '["told"]' && rewatched,
            `${JSON.stringify(whens)}, asked again under its id: ${rewatched}`);
        expect("a running session moving alone tells no parked or closed one",
            retold.parked === retoldBefore.parked && retold.closed === retoldBefore.closed,
            `parked told ${retold.parked - retoldBefore.parked}, closed `
            + `${retold.closed - retoldBefore.closed}`);
        PiSource.showing = false;
        // niri opening OTHER2's window anew, by the app id its record names,
        // is told by the windows daemon, and the session's window follows.
        const windows = Niri.windows;
        delete windows["21"];
        windows["23"] = { id: 23, pid: 600, title: "\u03c0 - Lighthouse", opened: "pi-session.7" };
        windowsTell({ changed: [windowRecord(23, 600, "pi-session.7")], gone: ["window:21"] });
        expect("a window opened anew by its app id is its session's once niri says so",
            find("OTHER2")?.window?.id === 23, JSON.stringify(find("OTHER2")?.window));
        // A window used is told with when, and the one used last goes
        // first where they're shown or picked, while the focus moving,
        // as it does while the launcher is shut, makes nothing again: no
        // windows by process, no sessions running.
        const inUse = () => Sessions.inUse(Sessions.running).map(session => session.address);
        const before = () => inUse().indexOf("NOWIN6") < inUse().indexOf("OTHER2");
        const nowin = find("NOWIN6").window;
        const made = root.windowsMade;
        const running = Sessions.running;
        windowsTell({ changed: [Object.assign({}, nowin, { order: 200000 }),
            windowRecord(23, 600, "pi-session.7", { order: 300000 })] });
        expect("the session whose window was used last goes first", !before(),
            inUse().join(" "));
        windowsTell({ changed: [Object.assign({}, nowin, { order: 400000 })] });
        expect("and the other once its window is used", before(), inUse().join(" "));
        expect("the focus moving makes nothing again",
            root.windowsMade === made && Sessions.running === running,
            `windows made ${root.windowsMade - made} times, running `
            + `${Sessions.running === running ? "kept" : "made again"}`);
        // A window retitled, or the keys leaving it, makes nothing again.
        windowsTell({ changed: [Object.assign({}, nowin, { order: 400000, title: "renamed",
            focused: false })] });
        expect("a window retitled makes no session's window again", root.windowsMade === made,
            `made ${root.windowsMade - made} times`);
        stage = "ended";
        tell({ changed: [{ key: "session:s-OTHER2", state: "ended", standing: "closed",
            told: "0000000000000002" }] });
    }

    // One the node says has ended goes, though its record stays.
    function ended() {
        if (addresses() !== "FARAWY NOWIN6 OUTRCH")
            return;
        stage = "lost";
        node.active = false;
        root.answering.connected = false;
    }

    // A node lost leaves what it said shown, never a moment with none.
    function lost() {
        expect("a node lost leaves what it said", addresses() === "FARAWY NOWIN6 OUTRCH",
            addresses());
        if (Sessions.node?.state !== "not answering" || drawnAsked)
            return;
        drawnAsked = true;
        drawn();
        console.log("a session is known by its own record");
        Qt.callLater(() => Qt.exit(0));
    }

    // What the sessions' kitty's unit writes beside its socket, as a case
    // writes it here: its pid, which says which windows it drew.
    FileView {
        id: kittyPidFile
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/pi-sessions/kitty.pid`
        blockWrites: true
    }
    property bool drawnAsked: false
    // Any program can give its window a viewer's app id, so once the
    // sessions' kitty's pid is known, as its unit writes it, a window it
    // didn't draw is never taken for a viewer: not raised or focused, not
    // let go of as one come, nor a running session's window. The pid read
    // is kept through a restart, so the case lets it go as it ends, leaving
    // any case after it the app id alone. Each window is heard as niri's
    // signal hears it, but called, so a throw fails the check rather than
    // passing as the signal's warning.
    function drawn() {
        const view = ["mycelium", "--realm", "harbour", "attach", "--host", "harbour-attended",
            "--session", "s-forged"];
        const named = "pi-hosted.s-forged";
        const unforged = Niri.windows;
        const focused = id => JSON.stringify(Niri.acted) === `{"FocusWindow":{"id":${id}}}`;
        // Each said, though one fails, as each holds apart from the others.
        const failed = [];
        const expect = (what, holds, said) => {
            if (!holds)
                failed.push(`${what}: ${said}`);
        };
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        const forged = { id: 88, pid: 880, opened: named };
        Niri.windows = Object.assign({}, unforged, { "88": forged });
        Niri.acted = null;
        const forgedFrom = Niri.spawns.length;
        Sessions.reopen({ id: "s-forged", cwd: "/tmp", hosted: true, view });
        expect("a window another program named as a viewer is never raised as one",
            !focused(88) && Niri.spawns.length === forgedFrom + 1,
            `${JSON.stringify(Niri.acted)} ${Niri.spawns.slice(forgedFrom).join("\n")}`);
        // As Go To asks for its viewer, so the one coming is focused.
        Sessions.goingTo = { app: named, at: Date.now(), from: Niri.focused };
        Niri.acted = null;
        Sessions.windowHeard(forged);
        expect("nor is it taken for the viewer come, nor focused as Go To's",
            Sessions.viewerComing(named) && Niri.acted === null
            && Sessions.goingTo?.app === named,
            `${JSON.stringify(Sessions.attaching)} ${JSON.stringify(Niri.acted)}`);
        const drawnHere = { id: 89, pid: 4242, opened: named };
        Niri.windows = Object.assign({}, unforged, { "88": forged, "89": drawnHere });
        Sessions.windowHeard(drawnHere);
        expect("while the one the sessions' kitty drew is taken as come, and focused",
            !Sessions.viewerComing(named) && Sessions.goingTo === null && focused(89),
            `${JSON.stringify(Sessions.attaching)} ${JSON.stringify(Niri.acted)}`);
        Niri.acted = null;
        Sessions.reopen({ id: "s-forged", cwd: "/tmp", hosted: true, view });
        expect("and raised as its viewer", focused(89), JSON.stringify(Niri.acted));
        // A running one's window, named by its viewer's app id, is only
        // one the sessions' kitty drew, so Go To raises none another did.
        const hosting = { hosted: true, session: "s-forged" };
        const window = Sessions.windowOf({ presence: hosting }, {}, { [named]: [forged] });
        Niri.windows = Object.assign({}, unforged, { "88": forged });
        Niri.acted = null;
        const goneFrom = Niri.spawns.length;
        Sessions.goTo({ address: "FORGED", presence: hosting, attach: view, window: forged,
            here: true });
        const goneFor = Niri.spawns.slice(goneFrom);
        expect("nor is it a running session's window, nor raised by Go To",
            window === null && !focused(88), `${JSON.stringify(window)} `
                + JSON.stringify(Niri.acted));
        // Its own viewer is asked for, and whose window holds its name is
        // said, so a pid read wrong, with every viewer refused, is seen at
        // the press.
        expect("Go To opens its own, saying another program's window holds its name",
            goneFor.length === 1 && asked(goneFor[0])?.classed === named
            && root.unhosted === "FORGED" && root.unhostedWhy === "unowned",
            `${root.unhosted} ${root.unhostedWhy} ${goneFor.join(" | ")}`);
        // Pressed again while that one comes, it's held back, said opening.
        Sessions.goTo({ address: "FORGED", presence: hosting, attach: view, window: forged,
            here: true });
        expect("while its own comes, Go To says it's opening, opening no other",
            Niri.spawns.length === goneFrom + 1 && root.unhostedWhy === "opening",
            `${root.unhostedWhy} ${Niri.spawns.slice(goneFrom).join(" | ")}`);
        // The window that comes for it is refused too, as a pid read wrong
        // refuses its own: Go To asks for none again, now or once its
        // patience runs out, and says why, rather than opening another
        // viewer of the same session with every press.
        const cameRefused = { id: 93, pid: 880, opened: named };
        Niri.windows = Object.assign({}, unforged, { "88": forged, "93": cameRefused });
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.windowHeard(cameRefused);
        expect("the press waiting on it is told its own came refused",
            root.unhosted === "FORGED" && root.unhostedWhy === "disowned",
            `${root.unhosted} ${root.unhostedWhy}`);
        expect("and is told only once", !(named in Sessions.pressedFor),
            JSON.stringify(Object.keys(Sessions.pressedFor)));
        // Nor is it still taken as coming, so a press once that window
        // closes asks afresh, rather than saying opening of nothing.
        expect("nor is its viewer still taken as coming", !Sessions.viewerComing(named),
            JSON.stringify(Sessions.attaching[named] ?? null));
        root.unhostedWhy = "";
        Sessions.goTo({ address: "FORGED", presence: hosting, attach: view, window: forged,
            here: true });
        expect("one whose own came refused is asked for no more, said so",
            Niri.spawns.length === goneFrom + 1 && root.unhostedWhy === "disowned",
            `${root.unhostedWhy} ${Niri.spawns.slice(goneFrom).join(" | ")}`);
        // Nor by a wake, which keeps where it slept for the press that can
        // open it once that window closes.
        const sleptForged = { address: "FORGED", here: true, window: null, kitty: null,
            attach: view, presence: { hosted: true, session: "s-forged", state: "parked" } };
        const keptBeforeWake = Sessions.keptPlaces;
        Sessions.keepPlaces(Object.assign({}, keptBeforeWake, { "s-forged": { workspace: 5,
            column: 2, at: 0, slept: true, asleep: true } }));
        root.unhostedWhy = "";
        Sessions.wake(sleptForged);
        expect("nor by a wake, said so, keeping where it slept",
            Niri.spawns.length === goneFrom + 1 && root.unhostedWhy === "disowned"
            && Sessions.keptPlaces["s-forged"]?.slept === true,
            `${root.unhostedWhy} ${JSON.stringify(Sessions.keptPlaces["s-forged"] ?? null)}`);
        // Nor by the walk's wake, which says nothing, being no press, and
        // keeps where it slept too.
        root.unhostedWhy = "";
        Sessions.wakeReopened(sleptForged, { one: { id: "s-forged" }, at: null, told: "" });
        expect("nor by the walk's wake, unsaid, keeping where it slept",
            Niri.spawns.length === goneFrom + 1 && root.unhostedWhy === ""
            && Sessions.keptPlaces["s-forged"]?.slept === true,
            `${root.unhostedWhy} ${JSON.stringify(Sessions.keptPlaces["s-forged"] ?? null)}`);
        Sessions.keepPlaces(keptBeforeWake);
        // The walk's wake (wakeReopened) is no press: with its viewer
        // already coming it says nothing and takes no focus, as a handover
        // wakes it with Joel nowhere near.
        const walkWords = view.slice(0, -1).concat(["s-walk-woken"]);
        const walkWoken = { address: "WALKWOKEN", here: true, window: null, kitty: null,
            attach: walkWords, presence: { hosted: true, session: "s-walk-woken",
                state: "parked" } };
        // Marked coming rather than asked for, so it spawns nothing the
        // counts below would take for another ask.
        Sessions.attaching = Object.assign({}, Sessions.attaching,
            { "pi-hosted.s-walk-woken": Date.now() });
        const keptBeforeWalk = Sessions.keptPlaces;
        Sessions.keepPlaces(Object.assign({}, keptBeforeWalk, { "s-walk-woken": {
            workspace: 5, column: 2, at: 0, slept: true, asleep: true } }));
        const goingBeforeWalk = Sessions.goingTo;
        root.unhosted = "";
        Sessions.wakeReopened(walkWoken, { one: { id: "s-walk-woken" }, at: null, told: "" });
        expect("the walk's wake while its viewer comes says nothing, nor takes the focus",
            root.unhosted === "" && Sessions.goingTo === goingBeforeWalk,
            `${root.unhosted} ${JSON.stringify(Sessions.goingTo)}`);
        Sessions.goingTo = goingBeforeWalk;
        Sessions.keepPlaces(keptBeforeWalk);
        Sessions.forgetAsk("pi-hosted.s-walk-woken");
        // Nor by a row's Reopen, nor by the shell's own asks, once its
        // patience has run out, and none of them lets Go To ask again.
        Sessions.attaching = Object.assign({}, Sessions.attaching,
            { [named]: Date.now() - Sessions.viewPatience - 1 });
        root.unhostedWhy = "";
        Sessions.reopen({ id: "s-forged", cwd: "/tmp", hosted: true, view });
        expect("nor by a row's Reopen, said so",
            Niri.spawns.length === goneFrom + 1 && root.unhostedWhy === "disowned",
            `${root.unhostedWhy} ${Niri.spawns.slice(goneFrom).join(" | ")}`);
        const askedAgain = Sessions.viewWith(view, named, null, "forged");
        root.unhostedWhy = "";
        Sessions.goTo({ address: "FORGED", presence: hosting, attach: view, window: forged,
            here: true });
        expect("nor by the shell's own asks, after which Go To still asks none",
            !askedAgain && Niri.spawns.length === goneFrom + 1
            && root.unhostedWhy === "disowned",
            `${askedAgain} ${root.unhostedWhy} ${Niri.spawns.slice(goneFrom).join(" | ")}`);
        // Once the window that came for it closes, a stray listed before
        // the ask holds the name alone: the refusal goes with the window it
        // was of, so Go To opens its own again, saying why, rather than
        // saying disowned of a window that came for no ask.
        Niri.windows = Object.assign({}, unforged, { "88": forged });
        root.unhostedWhy = "";
        Sessions.goTo({ address: "FORGED", presence: hosting, attach: view, window: forged,
            here: true });
        expect("once the window that came closes, Go To opens its own again, said so",
            Niri.spawns.length === goneFrom + 2 && root.unhostedWhy === "unowned",
            `${root.unhostedWhy} ${Niri.spawns.slice(goneFrom).join(" | ")}`);
        // No kitty took the ask whose window came refused, so that window
        // was another program's, not its own drawn by a pid read wrong: the
        // refusal goes, and Go To opens its own beside it, saying why,
        // rather than holding it unopenable while that window stays. A
        // give-up older than the ask is another's and leaves it.
        const untook = "pi-hosted.s-untook";
        const untookWords = view.slice(0, -1).concat(["s-untook"]);
        const untookSession = { address: "UNTOOK", presence: { hosted: true,
            session: "s-untook" }, attach: untookWords, here: true };
        Sessions.viewWith(untookWords, untook, null, "untook", { address: "UNTOOKFIRST" });
        const untookAsked = Number(Sessions.attaching[untook]);
        const untookStray = { id: 102, pid: 880, opened: untook };
        Niri.windows = Object.assign({}, unforged, { "102": untookStray });
        Sessions.windowHeard(untookStray);
        const refusedBefore = Sessions.refusedOwn(untook);
        // A press meanwhile, said disowned, takes the focus its own would;
        // once no kitty took the ask, it's told so, and that focus is let
        // go, as nothing it asked for comes.
        const goingBeforeUntook = Sessions.goingTo;
        Sessions.goTo(untookSession);
        const untookPressTook = Sessions.goingTo?.app === untook;
        // A give-up none of the ask's ends no refusal, nor lets the press's
        // focus go, nor tells it anything.
        const untookHeld = () => Sessions.refusedOwn(untook) && Sessions.goingTo?.app === untook
            && untook in Sessions.disownedSaid && root.unhostedWhy === "disowned";
        Sessions.untaken(untook, String(untookAsked - 1));
        const keptOnOlder = untookHeld();
        // A time from when the script can no longer answer for that ask is
        // none of its give-ups either.
        Sessions.untaken(untook, String(untookAsked + Sessions.answerPatience));
        const keptOnLast = untookHeld();
        Sessions.untaken(untook, String(untookAsked + Sessions.answerPatience + 1));
        const keptOnLater = untookHeld();
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.untaken(untook, String(untookAsked));
        const untookTold = `${root.unhosted} ${root.unhostedWhy}`;
        const untookLetGo = Sessions.goingTo?.app !== untook;
        expect("a give-up timed past the script's answer ends no refusal, nor touches the press",
            keptOnLast && keptOnLater, `${keptOnLast} ${keptOnLater} `
                + `${JSON.stringify(Sessions.refusedAsks)} ${JSON.stringify(Sessions.goingTo)}`);
        expect("a press told disowned is told no kitty took it, its focus let go",
            untookPressTook && untookTold === "UNTOOK untaken" && untookLetGo,
            `${untookPressTook} ${untookTold} ${JSON.stringify(Sessions.goingTo)}`);
        const untookFrom = Niri.spawns.length;
        root.unhostedWhy = "";
        Sessions.goTo(untookSession);
        expect("a refusal goes once no kitty took its ask, so Go To opens its own, said so",
            refusedBefore && keptOnOlder && !Sessions.refusedOwn(untook)
            && Niri.spawns.length === untookFrom + 1 && root.unhostedWhy === "unowned",
            `${refusedBefore} ${keptOnOlder} ${root.unhostedWhy} `
                + Niri.spawns.slice(untookFrom).join(" | "));
        Sessions.forgetAsk(untook);
        Sessions.goingTo = goingBeforeUntook;
        Niri.windows = Object.assign({}, unforged, { "88": forged });
        // A viewer Reopen Them All kept a place for, opened there by itself
        // once Joel isn't on it, that no kitty took keeps that place, as it
        // was, so it opens there once a kitty can, rather than wherever
        // niri puts it next, or nowhere.
        const keptNamed = "pi-hosted.s-kept-untaken";
        const keptPlace = { workspace: 5, column: 1, at: Date.now(), asked: true,
            waiting: view.slice(0, -1).concat(["s-kept-untaken"]), named: keptNamed,
            title: "Reef" };
        const keptBeforeUntaken = Sessions.keptPlaces;
        Sessions.keepPlaces({ "s-kept-untaken": keptPlace });
        const keptFrom = Niri.spawns.length;
        const keptSpawns = () => Niri.spawns.slice(keptFrom)
            .filter(command => command.includes(keptNamed)).length;
        Niri.acted = null;
        Sessions.viewKept();
        const keptAsked = keptSpawns() === 1
            && Sessions.keptPlaces["s-kept-untaken"] === undefined;
        const keptAskedAt = JSON.stringify(Niri.acted);
        Sessions.untaken(keptNamed, String(Sessions.attaching[keptNamed]));
        const keptBack = JSON.stringify(Sessions.keptPlaces["s-kept-untaken"]);
        // It's asked for again only once a kitty draws under another pid,
        // as one alive that only didn't answer may yet draw the first ask's
        // window, and one down comes back under a new one.
        Sessions.viewKept();
        const keptHeld = keptSpawns() === 1
            && Sessions.keptPlaces["s-kept-untaken"] !== undefined;
        // Written as a kitty started again writes it, in a folder made
        // anew, which no watch hears, its unit then saying it started
        // (kittyStarted), as niri may say nothing while Joel is away.
        kittyPidFile.setText("4343\n");
        Niri.acted = null;
        Sessions.kittyStarted?.();
        const keptAskedAgain = keptSpawns() === 2 && keptAskedAt.includes(keptNamed)
            && JSON.stringify(Niri.acted) === keptAskedAt;
        expect("a kept viewer no kitty took keeps its place as it was, under the kitty's pid",
            keptAsked
            && keptBack === JSON.stringify(Object.assign({}, keptPlace, { keptUnder: "4242" })),
            `${keptAsked} ${keptBack}`);
        expect("and is asked there again only once a kitty draws under another pid",
            keptHeld && keptAskedAgain,
            `${keptHeld} ${keptSpawns()} ${keptAskedAt} ${JSON.stringify(Niri.acted)}`);
        // Given up on again, its place is let go, so a kitty that stays down
        // isn't asked for it again with every move of niri's.
        Sessions.untaken(keptNamed, String(Sessions.attaching[keptNamed]));
        const keptLetGo = Sessions.keptPlaces["s-kept-untaken"] === undefined;
        kittyPidFile.setText("4444\n");
        Sessions.readKittyPid();
        Sessions.viewKept();
        expect("given up on again, its place is let go, and it's asked for no more",
            keptLetGo && keptSpawns() === 2,
            `${JSON.stringify(Sessions.keptPlaces["s-kept-untaken"])} ${keptSpawns()}`);
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        Sessions.forgetAsk(keptNamed);
        // A kitty down as it's asked, its pid file gone with its folder,
        // keeps the place under the pid last read, held while the file
        // says none, and asked for again once a kitty writes another.
        const keptOne = (id, marks) => Object.assign({}, keptPlace,
            { waiting: view.slice(0, -1).concat([id]), named: `pi-hosted.${id}` }, marks ?? {});
        const spawnsOf = (named, from) => Niri.spawns.slice(from)
            .filter(command => command.includes(named)).length;
        // The pid a place was kept under, kept still or asked for since.
        const keptUnderOf = id => Sessions.keptPlaces[id]?.keptUnder
            ?? Sessions.keptAsks[`pi-hosted.${id}`]?.place?.keptUnder;
        const downNamed = "pi-hosted.s-kept-down";
        Sessions.keepPlaces({ "s-kept-down": keptOne("s-kept-down") });
        const downFrom = Niri.spawns.length;
        kittyPidFile.setText("\n");
        Sessions.viewKept();
        Sessions.untaken(downNamed, String(Sessions.attaching[downNamed]));
        const downUnder = Sessions.keptPlaces["s-kept-down"]?.keptUnder;
        Sessions.viewKept();
        const downHeld = spawnsOf(downNamed, downFrom) === 1;
        kittyPidFile.setText("5353\n");
        Sessions.kittyStarted?.();
        expect("a kitty down as it's asked keeps the place under the pid last read, held so",
            downUnder === "4242" && downHeld, `${downUnder} ${spawnsOf(downNamed, downFrom)}`);
        expect("and a kitty come back, as its unit says it started, is asked for it",
            spawnsOf(downNamed, downFrom) === 2, String(spawnsOf(downNamed, downFrom)));
        Sessions.untaken(downNamed, String(Sessions.attaching[downNamed]));
        Sessions.forgetAsk(downNamed);
        // A kitty started again while the script waited, its pid read
        // before the give-up is heard, as its unit says it started first,
        // isn't the one the ask went to: the place is kept under the pid
        // asked, and the new kitty asked as it's kept, as nothing else
        // would ask while niri is quiet.
        const lateNamed = "pi-hosted.s-kept-late";
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        Sessions.keepPlaces({ "s-kept-late": keptOne("s-kept-late") });
        const lateFrom = Niri.spawns.length;
        Sessions.viewKept();
        kittyPidFile.setText("6363\n");
        Sessions.readKittyPid();
        Sessions.untaken(lateNamed, String(Sessions.attaching[lateNamed]));
        const lateUnder = keptUnderOf("s-kept-late");
        expect("a kitty started again before the give-up is heard is asked, as not the one asked",
            lateUnder === "4242" && spawnsOf(lateNamed, lateFrom) === 2,
            `${lateUnder} ${spawnsOf(lateNamed, lateFrom)}`);
        Sessions.untaken(lateNamed, String(Sessions.attaching[lateNamed]));
        Sessions.forgetAsk(lateNamed);
        // A kitty down as it's asked, come back while the script waits up
        // to 20 s for its socket, is the one the script asks: kept under it,
        // it isn't asked again at once, as it may yet draw the first ask's
        // window, which a second ask would open another beside.
        const backNamed = "pi-hosted.s-kept-back";
        Sessions.keepPlaces({ "s-kept-back": keptOne("s-kept-back") });
        const backFrom = Niri.spawns.length;
        kittyPidFile.setText("\n");
        Sessions.viewKept();
        kittyPidFile.setText("5454\n");
        Sessions.kittyStarted?.();
        Sessions.untaken(backNamed, String(Sessions.attaching[backNamed]));
        const backUnder = keptUnderOf("s-kept-back");
        expect("a kitty come back while the script waited is taken as asked, not asked again",
            backUnder === "5454" && spawnsOf(backNamed, backFrom) === 1,
            `${backUnder} ${spawnsOf(backNamed, backFrom)}`);
        Sessions.forgetAsk(backNamed);
        // A kitty the shell didn't hear start, as when the unit's word is
        // missed, asked by the pid its file says, isn't taken for another
        // at the give-up: the file is read again, not the pid last heard.
        const staleNamed = "pi-hosted.s-kept-stale";
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        Sessions.keepPlaces({ "s-kept-stale": keptOne("s-kept-stale") });
        const staleFrom = Niri.spawns.length;
        kittyPidFile.setText("6565\n");
        Sessions.viewKept();
        Sessions.untaken(staleNamed, String(Sessions.attaching[staleNamed]));
        const staleUnder = keptUnderOf("s-kept-stale");
        expect("a kitty the shell didn't hear start, asked, isn't asked again at the give-up",
            staleUnder === "6565" && spawnsOf(staleNamed, staleFrom) === 1,
            `${staleUnder} ${spawnsOf(staleNamed, staleFrom)}`);
        Sessions.forgetAsk(staleNamed);
        // A press holding the ask (heldBack), asked again of a kitty come
        // since, is told it's opening and keeps its focus, as its viewer is
        // on its way, rather than told that ask is over.
        const pressedNamed = "pi-hosted.s-kept-pressed";
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        Sessions.keepPlaces({ "s-kept-pressed": keptOne("s-kept-pressed") });
        const pressedFrom = Niri.spawns.length;
        Sessions.viewKept();
        const goingBeforePressed = Sessions.goingTo;
        Sessions.goTo({ address: "PRESSED",
            presence: { hosted: true, session: "s-kept-pressed" },
            attach: keptOne("s-kept-pressed").waiting, here: true });
        kittyPidFile.setText("6666\n");
        Sessions.readKittyPid();
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.untaken(pressedNamed, String(Sessions.attaching[pressedNamed]));
        expect("a press asked again of a kitty come since is told it's opening, focus kept",
            spawnsOf(pressedNamed, pressedFrom) === 2
            && `${root.unhosted} ${root.unhostedWhy}` === "PRESSED opening"
            && Sessions.goingTo?.app === pressedNamed
            && Sessions.pressedFor[pressedNamed]?.address === "PRESSED",
            `${spawnsOf(pressedNamed, pressedFrom)} ${root.unhosted} ${root.unhostedWhy} `
                + `${JSON.stringify(Sessions.goingTo)}`);
        Sessions.goingTo = goingBeforePressed;
        Sessions.untaken(pressedNamed, String(Sessions.attaching[pressedNamed]));
        Sessions.forgetAsk(pressedNamed);
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        // With no kitty's pid ever read, a kitty come again can't be told
        // from the one given up on, so the place is let go, not held unasked.
        const unknownNamed = "pi-hosted.s-kept-unknown";
        kittyPidFile.setText("\n");
        Sessions.kittyPidRead = "";
        Sessions.keepPlaces({ "s-kept-unknown": keptOne("s-kept-unknown") });
        Sessions.viewKept();
        const unknownAsked = unknownNamed in Sessions.keptAsks;
        Sessions.untaken(unknownNamed, String(Sessions.attaching[unknownNamed]));
        const unknownLeft = JSON.stringify(Sessions.keptPlaces["s-kept-unknown"]);
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        expect("with no kitty's pid known, a kept viewer no kitty took has its place let go",
            unknownAsked && unknownLeft === undefined, `${unknownAsked} ${unknownLeft}`);
        Sessions.forgetAsk(unknownNamed);
        // A press since the walk's ask, its window come refused and closed,
        // asks afresh; no kitty taking that, the walk's place isn't kept
        // again, as only the walk's own ask let it go, and the press is
        // told, its focus let go.
        const freshNamed = "pi-hosted.s-kept-fresh";
        const freshWords = view.slice(0, -1).concat(["s-kept-fresh"]);
        Sessions.keepPlaces({ "s-kept-fresh":
            Object.assign({}, keptPlace, { waiting: freshWords, named: freshNamed }) });
        Sessions.viewKept();
        const freshKeptAsk = freshNamed in Sessions.keptAsks;
        const freshStray = { id: 109, pid: 880, opened: freshNamed };
        const listedBeforeFresh = Niri.windows;
        Niri.windows = Object.assign({}, listedBeforeFresh, { "109": freshStray });
        Sessions.windowHeard(freshStray);
        Niri.windows = listedBeforeFresh;
        Sessions.viewWith(freshWords, freshNamed, null, "Reef", { address: "FRESHASK" });
        const goingBeforeFresh = Sessions.goingTo;
        // Joel's Go To while that ask comes takes it over, focus and all
        // (heldBack).
        Sessions.goTo({ address: "FRESH", presence: { hosted: true, session: "s-kept-fresh" },
            attach: freshWords, here: true });
        const freshTook = Sessions.goingTo?.app === freshNamed;
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.untaken(freshNamed, String(Sessions.attaching[freshNamed]));
        expect("a press since the walk's ask, no kitty taking it, keeps no walk's place, told so",
            freshKeptAsk && freshTook && Sessions.keptPlaces["s-kept-fresh"] === undefined
            && `${root.unhosted} ${root.unhostedWhy}` === "FRESH untaken"
            && Sessions.goingTo === null,
            `${freshKeptAsk} ${JSON.stringify(Sessions.keptPlaces["s-kept-fresh"])} `
                + `${root.unhosted} ${root.unhostedWhy} ${JSON.stringify(Sessions.goingTo)}`);
        Sessions.goingTo = goingBeforeFresh;
        Sessions.forgetAsk(freshNamed);
        // A walk's viewer whose window came refused, no kitty taking its ask,
        // keeps the walk's place as one no window came for does, under the
        // kitty's pid, as the refusal ends (refusedUntaken).
        const refusedKept = "pi-hosted.s-kept-refused";
        const refusedKeptPlace = Object.assign({}, keptPlace,
            { waiting: view.slice(0, -1).concat(["s-kept-refused"]), named: refusedKept });
        Sessions.keepPlaces({ "s-kept-refused": refusedKeptPlace });
        Sessions.viewKept();
        const refusedKeptAsk = refusedKept in Sessions.keptAsks;
        const refusedKeptStray = { id: 110, pid: 880, opened: refusedKept };
        const listedBeforeRefusedKept = Niri.windows;
        Niri.windows = Object.assign({}, listedBeforeRefusedKept, { "110": refusedKeptStray });
        Sessions.windowHeard(refusedKeptStray);
        const refusedKeptAt = Sessions.refusedAt[refusedKept];
        Sessions.untaken(refusedKept, String(refusedKeptAt));
        const refusedKeptBack = JSON.stringify(Sessions.keptPlaces["s-kept-refused"]);
        expect("a walk's viewer come refused, no kitty taking it, keeps the walk's place",
            refusedKeptAsk && refusedKeptAt !== undefined && !Sessions.refusedOwn(refusedKept)
            && refusedKeptBack
                === JSON.stringify(Object.assign({}, refusedKeptPlace, { keptUnder: "4242" })),
            `${refusedKeptAsk} ${refusedKeptAt} ${refusedKeptBack}`);
        Niri.windows = listedBeforeRefusedKept;
        Sessions.forgetAsk(refusedKept);
        Sessions.keepPlaces(keptBeforeUntaken);
        // A stray come for an ask while its press waits is said disowned at
        // once, as a pid read wrong is likelier. A press again meanwhile is
        // said so too, but takes the focus its own would; and its own come
        // after, the stray still listed, is focused and said come, so the
        // foot doesn't keep saying it isn't viewed.
        const cameAfter = "pi-hosted.s-came-after";
        const cameAfterWords = view.slice(0, -1).concat(["s-came-after"]);
        const cameAfterRow = { address: "CAMEAFTER", here: true, attach: cameAfterWords,
            presence: { hosted: true, session: "s-came-after" }, window: null };
        const listedBeforeAfter = Niri.windows;
        const goingBeforeAfter = Sessions.goingTo;
        Sessions.viewWith(cameAfterWords, cameAfter, null, "Reef", { address: "FIRSTPRESS" });
        const strayFirst = { id: 104, pid: 880, opened: cameAfter };
        Niri.windows = Object.assign({}, listedBeforeAfter, { "104": strayFirst });
        root.unhostedWhy = "";
        Sessions.windowHeard(strayFirst);
        const saidFirst = `${root.unhosted} ${root.unhostedWhy}`;
        Sessions.goingTo = null;
        root.unhostedWhy = "";
        Sessions.goTo(cameAfterRow);
        const saidAgain = `${root.unhosted} ${root.unhostedWhy}`;
        const tookFocus = Sessions.goingTo?.app === cameAfter;
        const ownAfter = { id: 105, pid: 4242, opened: cameAfter };
        Niri.windows = Object.assign({}, listedBeforeAfter, { "104": strayFirst, "105": ownAfter });
        Niri.acted = null;
        root.unhostedWhy = "";
        Sessions.windowHeard(ownAfter);
        expect("a stray come for an ask is said disowned at once",
            saidFirst === "FIRSTPRESS disowned", saidFirst);
        expect("a press again meanwhile is said disowned, taking the focus its own would",
            saidAgain === "CAMEAFTER disowned" && tookFocus, `${saidAgain} ${tookFocus}`);
        expect("its own come after is focused, and said come after the stray",
            focused(105) && root.unhosted === "CAMEAFTER" && root.unhostedWhy === "cameAfter",
            `${JSON.stringify(Niri.acted)} ${root.unhosted} ${root.unhostedWhy}`);
        Sessions.forgetAsk(cameAfter);
        Sessions.goingTo = goingBeforeAfter;
        Niri.windows = listedBeforeAfter;
        // The window refused for an ask, seen as the kitty's once its pid is
        // read again, is said viewed as its own after all, not as one come
        // after another, as no other came.
        const ownAll = "pi-hosted.s-own-after-all";
        Sessions.viewWith(view.slice(0, -1).concat(["s-own-after-all"]), ownAll, null, "Reef",
            { address: "OWNALL" });
        const misread = { id: 106, pid: 880, opened: ownAll };
        Niri.windows = Object.assign({}, listedBeforeAfter, { "106": misread });
        Sessions.windowHeard(misread);
        kittyPidFile.setText("880\n");
        Sessions.readKittyPid();
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.windowHeard(misread);
        const ownAllSaid = `${root.unhosted} ${root.unhostedWhy}`;
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        expect("the window refused, seen as its own once the pid is read again, is said so",
            ownAllSaid === "OWNALL ownAfterAll", ownAllSaid);
        Sessions.forgetAsk(ownAll);
        Niri.windows = listedBeforeAfter;
        // And so as Go To raises it, its pid read first, though niri says
        // nothing of it changing: the press told disowned is told then.
        const ownRaised = "pi-hosted.s-own-raised";
        const ownRaisedWords = view.slice(0, -1).concat(["s-own-raised"]);
        Sessions.viewWith(ownRaisedWords, ownRaised, null, "Reef", { address: "OWNRAISED" });
        const misreadRaised = { id: 111, pid: 880, opened: ownRaised };
        Niri.windows = Object.assign({}, listedBeforeAfter, { "111": misreadRaised });
        Sessions.windowHeard(misreadRaised);
        // A press meanwhile is told disowned and takes the focus its own
        // would; raised, its own focused by the press that raised it, that
        // focus is let go, or a retitle would take Joel back to it.
        const goingBeforeRaised = Sessions.goingTo;
        Sessions.goingTo = null;
        const raisedRow = { presence: { hosted: true, session: "s-own-raised" },
            attach: ownRaisedWords, here: true };
        Sessions.goTo(Object.assign({ address: "RAISEDFIRST" }, raisedRow));
        const raisedTookFocus = Sessions.goingTo?.app === ownRaised;
        kittyPidFile.setText("880\n");
        root.unhosted = "";
        root.unhostedWhy = "";
        Niri.acted = null;
        Sessions.goTo(Object.assign({ address: "RAISER" }, raisedRow));
        const ownRaisedSaid = `${root.unhosted} ${root.unhostedWhy}`;
        const raisedFocused = focused(111);
        const raisedLetGo = Sessions.goingTo === null;
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        expect("the window refused, raised by Go To once its pid is read again, is said so then",
            raisedFocused && ownRaisedSaid === "RAISEDFIRST ownAfterAll",
            `${raisedFocused} ${ownRaisedSaid}`);
        expect("and the focus a press told disowned took is let go as it's raised",
            raisedTookFocus && raisedLetGo,
            `${raisedTookFocus} ${JSON.stringify(Sessions.goingTo)}`);
        Sessions.goingTo = goingBeforeRaised;
        Sessions.forgetAsk(ownRaised);
        Niri.windows = listedBeforeAfter;
        // And as a row's Reopen raises it, waiting for its host
        // (reopenWaiting), as Go To does.
        const ownWaiting = "pi-hosted.s-own-waiting";
        Sessions.viewWith(view.slice(0, -1).concat(["s-own-waiting"]), ownWaiting, null,
            "Reef", { address: "OWNWAITING" });
        const misreadWaiting = { id: 113, pid: 880, opened: ownWaiting };
        Niri.windows = Object.assign({}, listedBeforeAfter, { "113": misreadWaiting });
        Sessions.windowHeard(misreadWaiting);
        kittyPidFile.setText("880\n");
        root.unhosted = "";
        root.unhostedWhy = "";
        Niri.acted = null;
        Sessions.reopenWaiting({ id: "s-own-waiting" });
        const ownWaitingSaid = `${root.unhosted} ${root.unhostedWhy}`;
        const waitingFocused = focused(113);
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        expect("and so as a row's Reopen raises it, waiting for its host",
            waitingFocused && ownWaitingSaid === "OWNWAITING ownAfterAll",
            `${waitingFocused} ${ownWaitingSaid}`);
        Sessions.forgetAsk(ownWaiting);
        Niri.windows = listedBeforeAfter;
        // A press whose ask came refused long ago, its window seen as the
        // kitty's only now, isn't told: the script long done with that ask,
        // its words would meet Joel's next open of the launcher, whatever
        // he opens it for.
        const ownStale = "pi-hosted.s-own-stale";
        Sessions.viewWith(view.slice(0, -1).concat(["s-own-stale"]), ownStale, null, "Reef",
            { address: "OWNSTALE" });
        const misreadStale = { id: 112, pid: 880, opened: ownStale };
        Niri.windows = Object.assign({}, listedBeforeAfter, { "112": misreadStale });
        Sessions.windowHeard(misreadStale);
        Sessions.refusedAt = Object.assign({}, Sessions.refusedAt,
            { [ownStale]: Date.now() - Sessions.answerPatience - 1 });
        kittyPidFile.setText("880\n");
        Sessions.readKittyPid();
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.windowHeard(misreadStale);
        const ownStaleSaid = `${root.unhosted} ${root.unhostedWhy}`;
        kittyPidFile.setText("4242\n");
        Sessions.readKittyPid();
        expect("a press refused past the script's answer isn't told its window was its own",
            ownStaleSaid === " ", ownStaleSaid);
        Sessions.forgetAsk(ownStale);
        Niri.windows = listedBeforeAfter;
        // A stray come long after an ask that never came isn't taken for
        // that ask's own refused, so Go To would open its own beside it.
        const stale = "pi-hosted.s-stale";
        Sessions.attaching = Object.assign({}, Sessions.attaching,
            { [stale]: Date.now() - Sessions.viewPatience - 1 });
        const lateStray = { id: 94, pid: 880, opened: stale };
        Niri.windows = Object.assign({}, unforged, { "94": lateStray });
        Sessions.windowHeard(lateStray);
        expect("a stray come after its ask's patience is no ask's own refused",
            !(stale in Sessions.refusedAsks), JSON.stringify(Sessions.refusedAsks));
        Niri.windows = Object.assign({}, unforged, { "88": forged, "93": cameRefused });
        const unstale = Object.assign({}, Sessions.attaching);
        delete unstale[stale];
        Sessions.attaching = unstale;
        // A press held back over the shell's own ask takes it over, so a
        // give-up is said to it; one the shell made by itself tells no one.
        const heldWords = view.slice(0, -1).concat(["s-held"]);
        Sessions.viewWith(heldWords, "pi-hosted.s-held", null, "held");
        Sessions.goTo({ address: "HELD", presence: { hosted: true, session: "s-held" },
            attach: heldWords, here: true });
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.untaken("pi-hosted.s-held", String(Date.now()));
        expect("a press held back is told when none took the window it waited on",
            root.unhosted === "HELD" && root.unhostedWhy === "untaken",
            `${root.unhosted} ${root.unhostedWhy}`);
        const byItself = { address: "BYITSELF", presence: { hosted: true, session: "s-own" },
            attach: view.slice(0, -1).concat(["s-own"]), here: true };
        const askedByItself = Sessions.view(byItself, null);
        root.unhosted = "";
        Sessions.untaken("pi-hosted.s-own", String(Date.now()));
        expect("while one the shell asked for by itself is told to no one",
            askedByItself && root.unhosted === "" && !Sessions.viewerComing("pi-hosted.s-own"),
            `${askedByItself} ${root.unhosted}`);
        // A stray niri listed before its viewer was asked for, first heard
        // as it changes while that ask stands, is no ask's own refused.
        const before = "pi-hosted.s-before";
        const listedStray = { id: 95, pid: 880, opened: before };
        Niri.windows = Object.assign({}, unforged,
            { "88": forged, "93": cameRefused, "95": listedStray });
        const askedBefore = Sessions.viewWith(view.slice(0, -1).concat(["s-before"]), before,
            null, "before");
        Sessions.windowHeard(listedStray);
        expect("a stray listed before its ask is no ask's own refused as it changes",
            askedBefore && !(before in Sessions.refusedAsks),
            `${askedBefore} ${JSON.stringify(Sessions.refusedAsks)}`);
        Sessions.forgetAsk(before);
        // Nor does Reopen Them All's walk take a stray for the viewer it
        // waits on, whose kept place would go to it.
        const walked = { id: 96, pid: 880, opened: "pi-hosted.s-walked" };
        Niri.windows = Object.assign({}, unforged, { "88": forged, "93": cameRefused,
            "96": walked });
        const waitedOn = { id: "s-walked", classed: walked.opened, opened: null };
        Sessions.reopening = { waiting: waitedOn, late: [] };
        Sessions.reopenedWindow(96);
        Sessions.reopening = null;
        expect("nor is one the walk waits on", waitedOn.opened === null,
            JSON.stringify(waitedOn));
        // Its viewer come refused, or taken by no kitty, the walk waits on
        // it no more, as none can come for that ask, rather than for its
        // patience, viewer after viewer; it goes on at its pace, keeping it
        // neither late nor begun, so the walk can end and a press asks for
        // it again once its window closes, and the refused window is no
        // late one's either.
        const refusedWalk = { id: 97, pid: 880, session: true,
            opened: "pi-hosted.s-refused-walk" };
        const waitedRefused = { id: "s-refused-walk", classed: refusedWalk.opened, opened: null };
        const walkNext = { one: { id: "s-walk-next", cwd: "/tmp", hosted: true }, at: null };
        Sessions.reopening = { waiting: waitedRefused, late: [], queue: [walkNext], placed: [],
            behind: [], begun: ["s-refused-walk"], looking: null, settling: false, back: null,
            focused: false };
        Sessions.reopenStartedAt = Date.now();
        Sessions.viewWith(view.slice(0, -1).concat(["s-refused-walk"]), refusedWalk.opened,
            null, "walk");
        Niri.windows = Object.assign({}, unforged, { "88": forged, "93": cameRefused,
            "97": refusedWalk });
        Sessions.windowHeard(refusedWalk);
        const goneOnRefused = Sessions.reopening?.waiting === null
            && Sessions.reopening?.late.includes(waitedRefused) === false
            && waitedRefused.opened === null
            && Sessions.reopening?.begun.includes("s-refused-walk") === false;
        const paced = Sessions.reopening?.queue.length === 1;
        const untakenWalk = "pi-hosted.s-untaken-walk";
        const waitedUntaken = { id: "s-untaken-walk", classed: untakenWalk, opened: null };
        Sessions.reopenStartedAt = 0;
        if (Sessions.reopening) {
            Sessions.reopening.waiting = waitedUntaken;
            Sessions.reopening.queue = [walkNext];
            Sessions.reopening.begun = ["s-refused-walk", "s-untaken-walk"];
        }
        Sessions.viewWith(view.slice(0, -1).concat(["s-untaken-walk"]), untakenWalk, null,
            "walk");
        Sessions.untaken(untakenWalk, String(Date.now()));
        // None can come once no kitty took it, so it's kept neither late
        // nor begun, and a Reopen Them All pressed again asks for it.
        const goneOnUntaken = Sessions.reopening?.waiting === null
            && Sessions.reopening?.late.includes(waitedUntaken) === false
            && Sessions.reopening?.begun.includes("s-untaken-walk") === false;
        // Begun at once, the pace before it moot, no pace stands to hold
        // the walk's settling (reopenPaced).
        const wentOn = Sessions.reopening?.queue.length === 0 && Sessions.reopenPaced === false;
        Sessions.reopening = null;
        Sessions.forgetAsk(refusedWalk.opened);
        // The script waits a minute for the kitty's answer, past the walk's
        // 30 s patience: one given up on already is late no more once no
        // kitty took it, and the walk, with nothing else left, ends.
        const lateWalk = "pi-hosted.s-late-walk";
        const givenUp = { id: "s-late-walk", classed: lateWalk, opened: null };
        Sessions.reopening = { waiting: null, late: [givenUp], queue: [], placed: [],
            behind: [], begun: ["s-late-walk"], looking: null, settling: false, back: null,
            focused: false };
        Sessions.viewWith(view.slice(0, -1).concat(["s-late-walk"]), lateWalk, null, "walk");
        Sessions.untaken(lateWalk, String(Date.now()));
        const lateLeft = JSON.stringify(Sessions.reopening);
        const endedOnLate = Sessions.reopening === null;
        Sessions.reopening = null;
        Sessions.forgetAsk(lateWalk);
        // A window come refused once the walk's patience gave up on it, as a
        // kitty slow to draw at a cold boot brings it, ends it as one come
        // in time does: it's late no more, and the walk ends.
        const lateRefused = "pi-hosted.s-late-refused";
        const refusedLate = { id: "s-late-refused", classed: lateRefused, opened: null };
        Sessions.reopening = { waiting: null, late: [refusedLate], queue: [], placed: [],
            behind: [], begun: ["s-late-refused"], looking: null, settling: false, back: null,
            focused: false };
        Sessions.viewWith(view.slice(0, -1).concat(["s-late-refused"]), lateRefused, null,
            "walk");
        Sessions.attaching = Object.assign({}, Sessions.attaching,
            { [lateRefused]: Date.now() - Sessions.viewPatience - 1 });
        const listedBeforeSlow = Niri.windows;
        const slowRefused = { id: 98, pid: 880, opened: lateRefused };
        Niri.windows = Object.assign({}, listedBeforeSlow, { "98": slowRefused });
        Sessions.heardWindow(slowRefused);
        const lateRefusedLeft = JSON.stringify(Sessions.reopening);
        const endedOnLateRefused = Sessions.reopening === null;
        // Taken as its ask's own, it holds off a second ask while listed.
        const lateRefusedHolds = Sessions.refusedOwn(lateRefused)
            && !Sessions.viewerComing(lateRefused);
        Sessions.reopening = null;
        Sessions.forgetAsk(lateRefused);
        Niri.windows = listedBeforeSlow;
        // But only while the script could still answer for its ask: a
        // window of that name come minutes after is a stray, which leaves
        // the walk holding it late and refuses nothing.
        const strayLate = "pi-hosted.s-stray-late";
        const heldLate = { id: "s-stray-late", classed: strayLate, opened: null };
        Sessions.reopening = { waiting: null, late: [heldLate], queue: [], placed: [],
            behind: [], begun: ["s-stray-late"], looking: null, settling: false, back: null,
            focused: false };
        Sessions.viewWith(view.slice(0, -1).concat(["s-stray-late"]), strayLate, null, "walk");
        Sessions.attaching = Object.assign({}, Sessions.attaching,
            { [strayLate]: Date.now() - 300000 });
        const strayAfter = { id: 100, pid: 880, opened: strayLate };
        Niri.windows = Object.assign({}, listedBeforeSlow, { "100": strayAfter });
        Sessions.heardWindow(strayAfter);
        const strayLeft = JSON.stringify(Sessions.reopening);
        const heldOnStray = Sessions.reopening?.late.includes(heldLate) === true
            && !(strayLate in Sessions.refusedAsks);
        Sessions.reopening = null;
        Sessions.forgetAsk(strayLate);
        Niri.windows = listedBeforeSlow;
        // Nor does the moment between its ask's patience and the walk's,
        // both 30 s, keep one still waited on from being taken as its own.
        const waitedLate = "pi-hosted.s-waited-late";
        const stillWaited = { id: "s-waited-late", classed: waitedLate, opened: null };
        Sessions.reopening = { waiting: stillWaited, late: [], queue: [], placed: [],
            behind: [], begun: ["s-waited-late"], looking: null, settling: false, back: null,
            focused: false };
        Sessions.viewWith(view.slice(0, -1).concat(["s-waited-late"]), waitedLate, null, "walk");
        Sessions.attaching = Object.assign({}, Sessions.attaching,
            { [waitedLate]: Date.now() - Sessions.viewPatience - 1 });
        const waitedRefusedLate = { id: 101, pid: 880, opened: waitedLate };
        Niri.windows = Object.assign({}, listedBeforeSlow, { "101": waitedRefusedLate });
        Sessions.heardWindow(waitedRefusedLate);
        const waitedLeft = JSON.stringify(Sessions.reopening);
        const goneOnWaited = Sessions.reopening === null
            || Sessions.reopening.waiting !== stillWaited;
        Sessions.reopening = null;
        Sessions.forgetAsk(waitedLate);
        Niri.windows = listedBeforeSlow;
        // Beside another still late, or behind the lock, where the walk is
        // settled once the lock lets go (placeBehind), it goes on without
        // it: neither late nor begun, so a press asks for it again. The
        // lock is held (locked) or let go a moment ago, niri not yet given
        // the keys back (lettingGo); either way the lock's end then settles
        // the walk, which ends, as reopenSettling's first round finds
        // nothing to place.
        const lateGoesOn = (named, others, locked, lettingGo) => {
            const given = { id: named.slice("pi-hosted.".length), classed: named, opened: null };
            // One held behind the lock whose window is gone, which only the
            // lock's end lets go (placeBehind), so the walk ending shows it
            // ran.
            const behind = locked || lettingGo ? [{ entry: { id: "s-behind-gone", at: null } }]
                : [];
            Sessions.reopening = { waiting: null, late: [given].concat(others), queue: [],
                placed: [], behind, begun: [given.id].concat(others.map(one => one.id)),
                looking: null, settling: false, back: null, focused: false };
            Sessions.behindTheLock = locked;
            if (lettingGo)
                Sessions.lockLetGo = false;
            Sessions.viewWith(view.slice(0, -1).concat([given.id]), named, null, "walk");
            Sessions.untaken(named, String(Date.now()));
            const going = Sessions.reopening;
            const left = JSON.stringify(going);
            const on = going !== null && going.late.length === others.length
                && !going.begun.includes(given.id)
                && others.every(one => going.late.includes(one) && going.begun.includes(one.id));
            let ends = null;
            let parts = "";
            if (locked || lettingGo) {
                Sessions.behindTheLock = false;
                // No pace stands: the walk's, begun by the cases above, ended
                // as its next was begun (reopenSoon, which the walk's pace
                // case proves), so the lock's end settles the walk itself
                // (placeBehind). Each part is said, so a failure says which.
                const paced = Sessions.reopenPaced === true;
                Sessions.lockGone();
                const letGoBehind = Sessions.reopening?.behind.length === 0;
                const settling = Sessions.reopening?.settling === true;
                Sessions.settleNextReopened();
                const ended = Sessions.reopening === null;
                ends = !paced && letGoBehind && settling && ended;
                parts = JSON.stringify({ paced, letGoBehind, settling, ended });
            }
            // Let go with no walk left for it, and at once, as the cases
            // after it place what they open.
            Sessions.reopening = null;
            Sessions.behindTheLock = false;
            Sessions.lockLetGo = true;
            Sessions.forgetAsk(named);
            return { on, left, ends, parts };
        };
        const stillLate = { id: "s-still-late", classed: "pi-hosted.s-still-late", opened: null };
        const besideLate = lateGoesOn("pi-hosted.s-late-beside", [stillLate], false);
        const lockedLate = lateGoesOn("pi-hosted.s-late-locked", [], true);
        const lettingGoLate = lateGoesOn("pi-hosted.s-late-letting-go", [], false, true);
        expect("the walk waits no more on one whose window came refused, nor keeps it",
            goneOnRefused, JSON.stringify(Sessions.reopening ?? waitedRefused));
        expect("nor on one no kitty took, which a press asks for again", goneOnUntaken,
            JSON.stringify(waitedUntaken));
        expect("the next waits out the walk's pace", paced, "begun at once");
        expect("and once it has passed, is begun, no pace left standing", wentOn,
            "left queued or paced");
        expect("nor keeps one its patience gave up on, so the walk ends", endedOnLate,
            lateLeft);
        expect("nor one whose window came refused once its patience gave up, so it ends",
            endedOnLateRefused, lateRefusedLeft);
        expect("and holds off a second ask while that window is listed", lateRefusedHolds,
            JSON.stringify(Sessions.refusedAsks));
        expect("but a window of its name come minutes after is a stray, refusing nothing",
            heldOnStray, strayLeft);
        expect("nor waits on one whose window came refused as its patience ends", goneOnWaited,
            waitedLeft);
        expect("nor beside another still late, the walk going on", besideLate.on,
            besideLate.left);
        expect("nor behind the lock, the walk left for the lock's end to settle",
            lockedLate.on, lockedLate.left);
        expect("and the lock's end settles it, the walk ending", lockedLate.ends === true,
            lockedLate.parts);
        expect("nor as the lock lets go, niri not yet given the keys, the lock's end settling it",
            lettingGoLate.on && lettingGoLate.ends === true,
            `${lettingGoLate.left} ${lettingGoLate.parts}`);
        Niri.windows = Object.assign({}, unforged, { "88": forged, "93": cameRefused });
        // A start's window in the sessions kitty is held to its pid too,
        // as any program can name its window as a start's.
        const startedNamed = "pi-session.3";
        const lone = { id: 87, pid: 880, opened: startedNamed };
        const inKitty = { socket: Sessions.sessionsSocket, window: 1 };
        const started = Sessions.windowOf({ opened: startedNamed, kitty: inKitty }, {},
            { [startedNamed]: [lone] });
        expect("nor is one named as a start's in the sessions kitty its window",
            started === null, JSON.stringify(started));
        // With niri listing nothing, the windows daemon's index is read,
        // held to the same pid.
        const indexed = { index: Sessions.niriWindows, kept: Sessions.windowsKept };
        Niri.windows = {};
        Sessions.niriWindows = { pids: { "880": [forged] }, opened: { [named]: [forged] } };
        Sessions.windowsKept = { "window:88": forged };
        const keptWindow = Sessions.windowOf({ presence: hosting }, {}, undefined);
        expect("nor is it a viewer listed in the index, nor its session's window there",
            Sessions.listedNamed(named) === undefined && keptWindow === null,
            `${JSON.stringify(Sessions.listedNamed(named))} ${JSON.stringify(keptWindow)}`);
        Sessions.niriWindows = indexed.index;
        Sessions.windowsKept = indexed.kept;
        // Its kitty started again under a new pid, which the file's watch
        // can miss, its folder made anew: the file is read again before a
        // window with the viewer's app id is refused.
        kittyPidFile.setText("5151\n");
        Sessions.attaching = Object.assign({}, Sessions.attaching, { [named]: Date.now() });
        const anew = { id: 90, pid: 5151, opened: named };
        // A look from a binding (running and the rest) reads no file, so
        // changes nothing those bindings read while they're worked out.
        expect("a look reads no file, so changes no pid its bindings read",
            !Sessions.drawnHere(anew) && Sessions.sessionsKittyPid === "4242",
            Sessions.sessionsKittyPid);
        Niri.windows = Object.assign({}, unforged, { "90": anew });
        Sessions.windowHeard(anew);
        expect("one drawn by the kitty started again is taken as come",
            !Sessions.viewerComing(named), JSON.stringify(Sessions.attaching));
        // Go To reads it again first, its viewer drawn by a kitty since
        // started again, though niri told the shell nothing of it.
        kittyPidFile.setText("6161\n");
        const since = { id: 92, pid: 6161, opened: named };
        Niri.windows = Object.assign({}, unforged, { "92": since });
        Niri.acted = null;
        Sessions.goTo({ address: "FORGED", presence: hosting, attach: view, here: true });
        expect("Go To raises a viewer drawn by a kitty started again since",
            focused(92), JSON.stringify(Niri.acted));
        // And so does Reopen Them All, before its walk judges any viewer.
        kittyPidFile.setText("7171\n");
        Sessions.reopenAll([], () => {}, () => null);
        expect("Reopen Them All reads it before its walk",
            Sessions.sessionsKittyPid === "7171", Sessions.sessionsKittyPid);
        // While its unit starts again, the file is gone, which reads as one
        // saying nothing (Quickshell's FileViewReader::read gives a missing
        // file no text): the pid read last still says which windows are
        // its, rather than any: the window is refused, not taken as come
        // (which would forget the ask, refusal and all).
        kittyPidFile.setText("");
        Sessions.readKittyPid();
        Sessions.attaching = Object.assign({}, Sessions.attaching, { [named]: Date.now() });
        const between = { id: 91, pid: 880, opened: named };
        Niri.windows = Object.assign({}, unforged, { "91": between });
        Sessions.windowHeard(between);
        Niri.windows = unforged;
        expect("while it says none, another program's is still no viewer",
            Sessions.refusedAsks[named] === "91" && Sessions.sessionsKittyPid === "7171",
            `${Sessions.sessionsKittyPid} ${JSON.stringify(Sessions.refusedAsks)}`);
        // A window refused once isn't read for again as it changes, as a
        // stray one retitling would read the file with every title.
        kittyPidFile.setText("880\n");
        Niri.windows = Object.assign({}, unforged, { "91": between });
        Sessions.windowHeard(between);
        Niri.windows = unforged;
        expect("a window refused once reads no file as it changes",
            Sessions.sessionsKittyPid === "7171", Sessions.sessionsKittyPid);
        // A start pi's host took reads the pid first, as Go To does, so a
        // viewer drawn by a kitty started again since is raised, not
        // another opened beside it.
        const startedViewer = "pi-hosted.s-started";
        const footedStart = { address: "q-started", machine: "", name: "Reef" };
        kittyPidFile.setText("8282\n");
        const startedSince = { id: 86, pid: 8282, opened: startedViewer };
        Niri.windows = Object.assign({}, unforged, { "86": startedSince });
        Niri.acted = null;
        const startedFrom = Niri.spawns.length;
        Sessions.viewStarted("harbour-attended", "s-started", startedViewer, "Reef", "/tmp",
            footedStart);
        expect("a start raises its viewer drawn by a kitty started again since",
            focused(86) && Niri.spawns.length === startedFrom,
            `${JSON.stringify(Niri.acted)} ${Niri.spawns.slice(startedFrom).join(" | ")}`);
        // One whose viewer came refused, that window still listed, is said
        // disowned, opening none, as a press of it is, but led by its start,
        // so the foot never reads as the start not taking.
        const disownedStart = "pi-hosted.s-disowned-start";
        const refusedStart = { id: 84, pid: 880, opened: disownedStart };
        Niri.windows = Object.assign({}, unforged, { "84": refusedStart });
        Sessions.refusedAsks = Object.assign({}, Sessions.refusedAsks, { [disownedStart]: "84" });
        Sessions.refusedAt = Object.assign({}, Sessions.refusedAt,
            { [disownedStart]: Date.now() - Sessions.answerPatience - 1 });
        root.unhosted = "";
        root.unhostedWhy = "";
        // Its ask long past what the script could answer for, it takes no
        // focus, as nothing of that ask's can come now.
        const goingBeforeDisowned = Sessions.goingTo;
        Sessions.viewStarted("harbour-attended", "s-disowned-start", disownedStart, "Reef",
            "/tmp", footedStart);
        expect("a start whose viewer came refused is said disowned, as started, opening none",
            root.unhosted === "q-started" && root.unhostedWhy === "startedDisowned"
            && Niri.spawns.length === startedFrom && Sessions.goingTo === goingBeforeDisowned,
            `${root.unhosted} ${root.unhostedWhy} ${JSON.stringify(Sessions.goingTo)} `
                + Niri.spawns.slice(startedFrom).join(" | "));
        Sessions.forgetAsk(disownedStart);
        Sessions.goingTo = goingBeforeDisowned;
        // A start again while the script may still answer for the ask whose
        // window came refused takes the focus its own would, and its own
        // come after is focused and said viewed, led by its start.
        const afterStart = "pi-hosted.s-start-after";
        Niri.windows = unforged;
        Sessions.viewStarted("harbour-attended", "s-start-after", afterStart, "Reef", "/tmp",
            footedStart);
        // A start again while its viewer comes is said opening, led by the
        // start too.
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.viewStarted("harbour-attended", "s-start-after", afterStart, "Reef", "/tmp",
            footedStart);
        const heldStart = `${root.unhosted} ${root.unhostedWhy}`;
        const strayStart = { id: 107, pid: 880, opened: afterStart };
        Niri.windows = Object.assign({}, unforged, { "107": strayStart });
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.windowHeard(strayStart);
        const saidStray = `${root.unhosted} ${root.unhostedWhy}`;
        Sessions.goingTo = null;
        Sessions.viewStarted("harbour-attended", "s-start-after", afterStart, "Reef", "/tmp",
            footedStart);
        const startTook = Sessions.goingTo?.app === afterStart;
        const ownStart = { id: 108, pid: Number(Sessions.sessionsKittyPid), opened: afterStart };
        Niri.windows = Object.assign({}, unforged, { "107": strayStart, "108": ownStart });
        Niri.acted = null;
        root.unhostedWhy = "";
        Sessions.windowHeard(ownStart);
        expect("a start again while its viewer comes is said opening, as started",
            heldStart === "q-started startedOpening", heldStart);
        expect("a start again after a stray takes the focus, its own come said viewed, as started",
            saidStray === "q-started startedDisowned" && startTook && focused(108)
            && root.unhostedWhy === "startedCameAfter",
            `${saidStray} ${startTook} ${JSON.stringify(Niri.acted)} ${root.unhostedWhy}`);
        Sessions.forgetAsk(afterStart);
        Sessions.goingTo = goingBeforeDisowned;
        // And one whose viewer no kitty took is said so, as started too.
        const untakenStart = "pi-hosted.s-untaken-start";
        Niri.windows = unforged;
        const untakenFrom = Niri.spawns.length;
        Sessions.viewStarted("harbour-attended", "s-untaken-start", untakenStart, "Reef",
            "/tmp", footedStart);
        root.unhosted = "";
        root.unhostedWhy = "";
        Sessions.untaken(untakenStart, String(Date.now()));
        expect("a start whose viewer no kitty took is said untaken, as started",
            Niri.spawns.length === untakenFrom + 1 && root.unhosted === "q-started"
            && root.unhostedWhy === "startedUntaken",
            `${root.unhosted} ${root.unhostedWhy} ${Niri.spawns.slice(untakenFrom).join(" | ")}`);
        Sessions.forgetAsk(untakenStart);
        Niri.windows = unforged;
        kittyPidFile.setText("");
        Sessions.kittyPidRead = "";
        if (failed.length > 0)
            throw new Error(failed.join("; "));
    }

    property string unjoined: ""
    property string unhosted: ""
    property string unhostedWhy: ""
    // The last that Go To couldn't open, in a terminal it can't reach.
    property string unreached: ""
    // Whom Sessions said it was waking as Go To's refusal came.
    property string wakingSaid: ""
    property bool steered: false
    // What the node said of waking the one parked elsewhere.
    property string strayed: ""
    // The act its refusal named.
    property string strayedAct: ""
    property string unfiledAct: ""
    property string unfiled: ""
    // How each Wake left asleep was said, by its address: "unfenced" for
    // one it might bring back unfenced, else the node's refusal as said.
    property var asleep: ({})
    // Each steer the node was asked, and whether the one parked had been
    // woken by then.
    property var steers: []

    // A steer is asked of the node, whose refusal is said.
    Connections {
        target: Sessions

        function onUnjoined(session) {
            root.unjoined = session.address;
        }

        function onUnhosted(session, why) {
            root.unhosted = session.address;
            root.unhostedWhy = why ?? "";
        }

        function onUnopened(session) {
            root.unreached = session.address;
            root.wakingSaid = Sessions.waking;
        }

        function onUnfenced(session) {
            root.expect(`${session.address} is said unfenced once`,
                !(session.address in root.asleep), JSON.stringify(root.asleep));
            root.asleep = Object.assign({}, root.asleep, { [session.address]: "unfenced" });
        }

        function onUnsteered(session, why, act) {
            try {
                // One Wake mustn't reopen, or can't, is said as the node
                // refused it (reopensUnwoken); one it reopens never is.
                if (session.address === "STRAY") {
                    root.strayed = why;
                    root.strayedAct = act;
                    return;
                }
                // One with no session file to reopen is said so too.
                if (session.address === "NOFILE") {
                    root.unfiled = why;
                    root.unfiledAct = act;
                    return;
                }
                if (session.address === "FENCED" || session.address === "UNSAID") {
                    root.asleep = Object.assign({}, root.asleep, { [session.address]: why });
                    return;
                }
                // One Wake reopens, said as refused, is said as itself.
                if (["ORPHAN", "WARDED", "HEDGED"].includes(session.address)) {
                    root.expect(`${session.address} reopens, never said as refused`, false, why);
                    return;
                }
                root.expect("a steer elsewhere is refused by the node, saying why",
                    session.address === "FARAWY" && session.machine === "lighthouse"
                    && why === "lighthouse is out of reach, so FARAWY can't be told now", why);
                root.expect("and names the act it refused, never a wake", act === "pause", act);
                // Its caller is told after the refusal is said.
                Qt.callLater(() => {
                    try {
                        root.expect("its caller told not put, the refusal, and that it was sent",
                            JSON.stringify(root.farawyTold) === JSON.stringify([false,
                                "lighthouse is out of reach, so FARAWY can't be told now", true]),
                            JSON.stringify(root.farawyTold));
                    } catch (error) {
                        root.fail(error.message);
                    }
                });
                const asked = root.steers.find(one => one.ask.address === "FARAWY")?.ask;
                root.expect("the node is asked with the steer, its address as the mesh writes it",
                    asked?.steer?.act === "pause" && asked?.steer?.from === "joel",
                    JSON.stringify(asked));
                root.steered = true;
                Qt.callLater(root.check);
            } catch (error) {
                root.fail(error.message);
            }
        }
    }

    // The node's end of the shell's watch, so the node can tell it more,
    // and be lost.
    property var answering: null

    // Tells the shell's watch a change, as the node does.
    function tell(change) {
        answering.write(`${JSON.stringify(Object.assign({ id: root.watchId }, change))}\n`);
        answering.flush();
    }
    property var watchId: null
    // The fields each watch of sessions was asked to be sent for, or null
    // for every change, and whether one was asked again under its id.
    property var whens: []
    property bool rewatched: false

    // A stand-in for the machine's node, listening only once asked, which
    // speaks the daemon interface as mycelium's does (its PROTOCOL.md, The
    // Daemon Interface): hello, a watch of sessions answered with each
    // whole, then each change saying only what differs, and a steer, which
    // it refuses for the one elsewhere, as for a machine out of reach.
    SocketServer {
        id: node
        active: false
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/harbour.sock`
        handler: Socket {
            id: asked
            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "node", state: "running" };
                    } else if (ask.ask === "watch" && ask.kind === "session"
                        && ask.keys === undefined) {
                        root.whens = root.whens.concat([ask.when ?? null]);
                        if (root.answering === asked && ask.id === root.watchId) {
                            // Asked again under its id, as the launcher
                            // showing or shutting asks: the watch keeps on,
                            // and sends what it held back, here nothing.
                            root.rewatched = true;
                            Qt.callLater(root.check);
                            return;
                        }
                        // The connection watching sessions, which pi's
                        // source makes beside its watches of runs and talks.
                        root.answering = asked;
                        root.watchId = ask.id;
                        answer = { id: ask.id, records: root.sessions() };
                    } else if (ask.ask === "watch" && (ask.kind === "run" || ask.kind === "talk")) {
                        answer = { id: ask.id, records: [] };
                    } else if (ask.ask === "steer") {
                        root.steers = root.steers.concat([{ ask: ask }]);
                        // A wake of one no holder keeps this boot is
                        // refused as the node refuses it.
                        const unheld = ask.steer?.act === "wake"
                            && ["ORPHAN", "STRAY", "NOFILE", "FENCED", "WARDED", "HEDGED",
                                "UNSAID", "ALIVE"].includes(ask.address);
                        answer = ask.address === "FARAWY" ? { id: ask.id, error: {
                            code: "refused",
                            message: "lighthouse is out of reach, so FARAWY can't be told now"
                        } } : unheld ? { id: ask.id, error: { code: "refused",
                            message: `${ask.address} isn't parked with a holder to wake it`
                                + " this boot" } }
                        : { id: ask.id, steered: { machine: Sessions.machine } };
                        Qt.callLater(root.check);
                    } else {
                        root.fail(`an ask the shell has no need of: ${line}`);
                        answer = { id: ask.id, error: { code: "unknown-ask", message: line } };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    // What the shell asked of the agents daemon, past its hello and watch,
    // each ask whole.
    property var agentsAsked: []
    // What the shell told the daemon of the places kept, apart from what
    // Joel says of the sessions to reopen.
    property var placesAsked: []

    // What the agents daemon serves of the sessions to reopen, as its
    // reopen.rs works them out of what the harness's restore.json kept
    // (modules/agents, whose tests say which are offered and why): the
    // last boot's first, then those offered apart, the newest first, and
    // the boot they're of. Answered out of order, as a watch's answer may
    // be, so each is placed by its own.
    function reopenRecords() {
        const session = (id, address, title, fields) => Object.assign(
            { id, address, title, cwd: "/tmp", lostAt: 1 }, fields);
        const far = ["ssh", "-t", "lighthouse", "mycelium", "attach", "s-far"];
        const offered = [
            ["boot", session("s-before", "BEFORE", "Harbour",
                { env: { PI_SANDBOX: "fence; rm -rf ~", LD_PRELOAD: "/tmp/x.so" } })],
            ["boot", session("s-fresh", "FRESH7", "Fresh", { fresh: true, quest: "QEST-FRESH",
                env: { PI_SANDBOX: "fence", PI_SPAWNER: "on" } })],
            ["boot", session("s-given-back", "GIVENB", "Given Back", { cwd: "/given-back" })],
            ["boot", session("s-hosted", "HOSTED", "Hosted", { hosted: true })],
            ["boot", session("s-far", "FARLST", "Far",
                { lostAt: "", machine: "lighthouse", attach: far })],
            ["carried", session("s-ferry", "FERRYQ", "Ferry", { lostAt: "",
                carried: { from: "lighthouse", at: "2026-10-30T12:00:00Z" } })],
            ["released", session("s-freed", "FREEDQ", "Freed", { lostAt: "",
                released: { from: "attended", at: "2026-10-30T11:00:00Z" } })],
            ["carried", session("s-ferried", "FERRID", "Ferried", { lostAt: "",
                carried: { from: "lighthouse", at: "2026-10-30T10:00:00Z" } })]
        ];
        return offered.map(([way, one], at) => ({ key: `reopen:agents:${one.id}`,
            kind: "reopen", order: (at + 1) * 1024, way, session: one })).reverse()
            .concat([{ key: "restoring:agents", kind: "restoring",
                restoring: { boot: "an-earlier-boot" } },
            { key: "arrangement:agents", kind: "arrangement", arrangement: { v: 1,
                boots: { "an-earlier-boot": { "s-elsewhere": { output: "DP-4", workspace: 2,
                    column: 1, tile: 1 } } },
                others: { "an-earlier-boot": [{ app: "slack", title: "Slack" }] } } },
            { key: "kept-places:agents", kind: "kept-places", places: {
                "s-moving": { workspace: 6, column: 1, at: Date.now() } } },
            // What two running sessions ask Joel, as the daemon judges it:
            // one stopped on his keys since now, which rings, and one whose
            // run is stuck, which For You lists but never rings for.
            { key: "want:agents:LIVE01", kind: "want", address: "LIVE01",
                since: new Date().toISOString(), rings: true, known: "LIVE01 question-1" },
            { key: "want:agents:OTHER2", kind: "want", address: "OTHER2",
                since: new Date().toISOString(), rings: false, known: "OTHER2 blocked" },
            // And one Joel put off, which rings for nothing until its hour's
            // over.
            { key: "want:agents:NOWIN6", kind: "want", address: "NOWIN6",
                since: new Date().toISOString(), rings: false, putOff: true,
                known: "NOWIN6 question@now" }]);
    }

    // A stand-in for the agents daemon (modules/agents), speaking the
    // daemon interface: hello, a watch of Agents' place's sessions to
    // reopen answered whole, and the asks Joel's word on them makes, each
    // answered and its change told as the daemon tells it: one forgotten
    // goes, and putting off the last boot takes its sessions.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/agents.sock`
        handler: Socket {
            id: agentsAsk
            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    const said = lines => {
                        for (const one of lines)
                            agentsAsk.write(`${JSON.stringify(one)}\n`);
                        agentsAsk.flush();
                    };
                    if (ask.ask === "hello") {
                        said([{ id: ask.id, version: 1, domain: "agents", state: "answering" }]);
                        return;
                    }
                    if (ask.ask === "watch") {
                        const watched = JSON.stringify([ask.kind, ask.place]);
                        if (watched
                            !== '[["reopen","restoring","to-reopen","arrangement","kept-places",'
                                + '"want"],'
                                + '"agents"]')
                            root.fail(`a watch the shell has no need of: ${line}`);
                        said([{ id: ask.id, records: root.reopenRecords() }]);
                        return;
                    }
                    if (ask.ask === "keep-places") {
                        root.placesAsked = root.placesAsked.concat([ask]);
                        said([{ id: ask.id, kept: Object.keys(ask.places ?? {}).length }]);
                        return;
                    }
                    root.agentsAsked = root.agentsAsked.concat([ask]);
                    const boot = root.reopenRecords().filter(one => one.way === "boot")
                        .map(one => one.key);
                    if (ask.ask === "forget")
                        said([{ id: ask.id, forgot: ask.session },
                            { id: "all", changed: [], gone: [`reopen:agents:${ask.session}`] }]);
                    else if (ask.ask === "put-off")
                        said([{ id: ask.id, "put-off": true },
                            { id: "all", changed: [], gone: boot }]);
                    else
                        root.fail(`an ask of the agents daemon the shell has no need of: ${line}`);
                    Qt.callLater(root.check);
                }
            }
        }
    }

    function terminal(scope, window) {
        return { driverId: "kitty", kind: "kitty-window", hostId: "here", value: window,
            scope: scope };
    }

    // The sessions the node holds, as records of the daemon interface.
    function sessions() {
        const now = new Date().toISOString();
        const record = (id, fields) => Object.assign({
            key: `session:${id}`, kind: "session", heard: Date.now(), id: id, at: now,
            groups: [], title: "", window: null, held: null
        }, fields);
        // One here, standing as the node says: running, or closed once
        // ended, unless it's said otherwise.
        const here = (address, state, presence, standing) => record(`s-${address}`, {
            address: address, machine: Sessions.machine, here: true, state: state,
            standing: standing ?? (state === "ended" ? "closed" : "running"),
            presence: Object.assign({ session: `s-${address}`, addresses: [address] }, presence)
        });
        // One on another machine, running unless the node passes it over.
        const far = (id, address, state, at, attach, standing, more) => record(id, {
            address: address, machine: "lighthouse", here: false, state: state, at: at,
            attach: attach, standing: standing,
            presence: Object.assign({ addresses: [address], title: `\u03c0 - ${address} - /far`,
                busy: true }, more)
        });
        return [
            here("LIVE01", "busy", { busy: true, title: "\u03c0 - Live One - core.nix",
                terminal: terminal("unix:@kitty-500", "3") }),
            // In the second of kitty 600's two windows, which the node
            // says was opened as pi-session.7.
            Object.assign(here("OTHER2", "idle", {
                title: "\u03c0 - Lighth\u2026 - wrong",
                questInfo: { id: "QEST-20261106-OTHER2", title: "Lighthouse - Keep It Lit",
                    short: "OTHER2", status: "active" },
                cwd: "/home/me/lamp/", terminal: terminal("unix:@kitty-600", "4")
            }), { window: { terminal: "kitty", id: "4", within: "unix:@kitty-600",
                app_id: "pi-session.7" } }),
            // In a kitty niri shows no window of, asking Joel what he put
            // off for an hour.
            here("NOWIN6", "idle", { terminal: terminal("unix:@kitty-700", "2"),
                wants: { kind: "question", said: "Shall I?", since: now } }),
            here("ENDED3", "ended", {}),
            // Parked, as the node tells of one: ended, saying it's parked.
            Object.assign(here("NAPPER", "ended", { state: "parked", busy: false,
                title: "\u03c0 - Napper - core.nix" }, "parked"),
            { window: { terminal: "kitty", id: "1", within: "unix:@kitty-800",
                app_id: "pi-spawned.9" } }),
            // Parked in a kitty whose window was opened with no app id.
            Object.assign(here("DOZER4", "ended", { state: "parked", busy: false,
                title: "\u03c0 - Dozer - core.nix" }, "parked"),
            { window: { terminal: "kitty", id: "5", within: "unix:@kitty-950" } }),
            // Closed, though each record says parked: its marker gone, or
            // laid before this boot, which the node sees and the shell
            // needn't.
            here("WOKEN8", "ended", { state: "parked" }),
            here("STALE7", "ended", { state: "parked",
                owner: { pid: 4242, bootId: "an-earlier-boot" } }),
            // Orchestrated from there, its orchestrator's workspace here
            // named for it.
            far("r-far", "FARAWY", "busy", now, "mycelium attach r-far", "running",
                { orchestratedBy: "FARLED" }),
            far("r-out", "OUTRCH", "unreachable", now, null, "running"),
            // Last heard of before this boot, so the node says nothing of
            // how it stands.
            far("r-old", "OLDONE", "busy", "2000-01-01T00:00:00Z", "mycelium attach r-old")
        ];
    }

    Timer {
        interval: 30000
        running: true
        onTriggered: root.fail(`still at ${root.stage}: ${root.addresses()}`)
    }
}

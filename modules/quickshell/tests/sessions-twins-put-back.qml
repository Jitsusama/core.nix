pragma ComponentBehavior: Bound
// Drives Reopen Them All's wiring between its sessions and the windows it
// puts back (modules/quickshell/Sessions.qml's reopenHere and putBack), with
// niri stood in (modules/quickshell/tests/stand-ins/hosted-reopen-niri.qml):
// two screens of one model without serials, kept on Virtual-1 and
// Virtual-2, are back on Virtual-3 and Virtual-4. A session kept on the
// second takes the first connector found, so a window kept on the first
// goes to the other, as the connectors the walk gave its sessions say. A
// window put back before the walk placed its sessions keeps the connector
// it was given, so a window kept on its twin goes to the other, unless a
// session took that connector, when the session's twin goes with it.
// Sessions handed over outside a walk give windows nothing, and a walk
// joined keeps the screen in use as it began. Twin sessions take a
// connector each, as the places each one's own boot kept say, and a session
// kept on a screen that's gone goes to the screen in use as its walk
// began. No walk starts anything. A window noted away from a
// screen named for a prototype's key, constructor or __proto__, is noted
// as any other, without a throw. Places.js and
// Reopening.js hold each answer to tests/places.qml; this holds Sessions to
// passing them on. It exits 0 once that holds, and otherwise 1, saying what
// didn't.
import qs
import Quickshell
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property var twin: ({ make: "Gull", model: "Wide", logical: {} })
    readonly property string twinSaid: "Gull Wide"

    // A place kept on the twin kept on `output`, for a window of `app`.
    function keptOn(output, app) {
        return { app, title: app, screen: root.twinSaid, output, workspace: 1, column: 1,
            tile: 1 };
    }

    // A window of `app`, as niri lists it, open beside Joel's on the screen
    // in use, where a window shows before it's put back.
    function opened(id, app) {
        const window = { id, pid: id * 10, app_id: app, title: app, workspace_id: 60,
            layout: { pos_in_scrolling_layout: [2, 1] } };
        Niri.windows = Object.assign({}, Niri.windows, { [id]: window });
        return window;
    }

    // Reopen Them All's windows of the boot being put back, with none back yet.
    function puttingBack(kept) {
        Sessions.puttingBack = { boot: "b0", kept, used: [], back: [], seen: [], given: [] };
    }

    // The workspaces a window was moved to, by niri's id.
    function movedTo(id) {
        return Niri.acts.filter(act => act.MoveWindowToWorkspace?.window_id === id)
            .map(act => act.MoveWindowToWorkspace.reference.Id);
    }

    // Answers the oldest look at the screens waiting with these outputs.
    function answerOutputs(outputs) {
        const waiting = Niri.asked.find(one => one.asking === "Outputs");
        Expect.expect("the shell looks at the screens", waiting !== undefined,
            JSON.stringify(Niri.asked));
        Niri.asked = Niri.asked.filter(one => one !== waiting);
        waiting.answer({ Outputs: outputs });
    }

    // Both twins back on new connectors, Joel on the first, each with an
    // empty workspace last, where a window given a workspace of its own goes.
    function docked() {
        Sessions.lookAtScreens(() => {});
        Niri.windows = { "90": { id: 90, pid: 900, workspace_id: 60, app_id: "kitty",
            layout: { pos_in_scrolling_layout: [1, 1] } } };
        Niri.current = 90;
        Niri.workspaces = {
            "60": { id: 60, idx: 1, output: "Virtual-3", is_focused: true, active_window_id: 90 },
            "61": { id: 61, idx: 2, output: "Virtual-3", is_focused: false,
                active_window_id: null },
            "70": { id: 70, idx: 1, output: "Virtual-4", is_focused: false,
                active_window_id: null }
        };
        Niri.revision += 1;
        root.answerOutputs({ "Virtual-3": root.twin, "Virtual-4": root.twin });
        Expect.same("both twins are connected now", Sessions.screensThere(),
            { "Virtual-3": root.twinSaid, "Virtual-4": root.twinSaid });
    }

    // The sessions each walk was asked to reopen and found gone, by id: each
    // is kept in a folder that's gone, with no quest to start afresh, so its
    // walk ends by the shell's own path, with nothing started.
    property var gone: []
    function goneFrom(one) {
        root.gone = root.gone.concat([one.id]);
    }
    // How many were found gone as the stage now running began, so each
    // stage reads only what the walk of the case before it found gone.
    property int goneAt: 0
    readonly property string goneFolder: "/nowhere/given-back"

    // The walk places a session kept on the second twin on the first
    // connector found, so a window kept on the first twin goes to the
    // other, and one kept on the session's own twin goes with it. It is the
    // first to catch a reopenHere that drops the walk's store,
    // `puttingBack.given = placing.given`: chat then takes the first
    // connector found too.
    function sessionsGive() {
        root.puttingBack([root.keptOn("Virtual-1", "chat"), root.keptOn("Virtual-2", "page")]);
        Sessions.reopenHere([{ id: "s-twin", cwd: root.goneFolder }], root.goneFrom,
            () => root.keptOn("Virtual-2", "pi"), Sessions.carryOn, () => [], true, null);
        Expect.same("the session kept on the second twin takes the first connector found",
            Sessions.reopening?.looking?.at?.output, "Virtual-3");
        Sessions.putBack(root.opened(91, "chat"));
        Expect.same("a window kept on the first twin goes to the one the session didn't take",
            root.movedTo(91), [70]);
        Sessions.putBack(root.opened(92, "page"));
        Expect.same("a window kept on the session's own twin goes to the one the session took",
            root.movedTo(92), [61]);
    }

    // A window put back before the walk placed its sessions takes the first
    // connector found, and keeps it through a walk of its own, so its twin's
    // window goes to the other. It alone catches a reopenHere that hands
    // placedGiven `[]` for `before` rather than `puttingBack?.given`: the
    // walk then gives the first connector found again, and notes goes
    // there too.
    function windowsBeforeKeep() {
        root.puttingBack([root.keptOn("Virtual-1", "mail"), root.keptOn("Virtual-2", "notes")]);
        Sessions.putBack(root.opened(93, "mail"));
        Expect.same("a window put back first takes the first connector found",
            root.movedTo(93), [61]);
        Sessions.reopenHere([], root.goneFrom, () => null, Sessions.carryOn, () => [], true,
            null);
        Sessions.putBack(root.opened(94, "notes"));
        Expect.same("its twin's window goes to the other, after the walk",
            root.movedTo(94), [70]);
    }

    // A session the walk places takes the connector an earlier window took,
    // so that window's connector is the session's now, and a window kept on
    // the session's own twin goes with the session. It alone catches a
    // reopenHere that adds the walk's connectors to `puttingBack.given`
    // rather than putting them in its place: jotter then goes to the other.
    // Dropping the store fails it too, but sessionsGive fails first.
    function sessionTakesOver() {
        root.puttingBack([root.keptOn("Virtual-1", "post"), root.keptOn("Virtual-2", "jotter")]);
        Sessions.putBack(root.opened(95, "post"));
        Expect.same("a window put back first takes the first connector found",
            root.movedTo(95), [61]);
        Sessions.reopenHere([{ id: "s-took", cwd: root.goneFolder }], root.goneFrom,
            () => root.keptOn("Virtual-2", "pi"), Sessions.carryOn, () => [], true, null);
        Expect.same("the session kept on the second twin takes that connector too",
            Sessions.reopening?.looking?.at?.output, "Virtual-3");
        Sessions.putBack(root.opened(96, "jotter"));
        Expect.same("a window kept on the session's twin goes with the session",
            root.movedTo(96), [61]);
    }

    // Sessions handed over while windows are being put back are no walk of
    // Reopen Them All's, so the connectors they're given aren't the
    // windows': a window kept on the first twin still takes the first
    // connector found, though a session kept on its twin took it. It alone
    // catches a reopenHere whose store drops `walk &&` from its guard.
    function handedOverGivesNone() {
        root.puttingBack([root.keptOn("Virtual-1", "letters"),
            root.keptOn("Virtual-2", "sketch")]);
        Sessions.reopenHere([{ id: "s-handed", cwd: root.goneFolder }], root.goneFrom,
            () => root.keptOn("Virtual-2", "pi"), "", () => [], true, null);
        Expect.same("the session handed over takes the first connector found",
            Sessions.reopening?.looking?.at?.output, "Virtual-3");
        Sessions.putBack(root.opened(97, "letters"));
        Expect.same("a window kept on the first twin takes it too, outside a walk",
            root.movedTo(97), [61]);
    }

    // Joel's focus on workspace `id` alone.
    function focusOn(id) {
        const workspaces = {};
        for (const [key, one] of Object.entries(Niri.workspaces))
            workspaces[key] = Object.assign({}, one, { is_focused: one.id === id });
        Niri.workspaces = workspaces;
        Niri.revision += 1;
    }

    // A walk joined while Joel's focus is on the other twin keeps the
    // screen in use as the walk began, so a window kept on a screen that's
    // gone goes there, not after the focus, which stays on the other twin
    // as the window comes. It alone catches a reopenHere whose store drops
    // `!("inUse" in puttingBack)` from its guard: the walk joined then
    // takes the other twin as the screen in use. It alone catches one that
    // stores no `puttingBack.inUse` too: the window then follows the focus.
    function rewalkedKeepsItsScreen() {
        root.puttingBack([{ app: "far", title: "far", screen: "Far Away", output: "Virtual-9",
            workspace: 1, column: 1, tile: 1 }]);
        Sessions.reopenHere([], root.goneFrom, () => null, Sessions.carryOn, () => [], true,
            null);
        root.focusOn(70);
        Sessions.reopenHere([], root.goneFrom, () => null, Sessions.carryOn, () => [], true,
            null);
        Sessions.putBack(root.opened(98, "far"));
        Expect.same("a window kept on a screen gone goes to the screen in use as the walk began",
            root.movedTo(98), [61]);
        root.focusOn(60);
    }

    // Where each session the walk holds was placed, by id.
    function placedNow() {
        const walk = Sessions.reopening;
        return [walk?.looking].concat(walk?.queue ?? []).filter(one => one)
            .map(one => [one.one.id, one.at?.output]).sort();
    }

    // Two sessions, kept on each twin, each placed by the places its own
    // boot kept (its moment). s-left's boot kept it alone, so says no twin
    // of its screen, and it takes the first connector found. s-right's kept
    // windows on three screens of the model, Virtual-1 to Virtual-3, so it
    // takes a connector none of those twins hold, the other. It alone
    // catches a reopenHere that hands placedGiven no moments,
    // `here.map(() => [])`, or one that looks up another session's moment:
    // with s-left's for both, s-right takes the first connector found too,
    // and with s-right's for s-left, s-left takes the other.
    function momentsSetTwinsApart() {
        const left = root.keptOn("Virtual-1", "pi");
        const right = root.keptOn("Virtual-2", "pi");
        const kept = { "s-left": left, "s-right": right };
        const moments = { "s-left": [left],
            "s-right": [left, right, root.keptOn("Virtual-3", "pi")] };
        Sessions.reopenHere([{ id: "s-left", cwd: root.goneFolder },
            { id: "s-right", cwd: root.goneFolder }], root.goneFrom, id => kept[id],
            Sessions.carryOn, id => moments[id], true, null);
        Expect.same("each twin session takes the connector its own boot's places say",
            root.placedNow(), [["s-left", "Virtual-3"], ["s-right", "Virtual-4"]]);
    }

    // A walk begun while Joel's focus is on the other twin places a session
    // kept on a screen that's gone on that twin, the screen in use. It
    // alone catches a reopenHere that hands placedGiven no screen in use:
    // the session is then left on the connector it was kept on.
    function walkBegunOnTheOther() {
        root.focusOn(70);
        Sessions.reopenHere([{ id: "s-far", cwd: root.goneFolder }], root.goneFrom,
            () => ({ app: "pi", title: "pi", screen: "Far Away", output: "Virtual-9",
                workspace: 1, column: 1, tile: 1 }), Sessions.carryOn, () => [], true, null);
        root.focusOn(60);
        Expect.same("a session kept on a screen gone goes to the screen in use as its walk began",
            Sessions.reopening?.looking?.at?.output, "Virtual-4");
    }

    // Kept on screens named for a prototype's keys, gone now, each goes to
    // the screen in use and is noted away from its own. The stand-in niri
    // records each move and applies none, so every window stays on 60 and
    // 61 stays the empty workspace last on the screen in use: both are
    // noted on 61. It alone catches a Sessions that notes a window away by
    // reading awayFrom by the screen's name, where a prototype's key finds
    // the prototype's own, or a putBack that stops noting at all.
    function prototypesNoted() {
        const kept = ["constructor", "__proto__"].map((screen, at) => ({ app: `tool-${at}`,
            title: `tool-${at}`, screen, output: screen, workspace: 1, column: 1, tile: 1 }));
        root.puttingBack(kept);
        Sessions.putBack(root.opened(99, "tool-0"));
        Sessions.putBack(root.opened(100, "tool-1"));
        const noted = Object.entries(Sessions.awayFrom)
            .filter(([screen]) => screen === "constructor" || screen === "__proto__")
            .map(([screen, away]) => [screen, away.map(one => one.id)]);
        Expect.same("a window kept on a prototype's key is noted away as any other", noted,
            [["constructor", [61]], ["__proto__", [61]]]);
    }

    // Each case in turn, each one's world the next one's ground. A case's
    // calls answer before it returns, but a walk it begins looks for its
    // session's folder and settles on timers of its own, so the next case
    // waits for that walk to end rather than join it.
    property string stage: "docking"
    function step() {
        // Before the wait for an open walk, so a walk that starts something
        // and stays open fails a tick later, naming what it started.
        Expect.same(`nothing is started at ${root.stage}, its sessions' folders gone`,
            Niri.spawns, []);
        if (root.stage !== "docking" && Sessions.reopening !== null)
            return;
        const walked = root.gone.slice(root.goneAt);
        root.goneAt = root.gone.length;
        if (root.stage === "docking") {
            root.docked();
            root.sessionsGive();
            root.stage = "given";
        } else if (root.stage === "given") {
            Expect.same("the walk ends with its session's folder found gone", walked,
                ["s-twin"]);
            root.windowsBeforeKeep();
            root.stage = "kept";
        } else if (root.stage === "kept") {
            root.sessionTakesOver();
            root.stage = "taken";
        } else if (root.stage === "taken") {
            Expect.same("the walk taken over ends with its session's folder found gone",
                walked, ["s-took"]);
            root.handedOverGivesNone();
            root.stage = "handed";
        } else if (root.stage === "handed") {
            Expect.same("the hand-over ends with its session's folder found gone", walked,
                ["s-handed"]);
            root.rewalkedKeepsItsScreen();
            root.stage = "rewalked";
        } else if (root.stage === "rewalked") {
            root.momentsSetTwinsApart();
            root.stage = "apart";
        } else if (root.stage === "apart") {
            Expect.same("the twins' walk ends with each session's folder found gone",
                walked.sort(), ["s-left", "s-right"]);
            root.walkBegunOnTheOther();
            root.stage = "begun";
        } else if (root.stage === "begun") {
            Expect.same("the walk begun on the other twin ends with its folder found gone",
                walked, ["s-far"]);
            root.prototypesNoted();
            Expect.same("no walk started anything, its sessions' folders gone", Niri.spawns,
                []);
            root.stage = "done";
            steps.stop();
            deadline.stop();
            console.log("twin screens each take their own windows back");
            Qt.exit(0);
        }
    }

    // A value in words, as JSON says it, else what kind of thing it is.
    function said(value) {
        try {
            return JSON.stringify(value);
        } catch (error) {
            return `a ${typeof value} JSON can't say, as ${Expect.thrown(error)}`;
        }
    }

    // What the walk still holds, so a look for a folder that never answers
    // reads apart from a walk wedged waiting on a window or settling. A
    // walk's looking is the session whose folder it waits to hear of, set
    // as it asks and cleared as the look answers; Sessions' looking holds
    // the looks not asked yet.
    function walkHolds() {
        const walk = Sessions.reopening;
        if (walk === null)
            return "its walk ended";
        const look = walk.looking ? `its look for ${walk.looking.one?.id}'s folder unanswered`
            : "no look for a folder unanswered";
        const held = walk.waiting === null ? "waiting on no window"
            : `waiting on a window, ${root.said(walk.waiting)}`;
        const settling = walk.settling ? `settling, round ${walk.rounds}` : "not settling";
        return `its walk open: ${look}, ${Sessions.looking.length} looks not asked yet, `
            + `${held}, ${settling}, ${walk.queue.length} queued, ${walk.late.length} late, `
            + `${walk.behind.length} behind`;
    }

    // Stepped on a tick rather than as the shell is made, since Quickshell
    // hears no exit asked of it before then, so the case would run to its
    // timeout.
    Timer {
        id: steps
        running: true
        repeat: true
        interval: 100
        onTriggered: {
            try {
                root.step();
            } catch (error) {
                steps.stop();
                deadline.stop();
                Expect.fail(Expect.thrown(error));
            }
        }
    }

    // A walk that never ends fails here, saying where it stopped, well
    // before the check's own timeout would kill it unsaid. What it says
    // past the stage is built in a try, so a throw there still fails it.
    Timer {
        id: deadline
        running: true
        interval: 20000
        onTriggered: {
            steps.stop();
            let message = `the case stopped at ${root.stage}`;
            try {
                message += `, ${root.walkHolds()}, having started ${root.said(Niri.spawns)}`;
            } catch (error) {
                message += `, and saying what its walk holds threw: ${Expect.thrown(error)}`;
            }
            Expect.fail(message);
        }
    }
}

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
// joined keeps the screen in use as it began. A window noted away from a
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
    readonly property string goneFolder: "/nowhere/given-back"

    // The walk places a session kept on the second twin on the first
    // connector found, so a window kept on the first twin goes to the
    // other, and one kept on the session's own twin goes with it. It alone
    // catches a reopenHere that drops the walk's store,
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
    // gone goes there, not after the focus. It alone catches a reopenHere
    // whose store drops `!("inUse" in puttingBack)` from its guard: the walk
    // joined then takes the other twin as the screen in use.
    function rewalkedKeepsItsScreen() {
        root.puttingBack([{ app: "far", title: "far", screen: "Far Away", output: "Virtual-9",
            workspace: 1, column: 1, tile: 1 }]);
        Sessions.reopenHere([], root.goneFrom, () => null, Sessions.carryOn, () => [], true,
            null);
        root.focusOn(70);
        Sessions.reopenHere([], root.goneFrom, () => null, Sessions.carryOn, () => [], true,
            null);
        root.focusOn(60);
        Sessions.putBack(root.opened(98, "far"));
        Expect.same("a window kept on a screen gone goes to the screen in use as the walk began",
            root.movedTo(98), [61]);
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
        if (root.stage === "docking") {
            root.docked();
            root.sessionsGive();
            root.stage = "given";
        } else if (Sessions.reopening !== null) {
            return;
        } else if (root.stage === "given") {
            Expect.same("the walk ends with its session's folder found gone", root.gone,
                ["s-twin"]);
            root.windowsBeforeKeep();
            root.stage = "kept";
        } else if (root.stage === "kept") {
            root.sessionTakesOver();
            root.stage = "taken";
        } else if (root.stage === "taken") {
            Expect.same("the walk taken over ends with its session's folder found gone",
                root.gone, ["s-twin", "s-took"]);
            root.handedOverGivesNone();
            root.stage = "handed";
        } else if (root.stage === "handed") {
            Expect.same("the hand-over ends with its session's folder found gone", root.gone,
                ["s-twin", "s-took", "s-handed"]);
            root.rewalkedKeepsItsScreen();
            root.stage = "rewalked";
        } else if (root.stage === "rewalked") {
            root.prototypesNoted();
            root.stage = "done";
            steps.stop();
            deadline.stop();
            console.log("twin screens each take their own windows back");
            Qt.exit(0);
        }
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
    // before the check's own timeout would kill it unsaid.
    Timer {
        id: deadline
        running: true
        interval: 20000
        onTriggered: {
            steps.stop();
            const walk = Sessions.reopening === null ? "ended" : "still open";
            Expect.fail(`the case stopped at ${root.stage}, its walk ${walk}`);
        }
    }
}

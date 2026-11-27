pragma ComponentBehavior: Bound
// Drives Reopen Them All's wiring between its sessions and the windows it
// puts back (modules/quickshell/Sessions.qml's reopenHere and putBack), with
// niri stood in (modules/quickshell/tests/stand-ins/hosted-reopen-niri.qml):
// two monitors of one model without serials, kept on Virtual-1 and
// Virtual-2, are back on Virtual-3 and Virtual-4. A session kept on the
// second takes the first connector found, so a window kept on the first
// goes to the other, as the connectors the walk gave its sessions say. A
// window put back before the walk placed its sessions keeps the connector
// it was given, so a window kept on its twin goes to the other. A window
// noted away from a screen named for a prototype's key, constructor or
// __proto__, is noted as any other, without a throw. Places.js and
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

    // The walk places a session kept on the second twin on the first
    // connector found, and a window kept on the first twin goes to the
    // other, past the connector the walk gave the session.
    function sessionsGive() {
        root.puttingBack([root.keptOn("Virtual-1", "chat"), root.keptOn("Virtual-2", "page")]);
        Sessions.reopenHere([{ id: "s-twin", cwd: "/tmp" }], () => {},
            () => root.keptOn("Virtual-2", "pi"), Sessions.carryOn, () => [], true, null);
        Expect.same("the session kept on the second twin takes the first connector found",
            Sessions.reopening?.looking?.at?.output, "Virtual-3");
        Sessions.putBack(root.opened(91, "chat"));
        Expect.same("a window kept on the first twin goes to the one the session didn't take",
            root.movedTo(91), [70]);
    }

    // A window put back before the walk placed its sessions takes the first
    // connector found, and keeps it through the walk, so its twin's window
    // goes to the other.
    function windowsBeforeKeep() {
        root.puttingBack([root.keptOn("Virtual-1", "mail"), root.keptOn("Virtual-2", "notes")]);
        Sessions.putBack(root.opened(92, "mail"));
        Expect.same("a window put back first takes the first connector found",
            root.movedTo(92), [61]);
        Sessions.reopenHere([], () => {}, () => null, Sessions.carryOn, () => [], true, null);
        Sessions.putBack(root.opened(93, "notes"));
        Expect.same("its twin's window goes to the other, after the walk",
            root.movedTo(93), [70]);
    }

    // Kept on screens named for a prototype's keys, gone now, each goes to
    // the screen in use and is noted away from its own.
    function prototypesNoted() {
        const kept = ["constructor", "__proto__"].map((screen, at) => ({ app: `tool-${at}`,
            title: `tool-${at}`, screen, output: screen, workspace: 1, column: 1, tile: 1 }));
        root.puttingBack(kept);
        Sessions.putBack(root.opened(94, "tool-0"));
        Sessions.putBack(root.opened(95, "tool-1"));
        const noted = Object.entries(Sessions.awayFrom)
            .filter(([screen]) => screen === "constructor" || screen === "__proto__")
            .map(([screen, away]) => [screen, away.map(one => one.id)]);
        Expect.same("a window kept on a prototype's key is noted away as any other", noted,
            [["constructor", [61]], ["__proto__", [61]]]);
    }

    // Each case runs at once, in turn, each one's world the next one's
    // ground: every call here answers before it returns, so nothing waits.
    function cases() {
        root.docked();
        root.sessionsGive();
        root.windowsBeforeKeep();
        root.prototypesNoted();
        console.log("twin monitors each take their own windows back");
    }

    Component.onCompleted: Expect.run(root.cases)
}

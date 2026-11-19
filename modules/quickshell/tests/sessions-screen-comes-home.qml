pragma ComponentBehavior: Bound
// Unplugs the monitor and plugs it back in under Sessions
// (modules/quickshell/Sessions.qml), with niri stood in
// (stand-ins/hosted-reopen-niri.qml): the laptop's screen with Joel's
// window on its one workspace, and the monitor's two. niri sends the monitor's two workspaces to
// the laptop's screen and, once the monitor is back, gives back only the
// first, since a window opened in the second meanwhile. niri never answers
// the shell's look at the screens as the monitor comes, as when it lagged
// and dropped its connection at a redock on 2026-11-15, and the second
// workspace still goes back to the monitor once niri answers a look again.
// Then a second monitor alike the first, as two of one model without serials
// are, comes and goes: its workspace waits while only the first is there, and
// goes to it once it's back, on a connector of its own, and isn't noted away
// again as niri moves it. It exits 0 once that holds, and otherwise 1, saying
// what didn't.
import qs
import Quickshell
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    property string stage: "docked"
    property real heldSince: 0
    property real twinGoneSince: 0
    property real settledSince: 0

    readonly property var laptop: ({ make: "Lark", model: "Book", serial: "1", logical: {} })
    readonly property var monitor: ({ make: "Gull", model: "Wide", serial: "2", logical: {} })
    readonly property var twin: ({ make: "Gull", model: "Wide", serial: "2", logical: {} })

    function workspacesAre(listed) {
        const now = {};
        for (const one of listed)
            now[one.id] = Object.assign({ is_focused: false, active_window_id: null }, one);
        Niri.workspaces = now;
        Niri.revision += 1;
    }

    // Answers the oldest look at the screens waiting, with these outputs,
    // and says whether one was waiting.
    function answerOutputs(outputs) {
        const waiting = Niri.asked.find(one => one.asking === "Outputs");
        if (waiting === undefined)
            return false;
        Niri.asked = Niri.asked.filter(one => one !== waiting);
        waiting.answer({ Outputs: outputs });
        return true;
    }

    function movedHome(id) {
        return Niri.acts.filter(act => act.MoveWorkspaceToMonitor?.reference?.Id === id)
            .map(act => act.MoveWorkspaceToMonitor.output);
    }

    function check() {
        try {
            if (stage === "docked")
                docked();
            else if (stage === "looked")
                looked();
            else if (stage === "undocked")
                undocked();
            else if (stage === "redocked")
                redocked();
            else if (stage === "answered")
                answered();
            else if (stage === "twinned")
                twinned();
            else if (stage === "twinGone")
                twinGone();
            else if (stage === "twinBack")
                twinBack();
            else if (stage === "twinHome")
                twinHome();
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    // Both screens there, as the shell first learns them. Sessions is made
    // as it's first named, so it's asked first, to see the workspaces as
    // they are before any moves.
    function docked() {
        Sessions.lookAtScreens(() => {});
        Niri.windows = { "90": { id: 90, pid: 900, workspace_id: 6, opened: "kitty",
            layout: { pos_in_scrolling_layout: [1, 1] } } };
        Niri.current = 90;
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 10, idx: 1, output: "Virtual-2" },
            { id: 14, idx: 2, output: "Virtual-2" }
        ]);
        stage = "looked";
    }

    function looked() {
        if (!root.answerOutputs({ "Virtual-1": root.laptop, "Virtual-2": root.monitor }))
            return;
        Expect.expect("the shell knows both screens", Sessions.screens["Virtual-2"] === "Gull Wide 2",
            JSON.stringify(Sessions.screens));
        // The monitor goes, and niri puts its workspaces after the laptop's.
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 10, idx: 2, output: "Virtual-1" },
            { id: 14, idx: 3, output: "Virtual-1" }
        ]);
        stage = "undocked";
    }

    // The shell notes both away from the monitor, then looks at the screens
    // as they settle, and hears the laptop's alone.
    function undocked() {
        const away = (Sessions.awayFrom["Gull Wide 2"] ?? []).map(one => one.id);
        if (away.length < 2 || !root.answerOutputs({ "Virtual-1": root.laptop }))
            return;
        Expect.expect("both the monitor's workspaces are noted away from it",
            away.includes(10) && away.includes(14), JSON.stringify(Sessions.awayFrom));
        // The monitor comes back. niri gives it its first workspace and an
        // empty one, and keeps the second, a window having opened in it.
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 14, idx: 2, output: "Virtual-1", active_window_id: 91 },
            { id: 10, idx: 1, output: "Virtual-2" },
            { id: 20, idx: 2, output: "Virtual-2" }
        ]);
        stage = "redocked";
    }

    // The shell looks at the screens again, and niri never answers, as on a
    // connection that closed.
    function redocked() {
        const waiting = Niri.asked.find(one => one.asking === "Outputs");
        if (waiting === undefined)
            return;
        if (root.heldSince === 0)
            root.heldSince = Date.now();
        if (Date.now() - root.heldSince < 3000)
            return;
        Expect.expect("nothing went home while niri hadn't answered", movedHome(14).length === 0,
            JSON.stringify(Niri.acts));
        Niri.asked = Niri.asked.filter(one => one !== waiting);
        stage = "answered";
    }

    // Any look after that is answered at once, and the workspace niri kept
    // on the laptop's screen goes to the monitor, the one it gave back left.
    function answered() {
        root.answerOutputs({ "Virtual-1": root.laptop, "Virtual-2": root.monitor });
        if (movedHome(14).length === 0)
            return;
        Expect.expect("the workspace niri kept goes to the monitor",
            movedHome(14).every(output => output === "Virtual-2"), JSON.stringify(Niri.acts));
        Expect.expect("the one niri gave back is left there", movedHome(10).length === 0,
            JSON.stringify(Niri.acts));
        console.log("a screen niri is late to name still takes its workspaces back");
        // niri moves it, and a second monitor alike the first comes on
        // Virtual-3 with a workspace of its own.
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 10, idx: 1, output: "Virtual-2" },
            { id: 14, idx: 2, output: "Virtual-2", active_window_id: 91 },
            { id: 30, idx: 1, output: "Virtual-3" }
        ]);
        Sessions.lookAtScreens(() => {});
        stage = "twinned";
    }

    function twinned() {
        root.answerOutputs({ "Virtual-1": root.laptop, "Virtual-2": root.monitor,
            "Virtual-3": root.twin });
        if (Sessions.screens["Virtual-3"] !== "Gull Wide 2")
            return;
        // The second monitor goes, and niri puts its workspace on the
        // laptop's screen.
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 30, idx: 2, output: "Virtual-1" },
            { id: 10, idx: 1, output: "Virtual-2" },
            { id: 14, idx: 2, output: "Virtual-2", active_window_id: 91 }
        ]);
        stage = "twinGone";
    }

    // The first monitor, alike it, is still there, so it isn't the second
    // come back: the workspace waits on the laptop's screen.
    function twinGone() {
        root.answerOutputs({ "Virtual-1": root.laptop, "Virtual-2": root.monitor });
        const away = (Sessions.awayFrom["Gull Wide 2"] ?? []).find(one => one.id === 30);
        if (away === undefined)
            return;
        if (root.twinGoneSince === 0) {
            Expect.expect("the second monitor's workspace is noted with the other screens as it left",
                JSON.stringify(away.others) === JSON.stringify({ "Virtual-1": "Lark Book 1",
                    "Virtual-2": "Gull Wide 2" }), JSON.stringify(Sessions.awayFrom));
            root.twinGoneSince = Date.now();
        }
        if (Date.now() - root.twinGoneSince < 5000)
            return;
        Expect.expect("nothing went to the monitor alike the one gone", movedHome(30).length === 0,
            JSON.stringify(Niri.acts));
        // It comes back on a connector of its own, with a workspace niri
        // makes there.
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 30, idx: 2, output: "Virtual-1" },
            { id: 10, idx: 1, output: "Virtual-2" },
            { id: 14, idx: 2, output: "Virtual-2", active_window_id: 91 },
            { id: 31, idx: 1, output: "Virtual-4" }
        ]);
        stage = "twinBack";
    }

    function twinBack() {
        root.answerOutputs({ "Virtual-1": root.laptop, "Virtual-2": root.monitor,
            "Virtual-4": root.twin });
        if (movedHome(30).length === 0)
            return;
        Expect.expect("the second monitor's workspace goes to it on its new connector",
            movedHome(30).every(output => output === "Virtual-4"), JSON.stringify(Niri.acts));
        // niri moves it there.
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 10, idx: 1, output: "Virtual-2" },
            { id: 14, idx: 2, output: "Virtual-2", active_window_id: 91 },
            { id: 30, idx: 1, output: "Virtual-4" },
            { id: 31, idx: 2, output: "Virtual-4" }
        ]);
        root.settledSince = Date.now();
        stage = "twinHome";
    }

    // Gone from the laptop's screen to the one it was sent to, it isn't
    // noted away from the connector it left before.
    function twinHome() {
        root.answerOutputs({ "Virtual-1": root.laptop, "Virtual-2": root.monitor,
            "Virtual-4": root.twin });
        if (Date.now() - root.settledSince < 3000)
            return;
        Expect.expect("no workspace waits once each is home", Object.keys(Sessions.awayFrom).length === 0,
            JSON.stringify(Sessions.awayFrom));
        console.log("a monitor alike another takes its workspace back, on whatever connector");
        Qt.exit(0);
    }

    // Every step waits on a timer of Sessions' own, so each is looked at
    // often rather than heard.
    Timer {
        running: true
        repeat: true
        interval: 100
        onTriggered: root.check()
    }

    Timer {
        running: true
        interval: 50000
        onTriggered: Expect.fail(`still at ${root.stage}: ${JSON.stringify(Niri.acts)} `
            + `${JSON.stringify(Sessions.awayFrom)} ${JSON.stringify(Sessions.screens)}`)
    }
}

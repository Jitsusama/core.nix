pragma ComponentBehavior: Bound
// Unplugs the monitor and plugs it back in under Sessions
// (modules/quickshell/Sessions.qml), with niri stood in
// (stand-ins/screens-niri.qml). niri sends the monitor's two workspaces to
// the laptop's screen and, once the monitor is back, gives back only the
// first, since a window opened in the second meanwhile. niri answers the
// shell's look at the screens after its patience has run out, as it did
// while lagging at a redock on 2026-11-15, and the second workspace still
// goes back to the monitor once niri has said the monitor is there. It
// exits 0 once that holds, and otherwise 1, saying what didn't.
import qs
import Quickshell
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    property string stage: "docked"
    property real heldSince: 0

    readonly property var laptop: ({ make: "Lark", model: "Book", serial: "1", logical: {} })
    readonly property var monitor: ({ make: "Gull", model: "Wide", serial: "2", logical: {} })

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
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    // Both screens there, as the shell first learns them. Sessions is made
    // as it's first named, so it's asked first, to see the workspaces as
    // they are before any moves.
    function docked() {
        Sessions.lookAtScreens(() => {});
        root.workspacesAre(Object.values(Niri.workspaces));
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

    // The shell looks at the screens again, and niri answers only once the
    // shell has stopped waiting for it.
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
        root.answerOutputs({ "Virtual-1": root.laptop, "Virtual-2": root.monitor });
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
        interval: 30000
        onTriggered: Expect.fail(`still at ${root.stage}: ${JSON.stringify(Niri.acts)} `
            + `${JSON.stringify(Sessions.awayFrom)} ${JSON.stringify(Sessions.screens)}`)
    }
}

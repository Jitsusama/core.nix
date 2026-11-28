pragma ComponentBehavior: Bound
// Unplugs two screens on connectors named for a prototype's keys,
// __proto__ and constructor, and plugs them back in under Sessions
// (modules/quickshell/Sessions.qml's noteGone and comeHome), with niri
// stood in (modules/quickshell/tests/stand-ins/hosted-reopen-niri.qml): the
// laptop's screen with Joel's window on its one workspace, and a workspace
// on each of the other two. niri sends both to the laptop's screen, and the
// shell notes each away by its screen's described name, not by what a
// plain object holds at that key, so the one on __proto__ isn't noted away
// from Object.prototype. Both wait while only the laptop's screen is
// there, and once the two are back, each workspace goes back to its own
// connector and is noted away no longer. Places.js holds each answer to
// tests/places.qml; this holds Sessions to passing them on. It exits 0
// once that holds, and otherwise 1, saying what didn't.
import qs
import Quickshell
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property var laptop: ({ make: "Lark", model: "Book", serial: "1", logical: {} })
    readonly property var proto: ({ make: "Gull", model: "Wide", serial: "3", logical: {} })
    readonly property var named: ({ make: "Gull", model: "Tall", serial: "4", logical: {} })

    // The screens niri says are connected, as each look at them is answered.
    // The key is computed, since a literal one named __proto__ would set the
    // object's prototype rather than name a connector.
    property var outputsNow: ({})
    function allConnected() {
        return { "Virtual-1": root.laptop, ["__proto__"]: root.proto,
            constructor: root.named };
    }

    function workspacesAre(listed) {
        const now = {};
        for (const one of listed)
            now[one.id] = Object.assign({ is_focused: false, active_window_id: null }, one);
        Niri.workspaces = now;
        Niri.revision += 1;
    }

    // Answers every look at the screens waiting, with the screens connected
    // now. A Places.js whose screensOf folds them into a plain object fails
    // here, since Qt's engine throws a type error as it sets that object's
    // prototype to the screen on __proto__, said as a string.
    function answerOutputs() {
        const waiting = Niri.asked.filter(one => one.asking === "Outputs");
        Niri.asked = Niri.asked.filter(one => one.asking !== "Outputs");
        const connectors = Object.keys(root.outputsNow).join(", ");
        waiting.forEach(one => {
            try {
                one.answer({ Outputs: root.outputsNow });
            } catch (error) {
                throw new Error(`the shell hears the screens on ${connectors}: `
                    + Expect.thrown(error));
            }
        });
    }

    function movedHome(id) {
        return Niri.acts.filter(act => act.MoveWorkspaceToMonitor?.reference?.Id === id)
            .map(act => act.MoveWorkspaceToMonitor.output);
    }

    // What each workspace noted away is noted with, by the screen it waits
    // for, as [screen, id, number, connector].
    function notedAway() {
        return Object.keys(Sessions.awayFrom).sort().reduce((all, screen) => all.concat(
            Sessions.awayFrom[screen].map(one => [screen, one.id, one.workspace, one.output])),
        []);
    }

    // All three screens there, as the shell first learns them. Sessions is
    // made as it's first named, so it's asked first, to see the workspaces
    // as they are before any moves.
    function docking() {
        Sessions.lookAtScreens(() => {});
        Niri.windows = { "90": { id: 90, pid: 900, workspace_id: 6, opened: "kitty",
            layout: { pos_in_scrolling_layout: [1, 1] } } };
        Niri.current = 90;
        root.outputsNow = root.allConnected();
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 10, idx: 1, output: "__proto__" },
            { id: 20, idx: 1, output: "constructor" }
        ]);
        root.stage = "looked";
    }

    // Once the shell has heard the screens, the two on a prototype's keys
    // go, and niri puts their workspaces after the laptop's. The shell is waited on for the
    // laptop's screen and constructor alone, so a screensOf that says no
    // screen on __proto__ fails where it's noted away, saying what it noted,
    // rather than at the deadline.
    function looked() {
        if (Sessions.screens["Virtual-1"] !== "Lark Book 1"
            || Sessions.screens["constructor"] !== "Gull Tall 4")
            return;
        root.outputsNow = { "Virtual-1": root.laptop };
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 10, idx: 2, output: "Virtual-1" },
            { id: 20, idx: 3, output: "Virtual-1" }
        ]);
        root.stage = "noted";
    }

    // The shell notes each away as niri moves it, by its screen's described
    // name, so the one on __proto__ waits for its screen and not for
    // Object.prototype, said as "[object Object]", nor for its connector.
    function noted() {
        Expect.same("each workspace is noted away by its screen's described name",
            root.notedAway(), [["Gull Tall 4", 20, 1, "constructor"],
                ["Gull Wide 3", 10, 1, "__proto__"]]);
        root.stage = "gone";
    }

    // Both wait, said held once comeHome has looked at the screens and
    // found only the laptop's, and nothing goes to a connector that's gone.
    // Then the two screens come back, each with an empty workspace niri
    // makes there, while niri keeps both workspaces on the laptop's screen.
    function gone() {
        if (!Sessions.heldSaid.includes(10) || !Sessions.heldSaid.includes(20))
            return;
        Expect.same("nothing goes to a screen that's gone", [root.movedHome(10),
            root.movedHome(20)], [[], []]);
        Expect.same("both still wait for their screens", root.notedAway(),
            [["Gull Tall 4", 20, 1, "constructor"], ["Gull Wide 3", 10, 1, "__proto__"]]);
        root.outputsNow = root.allConnected();
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 10, idx: 2, output: "Virtual-1" },
            { id: 20, idx: 3, output: "Virtual-1" },
            { id: 11, idx: 1, output: "__proto__" },
            { id: 21, idx: 1, output: "constructor" }
        ]);
        root.stage = "back";
    }

    // Each workspace goes back to its own connector, and none waits any
    // more. Then niri moves them there.
    function back() {
        if (root.movedHome(10).length === 0 && root.movedHome(20).length === 0)
            return;
        Expect.same("each workspace goes back to its own connector", [root.movedHome(10),
            root.movedHome(20)], [["__proto__"], ["constructor"]]);
        Expect.same("no workspace waits once each is sent home", root.notedAway(), []);
        root.workspacesAre([
            { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
            { id: 10, idx: 1, output: "__proto__" },
            { id: 11, idx: 2, output: "__proto__" },
            { id: 20, idx: 1, output: "constructor" },
            { id: 21, idx: 2, output: "constructor" }
        ]);
        root.settledSince = Date.now();
        root.stage = "home";
    }

    // Moved to the connector it was sent to, neither is noted away from
    // the laptop's screen it left, nor sent anywhere again.
    function home() {
        if (Date.now() - root.settledSince < 3000)
            return;
        Expect.same("none is noted away once each is home", root.notedAway(), []);
        Expect.same("neither is sent anywhere again", [root.movedHome(10), root.movedHome(20)],
            [["__proto__"], ["constructor"]]);
        root.stage = "done";
        steps.stop();
        deadline.stop();
        console.log("screens on a prototype's keys each take their workspace back");
        Qt.exit(0);
    }

    property string stage: "docking"
    property real settledSince: 0
    function step() {
        root.answerOutputs();
        if (root.stage === "docking")
            root.docking();
        else if (root.stage === "looked")
            root.looked();
        else if (root.stage === "noted")
            root.noted();
        else if (root.stage === "gone")
            root.gone();
        else if (root.stage === "back")
            root.back();
        else if (root.stage === "home")
            root.home();
    }

    // Stepped on a tick rather than as the shell is made, since Quickshell
    // hears no exit asked of it before then, and every step waits on a
    // timer of Sessions' own, so each is looked at often rather than heard.
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
                Expect.fail(`at ${root.stage}, ${Expect.thrown(error)}`);
            }
        }
    }
    // A stage that never ends fails here, saying where it stopped, well
    // before the check's own timeout would kill it unsaid.
    Timer {
        id: deadline
        running: true
        interval: 20000
        onTriggered: {
            steps.stop();
            Expect.fail(`the case stopped at ${root.stage}, having noted away `
                + `${JSON.stringify(root.notedAway())} and acted ${JSON.stringify(Niri.acts)}`);
        }
    }
}

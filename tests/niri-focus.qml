// Drives the shell's niri (modules/quickshell/Niri.qml) through Quickshell
// itself, telling it niri's events as its stream would. The keys moving,
// to another window or to a workspace's other window while a layer holds
// them, move `focused` and `current` and make no list again (`revision`);
// a window used moves `used` alone, for what puts the one used last first;
// and a window retitled, opened or closed still makes them again. It exits
// 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    // How many times what reads `current` alone woke, so a move of the
    // keys is seen to reach it.
    property int woke: 0
    readonly property var here: Niri.current
    onHereChanged: woke += 1

    function window(id, title, focused, secs) {
        return { id: id, title: title, app_id: "tester", pid: 100 + id, workspace_id: 1,
            is_focused: focused, is_floating: false, is_urgent: false,
            focus_timestamp: { secs: secs, nanos: 0 },
            layout: { pos_in_scrolling_layout: [id, 1] } };
    }

    function cases() {
        Niri.hear({ WorkspacesChanged: { workspaces: [
            { id: 1, idx: 1, output: "screen", is_active: true, is_focused: true,
                active_window_id: 1 }
        ] } });
        Niri.hear({ WindowsChanged: { windows: [window(1, "Logbook", true, 200),
            window(2, "Draft", false, 100)] } });
        Expect.same("the window the keys are in", Niri.current, 1);
        const listed = Niri.revision;
        const woken = root.woke;

        Niri.hear({ WindowFocusChanged: { id: 2 } });
        Expect.same("the keys moved to Draft", Niri.current, 2);
        Expect.expect("Draft says it has the keys", Niri.windows[2].is_focused === true,
            "it doesn't");
        Expect.same("a list made again as the keys moved", Niri.revision, listed);
        Expect.same("what reads the window the keys are in woke", root.woke, woken + 1);

        // The launcher takes the keys: no window has them, so the window
        // they were in is the workspace's active one, which then changes.
        Niri.hear({ WindowFocusChanged: { id: null } });
        Expect.same("a layer holding the keys keeps the active window", Niri.current, 1);
        Niri.hear({ WorkspaceActiveWindowChanged: { workspace_id: 1, active_window_id: 2 } });
        Expect.same("the active window changing under a layer", Niri.current, 2);
        Expect.same("a list made again as the active window changed", Niri.revision, listed);

        const usedBefore = Niri.used;
        Niri.hear({ WindowFocusTimestampChanged: { id: 2,
            focus_timestamp: { secs: 300, nanos: 0 } } });
        Expect.same("a list made again as a window was used", Niri.revision, listed);
        Expect.same("a window used says so", Niri.used, usedBefore + 1);
        Expect.same("its use kept", Niri.windows[2].focus_timestamp.secs, 300);
        Niri.hear({ WindowOpenedOrChanged: { window: window(2, "Letters", false, 300) } });
        Expect.expect("a window retitled is listed again", Niri.revision > listed,
            "the revision stayed at " + listed);
        const retitled = Niri.revision;
        Niri.hear({ WindowClosed: { id: 2 } });
        Expect.expect("a window closed is listed again", Niri.revision > retitled,
            "the revision stayed at " + retitled);
    }

    Component.onCompleted: {
        try {
            cases();
        } catch (error) {
            Expect.fail(error.message);
            return;
        }
        // The exit waits for the event loop, which Quickshell has yet to run.
        Qt.callLater(Qt.exit, 0);
    }
}

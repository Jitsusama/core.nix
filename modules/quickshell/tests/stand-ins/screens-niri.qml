// niri, for a monitor coming back while niri is slow to say so
// (tests/sessions-screen-comes-home.qml): the laptop's screen with Joel's
// window on its one workspace, and the monitor's two. What the shell asks
// is kept with the answer it waits on, for the case to answer when it
// pleases, as a lagging niri does; what it does is kept too.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var workspaces: ({
        "6": { id: 6, idx: 1, output: "Virtual-1", is_focused: true, active_window_id: 90 },
        "10": { id: 10, idx: 1, output: "Virtual-2", is_focused: false, active_window_id: null },
        "14": { id: 14, idx: 2, output: "Virtual-2", is_focused: false, active_window_id: null }
    })
    property var windows: ({
        "90": { id: 90, pid: 900, workspace_id: 6, opened: "kitty",
            layout: { pos_in_scrolling_layout: [1, 1] } }
    })
    property int revision: 0
    property var current: 90
    property var focused: null
    property var acts: []
    property var spawns: []
    property var asked: []

    signal windowOpenedOrChanged(var window)
    signal windowClosed(var id)
    signal layoutsChanged

    function act(request) {
        acts = acts.concat([request]);
    }

    function request(asking, answer) {
        asked = asked.concat([{ asking, answer, at: Date.now() }]);
    }

    // The workspace by its id, or the one in front with none named.
    function workspace(id) {
        if (id !== undefined)
            return workspaces[id] ?? null;
        return Object.values(workspaces).find(one => one.is_focused) ?? null;
    }

    function spawn(command) {
        spawns = spawns.concat([command.join(" ")]);
    }
}

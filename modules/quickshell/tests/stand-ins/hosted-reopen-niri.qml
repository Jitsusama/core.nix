// niri, for Reopen Them All opening the viewers of sessions pi's host runs
// (tests/sessions-hosted-reopen.qml): Joel's window on the monitor's first
// workspace, and the empty one niri keeps last, the test making more as
// windows open there. What the shell asks of niri is kept, acts and
// spawns alike, and asked for, never answered.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var workspaces: ({
        "40": { id: 40, idx: 1, output: "DP-4", is_focused: true, active_window_id: 90 },
        "41": { id: 41, idx: 2, output: "DP-4", is_focused: false, active_window_id: null }
    })
    property var windows: ({
        "90": { id: 90, pid: 900, workspace_id: 40, opened: "firefox",
            layout: { pos_in_scrolling_layout: [1, 1] } }
    })
    property int revision: 0
    property var current: null
    property var focused: null
    property var acts: []
    property var spawns: []

    signal windowOpenedOrChanged(var window)

    function act(request) {
        acts = acts.concat([request]);
    }

    function request(asked, answer) {}

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

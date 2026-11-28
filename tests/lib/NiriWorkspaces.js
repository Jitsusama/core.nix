// What a stand-in niri's workspaces are, for a check that stands niri in
// (modules/quickshell/tests/stand-ins/hosted-reopen-niri.qml) and moves its
// workspaces between screens: the workspaces it says are there, set as the
// check lists them, and where the shell sent each one back. A library can't
// see the Niri singleton, so each takes the stand-in as `niri`.
.pragma library

// Makes `listed` the workspaces niri says are there, each unfocused and with
// no window active unless it says so, and tells the shell they changed.
function are(niri, listed) {
    const now = {};
    for (const one of listed) {
        now[one.id] = Object.assign({ is_focused: false, active_window_id: null }, one);
    }
    niri.workspaces = now;
    niri.revision += 1;
}

// The connectors the shell asked niri to move workspace `id` to, in order.
function homeward(niri, id) {
    return niri.acts.filter(act => act.MoveWorkspaceToMonitor?.reference?.Id === id)
        .map(act => act.MoveWorkspaceToMonitor.output);
}

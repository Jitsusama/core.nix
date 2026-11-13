// What the windows daemon (modules/windows) says of niri's windows, for a
// check that stands niri in with windows of niri's own shape and gives them
// to a source as the daemon would, through its Daemon's own reading of a
// line (Daemon.qml's said). `lookup` stands in for the daemon's own lookup of
// a window's desktop entry, as Quickshell's DesktopEntries.heuristicLookup
// does over the check's entries. What a record's words would read is the
// daemon's to work out, and its own tests hold it there; here a record holds
// what a source draws or places by.
.pragma library

// When a window was last looked at, in milliseconds, as a record's `order`.
function order(window) {
    const stamp = window.focus_timestamp;
    return stamp ? stamp.secs * 1000 + Math.floor(stamp.nanos / 1e6) : 0;
}

// A window's record, `current` being the window the keys are in.
function record(window, lookup, current) {
    const entry = window.app_id ? lookup(window.app_id) : null;
    const program = entry ? entry.name : (window.app_id ?? "");
    const tile = window.layout?.pos_in_scrolling_layout ?? null;
    return {
        key: `window:${window.id}`,
        order: order(window),
        id: window.id,
        app_id: window.app_id ?? "",
        opened: window.opened ?? null,
        // Whether it holds an agent's session, as the daemon says of a kitty
        // window classed for one (modules/windows/src/desktop.rs's app_of).
        session: window.session === true,
        title: window.title ?? "",
        pid: window.pid ?? null,
        entry: entry ? entry.id : null,
        program,
        owner: entry ? entry.id : null,
        named: window.title || program,
        untitled: !window.title || window.title === program,
        alike: 1,
        workspace: window.workspace_id ?? null,
        workspace_idx: window.workspace_id ?? null,
        elsewhere: false,
        column: tile ? tile[0] : null,
        down: tile ? tile[1] : null,
        floating: window.is_floating === true,
        urgent: window.is_urgent === true,
        focused: window.id === current
    };
}

// Tells a source's daemon every window there is, as the daemon answers a
// watch, so one it held that's not among them goes.
function told(daemon, windows, lookup, current) {
    const records = Object.keys(windows).map(id => record(windows[id], lookup, current));
    daemon.said(JSON.stringify({ id: "all", records }));
}

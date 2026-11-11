// What Niri.qml holds, made from the windows daemon's records
// (modules/windows) rather than from niri's own event stream, so the shell
// wakes only for what something here reads: never for a window's tile
// moving, which niri says at every resize and scroll, unless a reader
// draws tiles (`layout` records, watched only then), nor for a window's
// name for its row or what it reads alike with, which only the launcher's
// own watch takes. Each window and workspace is given in niri's own shape,
// as every reader of Niri.qml reads it, and only those told are made
// again, in place, as niri's events changed them. Plain JavaScript, held
// to it by tests/niri-records.qml in Qt's own engine.
.pragma library

// The fields of a window record Niri.qml's watch is told, and only as one
// of them changes (the daemon's `fields`).
const WINDOW_FIELDS = ["id", "order", "title", "app_id", "opened", "session", "pid",
    "workspace", "floating", "urgent", "column", "down"];

// The fields a list of windows draws from, so a change to one is a change
// `revision` counts; when a window was used, and where it sits, aren't.
const LISTED = ["title", "app_id", "opened", "session", "pid", "workspace", "urgent",
    "floating"];

// What says a workspace is another, rather than only activated or
// focused: niri said these as a whole WorkspacesChanged.
const PLACED = ["idx", "name", "output"];

// A moment as niri says one, from the daemon's milliseconds; none for a
// window never used.
function stampOf(ms) {
    return typeof ms === "number" && ms > 0
        ? { secs: Math.floor(ms / 1000), nanos: (ms % 1000) * 1000000 } : null;
}

// A record without its envelope, as niri says a cast or a tile.
function bare(record) {
    const said = Object.assign({}, record);
    delete said.key;
    delete said.order;
    delete said.id;
    return said;
}

// A window's tile, as niri lays it out: its place in the scrolling layout
// from its record, and the rest from its `layout` record where watched.
function layoutOf(record, tile) {
    const layout = tile ? bare(tile) : {};
    layout.pos_in_scrolling_layout = record.column === null || record.column === undefined
        ? null : [record.column, record.down];
    return layout;
}

// A window as niri describes it, from its record, the window niri says
// has the keys, and its tile record where it's watched. One opened for a
// session is kitty's with `session`, as the daemon says it.
function windowOf(record, focused, tile) {
    const window = {
        id: record.id,
        title: record.title ?? "",
        app_id: record.app_id ?? "",
        opened: record.opened ?? record.app_id ?? "",
        pid: record.pid ?? null,
        workspace_id: record.workspace ?? null,
        is_focused: record.id === focused,
        is_floating: record.floating === true,
        is_urgent: record.urgent === true,
        focus_timestamp: stampOf(record.order),
        layout: layoutOf(record, tile)
    };
    if (record.session === true) {
        window.session = true;
    }
    return window;
}

function workspaceOf(record) {
    return {
        id: record.id,
        idx: record.idx,
        name: record.name ?? null,
        output: record.output ?? null,
        is_active: record.active === true,
        is_focused: record.focused === true,
        is_urgent: record.urgent === true,
        active_window_id: record.active_window ?? null
    };
}

function kindOf(key) {
    const at = String(key).indexOf(":");
    return at < 0 ? [String(key), ""] : [key.slice(0, at), key.slice(at + 1)];
}

// A new place to hold what the daemon tells, as Niri.qml holds it: its
// windows, workspaces and casts in niri's shape, the records as told by
// key, the window niri says has the keys, the overview, whether niri's
// settings failed, and how many screenshots were taken, none until told.
function held() {
    return {
        windows: {},
        workspaces: {},
        casts: {},
        records: {},
        focused: null,
        overview: false,
        settingsFailed: false,
        taken: null
    };
}

function differs(was, now, fields) {
    return was === undefined || fields.some(field => was[field] !== now[field]);
}

// Takes what the daemon told, the records `changed` and the keys `gone`,
// into `into` (held()), each window told of made again in place, as
// niri's events changed them; workspaces and casts are new objects where
// one came, went or moved (`spaces`, `casts`), as niri said them whole.
// It answers what to say: whether a list of windows or workspaces changed
// (`listed`, which `revision` counts); each window opened, changed in a
// way listed, or given the keys; each closed; whether a window moved in
// its layout or a tile did (`layouts`); and a screenshot taken since the
// first told, whose record is what was taken before (`shot`, its path).
function take(into, changed, gone) {
    const told = {
        listed: false,
        spaces: false,
        casts: false,
        opened: [],
        closed: [],
        layouts: false,
        shot: false,
        path: null
    };
    const focus = changed.find(record => record.key === "focus");
    const focusing = focus !== undefined && (focus.window ?? null) !== into.focused;
    if (focusing) {
        into.focused = focus.window ?? null;
        for (const id of Object.keys(into.windows)) {
            into.windows[id].is_focused = into.windows[id].id === into.focused;
        }
    }
    for (const record of changed) {
        const [kind, id] = kindOf(record.key);
        const was = into.records[record.key];
        into.records[record.key] = record;
        if (kind === "window") {
            into.windows[id] = windowOf(record, into.focused, into.records[`layout:${id}`]);
            if (differs(was, record, LISTED)) {
                told.listed = true;
                told.opened.push(into.windows[id]);
            } else if (focusing && record.id === into.focused) {
                told.opened.push(into.windows[id]);
            }
            if (was !== undefined && differs(was, record, ["column", "down"])) {
                told.layouts = true;
            }
        } else if (kind === "layout") {
            const window = into.windows[id];
            if (window) {
                window.layout = layoutOf(into.records[`window:${id}`], record);
            }
            told.layouts = true;
        } else if (kind === "workspace") {
            if (differs(was, record, PLACED)) {
                told.spaces = true;
            }
            into.workspaces[id] = workspaceOf(record);
            told.listed = true;
        } else if (kind === "cast") {
            told.casts = true;
        } else if (kind === "desktop") {
            into.overview = record.overview === true;
            into.settingsFailed = record.settings_failed === true;
        } else if (kind === "screenshot") {
            if (into.taken !== null && record.taken !== into.taken) {
                told.shot = true;
                told.path = record.path ?? null;
            }
            into.taken = record.taken ?? 0;
        }
    }
    if (focusing && !told.opened.some(window => window.id === into.focused)) {
        const window = into.windows[into.focused];
        if (window) {
            told.opened.push(window);
        }
    }
    for (const key of gone) {
        const [kind, id] = kindOf(key);
        delete into.records[key];
        if (kind === "window") {
            delete into.windows[id];
            told.closed.push(Number(id));
            told.listed = true;
        } else if (kind === "layout") {
            const window = into.windows[id];
            if (window) {
                window.layout = layoutOf(into.records[`window:${id}`], null);
            }
            told.layouts = true;
        } else if (kind === "workspace") {
            delete into.workspaces[id];
            told.spaces = true;
            told.listed = true;
        } else if (kind === "cast") {
            told.casts = true;
        }
    }
    if (told.spaces) {
        into.workspaces = Object.assign({}, into.workspaces);
    }
    if (told.casts) {
        into.casts = castsFrom(into.records);
    }
    return told;
}

// Every cast by its stream's id, as niri says each.
function castsFrom(records) {
    const all = {};
    for (const key of Object.keys(records)) {
        if (kindOf(key)[0] === "cast") {
            const cast = bare(records[key]);
            all[cast.stream_id] = cast;
        }
    }
    return all;
}

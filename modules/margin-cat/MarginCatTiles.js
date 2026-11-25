// How the margin cat reads niri's layout: each window's tile where niri
// says it will come to rest, in its workspace's own view, the screen's for
// the one shown (modules/niri/tiled-positions.patch), in the terms
// MarginCatPaths.js takes. MarginCatScreen.qml asks it, passing what Niri
// holds, so its bindings still follow every change niri reports; it's kept
// apart because what niri says, and how, changes for reasons of its own.
.pragma library

// A window's tile where it's on the shown workspace of the screen named
// `output`, null where it isn't: its left, top and width, and whether it
// floats, being in no column of niri's scrolling layout.
function tile(windows, workspaces, output, id) {
    const shown = id === null ? undefined : windows[id];
    const at = shown?.layout?.tile_pos_in_workspace_view;
    const workspace = shown ? workspaces[shown.workspace_id] : null;
    if (!at || workspace?.output !== output || !workspace.is_active) {
        return null;
    }
    return {
        left: at[0],
        top: at[1],
        width: shown.layout.tile_size[0],
        floats: !shown.layout.pos_in_scrolling_layout
    };
}

// Whether no window is on a workspace at all, laid out or not: niri says a
// workspace shown before its windows' tiles there, as it slides to one or
// takes a new scale, so their tiles alone would say it empty a moment.
function empty(windows, workspaceId) {
    return !Object.values(windows).some(one => one.workspace_id === workspaceId);
}

// What a walk off its seat rides from (MarginCatFalls.js's rides and
// drops): the tiles `saw` as last seen, or, where the window `seat` it
// stands on is gone from them, `held`, as they last held it. niri says a
// window closed before it lays its column out again, and sitting, the
// shell hears no other tile, so the walk off would know no row it sat on
// and fall rather than ride (a nested niri, 2026-11-14).
function before(saw, held, seat) {
    return seat === null || seat === undefined || saw.some(one => one.id === seat)
        ? saw : held;
}

// How long before niri last told of it a motion of `kind` began on a
// workspace's windows, in ms, the longest where several are under way, null
// for none: niri's own word, as its tile_animation
// (modules/niri/tiled-positions.patch), so a fall a window grows down over
// goes with that window's edge ("Resize"), and a cat a scroll carries with
// its window ("ViewMove").
function animating(windows, workspaceId, kind) {
    const began = Object.values(windows)
        .filter(one => one.workspace_id === workspaceId
            && one.layout?.tile_animation === kind
            && typeof one.layout?.tile_animation_began_ms_ago === "number")
        .map(one => one.layout.tile_animation_began_ms_ago);
    return began.length ? Math.max(...began) : null;
}

// What niri last said of a motion of `kind` on a workspace's windows, as a
// word that changes only when it tells of one again, "" for none. niri says
// a motion as it begins, never as it ends (tiled-positions.patch), so a
// window keeps the word of its last scroll until its next layout, and a
// floating window dragged long after would ride that scroll's spring.
function told(windows, workspaceId, kind) {
    return Object.values(windows)
        .filter(one => one.workspace_id === workspaceId && one.layout?.tile_animation === kind)
        .map(one => JSON.stringify([one.id, one.layout.tile_animation_began_ms_ago,
            one.layout.tile_pos_in_workspace_view]))
        .sort()
        .join();
}

// The word `told` says `now`, and when it was first heard: `heard`'s time
// for the word it already holds, `now` for a new one.
function hears(heard, word, now) {
    return heard?.word === word ? heard : { word, at: now };
}

// Whether the motion `heard` tells of still runs `now`, heard less than
// `lasts` ms ago, niri's spring's length; never for no word.
function runs(heard, now, lasts) {
    return heard !== null && heard.word !== "" && now - heard.at < lasts;
}

// The tiles of a workspace's windows, each with its column and its row in
// niri's scrolling layout, both null for a floating one, and its window's id.
function of(windows, workspaceId) {
    if (workspaceId === null || workspaceId === undefined) {
        return [];
    }
    return Object.values(windows)
        .filter(one => one.workspace_id === workspaceId
            && one.layout?.tile_pos_in_workspace_view && one.layout?.tile_size)
        .map(one => ({
            left: one.layout.tile_pos_in_workspace_view[0],
            top: one.layout.tile_pos_in_workspace_view[1],
            width: one.layout.tile_size[0],
            height: one.layout.tile_size[1],
            column: one.layout.pos_in_scrolling_layout?.[0] ?? null,
            row: one.layout.pos_in_scrolling_layout?.[1] ?? null,
            id: one.id
        }));
}

// How the margin cat reads niri's layout: each window's tile where niri
// says it will come to rest, in its workspace's own view, the screen's for
// the one shown (modules/niri/tiled-positions.patch), in the terms
// MarginCatPaths.js takes. MarginCatScreen.qml asks it, passing what Niri
// holds, so its bindings still follow every change niri reports; it's kept
// apart because what niri says, and how, changes for reasons of its own.
.pragma library

// A window's tile where it's on the shown workspace of the screen named
// `output`, null where it isn't: its left, top and width.
function tile(windows, workspaces, output, id) {
    const shown = id === null ? undefined : windows[id];
    const at = shown?.layout?.tile_pos_in_workspace_view;
    const workspace = shown ? workspaces[shown.workspace_id] : null;
    if (!at || workspace?.output !== output || !workspace.is_active) {
        return null;
    }
    return { left: at[0], top: at[1], width: shown.layout.tile_size[0] };
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

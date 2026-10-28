// The newest screenshot, opened in GIMP from the launcher, found by typing
// or in Capture, after what it captures. Every
// screenshot lands in one folder, niri's own and the region ones the shell
// takes with grim alike, so the folder is watched rather than niri, which
// tells the shell only of its own. One that was only copied has no file, so
// there's nothing to open.
import qs
import QtQuick
import Qt.labs.folderlistmodel

Provider {
    id: provider

    readonly property url folder: "file://" + settings.folder

    // Newest first, so the first is the one to edit.
    FolderListModel {
        id: shots
        folder: provider.folder
        nameFilters: ["*.png"]
        showDirs: false
        sortField: FolderListModel.Time
    }

    readonly property var newest: shots.status === FolderListModel.Ready && shots.count > 0
        ? { path: shots.get(0, "filePath"), at: shots.get(0, "fileModified") } : null

    readonly property var editing: newest === null ? null : thing({
        key: "gimp:screenshot",
        verb: Words.verb("gimp:screenshot"),
        name: Words.name("gimp:screenshot"),
        keywords: "picture image mark up draw",
        glyph: String.fromCodePoint(0xf338),
        hue: "action",
        note: `taken at ${Qt.formatTime(newest.at, "HH:mm")}`,
        later: true,
        run: () => Niri.spawn(["gimp", newest.path])
    })

    // The folder is there from login (home.nix), so it's watched from the
    // start.
    things: editing ? [editing] : []
}

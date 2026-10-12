// The newest screenshot, opened in GIMP from the launcher: at home, just
// below the window in front, for a couple of minutes after it's taken,
// since opening the launcher then is when marking it up is wanted, and
// found by typing after that. Every
// screenshot lands in one folder, niri's own and the region ones the shell
// takes with grim alike, so the folder is watched rather than niri, which
// tells the shell only of its own. One that was only copied has no file, so
// there's nothing to open.
import qs
import QtQuick
import Qt.labs.folderlistmodel

Provider {
    id: provider

    // How long a new screenshot stays at home.
    readonly property int fresh: 2 * 60 * 1000

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
        verb: "edit",
        name: "Edit the screenshot in GIMP",
        keywords: "picture image mark up draw",
        glyph: String.fromCodePoint(0xf338),
        hue: "action",
        note: `taken at ${Qt.formatTime(newest.at, "HH:mm")}`,
        later: true,
        run: () => Niri.spawn(["gimp", newest.path])
    })

    things: editing ? [editing] : []

    // The folder is there from login (home.nix), so it's watched from the
    // start.
    function home() {
        return editing && Date.now() - newest.at.getTime() < fresh ? [editing] : [];
    }
}

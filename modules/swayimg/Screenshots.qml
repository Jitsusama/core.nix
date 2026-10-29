// The screenshots folder as swayimg's gallery, found by typing or in
// Capture, newest first and selected, for a screenshot taken a while ago.
// The newest one just taken is GIMP's row beside it; this is for going
// back through the rest.
// It's offered only while there's a picture to look at, since an empty
// folder would open nothing.
import qs
import QtQuick
import Qt.labs.folderlistmodel

Provider {
    id: provider

    FolderListModel {
        id: shots
        folder: "file://" + provider.settings.folder
        nameFilters: ["*.png"]
        showDirs: false
    }

    readonly property var looking: thing({
        key: "swayimg:screenshots",
        verb: Words.verb("swayimg:screenshots"),
        name: Words.name("swayimg:screenshots"),
        keywords: "pictures images gallery screenshot",
        glyph: programGlyph("swayimg"),
        hue: "action",
        note: shots.count === 1 ? "1 picture" : `${shots.count} pictures`,
        later: true,
        // Newest first, as the folder's names may not sort that way: niri's
        // and the shell's region pictures are named differently.
        run: () => Niri.spawn(["swayimg", "--gallery",
            "--execute=swayimg.imagelist.order = 'mtime'; swayimg.imagelist.reverse = true",
            settings.folder])
    })

    things: shots.status === FolderListModel.Ready && shots.count > 0 ? [looking] : []
}

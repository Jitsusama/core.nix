// The screenshots folder as swayimg's gallery, found by typing or among the
// picture viewer's own acts in Programs (`programActs`), newest first and
// selected, for a screenshot taken a while ago; Capture's Recent holds the
// newest ones taken.
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

    // The picture viewer's own acts in Programs: the screenshots' gallery.
    function programActs(program) {
        return program === "swayimg" ? provider.things : [];
    }
}

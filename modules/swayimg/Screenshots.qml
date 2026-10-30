// The screenshots folder as swayimg's gallery, found by typing or among the
// picture viewer's own acts in Programs (`programActs`), newest first and
// selected, for a screenshot taken a while ago; Capture's Recent holds the
// newest ones taken.
// It's offered only while there's a picture to look at, since an empty
// folder would open nothing.
import qs
import QtQuick

Provider {
    id: provider

    // How many pictures the folder holds, as the one reader of what was
    // taken says (Taken.qml), which is read as the launcher opens.
    readonly property int count: Taken.countIn(settings.folder, /\.png$/i)

    Component.onCompleted: Taken.want(settings.folder)

    readonly property var looking: thing({
        key: "swayimg:screenshots",
        verb: Words.verb("swayimg:screenshots"),
        name: Words.name("swayimg:screenshots"),
        keywords: "pictures images gallery screenshot",
        glyph: programGlyph("swayimg"),
        hue: "action",
        note: provider.count === 1 ? "1 picture" : `${provider.count} pictures`,
        later: true,
        // Newest first, as the folder's names may not sort that way: niri's
        // and the shell's region pictures are named differently.
        run: () => Niri.spawn(["swayimg", "--gallery",
            "--execute=swayimg.imagelist.order = 'mtime'; swayimg.imagelist.reverse = true",
            settings.folder])
    })

    things: provider.count > 0 ? [looking] : []

    // The picture viewer's own acts in Programs: the screenshots' gallery.
    function programActs(program) {
        return program === "swayimg" ? provider.things : [];
    }
}

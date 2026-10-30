// The newest screenshot, opened in GIMP from the launcher, found by typing
// or among GIMP's own acts in Programs (`programActs`), and Edit in GIMP
// on every picture Capture's Recent holds (`fileActs`). Every
// screenshot lands in one folder, niri's own and the region ones the shell
// takes with grim alike, so the folder is read rather than niri asked,
// which tells the shell only of its own. One that was only copied has no
// file, so there's nothing to open.
import qs
import QtQuick

Provider {
    id: provider

    // The newest picture in the folder, as the one reader of what was
    // taken says it (Taken.qml), which is read as the launcher opens.
    readonly property var newest: Taken.newestIn(settings.folder, /\.png$/i)

    Component.onCompleted: Taken.want(settings.folder)

    readonly property var editing: newest === null ? null : thing({
        key: "gimp:screenshot",
        verb: Words.verb("gimp:screenshot"),
        name: Words.name("gimp:screenshot"),
        keywords: "picture image mark up draw",
        glyph: String.fromCodePoint(0xf338),
        hue: "action",
        note: `taken at ${Qt.formatTime(new Date(newest.at), "HH:mm")}`,
        later: true,
        run: () => Niri.spawn(["gimp", newest.path])
    })

    // The folder is there from login (home.nix), so it's read from the
    // start.
    things: editing ? [editing] : []

    // GIMP's own acts in Programs: the newest screenshot to edit.
    function programActs(program) {
        return program === "gimp" ? provider.things : [];
    }

    // What GIMP does to a picture shown anywhere: edits it.
    function fileActs(path) {
        if (!/\.(png|jpe?g|webp)$/i.test(path))
            return [];
        const key = "verb:gimp:edit";
        return [thing({
            key: `${key}:${path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: String.fromCodePoint(0xf338),
            hue: "action",
            run: () => Niri.spawn(["gimp", path])
        })];
    }
}

// GIMP's act on the picture copied, in Programs › GIMP after its newest
// screenshot (programActs, which Applications.qml asks every provider
// that has it), found from the top by its name too (foundActs): New Image
// From the Copied Picture, there only while the clipboard holds a
// picture (Copied.qml), saying its size, type and weight where the
// clipboard's history has it, else its type alone. GIMP opens it as a
// picture of its own, from the file the history keeps it in
// (launcher/Copies.qml's `picture`), or, where the history doesn't hold
// it, from a file wl-paste writes it to in the runtime folder, named for
// when it was copied, as GIMP's title then says.
import qs
import Quickshell
import QtQuick
import "../Copied.js" as Copying

Provider {
    id: provider

    // The type the clipboard's picture is asked for, or "".
    readonly property string picture: Copied.picture
    // So Programs › GIMP, held while it's copied, draws its act.
    onPictureChanged: changed()

    foundActs: picture !== "" ? [{ program: "gimp", key: "verb:gimp:picture" }] : []

    // The clipboard's history's newest copy, when it's a picture, which is
    // what the clipboard holds while it holds a picture, or null.
    function kept() {
        const copies = (provider.launcher?.sources ?? [])
            .find(source => source.what === "copy" && Array.isArray(source.all)) ?? null;
        const newest = copies?.all[0] ?? null;
        return newest?.picture ? newest : null;
    }

    // What the picture is: its width and height, type and weight as the
    // history says them, or its type as the clipboard offers it.
    function said(newest) {
        if (newest === null)
            return Copying.pictureSaid(picture);
        const size = newest.across > 0 ? `${newest.across} \u00d7 ${newest.down} \u00b7 ` : "";
        return `${size}${newest.about}`;
    }

    function programActs(program) {
        if (program !== "gimp" || picture === "")
            return [];
        const key = "verb:gimp:picture";
        return [thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: said(kept()),
            run: () => provider.open()
        })];
    }

    // Opens what the clipboard holds as it runs, which a row found from
    // the top, made as the act came, may not have changed with.
    function open() {
        const newest = kept();
        const gimp = provider.settings.gimp ?? "gimp";
        if (newest !== null) {
            Niri.spawn([gimp, newest.picture]);
            return;
        }
        const type = Copied.picture;
        if (type === "")
            return;
        const runtime = Quickshell.env("XDG_RUNTIME_DIR") ?? "/tmp";
        const named = Qt.formatDateTime(new Date(), "yyyy-MM-dd HH.mm.ss");
        const file = `${runtime}/gimp/Copied ${named}.${Copying.extension(type)}`;
        // Through niri, as every program the shell opens, the picture
        // written first so GIMP opens it whole.
        Niri.spawn(["sh", "-c",
            "mkdir -p \"${2%/*}\" && \"$3\" --type \"$1\" > \"$2\" && exec \"$4\" \"$2\"",
            "sh", type, file, provider.settings.paste ?? "wl-paste", gimp]);
    }
}

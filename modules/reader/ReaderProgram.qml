// The reader's own act in Programs › Reader (programActs, which
// Applications.qml asks every provider that has it), found from the top
// by its name too (foundActs): Read the Copied Markdown, there only while
// the clipboard holds an existing markdown file's path, said beside it
// (Copied.qml), which it opens as a page. The reader takes a file alone,
// so a line copied after the path is said but not gone to.
import qs
import QtQuick
import "../Copied.js" as Copying

Provider {
    id: provider

    // The desktop entry the reader's program row is made from.
    readonly property string program: "reader"

    // The existing markdown file's path the clipboard holds, or null.
    readonly property var place: Copied.place !== null && Copying.markdown(Copied.place.path)
        ? Copied.place : null
    // So Programs › Reader, held while it's copied, draws its act.
    onPlaceChanged: changed()

    foundActs: place !== null ? [{ program: program, key: "verb:reader:markdown" }] : []

    function programActs(asked) {
        if (asked !== program || place === null)
            return [];
        const key = "verb:reader:markdown";
        return [thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: Copying.placeSaid(place, Files.home),
            // What's copied when it runs, which a row found from the top,
            // made as the act came, may not have changed with.
            run: () => {
                if (provider.place !== null)
                    Niri.spawn([provider.settings.reader ?? "reader", provider.place.path]);
            }
        })];
    }
}

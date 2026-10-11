// Emoji, a collection: what's typed finds them by name or meaning (Emoji.js
// says how), the best shows as one row with a count of the rest, and Return
// copies it. The list is Unicode's, with CLDR's English keywords, built at
// build time into `list` in the skin tone `tone` names, and read once,
// a moment after the shell starts, so no keystroke waits on it.
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Emoji.js" as Emoji

Provider {
    id: provider

    collection: ({
        noun: "emoji",
        title: "Emoji",
        glyph: String.fromCodePoint(0xf01f5),
        ask: "which emoji?"
    })
    examples: ["tada"]

    // Every emoji, in Unicode's order, read the first time it's needed.
    property var all: null
    // The last search, so a longer query looks only among what it found.
    property string lastTyped: ""
    property var lastFound: []

    function loaded() {
        if (all === null) {
            all = [];
            if (settings.list) {
                file.path = settings.list;
                const lines = read(file).split("\n");
                for (let i = 0; i < lines.length; i++)
                    if (lines[i] !== "")
                        all.push(Emoji.prepare(JSON.parse(lines[i]), all.length));
            }
        }
        return all;
    }

    // An emoji's row, made once.
    function row(e) {
        if (!e.row)
            e.row = member({
                key: `emoji:${e.name}`,
                verb: "copy",
                name: e.name,
                glyph: e.emoji,
                hue: "text",
                now: true,
                run: () => Quickshell.clipboardText = e.emoji
            });
        return e.row;
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const joined = words.join(" ");
        const among = lastTyped !== "" && joined.startsWith(lastTyped) ? lastFound : loaded();
        const found = Emoji.search(among, words);
        lastTyped = joined;
        lastFound = found;
        return { rows: found.map(row), grade: Emoji.grade(found, words) };
    }

    function everything() {
        return loaded().map(row);
    }

    Timer {
        interval: 2000
        running: true
        onTriggered: provider.loaded()
    }

    FileView {
        id: file
        blockLoading: true
        printErrors: false
    }
}

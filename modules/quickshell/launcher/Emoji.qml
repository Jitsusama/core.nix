// Emoji, a collection: what's typed finds them by name or meaning (Emoji.js
// says how), the best shows as one row with a count of the rest, and Return
// pastes it into the window the launcher opened over (Pasting.qml), or
// copies it with no window there; a step in are Paste and Copy. The list is Unicode's, with CLDR's English keywords, built at
// build time into `list` in the skin tone `tone` names, and read once,
// a moment after the shell starts, so no keystroke waits on it.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Emoji.js" as Emoji

Provider {
    id: provider

    what: "emoji"

    // Found only here, by Super+E or by its name: in the main search an
    // emoji named like what was typed ("turkey" for "tu") is a row to read
    // and rule out.
    collection: Words.collection("emoji", { searched: false })
    examples: ["emoji"]

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

    // An act inside an emoji's row, Paste or Copy.
    function act(id, run) {
        return thing({
            key: id,
            verb: Words.verb(id),
            name: Words.name(id),
            keywords: Words.keywords(id),
            glyph: Words.glyph(id),
            hue: "action",
            now: true,
            run: run
        });
    }

    // An emoji's row, made once. Its verb follows the window under, so the
    // foot says whether Return pastes or only copies.
    function row(e) {
        if (!e.row) {
            const copied = () => {
                Quickshell.clipboardText = e.emoji;
            };
            const made = member({
                key: `emoji:${e.name}`,
                name: e.name,
                glyph: e.emoji,
                hue: "text",
                now: true,
                run: () => Pasting.paste(copied),
                verbs: () => {
                    const copy = provider.act("verb:emoji:copy", copied);
                    return Pasting.under() === null ? [copy]
                        : [provider.act("verb:emoji:paste", () => Pasting.paste(copied)), copy];
                }
            });
            Object.defineProperty(made, "verb", {
                enumerable: true,
                get: () => Pasting.verb()
            });
            e.row = made;
        }
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

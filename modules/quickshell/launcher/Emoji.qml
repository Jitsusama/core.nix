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
            // Copied by the clipboard's history, given it as JSON text, or
            // by Qt while there's no history (Pasting.qml).
            const what = JSON.stringify(e.emoji);
            const unhelped = () => {
                Quickshell.clipboardText = e.emoji;
            };
            const copied = () => Pasting.copy(what, unhelped);
            const pasted = () => Pasting.paste(what, unhelped);
            const made = member({
                thing: true,
                key: `emoji:${e.name}`,
                // Capitalised, as every row's name is; learning keeps its key.
                name: e.name.charAt(0).toUpperCase() + e.name.slice(1),
                glyph: e.emoji,
                hue: "text",
                now: true,
                run: pasted,
                // What y takes, as plain fields rather than a row made
                // searchable, since y over hundreds asks every row
                // (Launcher.qml's actionsOf).
                operands: () => [{
                    key: "verb:emoji:copy",
                    verb: Words.verb("verb:emoji:copy"),
                    copies: e.emoji,
                    run: copied
                }],
                verbs: () => {
                    const copy = Object.assign(provider.act("verb:emoji:copy", copied),
                        { copies: e.emoji });
                    return Pasting.under() === null ? [copy]
                        : [provider.act("verb:emoji:paste", pasted), copy];
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

    // Every row, and their names, made once, since the list is read once:
    // Desktop's row for Emoji reads them for its count and its drawer on
    // every step, and making 1,924 again each time was a fifth of a step
    // into Desktop under load (2026-11-10).
    // A plain object, so making them notifies nothing.
    readonly property var made: ({ rows: null, names: null })
    function everything() {
        if (made.rows === null)
            made.rows = loaded().map(row);
        return made.rows;
    }
    function memberNames(held = everything()) {
        if (held !== made.rows)
            return held.map(one => one.name);
        if (made.names === null)
            made.names = made.rows.map(one => one.name);
        return made.names;
    }

    // Its rows are made here too, while the launcher is shut, since the
    // first step into Desktop made all 1,924 for Emoji's row and took
    // 207 ms under load (2026-11-10).
    Timer {
        interval: 2000
        running: true
        onTriggered: provider.everything()
    }

    FileView {
        id: file
        blockLoading: true
        printErrors: false
    }
}

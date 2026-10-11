// Every shortcut, a collection: each layer's chords from jitsusama.keys, top
// to bottom (the desktop, the terminal, the multiplexer inside it, the
// program inside that), found by what they do, each with its chord and the
// layer it belongs to in the footer. It's opened by its own chord, or by
// finding "Show these shortcuts", rather than adding a row to every search,
// since the desktop's actions are found there already, chords and all.
// Return on a desktop shortcut does what it does; one inside a program can
// only be shown.
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    collection: ({
        noun: "shortcuts",
        glyph: String.fromCodePoint(0xf097b),
        ask: "which shortcut?",
        searched: false
    })

    readonly property var places: ({
        desktop: "on the desktop",
        terminal: "in the terminal",
        multiplexer: "in the multiplexer",
        program: "in the program"
    })

    readonly property var all: Shortcuts.layers.reduce((rows, layer) => rows.concat(
        layer.binds.map(bind => member({
            key: `shortcut:${layer.name}:${bind.does}`,
            verb: bind.run.length > 0 ? "run" : "close",
            name: bind.does,
            glyph: layer.glyph || bind.glyph,
            hue: layer.name === "desktop" ? "action" : "program",
            chord: bind.chord,
            note: places[layer.name],
            // Acts on the screen, so it waits for the launcher to be gone.
            later: true,
            run: () => {
                if (bind.run.length > 0)
                    Niri.spawn(bind.run);
            }
        }))), [])

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ item: row.prepared, score: m.score, grade: m.grade, row: row });
        }
        const kept = Fuzzy.keep(found);
        return { rows: kept.map(f => f.row), grade: kept.length > 0 ? kept[0].grade : 0 };
    }

    function everything() {
        return all;
    }
}

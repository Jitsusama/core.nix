// Every shortcut, a collection: each layer's chords from jitsusama.keys, top
// to bottom (the desktop, the terminal, then Neovim and pi inside it), found
// by what they do or by their program's name, each with its chord, and in
// the footer the layer it belongs to and where in it the chord works. The touchscreen's swipes follow the
// desktop's, from jitsusama.swipes, each drawn as its fingers and its way,
// and found by "swipe" or "touch" as well. It's opened by its own chord, or by
// finding "Show these shortcuts", rather than adding a row to every search,
// since the desktop's actions are found there already, chords and all.
// Return on a desktop shortcut does what it does; one inside a program can
// only be shown.
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "shortcut"

    collection: ({
        noun: "shortcuts",
        title: "Shortcuts",
        words: ["hotkeys", "chords", "bindings", "keybindings", "key combos"],
        glyph: String.fromCodePoint(0xf097b),
        ask: "which shortcut?",
        searched: false
    })

    readonly property var places: ({
        desktop: "on the desktop",
        touchscreen: "on the touchscreen",
        terminal: "in the terminal",
        neovim: "in Neovim",
        pi: "in pi"
    })

    // The words that find a layer's shortcuts besides their names.
    readonly property var also: ({
        touchscreen: "swipe touch touchscreen gesture fingers",
        terminal: "terminal kitty",
        neovim: "neovim vim nvim editor",
        pi: "pi agent"
    })

    readonly property var all: Shortcuts.layers.reduce((rows, layer) => rows.concat(
        layer.binds.map(bind => member({
            key: `shortcut:${layer.name}:${bind.does}`,
            // What it does, by its first word, as the desktop's own are.
            verb: bind.run.length > 0 ? bind.does.split(" ")[0].toLowerCase() : "close",
            name: bind.does,
            words: also[layer.name] ? Fuzzy.prepare(`${bind.does} ${also[layer.name]}`) : null,
            glyph: layer.glyph || bind.glyph,
            hue: layer.name === "desktop" || layer.name === "touchscreen" ? "action" : "program",
            chord: bind.chord,
            note: bind.when ? `${places[layer.name]}, ${bind.when}` : places[layer.name],
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
            const m = matched(row, words, need);
            if (m.score >= 0)
                found.push({ item: row.prepared, score: m.score, grade: m.grade, row: row });
        }
        const kept = Fuzzy.keep(found);
        return { rows: kept.map(f => f.row), grade: kept.length > 0 ? kept[0].grade : 0 };
    }

    // How well a row answers what's typed: by its name, or failing that by
    // its layer's words too, which count for half.
    function matched(row, words, need) {
        const missed = { score: -1, grade: 3 };
        const name = (row.prepared.mask & need) === need
            ? Fuzzy.match(row.prepared, words, false) : missed;
        if (name.score >= 0 || row.words === null || (row.words.mask & need) !== need)
            return name;
        const beside = Fuzzy.match(row.words, words, false);
        return beside.score < 0 ? beside : { score: beside.score / 2, grade: beside.grade };
    }

    function everything() {
        return all;
    }
}

// Every shortcut, a collection, in levels, each flat (Joel, 2026-10-24:
// "it's hard to find the needles in the haystacks"). Its top holds each
// layer's chords from jitsusama.keys as a row, top to bottom: the desktop,
// the touchscreen's swipes from jitsusama.swipes, the terminal, then Neovim
// and pi inside it, pi's named Agents. Each says how many it holds after
// its name, and its first groups at its right edge, muted, as many as fit
// whole, since what follows a name is cut short at a third of the row
// (Choice.qml). A layer holds its groups, by what a chord acts on
// (KeyGroups.js), then any chord no group takes; a group holds its chords,
// or groups of its own, and a short layer holds its chords alone. Typing at any level finds a group below it by its name or
// its words, then any chord below it by what it does or by its program's
// name, flat. Each chord shows its chord, and in the footer the layer it
// belongs to and where in it the chord works; a swipe is drawn as its
// fingers and its way, and found by "swipe" or "touch" as well. It's opened
// by its own chord, or by finding "Show these shortcuts", rather than
// adding a row to every search, since the desktop's actions are found
// there already, chords and all. Return on a desktop shortcut does what it
// does; one inside a program can only be shown.
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "KeyGroups.js" as KeyGroups

Provider {
    id: provider

    what: "shortcut"

    collection: Words.collection("shortcuts", { searched: false })

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

    // How many letters of a level's first groups its row names, whole
    // names only, so they never run out of the room its right edge has.
    readonly property int room: 24
    // How much of a hidden word finds it, short of the whole word.
    readonly property int hiddenFrom: 4

    // A chord, as its row.
    function chordRow(layer, bind, glyph) {
        return member({
            key: `shortcut:${layer.name}:${bind.does}`,
            // What it does, by its first word, as the desktop's own are.
            verb: bind.run.length > 0 ? bind.does.split(" ")[0].toLowerCase() : "close",
            name: bind.does,
            words: also[layer.name] ? Fuzzy.prepare(`${bind.does} ${also[layer.name]}`) : null,
            glyph: glyph === "" ? bind.glyph : glyph,
            hue: layer.name === "desktop" || layer.name === "touchscreen" ? "action" : "program",
            chord: bind.chord,
            note: bind.when ? `${places[layer.name]}, ${bind.when}` : places[layer.name],
            // Acts on the screen, so it waits for the launcher to be gone.
            later: true,
            run: () => {
                if (bind.run.length > 0)
                    Niri.spawn(bind.run);
            }
        });
    }

    // A level as the launcher steps through a collection (its row's
    // `from`): with nothing typed its rows, and what's typed finding the
    // levels below it, then its chords.
    function level(rows, below, chords) {
        return {
            settings: provider.settings,
            collection: provider.collection,
            everything: () => rows,
            search: typed => provider.found(typed, below, chords),
            opened: () => {}
        };
    }

    // A layer or a group, by its word in the vocabulary (shortcuts:desktop,
    // shortcuts:neovim:github:pulls), and what's in it as KeyGroups.js
    // arranged it: its row, and every level below it. A group the
    // vocabulary doesn't know, one a bind named itself, is named by its id.
    function levelOf(id, arranged, places, chords, glyph) {
        const groups = arranged.groups.map(group => levelOf(`${id}:${group.id}`, group.inside,
            group.at, chords, glyph));
        const rows = groups.map(group => group.row).concat(arranged.loose.map(at => chords[at]));
        const held = places.map(at => chords[at]);
        const below = groups.reduce((all, group) => all.concat([group.row], group.below), []);
        const entry = Words.of(id);
        const last = id.split(":").pop();
        const name = Words.things[id] ? entry.name : last.charAt(0).toUpperCase() + last.slice(1);
        // Its first groups, as many as fit whole.
        let first = "";
        for (const group of groups) {
            const more = first === "" ? group.row.name : `${first}, ${group.row.name}`;
            if (more.length > room)
                break;
            first = more;
        }
        const row = member({
            key: `shortcut-level:${id}`,
            verb: "open",
            name: name,
            plain: true,
            // It holds more, so it wears its glyph dimmed.
            nested: true,
            glyph: Words.glyph(id) || glyph,
            hue: "voice",
            count: String(held.length),
            side: first,
            chip: name.toLowerCase(),
            ask: provider.collection?.ask ?? "which shortcut?",
            keywords: entry.words.length > 0 ? Fuzzy.prepare(entry.words.join(" ")) : null,
            hidden: entry.hidden,
            from: level(rows, below, held),
            verbs: () => rows
        });
        return { row: row, below: below, chords: held };
    }

    // Each layer this machine has, as a level.
    readonly property var layers: Shortcuts.layers.filter(layer => layer.binds.length > 0)
        .map(layer => {
            const id = `shortcuts:${layer.name}`;
            const glyph = layer.glyph === "" ? "" : (Words.glyph(id) || layer.glyph);
            const chords = layer.binds.map(bind => chordRow(layer, bind, glyph));
            return levelOf(id, KeyGroups.arrange(layer.name, layer.binds),
                chords.map((chord, at) => at), chords, glyph);
        })

    readonly property var all: layers.reduce((rows, layer) => rows.concat(layer.chords), [])

    // Every level, found by its name from the top, a group saying its
    // layer beside it, since the terminal and Neovim both have tabs.
    readonly property var levels: layers.reduce((rows, layer) => rows.concat([layer.row],
        layer.below.map(row => Object.assign({}, row, { side: layer.row.name }))), [])

    function search(typed) {
        return found(typed, levels, all);
    }

    // What's typed finds the levels named by it first, then the chords, each
    // best first.
    function found(typed, below, chords) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const plain = Fuzzy.plainly(typed);
        const named = [];
        for (const row of below) {
            const m = called(row, words, plain);
            if (m.score >= 0)
                named.push({ item: row.prepared, score: m.score, grade: m.grade, row: row });
        }
        const done = [];
        for (const row of chords) {
            const m = matched(row, words, need);
            if (m.score >= 0)
                done.push({ item: row.prepared, score: m.score, grade: m.grade, row: row });
        }
        const levelsKept = Fuzzy.keep(named);
        const chordsKept = Fuzzy.keep(done);
        const best = levelsKept.length > 0 ? levelsKept[0] : chordsKept[0];
        return {
            rows: levelsKept.concat(chordsKept).map(f => f.row),
            grade: best ? best.grade : 0
        };
    }

    // How well a level answers what's typed: by its name, or its words, or
    // a hidden word typed whole or by its first four letters or more.
    function called(row, words, plain) {
        const name = Fuzzy.match(row.prepared, words, false);
        if (name.score >= 0)
            return name;
        const beside = row.keywords ? Fuzzy.match(row.keywords, words, false) : name;
        if (beside.score >= 0)
            return beside;
        return hides(row, plain) ? { score: 1, grade: 0 } : beside;
    }

    function hides(row, plain) {
        return row.hidden.some(word => word === plain
            || (plain.length >= hiddenFrom && word.startsWith(plain)));
    }

    // How well a chord answers what's typed: by its name, or failing that
    // by its layer's words too, which count for half.
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
        return layers.map(layer => layer.row);
    }

    // Home counts every chord, not the layers.
    function counted() {
        return all.length === 0 ? "empty" : String(all.length).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    }
}

// Every shortcut, a collection, in levels, each flat (Joel, 2026-10-24:
// "it's hard to find the needles in the haystacks"). Its top holds each
// layer's chords from jitsusama.keys as a row, top to bottom: the desktop,
// the touchscreen's swipes from jitsusama.swipes, the terminal, then Neovim
// and pi inside it, pi's named Agents; but the layers of the window it
// opened over come first (Joel: "ideally context aware with the app
// open"), read once as it opens: kitty's the terminal's, and Neovim's or
// pi's before it when its title says one runs there, then the desktop's.
// Its own chord opens straight into the first of them, the path saying
// which, so its groups are already showing. Each says how many it holds after
// its name, and its first groups at its right edge, muted, as many as fit
// whole, since what follows a name is cut short at a third of the row
// (Choice.qml). A layer holds its groups, by what a chord acts on
// (KeyGroups.js), then any chord no group takes; a group holds its chords,
// or groups of its own, and a short layer holds its chords alone. Typing
// at any level finds a group below it by its name or its words, saying
// what holds it when that's further down, then any chord below it by what
// it does or by its program's name, flat. Each chord shows its chord, and in the footer the layer it
// belongs to and where in it the chord works; a swipe is drawn as its
// fingers and its way, and found by "swipe" or "touch" as well. It's opened
// by its own chord, or by finding "Show these shortcuts", rather than
// adding a row to every search, since the desktop's actions are found
// there already, chords and all. Return on a desktop shortcut does what it
// does; one inside a program can only be shown. One chord doing one thing
// in several places of a group is one row saying each (KeyGroups.js's
// merged).
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

    // A chord as words: its keys joined by +, those pressed one after
    // another a space apart, as the launcher's foot says them. One the key
    // map gives as a string is already words.
    function spelled(chord) {
        if (typeof chord === "string")
            return chord;
        return (chord ?? []).map((key, index) => index === 0 ? key.text
            : `${key.after === true ? " " : "+"}${key.text}`).join("");
    }

    // A chord, as its row.
    function chordRow(layer, bind, glyph) {
        return member({
            // One act in two places is two rows, each its own key.
            key: bind.when ? `shortcut:${layer.name}:${bind.does}:${bind.when}`
                : `shortcut:${layer.name}:${bind.does}`,
            // What it does, by its first word, as the desktop's own are.
            verb: bind.run.length > 0 ? bind.does.split(" ")[0].toLowerCase() : "close",
            name: bind.does,
            words: also[layer.name] ? Fuzzy.prepare(`${bind.does} ${also[layer.name]}`) : null,
            glyph: glyph === "" ? bind.glyph : glyph,
            hue: layer.name === "desktop" || layer.name === "touchscreen" ? "action" : "program",
            chord: bind.chord,
            note: [bind.when ? `${places[layer.name]}, ${bind.when}` : places[layer.name]]
                .concat((bind.also ?? []).length > 0
                    ? [`also ${bind.also.map(chord => spelled(chord)).join(", ")}`] : [])
                .join("; "),
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
    // shortcuts:neovim:pulls:threads), and what's in it as KeyGroups.js
    // arranged it: its row, every level below it as its own search finds
    // them, and as a search above it does. A group the vocabulary doesn't
    // know, one a bind named itself, is named by its id.
    function levelOf(id, arranged, places, chords, glyph) {
        const groups = arranged.groups.map(group => levelOf(`${id}:${group.id}`, group.inside,
            group.at, chords, glyph));
        const rows = groups.map(group => group.row).concat(arranged.loose.map(at => chords[at]));
        const held = places.map(at => chords[at]);
        const entry = Words.of(id);
        const last = id.split(":").pop();
        const name = Words.things[id] ? entry.name : last.charAt(0).toUpperCase() + last.slice(1);
        // Found inside this level, a group of its own says its first
        // groups, as it does listed; one further down says the group
        // holding it, since Windows, Columns, Workspaces and Screens each
        // have Focus. Found from further up, its own groups say this one.
        const deeper = groups.reduce((all, group) => all.concat(group.deeper), []);
        const below = groups.map(group => group.row).concat(deeper);
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
        const said = groups.map(group => Object.assign({}, group.row, { side: name }));
        return { row: row, below: below, deeper: said.concat(deeper), chords: held };
    }

    // Each layer this machine has, as a level.
    readonly property var layers: Shortcuts.layers.filter(layer => layer.binds.length > 0)
        .map(layer => {
            const id = `shortcuts:${layer.name}`;
            const glyph = layer.glyph === "" ? "" : (Words.glyph(id) || layer.glyph);
            const binds = KeyGroups.merged(layer.name, layer.binds);
            const chords = binds.map(bind => chordRow(layer, bind, glyph));
            return Object.assign({ name: layer.name }, levelOf(id,
                KeyGroups.arrange(layer.name, binds), chords.map((chord, at) => at), chords,
                glyph));
        })

    // The layers whose keys work in the window the launcher opened over, its
    // own first, by their names; only the desktop's over another program's
    // window, or none.
    property var leads: ["desktop"]

    // Read once as Shortcuts opens, from what niri last said, since the
    // window under the launcher can't change while it holds the keys.
    function opened() {
        const under = provider.launcher?.cameFrom() ?? null;
        leads = KeyGroups.over(under?.app_id ?? "", under?.title ?? "");
    }

    // The layers in the order the window opened over puts them, the first
    // saying it's that window's in the foot while it's selected.
    readonly property var ranked: {
        const own = leads.map(name => layers.find(layer => layer.name === name))
            .filter(layer => layer !== undefined);
        const rest = layers.filter(layer => !leads.includes(layer.name));
        if (own.length === 0 || own[0].name === "desktop")
            return own.concat(rest);
        const first = Object.assign({}, own[0], {
            row: Object.assign({}, own[0].row, { note: "the keys of the window you were in" })
        });
        return [first].concat(own.slice(1), rest);
    }

    // The chord opens on the program's own layer, its groups showing, the
    // path saying which program it is; over the desktop, on the layers.
    function leading() {
        return ranked.length > 0 && ranked[0].name !== "desktop" ? ranked[0].row : null;
    }

    readonly property var all: ranked.reduce((rows, layer) => rows.concat(layer.chords), [])

    // Every level, found by its name from the top, a group saying what
    // holds it beside it, since the terminal and Neovim both have tabs.
    readonly property var levels: ranked.reduce((rows, layer) => rows.concat([layer.row],
        layer.deeper), [])

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
        return ranked.map(layer => layer.row);
    }

    // Home counts every chord, not the layers.
    function counted() {
        return all.length === 0 ? "empty" : String(all.length).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    }

    // The window read as Shortcuts opened is that opening's alone: once the
    // launcher closes, a chord typed at the top next time is ranked as the
    // desktop's, with no layer saying it's the window's.
    Connections {
        target: provider.launcher
        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.leads = ["desktop"];
        }
    }
}

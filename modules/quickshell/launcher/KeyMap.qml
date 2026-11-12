// Every shortcut, a collection, in levels, each flat (Joel, 2026-10-24:
// "it's hard to find the needles in the haystacks"). Its top holds each
// layer's chords from jitsusama.keys as a row, top to bottom: the desktop,
// with the touchscreen's swipes from jitsusama.swipes, the terminal, then Neovim
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
import "KeyLayers.js" as KeyLayers

Provider {
    id: provider

    what: "shortcut"

    collection: Words.collection("shortcuts", { searched: false })

    readonly property var places: ({
        desktop: "on the desktop",
        terminal: "in the terminal",
        neovim: "in Neovim",
        pi: "in pi",
        // Google's programs, each a page in Chrome, whose own keys are
        // found beside Chrome's, so a chord found says whose it is.
        "google-mail": "in Mail",
        "google-calendar": "in Calendar",
        "google-meet": "in Meet",
        "google-docs": "in Docs",
        "google-sheets": "in Sheets",
        "google-slides": "in Slides",
        "google-drive": "in Drive"
    })

    // The words that find a layer's shortcuts besides their names.
    readonly property var also: ({
        terminal: "terminal kitty",
        neovim: "neovim vim nvim editor",
        pi: "pi agent",
        "google-mail": "gmail email",
        "google-calendar": "google calendar",
        "google-meet": "google meet call",
        "google-docs": "google docs document",
        "google-sheets": "google sheets spreadsheet",
        "google-slides": "google slides presentation",
        "google-drive": "google drive files"
    })

    // A chord as words: its keys joined by +, those pressed one after
    // another a space apart, as the launcher's foot says them. One the key
    // map gives as a string is already words.
    function spelled(chord) {
        if (typeof chord === "string")
            return chord;
        // A swipe says itself in words, as "3 fingers up".
        if ((chord ?? [])[0]?.written)
            return chord[0].written;
        return (chord ?? []).map((key, index) => index === 0 ? key.text
            : `${key.after === true ? " " : "+"}${key.text}`).join("");
    }

    // The words that find a chord's row besides its name: its layer's, and
    // a swipe's, for a row that a swipe does too (home.nix's swipes).
    readonly property string swiping: "swipe swipes touch touchscreen gesture gestures fingers"

    function wordsOf(layer, bind) {
        const swiped = [bind.chord].concat(bind.also ?? []).some(chord => !!(chord ?? [])[0]?.written);
        const extra = [also[layer.name] ?? "", swiped ? swiping : ""].filter(one => one !== "");
        return extra.length > 0 ? Fuzzy.prepare(`${bind.does} ${extra.join(" ")}`) : null;
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
            words: wordsOf(layer, bind),
            glyph: glyph === "" ? bind.glyph : glyph,
            hue: layer.name === "desktop" ? "action" : "program",
            chord: bind.chord,
            // Where it works; the other chords doing it are its drawer's,
            // which has room for them (VKCF3Z, 358703a5).
            note: workedIn(layer, bind),
            // A little more than its line (docs/design.md, Every row says a
            // little more beside it): its keys in words, every chord doing
            // it and where it works, made as it's shown. A thing, so the
            // launcher draws it, as Capture's acts are (Launcher.qml's
            // drawable): a drawer on a row that isn't one is never asked.
            thing: true,
            drawer: () => provider.drawerOf(layer, bind),
            // Acts on the screen, so it waits for the launcher to be gone.
            later: true,
            run: () => {
                if (bind.run.length > 0)
                    Niri.spawn(bind.run);
            }
        });
    }

    // Where a chord works, and when: a layer with no place of its own above
    // is in its program, by the name its row has ("in Processes").
    function workedIn(layer, bind) {
        const place = placeOf(layer);
        return bind.when ? `${place}, ${bind.when}` : place;
    }

    function placeOf(layer) {
        return places[layer.name] ?? `in ${Words.name(`shortcuts:${layer.name}`)}`;
    }

    // A chord as its row draws it (Chord.qml): its keys a + apart, those
    // pressed one after another a space, so a drawer writes it the same
    // way as the row beside it (VKCF3Z, 358703a5).
    function spaced(chord) {
        if (typeof chord === "string" || (chord ?? [])[0]?.written)
            return spelled(chord);
        return (chord ?? []).map((key, index) => index === 0 ? key.text
            : `${key.after === true ? " " : " + "}${key.text}`).join("");
    }

    // A chord's drawer: its keys as its row draws them, any other chord
    // doing the same, one a line, and when it works, where that narrows
    // the place its eyebrow names, as every drawer's names where it is and
    // what (VKCF3Z, 358703a5). The command a desktop one runs is raw, as a
    // path is, so it isn't a fact (VKCF3Z, b8b34908).
    function drawerOf(layer, bind) {
        const fact = (id, value) => ({ label: Words.name(`shortcuts:${id}`), value: value });
        const said = Words.of("shortcuts").said ?? {};
        const place = layer.name === "desktop" ? said.desktop ?? placeOf(layer) : placeOf(layer);
        // Each other way a fact of its own, the first named Also and the
        // rest under it unnamed, since a value beside its name keeps to
        // one line (Drawer.qml, besideOnOneLine).
        const also = (bind.also ?? []).map((chord, index) => ({
            label: index === 0 ? Words.name("shortcuts:also") : "",
            value: spaced(chord)
        }));
        return {
            eyebrow: (said.eyebrow ?? "{place}").replace("{place}", place),
            title: bind.does,
            facts: [fact("keys", spaced(bind.chord))].concat(also,
                [fact("where", bind.when ? workedIn(layer, bind) : "")])
                .filter(one => one.value !== "")
        };
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
        // What it takes to work, where something must be switched on first
        // (Mail's), said on its row wherever it stands, and on its page.
        const condition = entry.said?.note ?? "";
        const last = id.split(":").pop();
        const name = Words.things[id] ? entry.name : last.charAt(0).toUpperCase() + last.slice(1);
        // Found inside this level, a group of its own says its first
        // groups, as it does listed; one further down says the group
        // holding it, since Windows, Columns, Workspaces and Screens each
        // have Focus. Found from further up, its own groups say this one.
        const deeper = groups.reduce((all, group) => all.concat(group.deeper), []);
        const below = groups.map(group => group.row).concat(deeper);
        const row = member({
            key: `shortcut-level:${id}`,
            verb: "open",
            name: name,
            plain: true,
            // It holds more, so it wears its glyph dimmed.
            nested: true,
            glyph: Words.glyph(id) || glyph,
            hue: "voice",
            // It's a place, so nothing after its name: how many keys at its
            // right, where a row's figure stands (VKCF3Z, 2026-11-05), and
            // its groups by name in its drawer, as Recent's (the launcher's
            // heldNames; VKCF3Z, dae7fb67, and of a program's Shortcuts,
            // 2026-11-09).
            heldNames: groups.map(group => group.row.name),
            note: condition,
            standing: () => condition,
            side: held.length > 0 ? String(held.length) : "",
            chip: name.toLowerCase(),
            ask: provider.collection?.ask ?? "which shortcut?",
            from: level(rows, below, held),
            verbs: () => rows
        });
        const said = groups.map(group => Object.assign({}, group.row, { count: name }));
        return { row: row, below: below, deeper: said.concat(deeper), chords: held };
    }

    // Each layer this machine has, as a level.
    readonly property var layers: Shortcuts.layers.filter(layer => layer.binds.length > 0)
        .map(layer => {
            const id = `shortcuts:${layer.name}`;
            const glyph = layer.glyph === "" ? "" : (Words.glyph(id) || layer.glyph);
            const binds = KeyGroups.merged(layer.name, layer.binds);
            const chords = binds.map(bind => chordRow(layer, bind, glyph));
            const made = levelOf(id, KeyGroups.arrange(layer.name, binds),
                chords.map((chord, at) => at), chords, glyph);
            // Which layer it is, so a program holding it, named as it is,
            // stands for it at the top (Rank.js's keysHeld).
            made.row.layer = layer.name;
            return Object.assign({ name: layer.name }, made);
        })

    // The layers whose keys work in the window the launcher opened over, its
    // own first, by their names; only the desktop's over another program's
    // window, or none.
    property var leads: ["desktop"]

    // Read once as Shortcuts opens, from what niri last said, since the
    // window under the launcher can't change while it holds the keys. Kept
    // as it was when it names the same layers, so opening over the same
    // kind of window again rebuilds none of them.
    function opened() {
        const under = provider.launcher?.cameFrom() ?? null;
        const now = KeyLayers.over(under?.app_id ?? "", under?.title ?? "");
        if (now.length !== leads.length || now.some((name, at) => name !== leads[at]))
            leads = now;
    }

    // What its rows follow of the window the launcher opens over, the layers
    // whose keys work there, so Shortcuts kept shut is drawn again only over
    // a window with the same keys (Launcher.qml's keptUnder).
    function keptOver(window) {
        return KeyLayers.over(window?.app_id ?? "", window?.title ?? "").join(" ");
    }

    // The layers in the order the window opened over puts them, the first
    // saying it's that window's in the foot while it's selected.
    readonly property var ranked: {
        const own = leads.map(name => layers.find(layer => layer.name === name))
            .filter(layer => layer !== undefined);
        const rest = layers.filter(layer => !leads.includes(layer.name));
        if (own.length === 0 || own[0].name === "desktop")
            return own.concat(rest);
        // What it needs to work says more than that it leads, which its
        // place already says (VKCF3Z, 2026-11-07).
        const note = own[0].row.note || "the keys of the window you were in";
        const first = Object.assign({}, own[0], {
            row: Object.assign({}, own[0].row, { note: note })
        });
        return [first].concat(own.slice(1), rest);
    }

    // The chord opens on the program's own layer, its groups showing, the
    // path saying which program it is; over the desktop, on the layers.
    // The layers of programs that run inside the terminal, by their desktop
    // entries' ids, since their windows are the terminal's and say which
    // they are only by their titles.
    readonly property var terminalPrograms: ({ nvim: "neovim", btop: "btop" })

    // The layer a program's own keys are, by its desktop entry's id: one
    // run inside the terminal by its entry, any other by its windows, and
    // "desktop" for one with none of its own.
    function ofProgram(app) {
        return terminalPrograms[app] ?? KeyLayers.over(app, "")[0];
    }

    // A program's own keys, by its desktop entry's id, as the layer it has
    // here, or null for one with none of its own, so a program's row can
    // hold them (Applications.qml) without knowing which layer is whose.
    function layerOf(app) {
        const own = ofProgram(app);
        const layer = own === "desktop" ? null : ranked.find(one => one.name === own) ?? null;
        return layer === null ? null : layer.row;
    }

    function leading() {
        return ranked.length > 0 && ranked[0].name !== "desktop" ? ranked[0].row : null;
    }

    // Built once with push, since a reduce that concatenates copies the
    // list at every layer.
    readonly property var all: {
        const rows = [];
        for (const layer of ranked)
            for (const chord of layer.chords)
                rows.push(chord);
        return rows;
    }

    // Every level, found by its name from the top, a group saying what
    // holds it beside it, since the terminal and Neovim both have tabs.
    readonly property var levels: {
        const rows = [];
        for (const layer of ranked) {
            rows.push(layer.row);
            for (const deeper of layer.deeper)
                rows.push(deeper);
        }
        return rows;
    }

    function search(typed) {
        return found(typed, levels, all);
    }

    // What's typed finds the levels named by it first, then the chords, each
    // best first.
    function found(typed, below, chords) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const named = [];
        for (const row of below) {
            const m = called(row, words);
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

    // How well a level answers what's typed: by its name, its one word.
    function called(row, words) {
        return Fuzzy.match(row.prepared, words, false);
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
        return countedAs(all.length);
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

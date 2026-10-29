// The keyboard, a collection: the keyboards and Shortcuts first (its
// parts), then showing its layout, then every key on every layer (KeyboardLayout.qml),
// found by its cap or by what it types in words, each with its layer, where
// it sits in the footer, and what holding it does. Return on a key shows the
// layout on that key's layer, the key marked, so finding where a symbol lives
// is typing its name. The board's Layer and Lights are settings of the board
// itself, among the keyboards plugged in (Keyboards.qml), as every
// keyboard's parts are, and so is flashing it, since only the ZSA board can
// be; their choices, locking a layer or letting one go and lighting the
// board, and Flash, are still found here from anywhere by what's typed.
// With no layout, as on a machine without a keyboard's module, it holds
// nothing. Every name, verb and word is the vocabulary's (Words.qml).
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Keyboards.js" as Keyboards

Provider {
    id: provider

    what: "key"

    // One keyboard to Joel: its layout and keys here, the keyboards plugged
    // in and every shortcut its parts (Words.qml).
    collection: Words.collection("keyboard")

    // The overlay's chord, from the desktop's binds.
    readonly property var showing: Shortcuts.actions
        .find(action => action.run.join(" ") === "qs ipc call keyboard toggle") ?? { chord: [] }

    // An act on the board, named, done and found as the vocabulary says
    // (Words.qml), with what it fills into its name ({layer}). An act,
    // not a key, so it says no kind (docs/design.md).
    function act(id, key, fields, fill) {
        return member(Object.assign({
            key: key,
            what: "",
            verb: Words.verb(id),
            name: Words.name(id, "", fill),
            hue: "machine"
        }, fields));
    }

    readonly property var acts: !KeyboardLayout.present ? [] : [
        act("keyboard:show", "keyboard:show", {
            glyph: Theme.glyph.keyboard,
            chord: showing.chord,
            later: true,
            run: () => KeyboardLayout.toggle()
        })
    ]

    // Flashing the board, the ZSA board's own act (Keyboards.qml's
    // flashing), found here by what's typed.
    readonly property var flash: !KeyboardLayout.present ? [] : [
        act("keyboard:flash", "keyboard:flash", {
            glyph: Theme.glyph.flash,
            later: true,
            run: () => KeyboardLayout.flash()
        })
    ]

    // Locking the board on a layer or letting it go, found from anywhere
    // by what's typed; listed, they're the board's Layer's choices.
    readonly property var layers: !KeyboardLayout.present ? []
        : Keyboards.layerChoices(KeyboardLayout.layers.map(layer => layer.name),
            KeyboardLayer.name, KeyboardLayer.locked)
            .map(choice => act(choice.id, choice.key, {
                glyph: Theme.glyph.layer,
                run: () => KeyboardLayout.lock(choice.index)
            }, { layer: choice.layer }))

    // What can be done to the board's lights, found from anywhere by its
    // own name; listed, they're the board's Lights' choices.
    readonly property var lights: !KeyboardLayout.present ? [] : [
        act("keyboard:lights-off", "keyboard:lights-off", {
            glyph: Theme.glyph.brightness,
            run: () => KeyboardLayout.lights(false)
        }),
        act("keyboard:lights-on", "keyboard:lights-on", {
            glyph: Theme.glyph.brightness,
            run: () => KeyboardLayout.lights(true)
        }),
        act("keyboard:brighter", "keyboard:brighter", {
            glyph: Theme.glyph.brightness,
            run: () => KeyboardLayout.brighter(true)
        }),
        act("keyboard:dimmer", "keyboard:dimmer", {
            glyph: Theme.glyph.brightness,
            run: () => KeyboardLayout.brighter(false)
        })
    ]

    // Every key with something of its own on each layer: a key that falls
    // through is found on the layer it falls to.
    readonly property var keys: KeyboardLayout.layers.reduce((rows, layer, number) => rows.concat(
        layer.keys.map((key, index) => ({ key: key, index: index }))
            .filter(found => found.key.cap !== undefined)
            .map(found => member({
                key: `keyboard:key:${layer.name}:${found.index}`,
                verb: Words.verb("keyboard:key"),
                name: provider.named(found.key),
                side: found.key.holds ? `${layer.name}, held ${found.key.holds}` : layer.name,
                keywords: `${layer.name} ${found.key.holds ?? ""}`,
                note: found.key.where,
                glyph: Theme.glyph.keyboard,
                hue: "machine",
                later: true,
                run: () => KeyboardLayout.show(number, found.index)
            }))), [])

    // A key by what it is: a character by itself, and beside its words where
    // they say more ("\ backslash"), and a key whose cap is a word by its
    // words alone ("Lock the screen").
    function named(key) {
        if ([...key.cap].length > 1)
            return key.words.charAt(0).toUpperCase() + key.words.slice(1);
        return key.words === key.cap.toLowerCase() ? key.cap : `${key.cap} ${key.words}`;
    }

    readonly property var all: acts.concat(keys)
    // What a search looks through: the board's acts, the choices of its
    // Layer and Lights and its Flash, so what's typed reaches them without
    // a step in, and its keys.
    readonly property var pool: acts.concat(flash, layers, lights, keys)

    onAllChanged: changed()

    // A row's words, its name's and those beside it, made ready once.
    property var said: ({})

    function matched(row, words) {
        const name = Fuzzy.match(row.prepared, words, false);
        if (name.score >= 0 || !row.keywords)
            return name;
        const key = row.key + row.keywords;
        const prepared = said[key] ?? (said[key] = Fuzzy.prepare(`${row.name} ${row.keywords}`));
        const byWord = Fuzzy.match(prepared, words, false);
        // A keyword's match counts for less than the name's.
        return byWord.score < 0 ? byWord : { score: byWord.score / 2, grade: byWord.grade };
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const found = [];
        for (const row of pool) {
            const m = matched(row, words);
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

// The keyboard, a collection: what can be done to the board first (show its
// layout, flash it, lock a layer or let one go, and its Lights, a setting
// whose choices are a step in and found from anywhere), then every key
// on every layer (KeyboardLayout.qml), found by its cap or by what it types
// in words, each with its layer, where it sits in the footer, and what
// holding it does. Return on a key shows the layout on that key's layer,
// the key marked, so finding where a symbol lives is typing its name. With
// no layout, as on a machine without a keyboard's module, it holds nothing.
// Every name, verb and word is the vocabulary's (Words.qml).
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "key"

    // One keyboard to Joel: its layout and keys here, the keyboards plugged
    // in and every shortcut its parts (Words.qml).
    collection: Words.collection("keyboard")

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    // The overlay's chord, from the desktop's binds.
    readonly property var showing: Shortcuts.actions
        .find(action => action.run.join(" ") === "qs ipc call keyboard toggle") ?? { chord: [] }

    // An act on the board, named, done and found as the vocabulary says
    // (Words.qml), with what it fills into its name ({layer}). An act,
    // not a key, so it says no kind (docs/design.md).
    function act(id, key, fields, fill, more) {
        return member(Object.assign({
            key: key,
            what: "",
            verb: Words.verb(id),
            name: Words.name(id, "", fill),
            keywords: [more ?? "", Words.keywords(id)].join(" ").trim(),
            hue: "machine"
        }, fields));
    }

    readonly property var acts: !KeyboardLayout.present ? [] : [
        act("keyboard:show", "keyboard:show", {
            glyph: Theme.glyph.keyboard,
            chord: showing.chord,
            later: true,
            run: () => KeyboardLayout.toggle()
        }),
        act("keyboard:flash", "keyboard:flash", {
            glyph: glyph(0xf0241),
            later: true,
            run: () => KeyboardLayout.flash()
        })
    ].concat(KeyboardLayer.locked ? [act("keyboard:release", "keyboard:release", {
        glyph: glyph(0xf0329),
        run: () => KeyboardLayout.lock(0)
    }, { layer: KeyboardLayer.name })] : [], KeyboardLayout.layers.slice(1)
        .filter(layer => !(KeyboardLayer.locked && layer.name === KeyboardLayer.name))
        .map(layer => act("keyboard:lock", `keyboard:lock:${layer.name}`, {
            glyph: Theme.glyph.layer,
            run: () => KeyboardLayout.lock(KeyboardLayout.layers.indexOf(layer))
        }, { layer: layer.name })))

    // The board's lights, a setting with what can be done to them a step
    // in. The board can be told to change them but never asked how they
    // are, so the setting says nothing of how they're set. Each choice is
    // still found by its own words from anywhere ("brighten keyboard"),
    // the lights' own words beside them.
    readonly property var lights: !KeyboardLayout.present ? [] : [
        act("keyboard:lights-off", "keyboard:lights-off", {
            glyph: glyph(0xf0e4f),
            run: () => KeyboardLayout.lights(false)
        }, {}, Words.keywords("keyboard:lights")),
        act("keyboard:lights-on", "keyboard:lights-on", {
            glyph: glyph(0xf06e8),
            run: () => KeyboardLayout.lights(true)
        }, {}, Words.keywords("keyboard:lights")),
        act("keyboard:brighter", "keyboard:brighter", {
            glyph: Theme.glyph.brightness,
            run: () => KeyboardLayout.brighter(true)
        }, {}, Words.keywords("keyboard:lights")),
        act("keyboard:dimmer", "keyboard:dimmer", {
            glyph: glyph(0xf00de),
            run: () => KeyboardLayout.brighter(false)
        }, {}, Words.keywords("keyboard:lights"))
    ]

    readonly property var lit: !KeyboardLayout.present ? [] : [holderRow({
        key: "keyboard:lights",
        what: "",
        name: Words.name("keyboard:lights"),
        glyph: Theme.glyph.brightness,
        offers: true
    }, () => provider.lights)]

    readonly property var actions: acts.concat(lit)

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

    readonly property var all: actions.concat(keys)
    // What a search looks through: the lights' choices rather than the
    // setting holding them, so what's typed reaches them without a step in.
    readonly property var pool: acts.concat(lights, keys)

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

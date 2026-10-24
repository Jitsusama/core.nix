// The keyboard, a collection: what can be done to the board first (show its
// layout, flash it, lock a layer or let one go, its lights), then every key
// on every layer (KeyboardLayout.qml), found by its cap or by what it types
// in words, each with its layer, where it sits in the footer, and what
// holding it does. Return on a key shows the layout on that key's layer,
// the key marked, so finding where a symbol lives is typing its name. With
// no layout, as on a machine without a keyboard's module, it holds nothing.
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "key"

    collection: ({
        noun: "keys",
        title: "Keyboard",
        words: ["layout", "layers", "ergodox", "colemak", "keymap", "typing", "backlight"],
        glyph: glyph(0xf030c),
        ask: "which key?"
    })

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    // The overlay's chord, from the desktop's binds.
    readonly property var showing: Shortcuts.actions
        .find(action => action.run.join(" ") === "qs ipc call keyboard toggle") ?? { chord: [] }

    readonly property var actions: !KeyboardLayout.present ? [] : [
        member({
            key: "keyboard:show",
            verb: "show",
            name: "Show the keyboard's layout",
            keywords: "map layers overlay keys",
            glyph: glyph(0xf030c),
            hue: "machine",
            chord: showing.chord,
            later: true,
            run: () => KeyboardLayout.toggle()
        }),
        member({
            key: "keyboard:flash",
            verb: "flash",
            name: "Flash the keyboard",
            keywords: "firmware write update ergodox",
            glyph: glyph(0xf0241),
            hue: "machine",
            later: true,
            run: () => KeyboardLayout.flash()
        })
    ].concat(KeyboardLayer.locked ? [member({
        key: "keyboard:release",
        verb: "let go",
        name: `Let ${KeyboardLayer.name} go`,
        keywords: "unlock layer release",
        glyph: glyph(0xf0329),
        hue: "machine",
        run: () => KeyboardLayout.lock(0)
    })] : [], KeyboardLayout.layers.slice(1)
        .filter(layer => !(KeyboardLayer.locked && layer.name === KeyboardLayer.name))
        .map(layer => member({
            key: `keyboard:lock:${layer.name}`,
            verb: "lock",
            name: `Lock ${layer.name} on`,
            keywords: "layer hold keep",
            glyph: glyph(0xf0328),
            hue: "machine",
            run: () => KeyboardLayout.lock(KeyboardLayout.layers.indexOf(layer))
        })), [
        member({
            key: "keyboard:lights-off",
            verb: "turn off",
            name: "Turn off the keyboard's lights",
            keywords: "backlight leds dark",
            glyph: glyph(0xf0e4f),
            hue: "machine",
            run: () => KeyboardLayout.lights(false)
        }),
        member({
            key: "keyboard:lights-on",
            verb: "turn on",
            name: "Turn on the keyboard's lights",
            keywords: "backlight leds",
            glyph: glyph(0xf06e8),
            hue: "machine",
            run: () => KeyboardLayout.lights(true)
        }),
        member({
            key: "keyboard:brighter",
            verb: "brighten",
            name: "Brighten the keyboard's lights",
            keywords: "backlight leds brighter more",
            glyph: glyph(0xf00e0),
            hue: "machine",
            run: () => KeyboardLayout.brighter(true)
        }),
        member({
            key: "keyboard:dimmer",
            verb: "dim",
            name: "Dim the keyboard's lights",
            keywords: "backlight leds dimmer less",
            glyph: glyph(0xf00de),
            hue: "machine",
            run: () => KeyboardLayout.brighter(false)
        })
    ])

    // Every key with something of its own on each layer: a key that falls
    // through is found on the layer it falls to.
    readonly property var keys: KeyboardLayout.layers.reduce((rows, layer, number) => rows.concat(
        layer.keys.map((key, index) => ({ key: key, index: index }))
            .filter(found => found.key.cap !== undefined)
            .map(found => member({
                key: `keyboard:key:${layer.name}:${found.index}`,
                verb: "show",
                name: provider.named(found.key),
                side: found.key.holds ? `${layer.name}, held ${found.key.holds}` : layer.name,
                keywords: `${layer.name} ${found.key.holds ?? ""}`,
                note: found.key.where,
                glyph: glyph(0xf030c),
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
        for (const row of all) {
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

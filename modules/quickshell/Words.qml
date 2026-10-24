pragma Singleton
// The launcher's vocabulary (launcher/words.json, from
// jitsusama.quickshell.launcher.words): for every thing it knows, a
// collection, an answer, an act or a part, its name as shown, the categories
// it's in, the other words that find it, the words that ask its answer, its
// verb and the glyph it wears, so a name is said in one place and a check
// (tests/launcher-words.py) can say no word finds two things. A provider
// asks it by its own name, or a row by its key.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: words

    FileView {
        id: file
        path: Quickshell.shellPath("launcher/words.json")
        blockLoading: true
    }

    // As the file says it, or nothing, so a provider falls back to its own
    // id rather than failing to load.
    readonly property var all: {
        try {
            return JSON.parse(file.text()) ?? {};
        } catch (error) {
            return {};
        }
    }
    readonly property var categories: all.categories ?? []
    readonly property var commands: all.commands ?? []
    readonly property var things: {
        const out = {};
        for (const entry of all.things ?? [])
            out[entry.id] = entry;
        return out;
    }

    // A thing as it is in a state ("on", "off", "floating"), the state's
    // own name, verb, glyph and old names over its usual ones.
    function of(id, state) {
        const entry = things[id] ?? { id: id, name: id };
        const over = state ? (entry.states ?? {})[state] ?? {} : {};
        return Object.assign({
            words: [],
            hidden: [],
            parts: [],
            answers: [],
            verb: ""
        }, entry, over);
    }

    // Its name, with what's in braces filled in: "Sign in to {network}".
    function name(id, state, fill) {
        let said = of(id, state).name;
        for (const key of Object.keys(fill ?? {}))
            said = said.replace(`{${key}}`, fill[key]);
        return said;
    }

    function verb(id, state) {
        return of(id, state).verb;
    }

    // What finds it besides its name, as one string, as a row's keywords
    // are: its words and its hidden ones, an old name among them.
    function keywords(id, state) {
        const entry = of(id, state);
        return entry.words.concat(entry.hidden).join(" ");
    }

    // The words that ask its answer, typed whole, best first, as a
    // provider's questions are (Provider.qml).
    function answers(id) {
        return of(id).answers;
    }

    // Those of them whose start, three letters or more, asks it too: every
    // one, unless it says which, as the weather's "forecast" is only asked
    // whole, so "for" doesn't ask the weather.
    function started(id) {
        const entry = of(id);
        return entry.started ?? entry.answers;
    }

    // A typed command, by its id: its start, form and the verb Return does
    // it by.
    function command(id) {
        return commands.find(entry => entry.id === id) ?? { id: id, verb: "" };
    }

    // The glyph it wears, by the theme's name for it.
    function glyph(id, state) {
        return Theme.glyph[of(id, state).glyph ?? ""] ?? "";
    }

    // A provider's collection as the launcher reads one (Provider.qml),
    // what it says from here, and what changes as it goes (`extra`) from
    // the provider.
    function collection(id, extra) {
        const entry = of(id);
        return Object.assign({
            noun: entry.noun ?? entry.name.toLowerCase(),
            title: entry.name,
            words: entry.words.concat(entry.hidden),
            glyph: glyph(id),
            ask: entry.ask ?? "which?",
            empty: entry.empty ?? `No ${entry.noun ?? entry.name.toLowerCase()} yet.`
        }, extra ?? {});
    }
}

pragma Singleton
// The launcher's vocabulary (launcher/words.json, from
// jitsusama.quickshell.launcher.words): for every thing it knows, a
// collection, an answer, an act or a part, its name as shown, the place
// it's in, the words that ask its answer, its verb and the glyph it wears,
// one word each (docs/design.md), so a name is said in one place and a check
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
    // What home lists, top to bottom, each a category's id or a thing's:
    // Now first, in the one place it always has, then the rest by name.
    readonly property var home: all.home ?? []
    readonly property var commands: all.commands ?? []
    // The query line's own words: each mode's name, as the foot says it
    // while the mode isn't normal, and what each of its keys does.
    readonly property var field: all.field ?? {}
    // What a place asks that asks nothing in particular, which the
    // launcher's head leaves unsaid, since its mark and caret already say
    // "type" (VKCF3Z, 2026-11-06, one calm head).
    readonly property string anyAsk: "which?"
    readonly property var things: {
        const out = {};
        for (const entry of all.things ?? [])
            out[entry.id] = entry;
        return out;
    }

    // A thing as it is in a state ("on", "off", "floating"), the state's
    // own name, verb and glyph over its usual ones.
    function of(id, state) {
        const entry = things[id] ?? { id: id, name: id };
        const over = state ? (entry.states ?? {})[state] ?? {} : {};
        return Object.assign({
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

    // The key an act takes in normal mode on its row, one letter normal
    // doesn't already take (each-acts-key-is-free-where-its-pressed), or
    // "" where it has none (VKCF3Z, 2026-11-12).
    function actKey(id, state) {
        return of(id, state).key ?? "";
    }

    // What an action is to the key language, as words.json marks it: an
    // ending action, the one d does (the key language's act ending),
    // carries { act: "ending", ends } where `ends` is what the foot's ask
    // says it does, "close" or "forget". One that ends something in use
    // but isn't d's, as a disconnect, carries `ends` alone, so Return typed
    // in insert leaves it be too (Catalogue.js's ends); any other carries
    // nothing.
    // Each carries its command too, the key language's whole name for it
    // (Options.commands), whose flags say whether d asks first.
    function ending(id) {
        const entry = of(id);
        const command = (entry.command ?? "") !== "" ? { command: entry.command } : {};
        if (entry.act === "ending")
            return Object.assign({ act: entry.act, ends: entry.ends ?? "" }, command);
        return (entry.ends ?? "") !== "" ? Object.assign({ ends: entry.ends }, command) : {};
    }

    // The words that ask its answer, typed whole, best first, as a
    // provider's questions are (Provider.qml), then the names it had, so a
    // rename never loses it: "ram" still asks Memory.
    function answers(id) {
        const entry = of(id);
        return entry.answers.concat(olds(entry));
    }

    // Those of them whose start, three letters or more, asks it too: every
    // one, unless it says which, as the weather's "forecast" is only asked
    // whole, so "for" doesn't ask the weather. A name it had is among them
    // to be asked whole, and the launcher never asks one by its start
    // (Launcher.qml's oldWords).
    function started(id) {
        const entry = of(id);
        return (entry.started ?? entry.answers).concat(olds(entry));
    }

    // The names it had, as a question is typed.
    function olds(entry) {
        return (entry.once ?? []).map(old => old.toLowerCase());
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
        const noun = entry.noun ?? entry.name.toLowerCase();
        return Object.assign({
            noun: noun,
            // How it says how many it holds, as its own words say one and
            // many where the noun adds what its name doesn't ("78 keys" on
            // Keyboard), else the bare figure, since "Windows \u00b7 3
            // windows" says the row's name again (VKCF3Z, 2026-11-08).
            one: entry.said?.one ?? "{count}",
            many: entry.said?.many ?? "{count}",
            title: entry.name,
            glyph: glyph(id),
            ask: entry.ask ?? words.anyAsk,
            empty: entry.empty ?? `No ${noun} yet.`,
            // What home's row notes while it holds none: a lowercase run,
            // never the sentence an empty place says inside (VKCF3Z,
            // 2026-11-08: "none running", "nobody lately").
            none: entry.none ?? "none",
            // What its drawer says it holds, where they're of one kind, in a
            // sentence rather than by each name (VKCF3Z, 00ca9231).
            holding: entry.tells?.holding ?? ""
        }, extra ?? {});
    }
}

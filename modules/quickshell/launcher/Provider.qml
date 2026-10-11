// The shape of one kind of thing the launcher finds: windows, applications,
// actions, the session. Each kind is a file starting from this one, listed
// in jitsusama.quickshell.launcher.providers, so leaving one out of the list
// removes it whole, and a machine can add its own beside them:
//
//     Provider {
//         things: [thing({ key: "mine:hello", verb: "say", name: "Say hello",
//                          glyph: "…", hue: "voice", run: () => … })]
//     }
//
// A thing is a row: what learning knows it by (key), what Return does to it
// (verb), its name and what else finds it (keywords), its glyph and the
// theme's colour for its kind (hue), its chord or what it is (chord, side),
// what Return and Shift+Return run (run, below), and whether running it
// waits for the launcher to be gone (later).
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Scope {
    id: provider

    // Its settings from Nix, as the option gives them.
    property var settings: ({})
    // How its matches rank against the rest's, as a multiple of their score.
    property real weight: 1
    // The launcher, for what a thing needs of it: the lock, opening below.
    property var launcher: null
    property var things: []

    // An application's own glyph where the icon font has one, by its desktop
    // entry's id or a window's app id, with a machine's own from
    // `settings.glyphs`, as hex code points.
    readonly property var programGlyphs: Object.assign({
        "kitty": "f011b",
        "google-chrome": "f268",
        "slack": "f198",
        "nvim": "f36f",
        "1password": "f0881"
    }, settings.glyphs ?? {})

    function programGlyph(id) {
        const code = programGlyphs[(id ?? "").toLowerCase()] ?? "f0614";
        return String.fromCodePoint(parseInt(code, 16));
    }

    // A row with what's left unsaid filled in, its name and keywords
    // prepared for matching once rather than on every key.
    function thing(fields) {
        const made = Object.assign({
            chord: [],
            side: "",
            below: null,
            later: false
        }, fields);
        made.weight = provider.weight;
        made.prepared = Fuzzy.prepare(made.name);
        made.keywords = fields.keywords ? Fuzzy.prepare(fields.keywords) : null;
        return made;
    }
}

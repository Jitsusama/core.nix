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
// what Return and Shift+Return run (run, below), whether running it waits
// for the launcher to be gone (later) or happens while it still holds the
// keys (now, for the clipboard), where it is (context), and what else can
// be done to it (verbs, a function giving rows of the same shape), nested
// when those rows are things with verbs of their own, a level down. A verb
// that needs a target asks for one (ask) from the rows `targets` gives,
// each with a `note` for the footer, and is named on Return by `verb`.
// One that needs words instead, such as a password, asks for them (ask)
// in the field, dotted when `secret`, and hands them to `given`. A thing
// whose outcome takes a while `stays`, keeping the launcher open, and its
// `state()` tells the footer how it's going.
//
// A provider can answer rather than find: `answer(typed)` gives what's
// typed plainly asks for, { headline, copy, rows, note }, or null. The
// headline is drawn after what's typed and copied by Return; the rows,
// made with copyRow, carry the detail. One that has to ask the network
// answers at once with what it has, `asking` true while it waits, and says
// `answered` when more arrives, so the launcher asks again; `ask` keeps
// what a URL said for a while.
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Asking.js" as Asking

Scope {
    id: provider

    // Its settings from Nix, as the option gives them.
    property var settings: ({})
    // How its matches rank against the rest's, as a multiple of their score.
    property real weight: 1
    // The launcher, for what a thing needs of it: the lock, opening below.
    property var launcher: null
    property var things: []

    signal answered
    // A collection's things changed while it might be open.
    signal changed

    function answer(typed) {
        return null;
    }

    // What it puts on the home screen, with nothing typed, before the
    // likeliest picks: the windows' provider gives the focused window.
    function home() {
        return [];
    }

    // Things worth typing to try it, which the field suggests in turn at
    // home, one each time the launcher opens.
    property var examples: []

    // A provider with too many things to mix into the main search, such as
    // emoji, is a collection: { noun, title, glyph, ask }, its title naming it
    // on the home screen, `empty` saying what it says with nothing in it,
    // and searched false if it adds no row to a search, only opened by its
    // chord or from home. Its members stay out of the main list, though its
    // `things` don't. The launcher asks `search(typed)` for its matches, best
    // first, with how scattered the best is as Fuzzy.js grades a match,
    // { rows, grade }, and shows the best as one row with a count of the
    // rest, below the main search's matches as tidy and above any looser;
    // the right arrow steps into all of them, with what was typed kept, and
    // `everything()` gives them all once nothing is.
    property var collection: null

    function search(typed) {
        return { rows: [], grade: 0 };
    }

    function everything() {
        return [];
    }

    // What a collection's row at home says after its title: how many it
    // holds, grouped by thousands.
    function counted() {
        const size = everything().length;
        return size === 0 ? "empty" : String(size).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    }

    // A collection was stepped into, so all of it has been seen.
    function opened() {
    }

    // A collection's row: a thing whose name is made ready to match only
    // when it's first drawn, since a collection has thousands.
    function member(fields) {
        const made = Object.assign({
            chord: [],
            side: "",
            count: "",
            context: "",
            note: "",
            below: null,
            verbs: null,
            plain: false,
            later: false,
            now: false,
            keywords: null,
            weight: provider.weight
        }, fields);
        let prepared = null;
        Object.defineProperty(made, "prepared", {
            enumerable: true,
            get: () => prepared ?? (prepared = Fuzzy.prepare(made.name))
        });
        return made;
    }

    // A file's text as it is now. A FileView blocks only on its first load:
    // after that, a reload or a new path loads in the background and text()
    // still says what the last load read, unless the load is waited for.
    function read(file) {
        file.reload();
        file.waitForJob();
        return file.text();
    }

    // A row whose Return copies `copy`, or its name, set while the launcher
    // holds the keys, which a Wayland program needs to set the clipboard.
    // A row with an empty `copy` says something and copies nothing.
    function copyRow(fields) {
        const text = fields.copy ?? fields.name;
        return thing(Object.assign({
            verb: text === "" ? "close" : "copy",
            hue: "voice",
            plain: true,
            now: true,
            run: () => {
                if (text !== "")
                    Quickshell.clipboardText = text;
            }
        }, fields));
    }

    // What each URL said, and when: { pending, value, failed, at }.
    property var asked: ({})

    // What a URL said as JSON, asking it again once `fresh` milliseconds
    // have passed, and only then: the value it last gave, if any, comes
    // back at once, and `answered` says when a new one has arrived. A
    // failure is asked again after ten seconds.
    function ask(url, fresh) {
        const now = Date.now();
        const kept = asked[url];
        if (Asking.stands(kept, now, fresh, 10000))
            return kept;
        const entry = {
            pending: true,
            value: kept ? kept.value : undefined,
            failed: false,
            at: now
        };
        asked[url] = entry;
        const request = new XMLHttpRequest();
        request.onreadystatechange = () => {
            if (request.readyState !== XMLHttpRequest.DONE)
                return;
            entry.pending = false;
            entry.at = Date.now();
            entry.status = request.status;
            try {
                if (request.status !== 200)
                    throw request.status;
                entry.value = JSON.parse(request.responseText);
            } catch (e) {
                entry.failed = true;
            }
            provider.answered();
        };
        request.open("GET", url);
        request.send();
        return entry;
    }

    // An application's own glyph where the icon font has one, by its desktop
    // entry's id or a window's app id, with a machine's own from
    // `settings.glyphs`, as hex code points.
    readonly property var programGlyphs: Object.assign({
        "kitty": "f011b",
        "google-chrome": "f268",
        "slack": "f198",
        "nvim": "f36f",
        "1password": "f0881",
        "gimp": "f338",
        "org.inkscape.inkscape": "f33b",
        "spotify": "f1bc",
        "swayimg": "f02e9",
        "org.pwmt.zathura": "f0219"
    }, settings.glyphs ?? {})

    function programGlyph(id) {
        const code = programGlyphs[(id ?? "").toLowerCase()] ?? "f0614";
        return String.fromCodePoint(parseInt(code, 16));
    }

    // What a desktop entry runs, spelled out for niri to spawn: in kitty
    // when the entry asks for a terminal, and from its own directory when it
    // names one.
    function command(entry) {
        const program = entry.runInTerminal ? ["kitty"].concat(entry.command) : entry.command;
        if (entry.workingDirectory === "")
            return program;
        return ["sh", "-c", "cd -- \"$0\" && exec \"$@\"", entry.workingDirectory].concat(program);
    }

    // A row with what's left unsaid filled in, its name and keywords
    // prepared for matching once rather than on every key.
    function thing(fields) {
        const made = Object.assign({
            chord: [],
            side: "",
            count: "",
            context: "",
            note: "",
            below: null,
            verbs: null,
            plain: false,
            later: false,
            now: false
        }, fields);
        made.weight = provider.weight;
        made.prepared = Fuzzy.prepare(made.name);
        made.keywords = fields.keywords ? Fuzzy.prepare(fields.keywords) : null;
        return made;
    }
}

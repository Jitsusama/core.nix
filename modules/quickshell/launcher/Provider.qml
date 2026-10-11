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
// be done to it (verbs, a function giving rows of the same shape). A verb
// that needs a target asks for one (ask) from the rows `targets` gives,
// each with a `note` for the footer, and is named on Return by `verb`.
//
// A provider can answer rather than find: `answer(typed)` gives what's
// typed plainly asks for, { headline, copy, rows, note }, or null. The
// headline is drawn after what's typed and copied by Return; the rows,
// made with copyRow, carry the detail. One that has to ask the network
// answers at once with what it has and says `answered` when more arrives,
// so the launcher asks again; `ask` keeps what a URL said for a while.
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

    signal answered

    function answer(typed) {
        return null;
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
        if (kept && (kept.pending || now - kept.at < (kept.failed ? 10000 : fresh)))
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
        "1password": "f0881"
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

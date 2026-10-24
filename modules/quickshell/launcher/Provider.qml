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
// A thing is a row: what learning knows it by (key), what thing it is now
// (is, when that's not its key: `window:<niri id>` for a window, so a pi
// session's window, its Agents row and its quest are one row, the one
// with the highest `rich`), what Return does to it
// (verb), its name and what else finds it (keywords), its glyph and the
// theme's colour for its kind (hue), its chord or what it is (chord, side),
// what Return and Shift+Return run (run, below), whether running it waits
// for the launcher to be gone (later) or happens while it still holds the
// keys (now, for the clipboard), where it is (context), the collection it
// lives in as well, by its provider's name (inside), so a search that
// finds the collection whole lists it only there, and what else can
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
// headline is drawn after what's typed and copied by Return, or, for an
// answer that does something, done by its `run` under its `verb`; the rows,
// made with copyRow, carry the detail. One that has to ask the network
// answers at once with what it has, `asking` true while it waits, and says
// `answered` when more arrives, so the launcher asks again; `ask` keeps
// what a URL said for a while. One asked by words rather than a pattern
// lists them as its `questions`, best first, and answers each read
// plainly; the start of one, three letters or more, shows its answer too,
// asked by the first it starts, so a word half remembered still finds it.
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
    // The desktop's actions its rows stand for, by what they do (Shortcuts'
    // `does`), each row drawing the action's chord, so the action is listed
    // only here rather than twice under two names: the session's Lock is
    // Super+L's "Lock the screen".
    property var draws: []

    signal answered
    // A collection's things changed while it might be open.
    signal changed

    function answer(typed) {
        return null;
    }

    property var questions: []

    // What's typed, read as answers read it.
    function plainly(typed) {
        return Fuzzy.plainly(typed);
    }

    // Which of its questions what's typed asks, read plainly, or its
    // plural ("fans" asks "fan"), or "", so an answer asked by words
    // answers exactly the words its start is found by.
    function asks(typed) {
        const text = Fuzzy.plainly(typed);
        return questions.find(question => question === text || `${question}s` === text) ?? "";
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
    // on the home screen, `words` the others a person might type for it, by
    // which and by its title and noun a search goes straight to it whole,
    // `empty` saying what it says with nothing in it,
    // and searched false if it adds no row to a search, only opened by its
    // chord or from home. Its members stay out of the main list, though its
    // `things` don't. In the main search the launcher asks `gather(typed)`
    // for its matches, best first, with how scattered the best is as
    // Fuzzy.js grades a match, { rows, grade }, and shows the best as one row
    // with a count of the rest, below the main search's matches as tidy and
    // above any looser; the right arrow steps into all of them, with what
    // was typed kept, and `search(typed)` narrows them there. `everything()`
    // gives them all once nothing is typed.
    property var collection: null

    function search(typed) {
        return { rows: [], grade: 0 };
    }

    // Its row in the main search: its matches, unless it answered what's
    // typed, since its answer above them already lists them.
    function gather(typed) {
        return answer(typed) ? { rows: [], grade: 0 } : search(typed);
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

    // What one of its rows is, in a word, said beside the field while one
    // is selected so a completion says what sort of thing it is: "window",
    // "program", "emoji". An answer's rows are called answers wherever they
    // come from, so a provider that only answers leaves it empty.
    property string what: ""

    // A collection's row: a thing whose name is made ready to match only
    // when it's first drawn, since a collection has thousands.
    function member(fields) {
        const made = Object.assign({
            what: provider.what,
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
    // back at once, and `answered` says when a new one has arrived. One
    // that hasn't answered in `patience` milliseconds has failed, and a
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
            // A late answer, once it was given up on, changes nothing.
            if (request.readyState !== XMLHttpRequest.DONE || !entry.pending)
                return;
            delete provider.requests[url];
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
        requests[url] = request;
        request.open("GET", url);
        request.send();
        if (!deadline.running) {
            deadline.interval = patience;
            deadline.start();
        }
        return entry;
    }

    readonly property int patience: 5000
    // What's still being asked, by URL, to give up on.
    property var requests: ({})

    // Gives up on what's taken its patience, as failed, and waits for the
    // next one still being asked, so it runs only while something is.
    function giveUp() {
        const now = Date.now();
        let soonest = null;
        let gaveUp = false;
        for (const url of Object.keys(asked)) {
            const entry = asked[url];
            const left = Asking.overdueIn(entry, now, patience);
            if (left === 0) {
                entry.pending = false;
                entry.failed = true;
                entry.at = now;
                requests[url]?.abort();
                delete requests[url];
                gaveUp = true;
            } else if (left !== null && (soonest === null || left < soonest)) {
                soonest = left;
            }
        }
        if (soonest !== null) {
            deadline.interval = soonest;
            deadline.start();
        }
        if (gaveUp)
            provider.answered();
    }

    Timer {
        id: deadline
        onTriggered: {
            provider.giveUp();
        }
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

    // A window's title without its program's name trailing it, as Chrome
    // adds " - Google Chrome", since the glyph and the row already say
    // whose window it is.
    function tidy(title, program) {
        if (program === "")
            return title;
        for (const joint of [" - ", " \u2013 ", " \u2014 ", " | "])
            if (title.endsWith(joint + program))
                return title.slice(0, title.length - joint.length - program.length);
        return title;
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
            what: provider.what,
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
        // Its keywords beside its name, so a search can say some words of
        // each: "wifi off" finds Turn off Wi-Fi, "wifi" being a keyword.
        // A word of its name joined by a hyphen is found closed up too, as
        // it's typed: "wifi" finds Wi-Fi.
        const closed = Fuzzy.words(made.name).filter(word => word.includes("-"))
            .map(word => word.replace(/-/g, ""));
        made.keywords = fields.keywords || closed.length > 0
            ? Fuzzy.prepare([made.name].concat(closed, fields.keywords ? [fields.keywords] : [])
                .join(" "))
            : null;
        return made;
    }
}

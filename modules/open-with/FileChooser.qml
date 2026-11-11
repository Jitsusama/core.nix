// The files a program opens or saves, chosen in the launcher for the
// portal's chooser of files (open-with, src/files.rs), which asks here
// rather than GTK drawing a dialog of its own. The launcher opens in the
// folder the program asked for, else the folder of the file it saves over,
// else home, every folder on the way held above it, up to /, which has
// nothing above it (`outermost`). As in his oil, Return goes into a folder
// and - to the one holding it, the cursor on the folder it came from, and
// Ctrl+O and Ctrl+I go back and forward through the folders he's been in,
// each way there (]b, :cd, :e) one jump; h and l mean nothing on a row
// (Joel, 2026-11-05: "enter and - should be the only level stepping
// keys").
//
// A folder lists what it holds as it's stepped into, read then and again as
// it changes, never on a timer: the files opened lately first, where it
// opened, apart from the rest; then its folders, then its files, by name,
// each drawn as every file and folder is (Files.qml); then the program's
// filters and its choices, apart again. What's hidden shows once a "." is
// typed. Opening, Return on a file picks it, by the program's own word for
// it; where several may be opened, Return takes every file in a V
// selection, and :arga gathers files across folders, which Return then
// takes, :args listing them (VKCF3Z's vi-paradigm spec, The File Picker);
// every folder lists by name, change or size, folders first, as :sort
// sets for every ask after; asked for a folder, each folder leads with
// Open This Folder, one folder picked though the program could take
// several. Saving, the line holds the name, and a name already there asks
// once. :q, or closing the launcher, picks nothing; Escape closes nothing
// (Joel, 2026-11-05: "fuck escape quitting").
//
// Each file and folder says beside its name how big it is and when it last
// changed, "2.41 MB \u00b7 28 Oct", a folder how many items it shows, read
// with the folder it's in; one opened lately elsewhere says its folder
// instead. The one selected shows its plate in the drawer (FileFacts.js):
// its kind, name, a preview where one is cheap (a picture, a PDF's first
// page, a video's frame, a file's first lines or a folder's first items)
// and its facts, which draw at once, the preview coming once file-preview
// has said, one file asked at a time, the latest wanted next (VKCF3Z,
// 2026-11-03), of the one file-preview kept for the ask, so nothing is
// started for each file selected (Y2YBTS, 2026-11-03).
//
// Other providers give it places kept elsewhere, as Google's Drive is
// (modules/google), by `chooserPlaces(chooser)`: asked once as an ask is
// shown, so what's listed never changes while it's shown, and listed after
// home's files, apart, found by typing from any folder (VKCF3Z,
// 2026-10-29). Their files and folders say beside them and on their plates
// what one here would, from what their place listed (`entry`), with no
// preview. Their files are fetched before they're picked: the launcher
// stays, its foot saying so, and closes once every path is here, or stays
// saying why one couldn't be; gx opens one the same way, once fetched. Saving, the launcher's line holds the name
// to save as, a prompt's words rather than a find, the caret before its
// extension, kept from folder to folder; each folder, here or there whose
// place gives `saveTo`, leads with it, and a name already there (`names`)
// asks once, "Replace notes.md? (y/n)" (VKCF3Z's vi-paradigm spec, The
// File Picker). Return in a place there makes it the place said to save in, here,
// and then answers the path, which the place takes from there (VKCF3Z,
// 2026-10-30). An ask from inside the shell for a local folder alone
// (`local`) is given no places. A program's save staged there is marked
// with who asked, its app id as niri knows it in `.<name>.from` beside
// it, so the place can offer to go back to that program if the file
// can't be taken (427IH3, 2026-10-31). The folder here a save last went
// to is kept (`savedLastIn`), so a save asked from inside the shell can
// start there.
//
// The answer goes back on the socket open-with listens on, as one line:
// the paths picked, the filter in use and the option of each choice, or an
// empty line for none. Asks wait their turn, oldest first, as OpenWith.qml's
// do, each known by its socket.
pragma ComponentBehavior: Bound

import qs
import Quickshell
import Quickshell.Io
import QtQuick
import Qt.labs.folderlistmodel
import "../Fuzzy.js" as Fuzzy
import "../FileFacts.js" as FileFacts
import "Recents.js" as Recents

Provider {
    id: provider

    what: "file"

    // Who's waiting, oldest first: { reply, request }, and, for an ask from
    // inside the shell, picked, called with the paths in place of a socket.
    property var asks: []
    // How many asks have come from inside the shell, each one's key.
    property int shellAsked: 0
    readonly property var asking: asks.length > 0 ? asks[0] : null
    readonly property var request: asking?.request ?? ({})
    // The socket of the ask the launcher holds, which closing it refuses.
    property string shown: ""

    readonly property string mode: request.mode ?? "open"
    readonly property bool saving: mode === "save"
    readonly property bool folders: mode === "save-files" || request.folders === true
    readonly property string start: startOf(request)
    // The name a save is asked to save as, as the line holds it: the
    // program's, then as it's typed, kept from folder to folder.
    property string saveName: ""
    readonly property var filters: request.filters ?? []
    // The words a file's row and plate say (FileFacts.js), the launcher's,
    // and days and months as the locale names them, as the launcher says
    // when everywhere (Y2YBTS, 2026-11-03).
    readonly property var factWords: FileFacts.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))

    // What each folder read holds, by its path: { name, path, folder,
    // size, modified }, and each of those by its own path.
    property var listed: ({})
    property var known: ({})
    // How many items each folder read holds shows, by its path, and what
    // file-preview said of each file or folder selected (false where it
    // couldn't), with the change time it was asked at.
    property var counts: ({})
    property var previews: ({})
    property var askedAt: ({})
    // The file whose preview is wanted next, while another is asked, the
    // folders still to count, and what file-preview is answering now.
    property string previewNext: ""
    property var toCount: []
    property var outstanding: null
    // The folders stepped into, read beside those on the way to the start.
    property var stepped: []
    // A ]b or [b waiting on the folder holding the one it was pressed in to
    // be read: that folder's path and which way, or nothing.
    property var besideNext: null
    readonly property var reading: asking === null ? [] : Array.from(new Set(["/"]
        .concat(onTheWay(start), stepped)))
    // The filter picked here, else the program's, and each choice's option.
    property int filterAt: -1
    readonly property int inUse: filters.length === 0 ? -1
        : filterAt >= 0 ? filterAt : Math.min(request.filter ?? 0, filters.length - 1)
    property var picked: ({})
    // The files opened lately, newest first, each once.
    property var lately: []
    // Where several files may be opened at once, each file is a switch,
    // Marked, and these are the ones marked, in the order they were.
    readonly property bool several: request.multiple === true && mode === "open" && !folders
    property var marked: []
    // The places other providers give, asked as the ask is shown, and what
    // each of their files says of itself, by its key, fetch included.
    property var places: []
    property var elsewhere: ({})
    // Fetching what's kept elsewhere: how many are asked, how many are
    // here, and why one couldn't be, or nothing while none is.
    property var fetching: null
    // Saving into a place kept elsewhere: the ask, the path and who asked,
    // while the folder the path is in is made.
    property var staging: null
    // The folder here a save last went to, kept in a file (`savedIn`) so
    // a restart keeps it, or "" before any.
    property string savedLast: ""
    // The order every folder lists in, one for every ask, kept in a file
    // (`orderIn`) so a restart keeps it; it starts on name (VKCF3Z,
    // 2026-11-04).
    readonly property var orders: ["name", "changed", "size"]
    property string order: "name"
    // Whether that order is turned round, as :sort! turns Vim's lines,
    // kept with it.
    property bool reversed: false
    // Whether a folder lists its hidden files, as :set hidden says, until
    // :set nohidden.
    property bool showsHidden: false
    // Each kind of file by how it's named, and the kinds each is one of.
    property var kinds: ({ names: {}, endings: {}, broader: {} })

    collection: asking === null ? null : ({
        noun: folders ? "folders" : "files",
        title: request.title || (saving || mode === "save-files" ? "Save" : "Open"),
        searched: false,
        glyph: Theme.glyph[folders ? "folder" : "path"],
        // A dialog, it has nothing above it: - at / says so, rather than
        // leaving the ask waiting for a home it never showed.
        outermost: true,
        // What it lists is /, so the path says so there, "Open › /", as it
        // says "~/Downloads" a folder below (VKCF3Z, 2026-11-05).
        place: Files.shown("/"),
        ask: question(),
        empty: "Nothing here."
    })

    // What the field asks in every folder of the ask, so he sees there
    // whether one file is taken or several: "open which file?", "open
    // which files?", "open which folder?", in the program's own word where
    // it gives one, "attach which files?" (VKCF3Z, 2026-11-04).
    function question() {
        const said = Words.of("file-chooser:question").said ?? {};
        if (mode === "save-files")
            return said.them ?? "";
        if (saving)
            return (request.name ?? "") === "" ? said.save ?? ""
                : (said.named ?? "").replace("{name}", request.name);
        const form = folders ? "folder" : several ? "files" : "file";
        return (said[form] ?? "").replace("{verb}",
            word(folders ? "verb:file-chooser:folder" : "verb:path:file:open"));
    }

    // The folder to start in: the program's, else the one holding the file
    // it saves over, else home.
    function startOf(asked) {
        const folder = String(asked.folder ?? "");
        if (folder.startsWith("/"))
            return folder.replace(/(.)\/+$/, "$1");
        const current = String(asked.current ?? "");
        if (current.startsWith("/"))
            return parentOf(current);
        return Files.home;
    }

    function parentOf(path) {
        const cut = path.lastIndexOf("/");
        return cut <= 0 ? "/" : path.slice(0, cut);
    }

    // Each folder from / down to this one, / left out: /home, /home/joel.
    function onTheWay(path) {
        const parts = path.split("/").filter(part => part !== "");
        return parts.map((_, at) => `/${parts.slice(0, at + 1).join("/")}`);
    }

    // The program's word for Return, without its mnemonic: "_Open" is Open.
    readonly property string accept: String(request.accept ?? "").replace(/_/g, "").trim()

    function word(id) {
        return accept !== "" ? accept.split(" ")[0].toLowerCase() : Words.verb(id);
    }

    function named(id) {
        return accept !== "" ? accept : Words.name(id);
    }

    // What a folder lists: the files opened lately, where it opened, then
    // its folders and files, then the filters and choices.
    function rowsOf(path) {
        const all = listed[path] ?? [];
        const held = heldKeys();
        const hidden = entry => !showsHidden && entry.name.startsWith(".")
            && !held.includes(`folder:${entry.path}`)
            && !(start === entry.path || start.startsWith(`${entry.path}/`));
        const shown = FileFacts.ordered(all.filter(entry => !hidden(entry)), order, counts,
            reversed);
        const own = shown.filter(entry => entry.folder).map(entry => folderRow(entry.path))
            .concat(folders ? [] : shown.filter(entry => !entry.folder && passes(entry.name))
                .map(entry => fileRow(entry.path)));
        // The name the line holds is the line's own row (naming), so a file
        // it already names isn't listed again below.
        const lead = saving && path !== "/" ? savingRow(path, provider.saveName) : null;
        const named = lead ? fileRow(lead.path).key : "";
        // The filters and choices wait for the folder's read as its Copy
        // the Path does (folderRow), to come after its files. What was
        // opened lately keeps its own order, apart and on top.
        return apart([path === start ? recent(path) : [],
            own.filter(row => row.key !== named), path === Files.home ? places.slice() : [],
            path in listed ? options() : []]);
    }

    // Groups one after another, each set apart from the one before it by a
    // gap, as Now is at home.
    // The gap goes on a copy, what reads as it's drawn kept, since a
    // place's rows are made once and shown elsewhere too.
    function apart(groups) {
        const rows = [];
        for (const group of groups) {
            if (group.length > 0 && rows.length > 0)
                group[0] = Object.assign(copied(group[0]), { gap: true });
            rows.push(...group);
        }
        return rows;
    }

    function copied(row) {
        const copy = {};
        for (const name of Object.getOwnPropertyNames(row))
            Object.defineProperty(copy, name, Object.getOwnPropertyDescriptor(row, name));
        return copy;
    }

    // The files opened lately the filter passes, or their folders where a
    // folder is asked for or saved in, nine at most, none of them in the
    // folder shown, since it lists them itself.
    function recent(path) {
        const elsewhere = each => parentOf(each) !== path && each !== path;
        if (!folders && !saving)
            return lately.filter(each => elsewhere(each) && passes(each.slice(each.lastIndexOf("/") + 1)))
                .slice(0, 9).map(each => fileRow(each, true));
        return Array.from(new Set(lately.map(each => parentOf(each)))).filter(elsewhere)
            .slice(0, 9).map(each => folderRow(each, true));
    }

    // A folder's row, stepped into to what it holds, led by Open This
    // Folder where one is asked for.
    //
    // Held, each folder on the way is titled as the one held deepest, read
    // as the launcher says its path, so the path says the program's title
    // and then one folder, "~/Downloads", which changes in place as - goes
    // up, rather than a crumb a folder (VKCF3Z, 2026-10-29).
    function folderRow(path, away) {
        const acts = !folders ? [] : [act(mode === "save-files" ? "verb:file-chooser:here"
            : "verb:file-chooser:folder", path, [path])];
        const row = Object.assign(Files.folder(path, thing, acts, () => rowsOf(readIn(path))), {
            empty: "Nothing here.",
            typed: letters => typedIn(path, letters),
            beside: by => provider.beside(path, by),
            commands: () => provider.commandsIn(path),
            standing: () => provider.gatheredSaid(),
            takes: rows => provider.takes(rows)
        }, saving ? naming(() => rowsOf(readIn(path)),
            () => path !== "/" ? savingRow(path, provider.saveName) : null) : {});
        // Until the folder's read, it holds no Copy the Path: rows shown
        // keep their place and those that come go below them, so a Copy
        // shown before its files were read headed the list for good, where
        // it goes last (VKCF3Z, 2026-11-03).
        const whole = row.verbs;
        const held = () => path in provider.listed ? whole()
            : whole().filter(one => !String(one.key).startsWith("verb:path:folder:copy:"));
        Object.assign(row, { verbs: held, targets: held });
        Object.defineProperties(row, {
            title: { get: () => provider.titled(Files.shown(path)), enumerable: true },
            ask: { get: () => provider.question(), enumerable: true }
        });
        return described(row, path, away === true);
    }

    // What a folder a save is asked in gives the launcher's line: the name
    // it saves as, a prompt's words rather than a find, kept from folder to
    // folder, the caret starting before its extension, and every row it
    // holds, never narrowed by it (VKCF3Z's vi-paradigm spec, The File
    // Picker). The name is the line's own row, the one Return stands on
    // above the rows, as an answer's headline is, rather than a row
    // saying the name again below it (VKCF3Z, #1812).
    function naming(rows, line) {
        return {
            line: line,
            words: () => provider.saveName,
            worded: letters => {
                provider.saveName = letters;
            },
            caretIn: words => FileFacts.nameEnd(words),
            finds: () => rows()
        };
    }

    // A file's or a folder's row says its size, or how many items it
    // shows, and when it changed, as it's drawn, once the folder it's in
    // is read; one listed away from its folder keeps saying where it is.
    // Its drawer is its plate, asked for as it's shown. The row is made
    // again with its side read as it's drawn, rather than its side
    // redefined in place: Qt's engine loses the value of a property set
    // just before one is turned from a value into a getter.
    function described(row, path, away) {
        const made = {};
        for (const name of Object.getOwnPropertyNames(row)) {
            if (away || name !== "side")
                Object.defineProperty(made, name, Object.getOwnPropertyDescriptor(row, name));
        }
        if (!away)
            Object.defineProperty(made, "side", {
                get: () => {
                    const entry = provider.known[path];
                    return entry ? FileFacts.side(entry, provider.counts[path], Date.now(),
                        provider.factWords)
                        : Files.within(path);
                },
                enumerable: true,
                configurable: true
            });
        // One listed in its folder is named as the folder holds it, so the
        // store's entries keep their hashes and stay apart (Files.named).
        if (!away)
            made.name = Files.named(path, true);
        made.drawer = () => provider.plateOf(path);
        return made;
    }

    // The plate the drawer shows for a file or a folder: its facts from
    // the folder it's in at once, and what file-preview says once it has.
    // One that changed since it was asked is asked again.
    function plateOf(path) {
        const entry = known[path] ?? null;
        const said = previews[path];
        if (settings.preview && (said === undefined || entry && askedAt[path] !== entry.modified))
            Qt.callLater(provider.preview, path);
        const file = { path: path, name: Files.named(path, true), where: Files.within(path),
            entry: entry };
        return FileFacts.plate(file, said || null, factWords);
    }

    // Wants file-preview's word on a file, asked once it's answered what
    // it's asked now, in place of any wanted before it.
    function preview(path) {
        const entry = known[path] ?? null;
        if (asking === null || path in previews && (!entry || askedAt[path] === entry.modified))
            return;
        previewNext = path;
        askNext();
    }

    // Wants what each folder inside one just read shows counted, so a
    // folder's row can say it.
    function count(path) {
        if (!settings.preview || asking === null || toCount.includes(path))
            return;
        toCount = toCount.concat([path]);
        askNext();
    }

    // Asks file-preview one thing at a time, the file wanted first, then
    // the folders to count, starting it again if it's stopped. A path
    // with a line end in it can't be asked as a line, so it's said to
    // have nothing to show.
    function askNext() {
        if (!settings.preview || asking === null || outstanding !== null)
            return;
        if (!previewer.ready) {
            previewer.running = true;
            return;
        }
        if (previewNext !== "") {
            const path = previewNext;
            previewNext = "";
            askedAt = Object.assign({}, askedAt, { [path]: known[path]?.modified });
            put("show", path);
        } else if (toCount.length > 0) {
            const path = toCount[0];
            toCount = toCount.slice(1);
            put("items", path);
        }
    }

    function put(verb, path) {
        outstanding = { verb: verb, path: path };
        if (path.includes("\n"))
            Qt.callLater(provider.previewerSaid, "");
        else
            previewer.write(`${verb} ${path}\n`);
    }

    // file-preview's answer to what it was asked, nothing where it
    // couldn't say, then the next thing to ask.
    function previewerSaid(line) {
        const was = outstanding;
        outstanding = null;
        if (was === null || asking === null)
            return;
        let said = false;
        try {
            said = JSON.parse(line);
        } catch (error) {
            said = false;
        }
        if (said && "error" in said)
            said = false;
        if (was.verb === "show") {
            previews = Object.assign({}, previews, { [was.path]: said });
        } else if (said) {
            counts = Object.assign({}, counts, said);
            changed();
        }
        Qt.callLater(provider.askNext);
    }

    // One file-preview for an ask, started as the first thing is wanted of
    // it, its folders read as it waits, and stopped as the ask ends
    // (forget), asked a line at a time, so nothing is started for each
    // file selected or folder read.
    // One that stops is started again for what's wanted next; one that
    // stops before answering anything is given up on for what's wanted,
    // so a file-preview that can't start isn't started again and again.
    Process {
        id: previewer

        property bool ready: false
        property bool answered: false

        command: (provider.settings.preview ?? []).concat(["serve"])
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => {
                previewer.answered = true;
                provider.previewerSaid(line);
            }
        }
        onStarted: {
            ready = true;
            answered = false;
            Qt.callLater(provider.askNext);
        }
        onExited: {
            const gaveUp = !answered;
            ready = false;
            if (provider.outstanding !== null)
                provider.previewerSaid("");
            if (gaveUp) {
                provider.previewNext = "";
                provider.toCount = [];
            }
            Qt.callLater(provider.askNext);
        }
    }

    // What every folder held says, the one held deepest: a folder here by
    // its path, one kept elsewhere by its name.
    function titled(own) {
        const keys = heldKeys().filter(key => key.startsWith("folder:") || elsewhere[key]?.folder);
        const deepest = keys.length > 0 ? keys[keys.length - 1] : "";
        if (deepest === "")
            return own;
        return deepest.startsWith("folder:") ? Files.shown(deepest.slice("folder:".length))
            : elsewhere[deepest].name;
    }

    // What a place gives the chooser to make its rows with, so they're
    // drawn, picked and marked as a folder's here are, and `changed`, said
    // once what a folder holds has come, so every folder held is asked
    // again.
    function chooser() {
        return {
            saving: saving || mode === "save-files",
            local: request.local === true,
            folders: folders,
            changed: () => provider.changed(),
            wants: (name, mime) => provider.wants(name, mime),
            folder: fields => provider.folderElsewhere(fields),
            file: fields => provider.fileElsewhere(fields)
        };
    }

    function placesFor() {
        const asked = chooser();
        if (asked.local)
            return [];
        return (launcher?.sources ?? [])
            .filter(other => other !== provider && typeof other.chooserPlaces === "function")
            .reduce((rows, other) => rows.concat(other.chooserPlaces(asked)), []);
    }

    // A folder kept elsewhere: stepped into to what it holds, saying the
    // one held deepest, as a folder here does. While it holds nothing it
    // says what its place says (`empty`), as "Listing it." while it's
    // asked for, else that there's nothing here.
    function folderElsewhere(fields) {
        elsewhere[fields.key] = { name: fields.name, folder: true };
        const saves = saving && typeof fields.saveTo === "function";
        const inside = () => fields.inside();
        const drawn = factsElsewhere(fields);
        const row = Object.assign(Files.elsewhere("folder", drawn, thing, [], inside),
            saves ? naming(inside, () => provider.namingElsewhere(fields, provider.saveName))
                : {});
        Object.defineProperties(row, {
            title: { get: () => provider.titled(fields.name), enumerable: true },
            ask: { get: () => provider.question(), enumerable: true },
            empty: { get: () => fields.empty?.() || "Nothing here.", enumerable: true }
        });
        return row;
    }

    // The row Return saves to in a folder kept elsewhere: the name given,
    // which, a file already there by that name, asks once first.
    function namingElsewhere(fields, name) {
        const trimmed = name.trim();
        if (trimmed === "" || trimmed === "." || trimmed === ".." || trimmed.includes("/"))
            return null;
        const path = fields.saveTo(trimmed);
        const taken = (fields.names?.() ?? []).includes(trimmed);
        const row = thing({
            key: `file-chooser:save:${path}`,
            name: trimmed,
            verb: accept !== "" ? word("") : "save",
            glyph: Words.glyph("path:file"),
            hue: "program",
            side: fields.name,
            names: true,
            stays: true,
            run: () => provider.replacing(taken, trimmed, () => provider.stage(path))
        });
        return gotten(row, { note: () => provider.fetchingSaid() });
    }

    // Makes the folder a path kept elsewhere is saved in, then answers the
    // path and closes the launcher; one that can't be made keeps the
    // launcher, saying so.
    function stage(path) {
        if (asking === null || staging !== null)
            return;
        staging = { reply: asking.reply, path: path, from: asking.from ?? "" };
        fetching = null;
        making.command = [settings.mkdir ?? "mkdir", "-p", "--", parentOf(path)];
        making.running = true;
    }

    function staged(code) {
        const was = staging;
        staging = null;
        if (was === null || asking?.reply !== was.reply)
            return;
        if (code !== 0) {
            fetching = { asked: 1, here: 0, why: "The folder to save it in couldn't be made." };
            changed();
            return;
        }
        if (was.from !== "") {
            fromFile.path = `${parentOf(was.path)}/.${was.path.slice(was.path.lastIndexOf("/") + 1)}.from`;
            fromFile.setText(was.from);
        }
        pick([was.path], true);
        launcher.open = false;
    }

    Process {
        id: making

        onExited: code => provider.staged(code)
    }

    // The folder :mkdir makes.
    Process {
        id: folding
    }

    // A file or a folder kept elsewhere whose place says what its folder's
    // read would, as `entry` ({ name, folder, size, modified }), with the
    // folders it's in (`where`) and what more it says (`facts`), says
    // them on its row and plate as one here does (FileFacts.js), so a
    // folder there reads as a folder here (Joel, 2026-11-03: "no clue
    // about sizes or when last updated"); one without says what its
    // place gives.
    function factsElsewhere(fields) {
        if (!fields.entry)
            return fields;
        return Object.assign({}, fields, {
            side: FileFacts.sideElsewhere(fields.entry, Date.now(), factWords),
            drawer: () => FileFacts.plateElsewhere(fields, factWords)
        });
    }

    // A file kept elsewhere: fetched, then picked by Return, or gathered
    // where several may be opened. One of Google's own kinds is the row it
    // is everywhere (`adopt`), picked the same way. Normal's gx opens it
    // in its own program, as a file here, once its place has it
    // (`open`), the ask still waiting.
    function fileElsewhere(fields) {
        elsewhere[fields.key] = { name: fields.name, fetch: fields.fetch };
        const made = acts => {
            // An adopted row says when it changed at its side, as every row
            // in a folder does, rather than when Joel last opened it, which
            // its place's listing doesn't say (VKCF3Z, 2026-11-08).
            const row = fields.adopt ? Object.assign({}, fields.adopt, fields.entry
                    ? { side: FileFacts.sideElsewhere(fields.entry, Date.now(), factWords) } : {})
                : Files.elsewhere("file", factsElsewhere(fields), thing, []);
            if (typeof fields.open === "function")
                row.openOwn = () => provider.openElsewhere(fields);
            Object.assign(row, { verb: acts[0].verb, run: acts[0].run, verbs: () => acts, stays: true });
            return gotten(row, { note: () => provider.fetchingSaid() || (fields.note ?? "") });
        };
        if (several)
            return gatheringRow(fields.key, made);
        return made([act("verb:path:file:open", fields.key, [fields.key])]);
    }

    // Opens a file kept elsewhere in its own program once its place has
    // fetched it, the foot saying so meanwhile or why it couldn't be, as
    // a pick's fetch does; one its place opens itself, as Google's own
    // kinds open in their program, comes back with no path.
    function openElsewhere(fields) {
        if (asking === null || fetching !== null && fetching.why === "")
            return;
        const reply = asking.reply;
        fetching = { asked: 1, here: 0, why: "" };
        changed();
        fields.open(path => {
            if (asking?.reply !== reply || fetching === null)
                return;
            fetching = null;
            changed();
            if (path !== "")
                Niri.spawn(["xdg-open", path]);
        }, why => {
            if (asking?.reply !== reply || fetching === null)
                return;
            fetching = Object.assign({}, fetching, { why: why });
            changed();
        });
    }

    // What the foot says while what's kept elsewhere is fetched.
    function fetchingSaid() {
        if (fetching === null)
            return "";
        if (fetching.why !== "")
            return fetching.why;
        return fetching.asked === 1 ? "Fetching it." : `Fetching ${fetching.here + 1} of ${fetching.asked}.`;
    }

    // A file's row: opened by Return, or, saving, stepped into to Replace It,
    // or, where several may be opened, opening what's gathered.
    function fileRow(path, away) {
        const made = saving
            ? Files.file(path, thing, [act("verb:file-chooser:replace", path, [path])], () => [])
            : several ? gatheringRow(path, acts => Files.file(path, thing, acts))
            : Files.file(path, thing, [act("verb:path:file:open", path, [path])]);
        // Normal's gx opens it in its own program, as his oil's does, the
        // ask still waiting.
        made.openOwn = () => Niri.spawn(["xdg-open", path]);
        // What a V selection's Return answers of it (takes).
        if (!saving)
            made.gathers = path;
        return described(made, path, away === true);
    }

    // A file where several may be opened: Return opens it, or rather
    // answers every file gathered, as :arga gathers them, or, with none,
    // this one; the foot says how many, "\u21b5 open 2", and a file
    // gathered wears the mark in an act's hue, the one colour on its row
    // changing with it (VKCF3Z, 2026-11-03), never the accent, which stays
    // where the keys go. How it stands is read as it's drawn, so the row
    // says it without being made again (VKCF3Z's vi-paradigm spec, The File
    // Picker).
    function gatheringRow(id, made) {
        const is = () => provider.marked.includes(id);
        const row = made([act("verb:path:file:open", id, [id])]);
        const opens = row.verb;
        row.run = () => provider.choose(provider.marked.length > 0 ? provider.marked.slice()
            : [id]);
        // What :arga gathers of it, whatever it's drawn as.
        row.gathers = id;
        // A file kept elsewhere keeps the launcher while it's fetched.
        const away = () => (provider.marked.length > 0 ? provider.marked : [id])
            .some(each => !each.startsWith("/"));
        return gotten(row, {
            stays: away,
            verb: () => provider.marked.length > 0 ? `${opens} ${provider.marked.length}`
                : opens,
            glyph: () => Words.glyph(is() ? "file-chooser:marked" : "path:file"),
            hue: () => is() ? "action" : "program"
        });
    }

    // Gathers the files of the rows given, as :arga adds them to Vim's
    // argument list, in the order given, each once; a row that isn't a
    // file is passed over.
    // How many are gathered, "2 gathered", or "" with none, as the mode
    // word's room says it (VKCF3Z's vi-paradigm spec, The File Picker).
    function gatheredSaid() {
        if (marked.length === 0)
            return "";
        const said = Words.of("verb:file-chooser:marked").said?.marked ?? "";
        return said.replace("{count}", `${marked.length}`);
    }

    // What :arga runs on: the files among the rows the line acts on, or,
    // where none of them is one, as when the line stands for the folder,
    // the cursor row's.
    function gatherOr(things, cursor) {
        const files = things.filter(row => typeof row?.gathers === "string");
        gather(files.length > 0 ? files : [cursor]);
    }

    function gather(rows) {
        const ids = rows.filter(row => typeof row?.gathers === "string").map(row => row.gathers);
        const fresh = ids.filter((id, at) => !provider.marked.includes(id)
            && ids.indexOf(id) === at);
        if (fresh.length === 0)
            return;
        provider.marked = provider.marked.concat(fresh);
        provider.changed();
    }

    // What :args steps into: the files gathered, listed as Vim's :args
    // lists its arguments, its plate counting them with their size
    // together, its crumb the place, Gathered.
    function gatheredPlace() {
        const name = Words.name("file-chooser:marked");
        return thing({
            key: "file-chooser:marked",
            name: name,
            title: name,
            verb: "more",
            glyph: Words.glyph("verb:file-chooser:marked"),
            hue: "action",
            side: `${marked.length}`,
            stays: true,
            now: false,
            plated: true,
            drawer: () => provider.markedPlate(name),
            verbs: () => provider.insideMarked(),
            targets: () => provider.insideMarked()
        });
    }

    // A file marked, by its name: its path's last part, or what the place
    // it's kept in calls it.
    function markedName(id) {
        return id.startsWith("/") ? Files.named(id) : elsewhere[id]?.name ?? id;
    }

    function markedPlate(title) {
        return FileFacts.markedPlate(title, marked.map(each => ({
            name: markedName(each),
            where: each.startsWith("/") ? Files.within(each) : "",
            size: known[each]?.size
        })), factWords);
    }

    // Whether Marked is the step shown now.
    function inMarked() {
        const steps = launcher?.held ?? [];
        return steps.length > 0 && steps[steps.length - 1].row.key === "file-chooser:marked";
    }

    // The files Marked has shown since it was stepped into, in the order
    // they were marked, kept apart from what's drawn so reading them never
    // asks for the rows again.
    readonly property var shownInMarked: ({ ids: [] })

    // Inside Gathered: the files gathered, in the order they were, each
    // with its folder as its note, Return answering them all as anywhere
    // else; then Let Them Go, then Copy the Paths, both with nothing to do
    // once none is gathered.
    function insideMarked() {
        const was = inMarked() ? shownInMarked.ids : [];
        shownInMarked.ids = was.concat(marked.filter(each => !was.includes(each)));
        const none = marked.length === 0;
        const paths = marked.filter(each => each.startsWith("/"));
        const files = shownInMarked.ids.map(id => {
            if (!id.startsWith("/"))
                return gatheringRow(id, acts => Files.elsewhere("file", { key: id, name: markedName(id) },
                    thing, acts));
            const row = gatheringRow(id, acts => Files.file(id, thing, acts));
            row.note = Files.within(id);
            return described(row, id, false);
        });
        return files.concat([thing({
            key: "verb:file-chooser:unmark-all",
            name: Words.name("verb:file-chooser:unmark-all"),
            verb: Words.verb("verb:file-chooser:unmark-all"),
            glyph: Words.glyph("verb:file-chooser:unmark-all"),
            hue: "action",
            still: none,
            stays: true,
            run: () => {
                provider.marked = [];
                provider.changed();
            }
        }), thing({
            key: "verb:file-chooser:copy-marked",
            name: Words.name("verb:file-chooser:copy-marked"),
            verb: Words.verb("verb:file-chooser:copy-marked"),
            glyph: Words.glyph("verb:file-chooser:copy-marked"),
            hue: "action",
            still: paths.length === 0,
            stays: paths.length === 0,
            copy: paths.join("\n"),
            run: () => {
                if (paths.length > 0)
                    Quickshell.clipboardText = paths.join("\n");
            }
        })]);
    }

    // Each path is here, and any kept elsewhere is fetched first; a file
    // kept elsewhere keeps the launcher while it's fetched.
    function act(id, path, paths) {
        const own = id === "verb:file-chooser:replace";
        return thing({
            key: `${id}:${path}`,
            name: own ? Words.name(id) : named(id),
            verb: own ? Words.verb(id) : word(id),
            glyph: Words.glyph(id),
            hue: "action",
            stays: !path.startsWith("/"),
            run: () => provider.choose(paths)
        });
    }

    // Picks what's here at once, and what's kept elsewhere once each is
    // fetched, in the order given, closing the launcher then; one that
    // can't be leaves it open, saying why, so Return tries again.
    function choose(ids) {
        const away = ids.filter(id => !id.startsWith("/"));
        if (away.length === 0 || asking === null) {
            pick(ids);
            return;
        }
        if (fetching !== null && fetching.why === "")
            return;
        const reply = asking.reply;
        const here = {};
        fetching = { asked: away.length, here: 0, why: "" };
        changed();
        for (const id of away)
            elsewhere[id].fetch(path => {
                if (asking?.reply !== reply || fetching === null)
                    return;
                here[id] = path;
                fetching = Object.assign({}, fetching, { here: Object.keys(here).length });
                changed();
                if (fetching.here < away.length)
                    return;
                fetching = null;
                pick(ids.map(each => each.startsWith("/") ? each : here[each]));
                launcher.open = false;
            }, why => {
                if (asking?.reply !== reply || fetching === null)
                    return;
                fetching = Object.assign({}, fetching, { why: why });
                changed();
            });
    }

    // The row Return saves to: the name given, in this folder, which, a
    // file already there by that name, asks once first. A folder by that
    // name can't be saved over, so it gives none.
    function savingRow(folder, name) {
        const trimmed = name.trim();
        if (trimmed === "" || trimmed === "." || trimmed === ".." || trimmed.includes("/"))
            return null;
        const path = folder === "/" ? `/${trimmed}` : `${folder}/${trimmed}`;
        const there = (listed[folder] ?? []).find(entry => entry.name === trimmed);
        if (there?.folder)
            return null;
        return Object.assign(thing({
            key: `file-chooser:save:${path}`,
            name: trimmed,
            verb: accept !== "" ? word("") : "save",
            glyph: Words.glyph("path:file"),
            hue: "program",
            side: Files.within(path),
            // It's the row the line's words name, so normal edits them on
            // it alone (Launcher.qml, editsWords).
            names: true,
            // One that asks first keeps the launcher until it's answered,
            // y closing it once saved, n leaving it as it was.
            stays: !!there,
            run: () => provider.replacing(!!there, trimmed, () => {
                provider.pick([path]);
                launcher.open = false;
            })
        }), { path });
    }

    // Saves at once, or, a file already there by that name, asks once
    // first, as Vim's :w says E13, y saving and anything else letting it go
    // (VKCF3Z's vi-paradigm spec, The File Picker).
    function replacing(taken, name, save) {
        if (!taken) {
            save();
            return;
        }
        const asked = Words.field.asked?.replace ?? "";
        launcher.askOnce(asked.replace("{name}", name), save, () => {});
    }

    // What's typed in a folder found, from a "." on, what's hidden there by
    // that name, and the places kept elsewhere it names.
    function typedIn(folder, letters) {
        const typed = letters.trim();
        const words = Fuzzy.words(typed);
        const placed = folder === Files.home || typed === "" || typed.startsWith(".") ? []
            : places.filter(row => Fuzzy.match(Fuzzy.prepare(row.name), words, false).score >= 0);
        const hidden = !typed.startsWith(".") ? [] : (listed[folder] ?? [])
            .filter(entry => entry.name.startsWith(".") && entry.name !== typed
                && entry.name.toLowerCase().startsWith(typed.toLowerCase())
                && (entry.folder || !folders && passes(entry.name)))
            .map(entry => entry.folder ? folderRow(entry.path) : fileRow(entry.path));
        return hidden.concat(placed);
    }

    // The program's filters, the one in use chosen, and its choices, each
    // saying the option it holds.
    function options() {
        const offered = filters.length < 2 ? [] : filters.map((filter, at) => thing({
            key: `file-chooser:filter:${at}`,
            name: filter.name,
            verb: Words.verb("file-chooser:filter"),
            glyph: Words.glyph("file-chooser:filter"),
            hue: "action",
            side: at === inUse ? "chosen" : "",
            stays: true,
            run: () => {
                provider.filterAt = at;
                provider.changed();
            }
        }));
        return offered.concat((request.choices ?? []).map(choice => choiceRow(choice)));
    }

    function optionOf(choice) {
        return picked[choice.id] ?? choice.default ?? "";
    }

    // A choice of options steps in to them; one without is a yes or no.
    function choiceRow(choice) {
        const now = optionOf(choice);
        const pick = (option) => {
            provider.picked = Object.assign({}, provider.picked, { [choice.id]: option });
            provider.changed();
        };
        const row = {
            key: `file-chooser:choice:${choice.id}`,
            name: choice.label,
            glyph: Words.glyph("file-chooser:choice"),
            hue: "action",
            stays: true
        };
        if ((choice.options ?? []).length === 0)
            return thing(Object.assign(row, {
                verb: Words.verb("file-chooser:choice"),
                side: now === "true" ? "chosen" : "",
                run: () => pick(now === "true" ? "false" : "true")
            }));
        const inside = () => choice.options.map(option => thing({
            key: `file-chooser:choice:${choice.id}:${option.id}`,
            name: option.label,
            verb: Words.verb("file-chooser:choice"),
            glyph: Words.glyph("file-chooser:choice"),
            hue: "action",
            side: option.id === optionOf(choice) ? "chosen" : "",
            stays: true,
            run: () => pick(option.id)
        }));
        return thing(Object.assign(row, {
            verb: "more",
            now: false,
            side: choice.options.find(option => option.id === now)?.label ?? "",
            verbs: inside,
            targets: inside
        }));
    }

    // Whether the filter in use passes a file by its name: by a glob, or by
    // its kind, as the desktop's kinds name files.
    function passes(name) {
        return wants(name, "");
    }

    // The same, for a file whose kind is said (`mime`), as one kept
    // elsewhere is, else known by its name.
    function wants(name, mime) {
        if (inUse < 0)
            return true;
        const filter = filters[inUse];
        const kind = mime || kindOf(name);
        return (filter.globs ?? []).some(glob => globbed(glob).test(name))
            || kind !== "" && (filter.kinds ?? []).some(wanted => isKind(kind, wanted, 0));
    }

    function globbed(glob) {
        const said = glob.replace(/[.+^${}()|\\]/g, "\\$&").replace(/\*/g, ".*").replace(/\?/g, ".");
        return new RegExp(`^${said}$`, "i");
    }

    // A file's kind, by its whole name, else by its longest ending.
    function kindOf(name) {
        const lower = name.toLowerCase();
        if (kinds.names[name] || kinds.names[lower])
            return kinds.names[name] ?? kinds.names[lower];
        for (let at = lower.indexOf("."); at >= 0; at = lower.indexOf(".", at + 1)) {
            const kind = kinds.endings[lower.slice(at)];
            if (kind)
                return kind;
        }
        return "";
    }

    // Whether a kind is the one wanted, a kind of it ("image/*" holds
    // image/png), or one the desktop says it's a kind of (text/markdown is
    // text/plain).
    function isKind(kind, wanted, depth) {
        if (kind === wanted || wanted.endsWith("/*") && kind.startsWith(wanted.slice(0, -1)))
            return true;
        return depth < 8 && (kinds.broader[kind] ?? []).some(each => isKind(each, wanted, depth + 1));
    }

    // The desktop's kinds of file (shared-mime-info's globs2, each line a
    // weight, a kind and a glob, the heaviest first), by a whole name or an
    // ending.
    function kindsFrom(text) {
        const names = {};
        const endings = {};
        for (const line of text.split("\n")) {
            if (line.startsWith("#"))
                continue;
            const [, kind, glob] = line.split(":");
            if (!kind || !glob)
                continue;
            // The first a name or an ending is given is the heaviest.
            const ending = glob.slice(1).toLowerCase();
            if (/^\*\.[^*?[]+$/.test(glob) && !(ending in endings))
                endings[ending] = kind;
            else if (!/[*?[]/.test(glob) && !(glob in names))
                names[glob] = kind;
        }
        return Object.assign({}, kinds, { names: names, endings: endings });
    }

    // The kinds each kind is one of: a line a kind, then the broader one.
    function broaderFrom(text) {
        const broader = {};
        for (const line of text.split("\n")) {
            const [kind, wider] = line.trim().split(/\s+/);
            if (kind && wider)
                broader[kind] = (broader[kind] ?? []).concat([wider]);
        }
        return Object.assign({}, kinds, { broader: broader });
    }

    // A folder whose rows were asked for is read, if it isn't already.
    function readIn(path) {
        if (!reading.includes(path))
            Qt.callLater(provider.want, path);
        return path;
    }

    function want(path) {
        if (asking !== null && !reading.includes(path))
            stepped = stepped.concat([path]);
    }

    function heard(path, entries) {
        listed = Object.assign({}, listed, { [path]: entries });
        const each = {};
        for (const entry of entries)
            each[entry.path] = entry;
        known = Object.assign({}, known, each);
        if (entries.some(entry => entry.folder))
            count(path);
        changed();
        const waiting = besideNext;
        if (waiting !== null && parentOf(waiting.path) === path) {
            besideNext = null;
            beside(waiting.path, waiting.by);
        }
    }

    // The folder beside this one in the folder holding it, the next (1) or
    // the one before (-1), as the folder holding it lists them: in the order
    // in use, the hidden left out. It takes this one's place, held where it
    // was, and from the last goes round to the first, as vi's :bnext does.
    // Where the folder holding it isn't read yet, it's read first, and the
    // step taken once it's heard.
    function beside(path, by) {
        const steps = launcher?.held ?? [];
        const deepest = steps[steps.length - 1]?.row.key;
        if (path === "/" || deepest !== `folder:${path}`)
            return;
        const parent = parentOf(path);
        if (!(parent in listed)) {
            besideNext = { path, by };
            readIn(parent);
            return;
        }
        const around = FileFacts.ordered(listed[parent].filter(entry => entry.folder
            && (entry.path === path || showsHidden || !entry.name.startsWith("."))), order,
            counts, reversed).map(entry => entry.path);
        const at = around.indexOf(path);
        if (at < 0 || around.length < 2)
            return;
        launcher.oneJump(() => {
            launcher.back();
            launcher.enter(folderRow(around[(at + by + around.length) % around.length]));
        });
    }

    function everything() {
        return rowsOf(readIn("/"));
    }

    function counted() {
        return `${everything().length} ${folders ? "folders" : "files"}`;
    }

    // Says what was picked, held to what the program offered. A save here
    // keeps the folder it went to; one staged for a place kept elsewhere
    // (`away`) doesn't, since its folder is only the place's staging.
    function pick(paths, away) {
        if (asking === null)
            return;
        if ((saving || mode === "save-files") && away !== true && String(paths[0] ?? "").startsWith("/"))
            keepSavedIn(saving ? parentOf(paths[0]) : paths[0]);
        const choices = {};
        for (const choice of request.choices ?? [])
            choices[choice.id] = optionOf(choice);
        settle(asking.reply, JSON.stringify({
            paths: paths,
            filter: inUse < 0 ? null : inUse,
            choices: choices
        }));
    }

    function keepSavedIn(folder) {
        if (folder === savedLast)
            return;
        savedLast = folder;
        if (settings.savedIn)
            savedInFile.setText(`${folder}\n`);
    }

    // The folder here a save last went to, through the portal or from
    // inside the shell, or "" before any: where Google's Drive starts a
    // Save a Copy of a file it couldn't take (427IH3 and VKCF3Z,
    // 2026-10-30).
    function savedLastIn() {
        return savedLast;
    }

    // Says the answer, or none, on the ask's socket, or to what the shell
    // asked with, and asks the next.
    function settle(reply, said) {
        const settled = asks.find(other => other.reply === reply);
        if (!settled)
            return;
        asks = asks.filter(other => other.reply !== reply);
        if (shown === reply)
            shown = "";
        // What was read goes with the last ask, since the next may be
        // anywhere; while one waits, its folders are being read already.
        if (asking === null)
            forget();
        if (settled.picked) {
            settled.picked(said === "" ? [] : JSON.parse(said).paths);
        } else {
            // It connects only once made: a local socket can connect, say
            // and close at once, and an object still being made can't be
            // destroyed.
            const answered = answering.createObject(provider, { path: reply, said: `${said}\n` });
            answered.connected = true;
        }
        if (asking !== null)
            Qt.callLater(provider.showNext);
    }

    // Asks, from inside the shell, where to save a file named name,
    // starting in folder, titled as given, as Mail's Save a Copy does
    // (427IH3, 2026-10-29). It waits its turn behind the portal's asks, and
    // picked is called with the one path chosen, a name already there
    // having been cleared to Replace It, or with none. With `local`, only
    // the folders here are offered, as Save a Copy of a file Drive
    // wouldn't take asks.
    function chooseToSave(fields, picked) {
        shellAsked += 1;
        const first = asking === null;
        asks = asks.concat([{
            reply: `shell:${shellAsked}`,
            request: {
                mode: "save",
                title: String(fields.title ?? ""),
                name: String(fields.name ?? ""),
                folder: String(fields.folder ?? ""),
                local: fields.local === true
            },
            picked: picked
        }]);
        if (first)
            Qt.callLater(provider.showNext);
    }

    // Holds the oldest ask in the launcher, in its folder, every folder on
    // the way held above it, unless it already does.
    function showNext() {
        if (asking === null || (shown === asking.reply && launcher.open))
            return;
        shown = asking.reply;
        filterAt = -1;
        picked = {};
        marked = [];
        stepped = [];
        besideNext = null;
        saveName = request.name ?? "";
        lately = [];
        fetching = null;
        elsewhere = {};
        // What each folder read said of its files, and what file-preview
        // said, go with the folders read (forget), since those are read
        // while an ask waits, before it's shown.
        previewNext = "";
        places = placesFor();
        listing.running = (settings.recent ?? []).length > 0;
        launcher.into(settings.name);
        for (const path of onTheWay(start))
            launcher.enter(folderRow(path));
        // The folder it opens in is where it opened: the folders on the
        // way were never shown, so Ctrl+O has none of them to go back to.
        launcher.openedHere();
    }

    // One answer: connected, said, then gone.
    Component {
        id: answering

        Socket {
            id: answer

            required property string said

            onConnectionStateChanged: {
                if (answer.connected) {
                    answer.write(answer.said);
                    answer.flush();
                    answer.connected = false;
                } else {
                    answer.destroy();
                }
            }
            onError: answer.destroy()
        }
    }

    // Each folder held or on the way, read as it's stepped into and again
    // as it changes.
    Variants {
        model: provider.reading

        FolderListModel {
            id: reader

            required property string modelData

            folder: `file://${reader.modelData}`
            showDirsFirst: true
            showHidden: true
            showDotAndDotDot: false
            sortCaseSensitive: false

            function told() {
                if (reader.status !== FolderListModel.Ready)
                    return;
                const entries = [];
                for (let at = 0; at < reader.count; at++)
                    entries.push({
                        name: reader.get(at, "fileName"),
                        path: reader.get(at, "filePath"),
                        folder: reader.get(at, "fileIsDir"),
                        size: reader.get(at, "fileSize"),
                        modified: reader.get(at, "fileModified").getTime()
                    });
                provider.heard(reader.modelData, entries);
            }

            onStatusChanged: Qt.callLater(reader.told)
            onRowsInserted: Qt.callLater(reader.told)
            onRowsRemoved: Qt.callLater(reader.told)
            onModelReset: Qt.callLater(reader.told)
        }
    }

    // The files opened lately, read once an ask: of each line recent-files
    // prints (a program, a file and when), the file, as Recent reads it.
    Process {
        id: listing

        command: provider.settings.recent ?? []
        stdout: StdioCollector {
            id: said

            onStreamFinished: {
                const paths = said.text.split("\n").map(line => Recents.openedPath(line))
                    .filter(path => path.startsWith("/"));
                provider.lately = Array.from(new Set(paths));
                provider.changed();
            }
        }
    }

    // The program whose window had the keys as it asked, by its app id,
    // or "" with none: the portal's own app id for a program run outside a
    // sandbox is often empty, or a name niri doesn't know it by.
    function askedFrom() {
        return Niri.windows[Niri.current]?.app_id ?? "";
    }

    // Who asked for a save staged elsewhere, written beside it before the
    // program is answered, so it's there before the file is.
    FileView {
        id: fromFile

        preload: false
        blockWrites: true
        printErrors: false
    }

    // Read once as the shell starts; this is its only writer.
    FileView {
        id: savedInFile

        path: provider.settings.savedIn ?? ""
        printErrors: false
        onLoaded: provider.savedLast = text().trim()
    }

    // Read once as the shell starts; this is its only writer, written whole
    // before it goes on, so a shell started just after reads the order.
    FileView {
        id: orderFile

        path: provider.settings.orderIn ?? ""
        blockWrites: true
        printErrors: false
        onLoaded: {
            const kept = text().trim();
            const turned = kept.endsWith("!");
            const each = turned ? kept.slice(0, -1) : kept;
            if (provider.orders.includes(each)) {
                provider.order = each;
                provider.reversed = turned;
            }
        }
    }

    FileView {
        path: provider.asking === null || !provider.settings.kinds ? ""
            : `${provider.settings.kinds}/globs2`
        printErrors: false
        onLoaded: provider.kinds = provider.kindsFrom(text())
    }

    FileView {
        path: provider.asking === null || !provider.settings.kinds ? ""
            : `${provider.settings.kinds}/subclasses`
        printErrors: false
        onLoaded: provider.kinds = provider.broaderFrom(text())
    }

    // Closing the launcher on an ask, or leaving it for something else,
    // picks nothing. The launcher closes before it runs the row picked, so
    // the refusal waits a turn, by when a pick has answered the ask.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open && provider.shown !== "")
                Qt.callLater(provider.refuse, provider.shown);
        }
    }

    function refuse(reply) {
        if (!launcher.open && shown === reply)
            settle(reply, "");
    }

    // Asks for files for the chooser listening on reply, as the request
    // says (files.rs). Only a socket the chooser makes is answered, and a
    // request that isn't JSON is refused at once.
    function requested(reply, request) {
        const runtime = Quickshell.env("XDG_RUNTIME_DIR");
        const name = reply.startsWith(`${runtime}/open-with/`)
            ? reply.slice(runtime.length + "/open-with/".length) : "";
        if (!/^[0-9]+\.sock$/.test(name) || asks.some(other => other.reply === reply))
            return;
        let asked = null;
        try {
            asked = JSON.parse(request);
        } catch (error) {
            asked = null;
        }
        const first = asking === null;
        asks = asks.concat([{ reply: reply, request: asked ?? {}, from: askedFrom() }]);
        if (asked === null || typeof asked !== "object")
            settle(reply, "");
        else if (first)
            Qt.callLater(provider.showNext);
    }

    // Stops asking, once the program gave up or the chooser stopped
    // waiting.
    // Lets go of every folder read and what was said of each file.
    function forget() {
        listed = {};
        known = {};
        counts = {};
        previews = {};
        askedAt = {};
        toCount = [];
        outstanding = null;
        previewer.running = false;
    }

    // Lists every folder in an order, turned round or not, as :sort and
    // :sort! say, kept for the next ask and a restart; the folder shown is
    // made afresh on its first row, since rows shown keep their places.
    function orderBy(each, turned) {
        if (!orders.includes(each))
            return;
        if (each !== order || turned !== reversed) {
            order = each;
            reversed = turned;
            if (settings.orderIn)
                orderFile.setText(`${each}${turned ? "!" : ""}\n`);
        }
        afresh();
    }

    // The picker's own commands on the launcher's command line, as Vim's
    // and his oil's are, each { command, short, title, key, ranged, bang,
    // undoable, args, run } (V3's line, Commands.js): :sort, :set hidden,
    // :mkdir, :e and :cd with their paths completed from folders already
    // read, a save's :w and :x, :q answering nothing, and, where several
    // may be opened, :arga and :args (VKCF3Z's vi-paradigm spec, The File
    // Picker). Short forms sit here until the key language says them.
    function commandsIn(path) {
        const titled = (command, fields) => Object.assign({
            command,
            title: Words.name(`command:file-chooser:${command}`),
            key: "",
            ranged: false,
            bang: false,
            undoable: true,
            args: []
        }, fields);
        const notes = Words.of("file-chooser:order").said ?? {};
        const options = Words.of("command:file-chooser:set").said ?? {};
        const rows = [
            titled("sort", {
                short: "sor",
                bang: true,
                args: orders.map(each => ({ value: each, note: notes[each] ?? "" })),
                run: call => provider.orderBy(call.arg === "" ? provider.order : call.arg, call.bang)
            }),
            titled("set", {
                short: "se",
                args: ["hidden", "nohidden"].map(each => ({ value: each, note: options[each] ?? "" })),
                run: call => provider.setOption(call.arg)
            }),
            titled("mkdir", { short: "mkdir", run: call => provider.makeFolder(path, call.arg) }),
            titled("edit", {
                short: "e",
                args: typed => provider.completed(path, typed, false),
                run: call => provider.edit(path, call.arg)
            }),
            titled("cd", {
                short: "cd",
                args: typed => provider.completed(path, typed, true),
                run: call => provider.goTo(provider.resolved(path, call.arg))
            }),
            titled("quit", { short: "q", run: () => launcher.open = false })
        ];
        if (saving)
            rows.push(titled("write", {
                short: "w",
                bang: true,
                run: call => provider.write(path, call.arg, call.bang)
            }), titled("xit", {
                short: "x",
                run: call => provider.write(path, call.arg, false)
            }));
        if (several)
            rows.push(titled("argadd", {
                short: "arga",
                ranged: true,
                run: call => provider.gatherOr(call.things ?? [], launcher.selected)
            }), titled("args", { short: "ar", run: () => launcher.enter(provider.gatheredPlace()) }));
        return rows;
    }

    // Return in a V selection, as V4's visual hands it the rows it took:
    // every file among them is answered, in order, or the first alone
    // where one is taken; with none, the launcher does what it would
    // (VKCF3Z's vi-paradigm spec, The File Picker).
    function takes(rows) {
        const ids = (rows ?? []).map(taken => taken?.row ?? taken)
            .filter(row => typeof row?.gathers === "string").map(row => row.gathers);
        if (ids.length === 0 || saving || folders)
            return false;
        choose(several ? ids : ids.slice(0, 1));
        if (ids.every(id => id.startsWith("/")))
            launcher.open = false;
        return true;
    }

    // :set hidden lists a folder's hidden files, and :set nohidden leaves
    // them out again; anything else is no option of the picker's.
    function setOption(option) {
        if (option !== "hidden" && option !== "nohidden")
            return;
        showsHidden = option === "hidden";
        afresh();
    }

    // A path typed on the line, from the folder shown: ~ is home, and one
    // not starting at / is inside the folder.
    function resolved(path, typed) {
        const text = typed.startsWith("~") ? `${Files.home}${typed.slice(1)}` : typed;
        const whole = text.startsWith("/") ? text : `${path === "/" ? "" : path}/${text}`;
        return whole.length > 1 ? whole.replace(/\/+$/, "") : whole;
    }

    // What a path typed so far could go on to, from the folders already
    // read alone, never reading one as a key is typed: each entry of the
    // folder it names starting as typed, a folder's with a / after it, the
    // hidden only once a . is typed or :set hidden says.
    function completed(path, typed, foldersOnly) {
        const said = typed.slice(0, typed.lastIndexOf("/") + 1);
        const whole = resolved(path, typed);
        const ends = typed === "" || typed.endsWith("/");
        const folder = ends ? whole : parentOf(whole);
        const start = ends ? "" : whole.slice(whole.lastIndexOf("/") + 1);
        const note = Words.of("command:file-chooser:edit").said?.folder ?? "";
        return (listed[folder] ?? []).filter(entry => entry.name.startsWith(start)
            && (!foldersOnly || entry.folder)
            && (showsHidden || start.startsWith(".") || !entry.name.startsWith(".")))
            .map(entry => ({ value: `${said}${entry.name}${entry.folder ? "/" : ""}`,
                note: entry.folder ? note : "" }));
    }

    // Goes to a folder as if the ask had opened there: each folder on the
    // way held, as :cd moves Vim, in one jump, so Ctrl+O goes back to the
    // folder it was in.
    function goTo(folder) {
        if (asking === null || !folder.startsWith("/"))
            return;
        launcher.oneJump(() => {
            launcher.into(settings.name);
            for (const each of onTheWay(folder))
                launcher.enter(folderRow(each));
        });
    }

    // :e a folder goes there, as :e on a directory opens his oil; a file
    // is opened, or, saving, named and gone to, the name on the line.
    function edit(path, typed) {
        if (typed === "")
            return;
        const target = resolved(path, typed);
        const entry = (listed[parentOf(target)] ?? []).find(each => each.path === target);
        if (entry?.folder || typed.endsWith("/")) {
            goTo(target);
        } else if (saving) {
            saveName = target.slice(target.lastIndexOf("/") + 1);
            goTo(parentOf(target));
        } else {
            choose([target]);
            launcher.open = false;
        }
    }

    // :mkdir makes a folder in the one shown, which lists it once its
    // watch hears it.
    function makeFolder(path, name) {
        if (name === "" || name === "." || name === "..")
            return;
        folding.command = [settings.mkdir ?? "mkdir", "-p", "--", resolved(path, name)];
        folding.running = true;
    }

    // :w saves as the name given, or the line's, asking once over a file
    // already there as :w says E13; :w! saves over it without asking.
    function write(path, name, over) {
        if (name !== "")
            saveName = name;
        const row = savingRow(path, saveName);
        if (row === null)
            return;
        if (over) {
            pick([row.path]);
            launcher.open = false;
            return;
        }
        row.run();
    }

    // The folder shown made again, as it's stepped back into, so its rows
    // take what's changed: an order, or what's hidden.
    function afresh() {
        const steps = launcher?.held ?? [];
        const folder = steps[steps.length - 1]?.row;
        if (!String(folder?.key ?? "").startsWith("folder:"))
            return;
        // Made again where it is, it's no jump.
        launcher.oneJump(() => {
            launcher.back();
            launcher.enter(folder);
        });
    }

    function withdrawn(reply) {
        const shownNow = shown === reply;
        asks = asks.filter(other => other.reply !== reply);
        if (asking === null)
            forget();
        if (!shownNow)
            return;
        shown = "";
        if (asking !== null)
            showNext();
        else
            launcher.open = false;
    }

    IpcHandler {
        target: "file-chooser"

        function ask(reply: string, request: string): void {
            provider.requested(reply, request);
        }

        function withdraw(reply: string): void {
            provider.withdrawn(reply);
        }
    }
}

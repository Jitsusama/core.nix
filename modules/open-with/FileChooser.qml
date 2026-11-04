// The files a program opens or saves, chosen in the launcher for the
// portal's chooser of files (open-with, src/files.rs), which asks here
// rather than GTK drawing a dialog of its own. The launcher opens in the
// folder the program asked for, else the folder of the file it saves over,
// else home, every folder on the way held above it, so the left arrow goes
// one out, and back the way Joel came once he's stepped in.
//
// A folder lists what it holds as it's stepped into, read then and again as
// it changes, never on a timer: the files opened lately first, where it
// opened, apart from the rest; then its folders, then its files, by name,
// each drawn as every file and folder is (Files.qml); then the program's
// filters and its choices, apart again. What's hidden shows once a "." is
// typed. Opening, Return on a file picks it, by the program's own word for
// it; where several may be opened, Return marks it instead, its glyph the
// mark while it's marked, and Open the Marked, leading every folder from
// the start, counts them, names them and opens every file marked, wherever
// each was, its right arrow stepping into Marked to unmark them (VKCF3Z,
// 2026-11-03); asked for a folder, each folder leads with Open This
// Folder, one folder picked though the program could take several. Saving,
// each folder leads with the name the program gave, and what's typed names
// it instead; a name already there steps in to Replace It, the one ask.
// Escape, or closing the launcher, picks nothing.
//
// Each file and folder says beside its name how big it is and when it last
// changed, "2.4 MB \u00b7 28 Oct", a folder how many items it shows, read
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
// 2026-10-29). Their files are fetched before they're picked: the launcher
// stays, its foot saying so, and closes once every path is here, or stays
// saying why one couldn't be. Saving, a folder there whose place gives
// `saveTo` leads with the name, as a folder here does, what's typed
// naming it instead, and a name already there (`names`) steps in to
// Replace It; Return makes the folder the place said to save in, here,
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
    // Each kind of file by how it's named, and the kinds each is one of.
    property var kinds: ({ names: {}, endings: {}, broader: {} })

    collection: asking === null ? null : ({
        noun: folders ? "folders" : "files",
        title: request.title || (saving || mode === "save-files" ? "Save" : "Open"),
        searched: false,
        glyph: Theme.glyph[folders ? "folder" : "path"],
        ask: question(),
        empty: "Nothing here."
    })

    function question() {
        if (mode === "save-files")
            return "save them where?";
        if (saving)
            return (request.name ?? "") === "" ? "save it where?" : `save ${request.name} where?`;
        return folders ? "open which folder?" : "open which file?";
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
        const hidden = entry => entry.name.startsWith(".") && !held.includes(`folder:${entry.path}`)
            && !(start === entry.path || start.startsWith(`${entry.path}/`));
        const shown = all.filter(entry => !hidden(entry));
        const own = shown.filter(entry => entry.folder).map(entry => folderRow(entry.path))
            .concat(folders ? [] : shown.filter(entry => !entry.folder && passes(entry.name))
                .map(entry => fileRow(entry.path)));
        // A file the name given is already, which it leads with, isn't
        // listed again below.
        const naming = saving && path !== "/" ? savingRow(path, request.name ?? "") : null;
        // The filters and choices wait for the folder's read as its Copy
        // the Path does (folderRow), to come after its files.
        return apart([several ? [markedLead()] : [],
            (naming ? [naming] : []).concat(path === start ? recent(path) : []),
            own.filter(row => row.key !== naming?.key), path === Files.home ? places.slice() : [],
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
    // and then one folder, "~/Downloads", which changes in place as the
    // left arrow goes up, rather than a crumb a folder (VKCF3Z,
    // 2026-10-29).
    function folderRow(path, away) {
        const acts = !folders ? [] : [act(mode === "save-files" ? "verb:file-chooser:here"
            : "verb:file-chooser:folder", path, [path])];
        const row = Object.assign(Files.folder(path, thing, acts, () => rowsOf(readIn(path))), {
            empty: "Nothing here.",
            typed: letters => typedIn(path, letters)
        });
        // Until the folder's read, it holds no Copy the Path: rows shown
        // keep their place and those that come go below them, so a Copy
        // shown before its files were read headed the list for good, where
        // it goes last (VKCF3Z, 2026-11-03).
        const whole = row.verbs;
        const held = () => path in provider.listed ? whole()
            : whole().filter(one => !String(one.key).startsWith("verb:path:folder:copy:"));
        Object.assign(row, { verbs: held, targets: held });
        Object.defineProperty(row, "title", {
            get: () => provider.titled(Files.shown(path)),
            enumerable: true
        });
        return described(row, path, away === true);
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
        const file = { path: path, name: Files.named(path), where: Files.within(path), entry: entry };
        return FileFacts.plate(file, said || null, Date.now(), factWords);
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
        const inside = !saves ? () => fields.inside() : () => {
            const naming = provider.namingElsewhere(fields, request.name ?? "");
            return provider.apart([naming ? [naming] : [], fields.inside()]);
        };
        const row = Files.elsewhere("folder", fields, thing, [], inside);
        Object.defineProperties(row, {
            title: { get: () => provider.titled(fields.name), enumerable: true },
            empty: { get: () => fields.empty?.() || "Nothing here.", enumerable: true }
        });
        if (saves)
            row.typed = letters => {
                const naming = provider.namingElsewhere(fields, letters.trim());
                return naming ? [naming] : [];
            };
        return row;
    }

    // The row Return saves to in a folder kept elsewhere: the name given,
    // or, one already there by that name, a file stepped into to Replace
    // It first.
    function namingElsewhere(fields, name) {
        if (name === "" || name === "." || name === ".." || name.includes("/"))
            return null;
        const path = fields.saveTo(name);
        if ((fields.names?.() ?? []).includes(name))
            return Files.elsewhere("file", { key: `${fields.key}:${name}`, name: name, side: fields.name },
                thing, [thing({
                    key: `verb:file-chooser:replace:${path}`,
                    name: Words.name("verb:file-chooser:replace"),
                    verb: Words.verb("verb:file-chooser:replace"),
                    glyph: Words.glyph("verb:file-chooser:replace"),
                    hue: "action",
                    stays: true,
                    run: () => provider.stage(path)
                })], () => []);
        const row = thing({
            key: `file-chooser:save:${path}`,
            name: name,
            verb: accept !== "" ? word("") : "save",
            glyph: Words.glyph("path:file"),
            hue: "program",
            side: fields.name,
            stays: true,
            run: () => provider.stage(path)
        });
        Object.defineProperty(row, "note", { get: () => provider.fetchingSaid(), enumerable: true });
        return row;
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

    // A file kept elsewhere: fetched, then picked by Return, or marked
    // where several may be opened. One of Google's own kinds is the row it
    // is everywhere (`adopt`), picked the same way.
    function fileElsewhere(fields) {
        elsewhere[fields.key] = { name: fields.name, fetch: fields.fetch };
        const made = acts => {
            const row = fields.adopt ? Object.assign({}, fields.adopt)
                : Files.elsewhere("file", fields, thing, []);
            Object.assign(row, { verb: acts[0].verb, run: acts[0].run, verbs: () => acts, stays: true });
            Object.defineProperty(row, "note", {
                get: () => provider.fetchingSaid() || (fields.note ?? ""),
                enumerable: true
            });
            return row;
        };
        if (several)
            return markingRow(fields.key, made);
        return made([act("verb:path:file:open", fields.key, [fields.key])]);
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
    // or, where several may be opened, marked by Return.
    function fileRow(path, away) {
        const made = saving
            ? Files.file(path, thing, [act("verb:file-chooser:replace", path, [path])], () => [])
            : several ? markingRow(path, acts => Files.file(path, thing, acts))
            : Files.file(path, thing, [act("verb:path:file:open", path, [path])]);
        return described(made, path, away === true);
    }

    // A file where several may be opened: Return flips its Marked, the
    // launcher staying, as Muted is flipped (Sound.qml), and the row wears
    // the mark while it's marked; inside, Marked, then Open for it alone,
    // then Copy the Path. How it stands is read as it's drawn, so the row
    // says what Return just did without being made again (VKCF3Z,
    // 2026-10-29).
    function markingRow(path, made) {
        const is = () => provider.marked.includes(path);
        const flip = () => {
            provider.marked = is() ? provider.marked.filter(each => each !== path)
                : provider.marked.concat([path]);
            provider.changed();
        };
        const verb = () => Words.verb("file-chooser:marked", is() ? "marked" : "");
        const marking = thing({
            key: `file-chooser:marked:${path}`,
            name: Words.name("file-chooser:marked"),
            glyph: Words.glyph("file-chooser:marked"),
            hue: "action",
            stays: true,
            run: flip
        });
        Object.defineProperties(marking, {
            verb: { enumerable: true, get: verb },
            side: { enumerable: true, get: () => is() ? "yes" : "no" }
        });
        const row = made([marking, act("verb:path:file:open", path, [path])]);
        row.stays = true;
        // Marked, its glyph is the mark, in an act's hue rather than a
        // file's, the one colour on the row changing with it (VKCF3Z,
        // 2026-11-03), never the accent, which stays where the keys go.
        Object.defineProperties(row, {
            verb: { enumerable: true, get: verb },
            glyph: {
                enumerable: true,
                get: () => Words.glyph(is() ? "file-chooser:marked" : "path:file")
            },
            hue: { enumerable: true, get: () => is() ? "action" : "program" }
        });
        return row;
    }

    // The lead where several may be opened, there from the start so nothing
    // slides when the first is marked (VKCF3Z, 2026-11-03). Until one is,
    // it says "none" and nothing more, and Return on it does nothing, the
    // foot saying no Return; then it counts them, its note names them in
    // the order marked, its plate lists them with their size together,
    // Return opens them all, and the right arrow steps into Marked, which
    // stays while he's in it though the last is unmarked.
    function markedLead() {
        const count = marked.length;
        const said = Words.of("verb:file-chooser:marked").said ?? {};
        const name = accept !== "" ? `${accept} the Marked` : Words.name("verb:file-chooser:marked");
        const row = {
            key: "file-chooser:marked",
            name: name,
            verb: word("verb:file-chooser:marked"),
            glyph: Words.glyph("verb:file-chooser:marked"),
            hue: "action",
            side: count === 0 ? said.none ?? "" : `${count}`,
            note: fetchingSaid() || marked.map(each => markedName(each)).join(" \u00b7 "),
            still: count === 0,
            stays: count === 0 || marked.some(each => !each.startsWith("/")),
            run: () => {
                if (provider.marked.length > 0)
                    provider.choose(provider.marked.slice());
            }
        };
        // The launcher draws a plate only for a thing, by its kind, or a
        // row that says it's plated, as Now does at home; the lead is no
        // thing, so it says so while it has the marked files to list.
        if (count > 0)
            Object.assign(row, { plated: true, drawer: () => provider.markedPlate(name) });
        if (count > 0 || inMarked())
            row.verbs = () => provider.insideMarked();
        return thing(row);
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

    // Inside Marked: the files marked, in the order they were, each with
    // its folder as its note, Return flipping its mark as anywhere else, so
    // one unmarked stays there, unmarked, until he leaves; then Unmark All,
    // then Copy the Paths, both with nothing to do once none is marked.
    function insideMarked() {
        const was = inMarked() ? shownInMarked.ids : [];
        shownInMarked.ids = was.concat(marked.filter(each => !was.includes(each)));
        const none = marked.length === 0;
        const paths = marked.filter(each => each.startsWith("/"));
        const files = shownInMarked.ids.map(id => {
            if (!id.startsWith("/"))
                return markingRow(id, acts => Files.elsewhere("file", { key: id, name: markedName(id) },
                    thing, acts));
            const row = markingRow(id, acts => Files.file(id, thing, acts));
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

    // The row Return saves to: the name given, in this folder, or the file
    // already there by that name, which asks first.
    function savingRow(folder, name) {
        if (name === "" || name === "." || name === ".." || name.includes("/"))
            return null;
        const path = folder === "/" ? `/${name}` : `${folder}/${name}`;
        const there = (listed[folder] ?? []).find(entry => entry.name === name);
        if (there)
            return there.folder ? null : fileRow(path);
        return thing({
            key: `file-chooser:save:${path}`,
            name: name,
            verb: accept !== "" ? word("") : "save",
            glyph: Words.glyph("path:file"),
            hue: "program",
            side: Files.within(path),
            run: () => provider.pick([path])
        });
    }

    // What's typed in a folder: saving, the name it saves as; and from a
    // "." on, what's hidden there by that name.
    function typedIn(folder, letters) {
        const typed = letters.trim();
        const naming = saving ? savingRow(folder, typed) : null;
        const words = Fuzzy.words(typed);
        const placed = folder === Files.home || typed === "" || typed.startsWith(".") ? []
            : places.filter(row => Fuzzy.match(Fuzzy.prepare(row.name), words, false).score >= 0);
        const hidden = !typed.startsWith(".") ? [] : (listed[folder] ?? [])
            .filter(entry => entry.name.startsWith(".") && entry.name !== typed
                && entry.name.toLowerCase().startsWith(typed.toLowerCase())
                && (entry.folder || !folders && passes(entry.name)))
            .map(entry => entry.folder ? folderRow(entry.path) : fileRow(entry.path));
        return (naming ? [naming] : []).concat(hidden, placed);
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

    // The files opened lately, read once an ask.
    Process {
        id: listing

        command: provider.settings.recent ?? []
        stdout: StdioCollector {
            id: said

            onStreamFinished: {
                const paths = said.text.split("\n").map(line => line.slice(line.indexOf("\t") + 1))
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

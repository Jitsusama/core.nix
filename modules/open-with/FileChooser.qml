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
// it; where several may be opened, Return marks it instead, and Open the
// Marked, leading every folder from the start, opens every file marked,
// wherever each was; asked for a folder, each folder leads with Open This
// Folder, one folder picked though the program could take several. Saving,
// each folder leads with the name the program gave, and what's typed names
// it instead; a name already there steps in to Replace It, the one ask.
// Escape, or closing the launcher, picks nothing.
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
// (`local`) is given no places.
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

    // What each folder read holds, by its path: { name, path, folder }.
    property var listed: ({})
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
    // Saving into a place kept elsewhere: the ask and the path, while the
    // folder the path is in is made.
    property var staging: null
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
        return apart([several ? [markedLead()] : [],
            (naming ? [naming] : []).concat(path === start ? recent(path) : []),
            own.filter(row => row.key !== naming?.key), path === Files.home ? places.slice() : [],
            options()]);
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
                .slice(0, 9).map(each => fileRow(each));
        return Array.from(new Set(lately.map(each => parentOf(each)))).filter(elsewhere)
            .slice(0, 9).map(each => folderRow(each));
    }

    // A folder's row, stepped into to what it holds, led by Open This
    // Folder where one is asked for.
    //
    // Held, each folder on the way is titled as the one held deepest, read
    // as the launcher says its path, so the path says the program's title
    // and then one folder, "~/Downloads", which changes in place as the
    // left arrow goes up, rather than a crumb a folder (VKCF3Z,
    // 2026-10-29).
    function folderRow(path) {
        const acts = !folders ? [] : [act(mode === "save-files" ? "verb:file-chooser:here"
            : "verb:file-chooser:folder", path, [path])];
        const row = Object.assign(Files.folder(path, thing, acts, () => rowsOf(readIn(path))), {
            empty: "Nothing here.",
            typed: letters => typedIn(path, letters)
        });
        Object.defineProperty(row, "title", {
            get: () => provider.titled(Files.shown(path)),
            enumerable: true
        });
        return row;
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
        staging = { reply: asking.reply, path: path };
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
        pick([was.path]);
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
    function fileRow(path) {
        if (saving)
            return Files.file(path, thing, [act("verb:file-chooser:replace", path, [path])], () => []);
        if (several)
            return markingRow(path, acts => Files.file(path, thing, acts));
        return Files.file(path, thing, [act("verb:path:file:open", path, [path])]);
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
        Object.defineProperties(row, {
            verb: { enumerable: true, get: verb },
            glyph: {
                enumerable: true,
                get: () => Words.glyph(is() ? "file-chooser:marked" : "path:file")
            }
        });
        return row;
    }

    // The lead where several may be opened, there from the start so nothing
    // slides when the first is marked: it says how many are, and Return
    // opens them, or, with none, opens nothing and says why.
    function markedLead() {
        const count = marked.length;
        return thing({
            key: "file-chooser:marked",
            name: accept !== "" ? `${accept} the Marked` : Words.name("verb:file-chooser:marked"),
            verb: word("verb:file-chooser:marked"),
            glyph: Words.glyph("verb:file-chooser:marked"),
            hue: "action",
            side: count === 0 ? "none" : `${count}`,
            note: count === 0 ? "Nothing is marked yet: Return on a file marks it." : fetchingSaid(),
            stays: count === 0 || marked.some(each => !each.startsWith("/")),
            run: () => {
                if (provider.marked.length > 0)
                    provider.choose(provider.marked.slice());
            }
        });
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
        changed();
    }

    function everything() {
        return rowsOf(readIn("/"));
    }

    function counted() {
        return `${everything().length} ${folders ? "folders" : "files"}`;
    }

    // Says what was picked, held to what the program offered.
    function pick(paths) {
        if (asking === null)
            return;
        const choices = {};
        for (const choice of request.choices ?? [])
            choices[choice.id] = optionOf(choice);
        settle(asking.reply, JSON.stringify({
            paths: paths,
            filter: inUse < 0 ? null : inUse,
            choices: choices
        }));
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
            listed = {};
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
                        folder: reader.get(at, "fileIsDir")
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
        asks = asks.concat([{ reply: reply, request: asked ?? {} }]);
        if (asked === null || typeof asked !== "object")
            settle(reply, "");
        else if (first)
            Qt.callLater(provider.showNext);
    }

    // Stops asking, once the program gave up or the chooser stopped
    // waiting.
    function withdrawn(reply) {
        const shownNow = shown === reply;
        asks = asks.filter(other => other.reply !== reply);
        if (asking === null)
            listed = {};
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

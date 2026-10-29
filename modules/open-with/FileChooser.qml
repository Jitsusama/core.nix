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
            own.filter(row => row.key !== naming?.key), options()]);
    }

    // Groups one after another, each set apart from the one before it by a
    // gap, as Now is at home.
    function apart(groups) {
        const rows = [];
        for (const group of groups) {
            if (group.length > 0 && rows.length > 0)
                group[0].gap = true;
            rows.push(...group);
        }
        return rows;
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
            get: () => Files.shown(provider.deepest() || path),
            enumerable: true
        });
        return row;
    }

    // The folder held deepest in the launcher, or none.
    function deepest() {
        const keys = heldKeys().filter(key => key.startsWith("folder:"));
        return keys.length > 0 ? keys[keys.length - 1].slice("folder:".length) : "";
    }

    // A file's row: opened by Return, or, saving, stepped into to Replace It,
    // or, where several may be opened, marked by Return.
    function fileRow(path) {
        if (saving)
            return Files.file(path, thing, [act("verb:file-chooser:replace", path, [path])], () => []);
        if (several)
            return markingRow(path);
        return Files.file(path, thing, [act("verb:path:file:open", path, [path])]);
    }

    // A file where several may be opened: Return flips its Marked, the
    // launcher staying, as Muted is flipped (Sound.qml), and the row wears
    // the mark while it's marked; inside, Marked, then Open for it alone,
    // then Copy the Path. How it stands is read as it's drawn, so the row
    // says what Return just did without being made again (VKCF3Z,
    // 2026-10-29).
    function markingRow(path) {
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
        const row = Files.file(path, thing, [marking, act("verb:path:file:open", path, [path])]);
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
            note: count === 0 ? "Nothing is marked yet: Return on a file marks it." : "",
            stays: count === 0,
            run: () => {
                if (provider.marked.length > 0)
                    provider.pick(provider.marked.slice());
            }
        });
    }

    function act(id, path, paths) {
        const own = id === "verb:file-chooser:replace";
        return thing({
            key: `${id}:${path}`,
            name: own ? Words.name(id) : named(id),
            verb: own ? Words.verb(id) : word(id),
            glyph: Words.glyph(id),
            hue: "action",
            run: () => provider.pick(paths)
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
        const hidden = !typed.startsWith(".") ? [] : (listed[folder] ?? [])
            .filter(entry => entry.name.startsWith(".") && entry.name !== typed
                && entry.name.toLowerCase().startsWith(typed.toLowerCase())
                && (entry.folder || !folders && passes(entry.name)))
            .map(entry => entry.folder ? folderRow(entry.path) : fileRow(entry.path));
        return (naming ? [naming] : []).concat(hidden);
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
        if (inUse < 0)
            return true;
        const filter = filters[inUse];
        const kind = kindOf(name);
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
    // having been cleared to Replace It, or with none.
    function chooseToSave(fields, picked) {
        shellAsked += 1;
        const first = asking === null;
        asks = asks.concat([{
            reply: `shell:${shellAsked}`,
            request: {
                mode: "save",
                title: String(fields.title ?? ""),
                name: String(fields.name ?? ""),
                folder: String(fields.folder ?? "")
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

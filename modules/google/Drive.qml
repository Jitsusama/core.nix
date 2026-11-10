pragma Singleton
pragma ComponentBehavior: Bound
// The documents Joel looked at lately, for the Recent of Docs, Sheets and
// Slides and for search: each kind's last first, as the google program
// keeps them (src/drive.rs) and Google.qml reads them. With no account
// there are none, and no Recent shows.
//
// `row` is a Drive file's one row, the same in its program's Recent, in
// search, in the picker and in Drive's own places: its title, when he
// last opened it, its details beside it, Return opening it
// through google-opens (opens.sh) in a window of its own, its program's
// for one of Google's own kinds and Drive's for any other, and the right
// arrow listing Open and Copy the Link.
//
// And Drive's own places, for Programs › Google Drive
// (DriveProgram.qml): its Recent, Starred, Shared with Me and the shared
// drives, as the google program reads them every few minutes: `recent`
// is Drive's Recent's rows, and `placeRows` a row for each other place,
// stepping in to its files.
//
// And Drive's folders, for the launcher's picker and Files, which draw
// them: `list` asks what one holds and `listing` gives it; `download`
// asks for a file, kept in the cache home's google/drive to open or to
// hand the program that asked the picker, fetched again only once Drive
// changes it (src/files.rs), or saved where Joel picked, and
// `downloading` says how the one asked stands. Nothing is listed or
// fetched until asked.
import qs
import Quickshell
import QtQuick
import "Drive.js" as Said
import "DrivePlaces.js" as Places

Singleton {
    id: root

    // Each { id, title, kind, link, viewed, changed, owner }, and the
    // rest a Drive file says (src/drive.rs's Document).
    readonly property var documents: Google.documents

    // Drive's own places: { recent, starred, shared, drives }, Recent its
    // files newest first and each other { files, more }.
    readonly property var places: Google.drive.places
    // How much its copies kept here to open take and may: { held, most }.
    readonly property var kept: Google.drive.kept

    // Drive's folders, for the picker and Files: what each folder asked
    // lately holds, newest first, { folder, asked, files, more, failed },
    // each file { id, name, kind, mime, size, changed, owner, link, runs },
    // kind being folder, document, spreadsheet, presentation or file.
    readonly property var listings: Google.drive.listings
    // The downloads asked lately, newest first, each { file, how, asked,
    // path, failed }.
    readonly property var downloads: Google.drive.downloads

    // Asks what a folder holds: "root", "shared", "drives" or an id.
    function list(folder) {
        Google.list(folder);
    }

    // What the folder asked for at `since` holds, or null until it's said.
    function listing(folder, since) {
        return Google.listing(folder, since);
    }

    // Asks for a file to "open", "pick" or "save" (`to` where Joel picked).
    // Google's own kinds are opened by their link instead, never this way.
    function download(file, how, to) {
        Google.download(file, how, to ?? "");
    }

    // How the file asked for at `since` stands: { state, path, failed },
    // state "fetching", "done" or "failed".
    function downloading(file, since) {
        return Google.downloading(file, since);
    }

    // A Drive file's one row, keyed by Drive's id as a document's always
    // was, so a document is one row in Docs' Recent and Drive's alike.
    function row(document, provider) {
        const words = Said.kind(document);
        const name = document.title !== "" ? document.title : Words.name(words.thing, "untitled");
        const made = provider.thing({
            key: `gdoc:${document.id}`,
            what: "document",
            component: "document",
            kind: document.kind,
            name: name,
            verb: Words.verb(words.open),
            glyph: Words.glyph(words.thing, document.kind),
            hue: "program",
            side: root.side(document),
            later: true,
            run: () => root.open(document),
            verbs: () => root.acts(document, provider)
        });
        // Its drawer, made when first shown, and again once the clock has
        // moved, rather than with the row: a row is kept while it reads the
        // same (Documents.qml), so a drawer made with it would go on saying
        // the minute it was made.
        let drawn = null;
        let drawnAt = -1;
        Object.defineProperty(made, "drawer", {
            enumerable: true,
            get: () => {
                if (drawn === null || drawnAt !== Calendar.now) {
                    const details = Said.details(document, Words.name(words.thing, "untitled"),
                        Calendar.now, Mail.dates);
                    drawn = Places.drawn(details, document, Object.assign({
                        drives: root.places?.drives?.files ?? [],
                        now: Calendar.now,
                        dates: Mail.dates
                    }, root.said));
                    drawnAt = Calendar.now;
                }
                return drawn;
            }
        });
        return made;
    }

    // What a file's row says at its right: when he last opened it.
    function side(document) {
        return Said.beside(document, Calendar.now, Mail.dates, root.said.said.opened);
    }

    // Its acts, in the one order every surface lists them.
    function acts(document, provider) {
        const words = Said.kind(document);
        return [provider.thing({
            key: words.open,
            name: Words.name(words.open),
            verb: Words.verb(words.open),
            glyph: Words.glyph(words.open),
            hue: "action",
            later: true,
            run: () => root.open(document)
        }), provider.copyRow({
            key: words.link,
            name: Words.name(words.link),
            copy: document.link,
            glyph: Words.glyph(words.link),
            figure: false,
            hue: "action"
        })];
    }

    // The kind a program's desktop entry opens, `every` for Drive's own,
    // or "" for one that opens none.
    function kindOf(program) {
        return Said.kindOf(program);
    }

    // What `kindOf` says of Drive's own program, whose Recent is every
    // kind's.
    readonly property string every: Said.EVERY

    // The words a Drive file's drawer says its kind and where it is in,
    // from the vocabulary (DrivePlaces.js's drawn): { kinds, said }, said
    // naming each place by its own place's name, so the eyebrow and the
    // place Joel steps into read alike.
    readonly property var said: ({
            kinds: {
                document: Words.name("docs:document"),
                spreadsheet: Words.name("sheets:spreadsheet"),
                presentation: Words.name("slides:presentation"),
                folder: Words.name("drive:file", "folder"),
                drive: Words.name("drive:file", "drive"),
                file: Words.name("drive:file")
            },
            said: Object.assign({}, Words.of("drive:file").said ?? {}, {
                mine: Words.name("drive:mine"),
                sharedWithMe: Words.name("drive:place:shared"),
                drives: Words.name("drive:place:drives")
            })
        })

    // Drive's Recent, its files of every kind newest first, each its one
    // row.
    function recent(provider) {
        return (root.places?.recent ?? []).map(file => root.row(file, provider));
    }

    // Each of Drive's other places holding anything, as a row saying how
    // many it holds that steps in to each, as Recent does.
    function placeRows(provider) {
        const over = key => Words.of(key).said?.more ?? "{count}";
        return Places.listed(root.places, over).map(place => provider.thing({
                key: place.key,
                name: Words.name(place.key),
                verb: Words.verb(place.key),
                glyph: Words.glyph(place.key),
                hue: "program",
                side: place.count,
                ask: "which?",
                nested: true,
                targets: () => place.files.map(file => root.row(file, provider))
            }));
    }

    // What Google Drive's drawer in Programs says it holds: each of its
    // places holding anything, by its name, and how many, as its row says
    // beside it (VKCF3Z, 2026-11-08: a place's row is another row, so it
    // isn't said twice), then what its copies kept here take, Kept Here.
    function placeFacts() {
        const over = key => Words.of(key).said?.more ?? "{count}";
        const kept = Places.kept(root.kept, Words.of("drive:kept").said);
        return Places.listed(root.places, over)
            .map(place => ({ label: Words.name(place.key), value: place.count }))
            .concat(kept === "" ? [] : [{ label: Words.name("drive:kept"), value: kept }]);
    }

    function open(document) {
        if (document && document.link)
            Niri.spawn(["google-opens", document.link]);
    }

    // Google Drive as a place in the launcher's picker (open-with's
    // FileChooser.qml, which asks each provider's chooserPlaces as it
    // opens): one Google Drive folder, holding My Drive, Shared with Me,
    // and Shared Drives once Drive says he has any. None while signed out,
    // or while a folder is asked. Saving, it holds My Drive and Shared
    // Drives alone, since another's folder takes nothing of his by
    // default, and each folder only its folders; a file saved in one is
    // staged in the state home's google/drive-saves/<folder>/<name>, which
    // the google program uploads once it's written (VKCF3Z and 427IH3,
    // 2026-10-30). Each opening starts afresh, so what's listed is never
    // older than the picker.
    function chooserPlaces(chooser, provider) {
        if (Google.account !== "ready" || chooser.folders)
            return [];
        root.saving = chooser.saving;
        root.chooser = chooser;
        root.provider = provider;
        root.shown = {};
        root.asked = {};
        return [chooser.folder({
                key: "drive",
                name: Words.name("drive:drive"),
                note: "",
                inside: () => root.tops(),
                empty: () => ""
            })];
    }

    // The picker last opened, and the provider its document rows are made by.
    property var chooser: null
    // Whether it opened to save.
    property bool saving: false
    // Where a file saved into Drive waits to be uploaded, a folder each
    // folder of Drive's, by its id.
    readonly property string saves: `${Quickshell.env("XDG_STATE_HOME")
        || `${Quickshell.env("HOME")}/.local/state`}/google/drive-saves`
    property var provider: null
    // What each folder listed since it opened, by its id: { files } or
    // { failed }. The program keeps only the last listing, and the
    // launcher asks each held step again whenever one comes, so the folders
    // above the one stepped into answer from here rather than asking Drive
    // again.
    property var shown: ({})
    // The folders asked for since it opened, each asked once: when it was
    // asked while its listing is awaited, so one from before isn't taken
    // for it, then false.
    property var asked: ({})
    // The files fetched for the picker that it still waits on, each
    // { file, since, done, failed }.
    property var fetches: []

    function tops() {
        const drives = root.holds("drives");
        const place = (folder, word) => root.chooser.folder(root.savedIn(folder, {
                key: `drive:folder:${folder}`,
                name: Words.name(word),
                note: "",
                inside: () => root.inside(folder, Words.name(word)),
                empty: () => root.empty(folder)
            }));
        return [place("root", "drive:mine")].concat(root.saving ? [] : [place("shared", "drive:shared")])
            .concat(drives?.files?.length > 0 ? [place("drives", "drive:drives")] : []);
    }

    // A folder's fields, and, saving, where a file named so is staged and
    // the names its files have, so the picker asks before one is replaced.
    // Shared Drives itself holds only drives, so nothing is saved in it.
    function savedIn(folder, fields) {
        if (!root.saving || folder === "drives")
            return fields;
        return Object.assign(fields, {
            saveTo: name => `${root.saves}/${folder}/${name}`,
            names: () => (root.shown[folder]?.files ?? []).filter(file => file.kind !== "folder")
                .map(file => file.name)
        });
    }

    // What a folder holds, asking Drive the first time, once the picker
    // has made its rows; null until it's said. Each ask is a closure of
    // its own, since Qt.callLater runs one function once a turn, with the
    // last arguments it was given.
    function holds(folder) {
        if (!(folder in root.asked)) {
            root.asked[folder] = Date.now();
            Qt.callLater(() => Google.list(folder));
        }
        return root.shown[folder] ?? null;
    }

    // A folder's rows: its folders, stepped into, then the files the
    // program that asked would take, each picked by fetching it. `where`
    // is the folders it's in and itself, from its place, as each row's
    // plate says it.
    function inside(folder, where) {
        const files = root.holds(folder)?.files ?? [];
        return files.filter(file => file.kind === "folder" || !root.saving
            && root.chooser.wants(Said.pickedAs(file).name, Said.pickedAs(file).mime))
            .map(file => root.pickable(file, where));
    }

    // Where a file the picker lists is, as its eyebrow says it before its
    // kind, in the words Drive's own drawer says it in: "in My Drive/Trips".
    function within(where) {
        return where === "" ? "" : root.said.said.in.replace("{place}", where);
    }

    // A file or a folder in Drive as the picker draws one here, its row
    // and plate saying what Drive listed (Drive.js's placed). Normal's gx
    // opens a file as Return opens it in Drive's own places: one of
    // Google's own kinds by its link, in its program's window, and any
    // other fetched into the cache and opened by the desktop's program
    // for its kind, never one that runs.
    function pickable(file, where) {
        const facts = Said.placed(file);
        if (file.kind === "folder")
            return root.chooser.folder(root.savedIn(file.id, {
                key: facts.key,
                name: facts.name,
                note: facts.note,
                entry: facts.entry,
                where: where,
                within: root.within(where),
                facts: facts.facts,
                inside: () => root.inside(file.id, `${where}/${file.name}`),
                empty: () => root.empty(file.id)
            }));
        const document = Said.asDocument(file);
        return root.chooser.file({
            key: facts.key,
            name: facts.name,
            note: facts.note,
            entry: facts.entry,
            where: where,
            within: root.within(where),
            facts: facts.facts,
            adopt: document === null ? null : root.row(document, root.provider),
            fetch: (done, failed) => root.fetch(file.id, done, failed, "pick"),
            open: (done, failed) => {
                if (document === null) {
                    root.fetch(file.id, done, failed, "open");
                    return;
                }
                root.open(document);
                done("");
            }
        });
    }

    // What a folder with no rows says: that it's being listed, why it
    // couldn't be, or nothing once it's listed and empty.
    function empty(folder) {
        const listed = root.shown[folder];
        return Said.unlisted(listed === undefined ? "listing" : listed.failed ?? "");
    }

    // Fetches a file to "pick" or to "open", calling `done` with where it
    // is or `failed` with why not.
    function fetch(file, done, failed, how) {
        root.fetches = root.fetches.concat([{ file, since: Date.now(), done, failed }]);
        Google.download(file, how, "");
    }

    // Each folder awaited is kept once the program lists it, from among
    // the folders it listed lately, so two asked together are each had,
    // and the picker told, so it asks its steps again; each fetch it waits
    // on is settled once the program says where the file is or why not,
    // after this change, since the picker may close the launcher on it.
    function heard() {
        let came = false;
        for (const [folder, since] of Object.entries(root.asked)) {
            const listed = since === false ? null : Google.listing(folder, since);
            if (listed !== null) {
                root.shown[folder] = listed.failed ? { failed: listed.failed }
                    : { files: listed.files };
                root.asked[folder] = false;
                came = true;
            }
        }
        if (came && root.chooser !== null)
            Qt.callLater(root.chooser.changed);
        const waiting = [];
        for (const each of root.fetches) {
            const stands = Google.account === "ready" ? Google.downloading(each.file, each.since)
                : { state: "failed", path: "", failed: "unreachable" };
            if (stands.state === "done")
                Qt.callLater(() => each.done(stands.path));
            else if (stands.state === "failed")
                Qt.callLater(() => each.failed(Said.unfetched(stands.failed)));
            else
                waiting.push(each);
        }
        if (waiting.length !== root.fetches.length)
            root.fetches = waiting;
    }

    Connections {
        target: Google

        function onDriveChanged() {
            root.heard();
        }

        function onAccountChanged() {
            root.heard();
        }
    }
}

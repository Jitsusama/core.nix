pragma Singleton
pragma ComponentBehavior: Bound
// The documents Joel looked at lately, for the Recent of Docs, Sheets and
// Slides and for search: each kind's last first, as the google program
// keeps them (src/drive.rs) and Google.qml reads them. With no account
// there are none, and no Recent shows.
//
// `row` is a document's one row, the same in its program's Recent and in
// search: its title, who owns it and when he last opened it, its details
// beside it, Return opening it through google-opens (opens.sh) in a
// window of its own, and the right arrow listing Open and Copy the Link.
//
// And Drive's folders, for the launcher's picker and Files, which draw
// them: `list` asks what one holds and `listed` is the answer; `download`
// asks for a file, kept in the session's runtime folder to open or to
// hand the program that asked the picker, or saved where Joel picked, and
// `downloading` says how the one asked stands. Nothing is listed or
// fetched until asked.
import qs
import Quickshell
import QtQuick
import "Drive.js" as Said

Singleton {
    id: root

    // Each { id, title, kind, link, viewed, changed, owner }.
    readonly property var documents: Google.documents

    // Drive's folders, for the picker and Files: what the folder last
    // asked holds, { folder, asked, files, more, failed }, each file
    // { id, name, kind, mime, size, changed, owner, link, runs }, kind
    // being folder, document, spreadsheet, presentation or file; or null.
    readonly property var listed: Google.drive.listed
    // The downloads asked lately, newest first, each { file, how, asked,
    // path, failed }.
    readonly property var downloads: Google.drive.downloads

    // Asks what a folder holds: "root", "shared", "drives" or an id.
    function list(folder) {
        Google.list(folder);
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

    function row(document, provider) {
        const words = Said.kind(document);
        const name = document.title !== "" ? document.title : Words.name(words.thing, "untitled");
        return provider.thing({
            key: `gdoc:${document.id}`,
            what: "document",
            component: "document",
            kind: document.kind,
            name: name,
            verb: Words.verb(words.open),
            glyph: Words.glyph(words.thing),
            hue: "program",
            side: Said.beside(document, Calendar.now),
            drawer: Said.details(document, Words.name(words.thing, "untitled")),
            later: true,
            run: () => root.open(document),
            verbs: () => root.acts(document, provider)
        });
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

    // The kind a program's desktop entry opens, or "" for one that opens
    // none.
    function kindOf(program) {
        return Said.kindOf(program);
    }

    function open(document) {
        if (document && document.link)
            Niri.spawn(["google-opens", document.link]);
    }

    // Google Drive as a place in the launcher's picker (open-with's
    // FileChooser.qml, which asks each provider's chooserPlaces as it
    // opens): one Google Drive folder, holding My Drive, Shared with Me,
    // and Shared Drives once Drive says he has any. None while signed out,
    // while a save is asked, or while a folder is. Each opening starts
    // afresh, so what's listed is never older than the picker.
    function chooserPlaces(chooser, provider) {
        if (Google.account !== "ready" || chooser.saving || chooser.folders)
            return [];
        root.chooser = chooser;
        root.provider = provider;
        root.opened = Date.now();
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
    property var provider: null
    // When it opened, so a listing asked before then isn't taken for one
    // asked since.
    property double opened: 0
    // What each folder listed since it opened, by its id: { files } or
    // { failed }. The program keeps only the last listing, and the
    // launcher asks each held step again whenever one comes, so the folders
    // above the one stepped into answer from here rather than asking Drive
    // again.
    property var shown: ({})
    // The folders asked for since it opened, each asked once.
    property var asked: ({})
    // The files fetched for the picker that it still waits on, each
    // { file, since, done, failed }.
    property var fetches: []

    function tops() {
        const drives = root.listing("drives");
        const place = (folder, word) => root.chooser.folder({
                key: `drive:folder:${folder}`,
                name: Words.name(word),
                note: "",
                inside: () => root.inside(folder),
                empty: () => root.empty(folder)
            });
        return [place("root", "drive:mine"), place("shared", "drive:shared")]
            .concat(drives?.files?.length > 0 ? [place("drives", "drive:drives")] : []);
    }

    // What a folder holds, asking Drive the first time, once the picker
    // has made its rows; null until it's said. Each ask is a closure of
    // its own, since Qt.callLater runs one function once a turn, with the
    // last arguments it was given.
    function listing(folder) {
        if (!(folder in root.asked)) {
            root.asked[folder] = true;
            Qt.callLater(() => Google.list(folder));
        }
        return root.shown[folder] ?? null;
    }

    // A folder's rows: its folders, stepped into, then the files the
    // program that asked would take, each picked by fetching it.
    function inside(folder) {
        const files = root.listing(folder)?.files ?? [];
        return files.filter(file => file.kind === "folder"
            || root.chooser.wants(Said.pickedAs(file).name, Said.pickedAs(file).mime))
            .map(file => root.pickable(file));
    }

    function pickable(file) {
        const facts = Said.placed(file, Calendar.now);
        if (file.kind === "folder")
            return root.chooser.folder({
                key: facts.key,
                name: facts.name,
                note: facts.note,
                inside: () => root.inside(file.id),
                empty: () => root.empty(file.id)
            });
        const document = Said.asDocument(file);
        return root.chooser.file({
            key: facts.key,
            name: facts.name,
            note: facts.note,
            drawer: facts.drawer,
            adopt: document === null ? null : root.row(document, root.provider),
            fetch: (done, failed) => root.fetch(file.id, done, failed)
        });
    }

    // What a folder with no rows says: that it's being listed, why it
    // couldn't be, or nothing once it's listed and empty.
    function empty(folder) {
        const listed = root.shown[folder];
        return Said.unlisted(listed === undefined ? "listing" : listed.failed ?? "");
    }

    function fetch(file, done, failed) {
        root.fetches = root.fetches.concat([{ file, since: Date.now(), done, failed }]);
        Google.download(file, "pick", "");
    }

    // Each listing asked since the picker opened is kept as it comes, and
    // the picker told, so it asks its steps again; each fetch it waits on
    // is settled once the program says where the file is or why not, after
    // this change, since the picker may close the launcher on it.
    function heard() {
        const listed = Google.drive.listed;
        if (root.chooser !== null && listed && listed.asked >= root.opened
                && root.asked[listed.folder] === true) {
            root.shown[listed.folder] = listed.failed ? { failed: listed.failed }
                : { files: listed.files };
            root.asked[listed.folder] = false;
            Qt.callLater(root.chooser.changed);
        }
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

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
// them: `list` asks what one holds and `listing` gives it; `download`
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
}

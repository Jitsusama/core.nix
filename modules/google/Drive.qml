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
import qs
import Quickshell
import QtQuick
import "Drive.js" as Said

Singleton {
    id: root

    // Each { id, title, kind, link, viewed, changed, owner }.
    readonly property var documents: Google.documents

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

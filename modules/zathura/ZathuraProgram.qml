// zathura's own in the launcher's Programs › zathura. Each document a
// zathura has open leads its Recent, as every program's open files do
// (Applications.qml), and here says the page it's on in open's place,
// "p. 41 of 212" before its folder, Return going to its window: open-now
// reads each zathura's document, page and page count over its bus as
// zathura is stepped into or listed under its row, once each time the
// launcher opens, never on a timer, and this hands each page to the row
// through Files.saying. Its drawer (`programFacts`, which Applications.qml
// asks every provider that has it) says each document open by its name
// and how far through it is, "p. 41 of 212 · 19% through".
//
// Its act Read the Copied Document is there only while the clipboard holds
// the path of an existing document zathura reads, said beside it
// (Copied.qml), and gone otherwise rather than dimmed; it's found from the
// top by its name too, as the reader's Read the Copied Markdown is. A page
// after the path, as pdfgrep's "report.pdf:12:" says one, opens it there.
// A copied web address is left out: zathura opens one only through GVfs,
// which the machine doesn't have.
import qs
import QtQuick
import "../Copied.js" as Copying
import "Zathura.js" as Zathura

Provider {
    id: provider

    // zathura's desktop entry, whose program these are given to.
    readonly property string program: "org.pwmt.zathura"

    // The existing document's path the clipboard holds, or null.
    readonly property var place: Copied.place !== null && Copying.document(Copied.place.path)
        ? Copied.place : null
    // So Programs › zathura, held while it's copied, draws its act.
    onPlaceChanged: changed()

    foundActs: place !== null ? [{ program: provider.program, key: "verb:zathura:copied" }] : []

    // The documents each zathura has open, as open-now last said them, each
    // { pid, path, page, pages }.
    property var open: []

    // The words its pages are said in.
    readonly property var said: Words.of("zathura:reading").said ?? {}

    // Whether zathura's own step is held, from Programs or from the top, or
    // its row is listed with what it has open under it, and whether its
    // documents were read since the launcher opened.
    readonly property bool lookedIn: (launcher?.open ?? false)
        && ((launcher?.held ?? []).some(step => step.row?.key === `program:${provider.program}`)
            || launcher?.underListed === `program:${provider.program}`)
    property bool lookedSince: false

    onLookedInChanged: {
        if (lookedIn)
            lookedSince = true;
    }

    // The next opening reads them again, as the pages may have turned since.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.lookedSince = false;
        }
    }

    // What open-now printed of each zathura, as the programs daemon holds
    // it under the key its settings name, from the first look in an
    // opening, so the shell starts no process for it (Listed.qml).
    Listed {
        id: listing

        key: provider.settings.opened ?? ""
        wanted: provider.lookedSince

        onHeard: {
            provider.open = Zathura.opened(listing.lines);
            Files.saying(provider.program, Zathura.through(provider.open, provider.said));
            provider.changed();
        }
    }

    function programActs(asked) {
        if (asked !== program || place === null)
            return [];
        const key = "verb:zathura:copied";
        return [thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: Copying.placeSaid(place, Files.home),
            // What's copied when it runs, which a row found from the top,
            // made as the act came, may not have changed with.
            run: () => {
                const now = provider.place;
                if (now === null)
                    return;
                const page = now.line > 0 ? [`--page=${now.line}`] : [];
                Niri.spawn([provider.settings.zathura ?? "zathura"].concat(page, [now.path]));
            }
        })];
    }

    // What its drawer in Programs says of what it has open, from what
    // open-now said when zathura was last stepped into.
    function programFacts(asked) {
        if (asked !== program)
            return [];
        return Zathura.facts(open, said, path => Files.named(path));
    }
}

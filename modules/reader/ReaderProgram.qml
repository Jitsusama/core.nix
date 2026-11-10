// The reader's own in the launcher's Programs › Reader. Its Recent is
// every file the reader kept a place in (`recentOf`, which Applications.qml
// asks every provider that has it), each the one row every surface shows
// of a file (Files.qml), keyed by its path as Applications keys a file, so
// the same file from the list programs share is listed once, and dated by
// when it was left (`at`), so Recent orders it among the rest, newest
// first (Recent.js). Beside it, where it was left and when, as
// "Installing · 40% · 14:32", or "read to the end · yesterday"; Return
// reads on there, as the reader opens a file where it was left by itself,
// and inside, each mark kept in it by its letter, which opens it there
// (reader --at). Its drawer (Reader.js's plate) says the heading, how far
// down, each mark and its path. reader --places lists them as the reader
// is stepped into, once each time the launcher opens, never on a timer
// or a key, so the launcher never reads the folder of places itself. A
// file gone since is left out by the reader as it lists them.
//
// Its act Read the Copied Markdown (programActs), found from the top by
// its name too (foundActs), is there only while the clipboard holds an
// existing markdown file's path, said beside it (Copied.qml), which it
// opens as a page. The reader takes a file alone, so a line copied after
// the path is said but not gone to.
import qs
import QtQuick
import "../Copied.js" as Copying
import "Reader.js" as Reader

Provider {
    id: provider

    // The desktop entry the reader's program row is made from.
    readonly property string program: "reader"

    // The existing markdown file's path the clipboard holds, or null.
    readonly property var place: Copied.place !== null && Copying.markdown(Copied.place.path)
        ? Copied.place : null
    // So Programs › Reader, held while it's copied, draws its act.
    onPlaceChanged: changed()

    foundActs: place !== null ? [{ program: program, key: "verb:reader:markdown" }] : []

    // The files the reader left, as reader --places last listed them, each
    // { path, at, end, heading, through, marks }.
    property var left: []

    // What its rows and drawers say, the launcher's words, and days and
    // months as the locale names them, as the launcher says when
    // everywhere (Y2YBTS, 2026-11-03).
    readonly property var words: Reader.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))

    // Whether the reader's own step is held, from Programs or from the
    // top, and whether its places were read since the launcher opened.
    readonly property bool steppedIn: (launcher?.open ?? false)
        && (launcher?.held ?? []).some(step => step.row?.key === `program:${provider.program}`)
    property bool lookedSince: false

    onSteppedInChanged: {
        if (steppedIn)
            lookedSince = true;
    }

    // The next opening reads them again, as they may have changed since.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.lookedSince = false;
        }
    }

    // What reader --places printed, as the programs daemon holds it under
    // the key its settings name, from the first step in an opening, so the
    // shell starts no process for it (Listed.qml).
    Listed {
        id: listing

        key: provider.settings.places ?? ""
        wanted: provider.lookedSince

        onHeard: {
            provider.left = Reader.places(listing.lines);
            provider.changed();
        }
    }

    function programActs(asked) {
        if (asked !== program || place === null)
            return [];
        const key = "verb:reader:markdown";
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
                if (provider.place !== null)
                    Niri.spawn([provider.settings.reader ?? "reader", provider.place.path]);
            }
        })];
    }

    function recentOf(asked) {
        return asked === program ? left.map(leftRow) : [];
    }

    // What its drawer in Programs says is still to finish (Applications.qml
    // asks every provider's programFacts), from what reader --places said
    // when the reader was last stepped into.
    function programFacts(asked) {
        if (asked !== program)
            return [];
        return Reader.held(left, words.said)
            .map(fact => ({ label: Words.name(fact.id), value: fact.value }));
    }

    // What reads a file in the reader, where it was left or at a mark.
    function reading(path, letter) {
        return Reader.reading(provider.settings.reader ?? "reader", path, letter);
    }

    // Read On, or one mark's act, by its key in the launcher's words.
    function act(key, one, fields) {
        return thing(Object.assign({
            key: `${key}:${one.path}`,
            wordsId: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action"
        }, fields));
    }

    // One file the reader left, as its Recent's row: Return reads on where
    // it was left, as Vim's :edit comes back to a file's last place, and
    // each mark it holds reads from there, by its letter, as :edit 'a on
    // the line. What the shared list's row
    // says of it, its folder, its drawer's eyebrow says instead, so its
    // side has room for where and when.
    function leftRow(one) {
        const said = words.said;
        const on = act("verb:reader:on", one, {
            side: Reader.whereLeft(one, said),
            run: () => Niri.spawn(provider.reading(one.path, ""))
        });
        const marks = one.marks.map(mark => act("verb:reader:mark", one, {
            key: `verb:reader:mark:${mark.letter}:${one.path}`,
            name: Reader.markName(Words.name("verb:reader:mark"), mark.letter),
            side: Reader.where(mark, said),
            fill: { letter: mark.letter },
            run: () => Niri.spawn(provider.reading(one.path, mark.letter))
        }));
        const row = Files.file(one.path, thing, [on].concat(marks));
        const shown = {
            name: row.name,
            folder: Files.within(one.path),
            path: Files.shown(one.path)
        };
        return Object.assign(row, {
            side: Reader.side(one, Date.now(), words),
            at: one.at,
            drawer: Reader.plate(one, shown, words)
        });
    }
}

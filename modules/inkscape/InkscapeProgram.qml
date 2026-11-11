// Inkscape's own in the launcher's Programs › Inkscape. Its acts
// (`programActs`, which Applications.qml asks every provider that has
// it), each found from the top by its name too (`foundActs`): New
// Drawing opens Inkscape on a fresh copy of the account's template, the
// one Inkscape starts a new drawing from (home.nix), so the template is
// never what's drawn on, and New Drawing From the Copied Picture, there
// only while the clipboard holds an SVG, or a picture whose size its
// history keeps (Inkscape.js's copied), opens one made from it. Each is
// a file in the drawings folder, named for when it was made, so saving
// it in Inkscape keeps it there.
//
// Its Recent is the drawings Inkscape opened lately (`recentOf`), from
// the desktop's list of files opened lately, which Inkscape adds to as it
// opens and saves each: inkscape-drawings (drawings.sh) says which are
// still there, with each one's weight and size, run and held by the
// programs daemon under the key its settings name (Listed.qml), so the
// shell reads no file and starts no process for it (Joel, 2026-11-10: the
// daemons are the data layer). It's read once as the shell starts, so
// Inkscape's drawer in Programs says them, and again from the first step
// into Inkscape in each opening, so one deleted since is gone, and never
// on a timer or a key. The page still makes its rows from the lines, a
// stopgap until the programs daemon serves Programs' ready rows (Thu
// 11-12). Each is the one row every surface shows of a file (Files.qml), keyed by
// its path, so the list programs share lists it once; its side says when
// Inkscape last opened it, and its drawer (Inkscape.js's plate) where it
// is, its size as its SVG says it, its weight and its path. Return opens
// it in Inkscape. Its drawer in Programs says the last one and how many
// (`programFacts`).
import qs
import Quickshell
import QtQuick
import "Inkscape.js" as Inkscape

Provider {
    id: provider

    // Inkscape's desktop entry, whose program these are given to.
    readonly property string program: "org.inkscape.Inkscape"
    // As many as Recent lists (Recent.js).
    readonly property int most: 9

    // The clipboard's history, as the launcher's copies keep it, whose
    // newest copy is what the clipboard holds while it holds a picture,
    // as GIMP's act reads it (CopiedPicture.qml).
    readonly property var copies: (launcher?.sources ?? [])
        .find(source => source.what === "copy" && Array.isArray(source.all)) ?? null
    // What a new drawing can be made from now, or null.
    readonly property var copy: Inkscape.copied(Copied.types, Copied.picture,
        copies?.all[0] ?? null)
    // So Programs › Inkscape, held as something is copied, gains it or
    // loses it.
    onCopyChanged: changed()

    foundActs: [{ program: provider.program, key: "verb:inkscape:drawing" }]
        .concat(copy === null ? [] : [{ program: provider.program, key: "verb:inkscape:copied" }])

    // What its rows and drawers say, the launcher's words, and days and
    // months as the locale names them, as the launcher says when
    // everywhere (Y2YBTS, 2026-11-03).
    readonly property var words: Inkscape.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))

    // The drawings Inkscape opened lately still there, as
    // inkscape-drawings said them, each { path, at, size, extent }, newest
    // first.
    property var drawings: []

    // Whether the launcher shows; the next opening reads them again, as
    // they may have changed since.
    readonly property bool showing: launcher?.open ?? false

    onShowingChanged: {
        if (!showing)
            lookedSince = false;
    }

    // Whether Inkscape's own step is held, from Programs or from the top,
    // and whether its drawings were looked at since the launcher opened.
    readonly property bool steppedIn: showing
        && (launcher?.held ?? []).some(step => step.row?.key === `program:${provider.program}`)
    property bool lookedSince: false

    onSteppedInChanged: {
        if (steppedIn)
            lookedSince = true;
    }

    // What inkscape-drawings printed, as the programs daemon holds it,
    // asked for as the shell starts and from the first step in an opening.
    Listed {
        id: listing

        key: provider.settings.lately ?? ""
        wanted: provider.lookedSince || !listing.read

        onHeard: {
            provider.drawings = Inkscape.drawings(listing.lines, provider.most);
            provider.changed();
        }
    }

    // What its drawings are written with and opened by, from home.nix.
    function tools() {
        return {
            folder: settings.drawings ?? "",
            template: settings.template ?? "",
            inkscape: settings.inkscape ?? "inkscape",
            paste: settings.paste ?? "wl-paste",
            border: settings.border ?? "",
            desk: settings.desk ?? ""
        };
    }

    // When a new drawing is made, as its file is named.
    function stamp() {
        return Qt.formatDateTime(new Date(), "yyyy-MM-dd HH.mm.ss");
    }

    // One of its acts, by its key in the launcher's words.
    function act(key, fields) {
        return thing(Object.assign({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action"
        }, fields));
    }

    function programActs(asked) {
        if (asked !== program)
            return [];
        const fresh = act("verb:inkscape:drawing", {
            run: () => {
                const made = provider.tools();
                Niri.spawn(Inkscape.fresh(Inkscape.fileFor(made.folder, false, provider.stamp()),
                    made));
            }
        });
        if (copy === null)
            return [fresh];
        return [fresh, act("verb:inkscape:copied", {
            side: Inkscape.copiedSaid(copy, words.said),
            run: () => provider.fromCopied()
        })];
    }

    // Makes a drawing of what the clipboard holds as it runs, which a row
    // found from the top, made as the act came, may not have changed with.
    function fromCopied() {
        const now = Inkscape.copied(Copied.types, Copied.picture, copies?.all[0] ?? null);
        if (now === null)
            return;
        const made = tools();
        Niri.spawn(Inkscape.fromCopy(now, Inkscape.fileFor(made.folder, true, stamp()), made));
    }

    // So its Recent stands, saying none, before Inkscape opens anything.
    function keepsRecentOf(asked) {
        return asked === program;
    }

    function recentOf(asked) {
        return asked === program ? drawings.map(drawingRow) : [];
    }

    // What its drawer in Programs says of its drawings lately
    // (Applications.qml asks every provider's programFacts), from what was
    // read last, never a read of its own.
    function programFacts(asked) {
        if (asked !== program)
            return [];
        return Inkscape.held(drawings, Date.now(), words)
            .map(fact => ({ label: Words.name(fact.id), value: fact.value }));
    }

    // One drawing, as its Recent's row: Return opens it in Inkscape. What
    // the shared list's row says of it, its folder, its drawer's eyebrow
    // says instead, so its side has room for when.
    function drawingRow(one) {
        const key = "verb:path:file:open";
        const open = thing({
            key: `${key}:${one.path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            run: () => Niri.spawn([provider.tools().inkscape, one.path])
        });
        const row = Files.file(one.path, thing, [open]);
        const shown = {
            name: row.name,
            folder: Files.within(one.path),
            path: Files.shown(one.path)
        };
        return Object.assign(row, {
            side: Inkscape.side(one, Date.now(), words),
            at: one.at,
            drawer: Inkscape.plate(one, shown, words)
        });
    }
}

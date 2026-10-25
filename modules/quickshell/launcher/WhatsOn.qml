// What's on, a collection, and Super+O's: a row for each glyph in the
// margin (Margin.qml), saying in words what the glyph only pictures. Each
// row is the glyph in the colour the margin gives it, the piece's name, and
// how it stands at the right, with what more there is to know in the footer
// while it's picked. They're listed by name, as every list in the launcher
// is, so one keeps its place however many are on; what each asks of Joel
// shows in its glyph's colour. Return does a piece's first action, and the
// right arrow steps into the rest.
//
// It holds nothing of its own: the pieces are the margin's (Facts.qml), so
// a glyph and its row come and go together, and it adds no row to a search.
// It's home's first row, Now, saying what's in it by name.
import qs
import QtQuick
import "../Marks.js" as Marks

Provider {
    id: provider

    what: "on"

    // Named Now, as the vocabulary says (Words.qml), its old name a hidden
    // word, under the calendar clock.
    collection: Words.collection("on", { searched: false })

    // The kinds of piece it lists: what wants Joel, what's on air, and
    // what was left on.
    readonly property var groups: ["wants", "onAir", "leftOn"]
    // What the thing is where the launcher has a row of its own for it, by
    // its mark, so one found in both is listed once (Rank.identity).
    readonly property var same: ({
        awake: "session:awake",
        quiet: "session:quiet",
        recording: "session:recording"
    })

    readonly property var all: {
        const lit = Facts.all.filter(fact => fact.entry.mark !== null && fact.piece
            && fact.piece.present && fact.piece.active);
        const rows = [];
        // By name, as every list the launcher holds is, so each keeps one
        // place however many are on (RSCH-20261025-U7566F); what each asks
        // of Joel is its glyph's colour, not its place.
        for (const kind of groups)
            lit.filter(fact => fact.piece.kind === kind).forEach(fact => rows.push(made(fact)));
        return rows.sort((a, b) => a.name.localeCompare(b.name, undefined,
            { numeric: true, sensitivity: "base" }));
    }
    onAllChanged: changed()

    // The launcher, as a piece's actions are given it, so one that opens a
    // collection opens it here.
    readonly property var shell: ({ launcher: provider.launcher })

    function made(fact) {
        const piece = fact.piece;
        const actions = piece.actions.map(action => thing({
            key: `verb:on:${fact.entry.mark}:${action.name}`,
            name: action.name,
            glyph: action.glyph,
            hue: "action",
            chord: action.chord ?? [],
            run: () => action.run(provider.shell)
        }));
        return member({
            key: `on:${fact.entry.mark}`,
            is: provider.same[fact.entry.mark] ?? `on:${fact.entry.mark}`,
            // What Return does, said in the foot: the first action's own
            // verb where it gives one, else its name ("let it sleep").
            verb: piece.actions.length === 0 ? "close"
                : piece.actions[0].verb ?? piece.actions[0].name.toLowerCase(),
            name: piece.title,
            side: piece.value,
            note: piece.context,
            glyph: Theme.glyph[fact.entry.mark],
            hue: Marks.tone(piece),
            plain: actions.length === 0,
            verbs: actions.length > 1 ? () => actions.slice(1) : null,
            run: () => {
                if (actions.length > 0)
                    actions[0].run();
            }
        });
    }

    function everything() {
        return all.slice();
    }

    function counted() {
        return all.length === 0 ? "nothing" : String(all.length);
    }
}

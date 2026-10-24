// What's on, a collection, and Super+O's: a row for each glyph in the
// margin (Margin.qml), saying in words what the glyph only pictures. Each
// row is the glyph in the colour the margin gives it, the piece's name, and
// how it stands at the right, with what more there is to know in the footer
// while it's picked. They're grouped by how much they ask, what
// wants Joel first, so Super+O then Return deals with it, then what's on
// air, then what was left on, each group a step apart. Return does a
// piece's first action, and the right arrow steps into the rest.
//
// It holds nothing of its own: the pieces are the margin's (Facts.qml), so
// a glyph and its row come and go together, and it adds no row to a search
// or to home, where the same things are found by their own words.
import qs
import QtQuick
import "../Marks.js" as Marks

Provider {
    id: provider

    what: "on"

    collection: ({
        noun: "what's on",
        title: "What's on",
        words: ["status", "margin"],
        glyph: String.fromCodePoint(0xf1513),
        ask: "which?",
        empty: "Nothing's on or waiting.",
        searched: false
    })

    // How the groups follow one another, top to bottom.
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
        for (const kind of groups) {
            const these = lit.filter(fact => fact.piece.kind === kind)
                .sort((a, b) => a.entry.order - b.entry.order);
            these.forEach((fact, index) => rows.push(made(fact, index === 0 && rows.length > 0)));
        }
        return rows;
    }
    onAllChanged: changed()

    // The launcher, as a piece's actions are given it, so one that opens a
    // collection opens it here.
    readonly property var shell: ({ launcher: provider.launcher })

    function made(fact, gap) {
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
            // What Return does, said in the foot as the action says it when
            // that's short ("let it sleep", "unmute it"), else its verb.
            verb: actions.length === 0 ? "close"
                : actions[0].name.split(" ").length <= 3 ? actions[0].name.toLowerCase()
                : actions[0].name.toLowerCase().split(" ")[0],
            name: piece.title,
            side: piece.value,
            note: piece.context,
            glyph: Theme.glyph[fact.entry.mark],
            hue: Marks.tone(piece),
            gap: gap,
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

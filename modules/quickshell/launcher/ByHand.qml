// The theme for programs that only take one by hand, such as Slack, whose
// colours live with the account rather than in a file: a part of its
// program, found inside it ("Theme"), or from anywhere by a word past the
// program's name ("slack theme"), so the program's name alone lands on the
// program. Return copies it, and the row says where it's pasted. Its
// settings: `programs`, each { app, side, copy }, `app` naming the program
// and its glyph and `copy` the theme as the program reads it, rendered from
// the theme at build; its name and words are the vocabulary's
// (`byhand:<app>`, Words.qml).
import qs
import QtQuick

Provider {
    id: provider

    things: (settings.programs ?? []).map(program => {
        const key = `byhand:${program.app}`;
        // A machine's own program the vocabulary doesn't name keeps the
        // name and words its settings give.
        const said = Words.of(key);
        const known = Words.things[key] !== undefined;
        return copyRow({
            key: key,
            name: known ? said.name : program.name ?? program.app,
            keywords: known ? "" : program.keywords ?? "",
            // The program's row it's a part of, and its name there.
            of: said.of ?? `program:${program.app}`,
            part: said.part ?? said.name,
            side: program.side,
            copy: program.copy,
            glyph: provider.programGlyph(program.app),
            hue: "program",
            // A theme to paste is an act, not a figure, so it keeps its glyph.
            figure: false
        });
    })
}

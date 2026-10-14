// The theme for programs that only take one by hand, such as Slack, whose
// colours live with the account rather than in a file: typing the program's
// name finds its theme, Return copies it, and the row says where it's
// pasted. Its settings: `programs`, each { name, app, keywords, side, copy },
// `app` naming the program's glyph and `copy` the theme as the program
// reads it, rendered from the theme at build.
import QtQuick

Provider {
    id: provider

    things: (settings.programs ?? []).map(program => copyRow({
        key: `byhand:${program.app}`,
        name: program.name,
        keywords: program.keywords,
        side: program.side,
        copy: program.copy,
        glyph: provider.programGlyph(program.app),
        hue: "program"
    }))
}

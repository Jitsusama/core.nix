// The cases of colours for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Colours.js" as Colours
.import "../colour-cases.js" as ColourCases

function run(failures) {
    let asked = 0;
    const quoted = Common.quoted;
    for (const [typed, headline, rows] of ColourCases.answers) {
        asked++;
        const got = Colours.answer(typed, ColourCases.palette, ColourCases.roles);
        const said = got?.rows.map(r => [r.text, r.side, r.note].filter(s => s)
            .join(" "));
        if (!got || got.headline !== headline || quoted(said) !== quoted(rows)) {
            failures.push(`"${typed}" gave ${quoted(got?.headline)} and `
                + `${quoted(said)}, not "${headline}" and ${quoted(rows)}`);
        }
    }
    asked++;
    const [swatchTyped, worn] = ColourCases.swatch;
    const swatched = Colours.answer(swatchTyped, ColourCases.palette, ColourCases.roles);
    if (!swatched || swatched.rows[0].swatch !== worn) {
        failures.push(`"${swatchTyped}" wore ${quoted(swatched?.rows[0].swatch)}`);
    }
    for (const typed of ColourCases.not) {
        asked++;
        const got = Colours.answer(typed, ColourCases.palette, ColourCases.roles);
        if (got) {
            failures.push(`"${typed}" gave ${quoted(got.headline)}, but nothing should`);
        }
    }
    // It's asked on every key, so what's typed on the way to a word, or a
    // colour not yet whole, never works out the theme's pigments: a role
    // counts each time it's read.
    asked++;
    let read = 0;
    const accent = ColourCases.roles.accent;
    const counted = { accent: { toString: () => { read++; return accent; } } };
    for (const typed of ["c", "ca", "bat", "meet", "#", "#12345", "rgb", "colour"]) {
        Colours.answer(typed, ColourCases.palette, counted);
    }
    if (read > 0) {
        failures.push(`words that aren't colours read the theme's roles ${read} times`);
    }
    return asked;
}

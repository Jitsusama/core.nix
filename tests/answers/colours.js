// The cases of colours for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Colours.js" as Colours
.import "../colour-cases.js" as ColourCases

function run(failures, fromRoot) {
    let asked = 0;
    const Maths = fromRoot.maths;
    const quoted = Common.quoted;
    for (const [typed, headline, rows] of ColourCases.answers) {
        asked++;
        const got = Colours.answer(typed, ColourCases.palette, ColourCases.roles);
        const said = got && got.rows.map(r => [r.text, r.side, r.note].filter(s => s)
            .join(" "));
        if (!got || got.headline !== headline || quoted(said) !== quoted(rows))
            failures.push(`"${typed}" gave ${quoted(got && got.headline)} and `
                + `${quoted(said)}, not "${headline}" and ${quoted(rows)}`);
    }
    asked++;
    const [typed, worn] = ColourCases.swatch;
    const swatched = Colours.answer(typed, ColourCases.palette, ColourCases.roles);
    if (!swatched || swatched.rows[0].swatch !== worn)
        failures.push(`"${typed}" wore ${quoted(swatched && swatched.rows[0].swatch)}`);
    for (const typed of ColourCases.not) {
        asked++;
        const got = Colours.answer(typed, ColourCases.palette, ColourCases.roles);
        if (got)
            failures.push(`"${typed}" gave ${quoted(got.headline)}, but nothing should`);
    }
    return asked;
}

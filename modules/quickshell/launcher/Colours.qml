// A colour in its other forms, answered in the field: "#f53" shows its RGB,
// with HSL and OKLCH as rows, the first wearing the colour, and the theme's
// nearest pigment, with the roles that wear it. "colour accent" looks up one
// of the theme's own. Colours.js says how, and is the one that names
// colours. Its settings: the theme's `palette` and `roles`, rendered at
// build.
import QtQuick
import "Colours.js" as Colours

Provider {
    examples: ["#f53", "colour accent", "oklch(70% 0.1 160)"]

    function answer(typed) {
        const said = Colours.answer(typed, settings.palette, settings.roles);
        if (!said)
            return null;
        const swatch = String.fromCodePoint(0xf14fb);
        return {
            headline: said.headline,
            copy: said.copy,
            rows: said.rows.map(row => copyRow({
                name: row.text,
                side: row.side,
                copy: row.copy,
                glyph: row.swatch ? swatch : "",
                swatch: row.swatch
            })),
            note: "sRGB"
        };
    }
}

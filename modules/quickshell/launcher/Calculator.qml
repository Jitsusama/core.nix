// Sums and number bases, answered in the field as they're typed: "12*7"
// shows "= 84", "0x1f" shows 31 with its other bases as rows, and "255 in
// hex" asks for one. Numbers.js says what counts as a sum. Its settings:
// `decimals`, and the `group` and `point` marks numbers are written with.
// A base's row reads its name first, "Hex 0x1f", as every part does.
import qs
import QtQuick
import "Numbers.js" as Numbers

Provider {
    examples: ["255 in hex", "2^32", "sqrt(2)", "0b1010"]

    function answer(typed) {
        const said = Numbers.answer(typed, settings);
        if (!said)
            return null;
        return {
            headline: said.headline,
            copy: said.copy,
            rows: said.rows.map(row => copyRow({
                name: Words.name(`calculator:${row.side}`),
                side: row.text,
                copy: row.copy,
                glyph: Theme.glyph.number
            })),
            note: ""
        };
    }
}

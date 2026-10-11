// Sums and number bases, answered in the field as they're typed: "12*7"
// shows "= 84", "0x1f" shows 31 with its other bases as rows, and "255 in
// hex" asks for one. Numbers.js says what counts as a sum. Its settings:
// `decimals`, and the `group` and `point` marks numbers are written with.
import QtQuick
import "Numbers.js" as Numbers

Provider {
    function answer(typed) {
        const said = Numbers.answer(typed, settings);
        if (!said)
            return null;
        return {
            headline: said.headline,
            copy: said.copy,
            rows: said.rows.map(row => copyRow({
                name: row.text,
                side: row.side,
                copy: row.copy,
                glyph: String.fromCodePoint(0xf03a0)
            })),
            note: ""
        };
    }
}

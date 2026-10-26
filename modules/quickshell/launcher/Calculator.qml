// Sums and number bases, answered in the field as they're typed: "12*7"
// shows "= 84", "0x1f" shows 31 with its other bases as rows, "255 in
// hex" asks for one, and a sum on bits, "0b1011 << 3", shows them too.
// Numbers.js says what counts as a sum, and the shell's maths (Maths,
// libqalculate in the shell's own process) works it out. Its settings:
// `decimals`, and the `group` and `point` marks numbers are written with.
// A base's row reads its name first, "Hex 0x1f", as every part does.
import qs
import QtQuick
import Maths
import "Numbers.js" as Numbers

Provider {
    examples: ["255 in hex", "2^32", "sqrt(2)", "0b1011 << 3"]

    // The maths reads its definitions as the launcher is made, so the
    // first sum typed doesn't wait for them.
    Component.onCompleted: Maths.prepare()

    function answer(typed) {
        const said = Numbers.answer(typed, settings, Maths);
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

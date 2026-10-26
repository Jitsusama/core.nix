// Measures from one unit to another, answered in the field: "5 km in mi"
// shows "= 3.10686 mi", with the rate each way as rows, or for a
// temperature the third scale, and a measure alone, "25ms", in its kind's
// base unit. Units.js holds every unit and the names it's typed as, and
// hands what it can't answer, "5 ft + 3 in in cm", to the shell's maths
// (Maths, libqalculate). Its settings: the `group` and `point` marks
// numbers are written with.
import qs
import QtQuick
import Maths
import "Units.js" as Units

Provider {
    examples: ["5 km in mi", "350 f in c", "25ms", "5 ft + 3 in in cm"]

    function answer(typed) {
        const said = Units.answer(typed, settings, Maths);
        if (!said)
            return null;
        return {
            headline: said.headline,
            copy: said.copy,
            rows: said.rows.map(row => copyRow({
                name: row.text,
                side: row.side,
                copy: row.copy,
                glyph: Theme.glyph[said.glyph]
            })),
            note: said.kind
        };
    }
}

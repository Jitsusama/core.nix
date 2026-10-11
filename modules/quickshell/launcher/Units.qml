// Measures from one unit to another, answered in the field: "5 km in mi"
// shows "= 3.10686 mi", with the rate each way as rows, or for a
// temperature the third scale. Units.js holds every unit and the names it's
// typed as. Its settings: the `group` and `point` marks numbers are written
// with.
import QtQuick
import "Units.js" as Units

Provider {
    examples: ["5 km in mi", "350 f in c", "1 gib in mb"]

    function answer(typed) {
        const said = Units.answer(typed, settings);
        if (!said)
            return null;
        return {
            headline: said.headline,
            copy: said.copy,
            rows: said.rows.map(row => copyRow({
                name: row.text,
                side: row.side,
                copy: row.copy,
                glyph: String.fromCodePoint(said.glyph)
            })),
            note: said.kind
        };
    }
}

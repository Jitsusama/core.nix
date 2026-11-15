// Where the rows drawn stand, run in Qt's own JavaScript engine, the one
// the shell runs it in. The `the-rows-hear-only-a-new-place` check copies
// modules/quickshell/launcher/RowsPlace.js beside this file and runs it
// offscreen. The rows stand in for the launcher's (Launcher.qml's
// rowsGlyph, rowsHue and rowsAtHome), counting what each is told: a
// refresh that makes held again for the place it stays in tells them
// nothing, and a step tells each what changed, once.
import QtQuick
import "Expect.js" as Expect
import "RowsPlace.js" as RowsPlace

QtObject {
    id: test

    Component.onCompleted: {
        try {
            run();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    component Rows: QtObject {
        property string rowsGlyph: ""
        property string rowsHue: ""
        property bool rowsAtHome: true
        property int told: 0
        onRowsGlyphChanged: told += 1
        onRowsHueChanged: told += 1
        onRowsAtHomeChanged: told += 1
    }

    property Rows rows: Rows {}

    // A step into `name`, made afresh each time, as a refresh makes it.
    function step(name, glyph, hue) {
        return { key: name, row: { key: `collection:${name}`, name: name, glyph: glyph, hue: hue },
            rows: [] };
    }

    function run() {
        Expect.same("home's rows say they're home's and wear no thing's glyph or hue",
            RowsPlace.of([]), { glyph: "", hue: "", home: true });
        Expect.same("a place's rows wear the last step's glyph and hue, and aren't home's",
            RowsPlace.of([step("machine", "M", "machine"), step("cpu", "C", "cpu")]),
            { glyph: "C", hue: "cpu", home: false });
        Expect.same("a thing with no glyph gives none",
            RowsPlace.of([{ row: { name: "bare" } }]).glyph, "");

        // Stepping in tells the rows each value that changed, once.
        RowsPlace.tell(rows, [step("machine", "M", "machine")]);
        Expect.same("a step in tells its glyph, hue and that it left home", rows.told, 3);
        // A refresh makes held again, every step a new object, for the
        // same place: the rows are told nothing.
        rows.told = 0;
        RowsPlace.tell(rows, [step("machine", "M", "machine")]);
        Expect.same("a refresh in the same place tells the rows nothing", rows.told, 0);
        // A step beside, to a place with the same glyph, tells only its hue.
        RowsPlace.tell(rows, [step("desktop", "M", "desktop")]);
        Expect.same("a place alike but for its hue tells only that", rows.told, 1);
        // Back home tells them all again.
        rows.told = 0;
        RowsPlace.tell(rows, []);
        Expect.same("back home tells each", rows.told, 3);
    }
}

// Where the rows drawn stand, run in Qt's own JavaScript engine, the one
// the shell runs it in. The `the-rows-hear-only-a-new-place` check copies
// modules/quickshell/launcher/RowsPlace.js beside this file and runs it
// offscreen. The rows stand in for the launcher's (Launcher.qml's
// rowsGlyph, rowsHue and rowsAtHome), counting what each is told: a
// refresh that makes held again for the place it stays in tells them
// nothing, and a step tells each what changed, once.
import QtQuick
import "RowsPlace.js" as RowsPlace

QtObject {
    id: test

    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
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

    function check(failures, what, got, want) {
        if (got !== want)
            failures.push(`${what}: got "${got}", wanted "${want}"`);
    }

    // A step into `name`, made afresh each time, as a refresh makes it.
    function step(name, glyph, hue) {
        return { key: name, row: { key: `collection:${name}`, name: name, glyph: glyph, hue: hue },
            rows: [] };
    }

    function run() {
        const failures = [];

        const home = RowsPlace.of([]);
        check(failures, "home's rows say they're home's", home.home, true);
        check(failures, "and wear no thing's glyph", home.glyph, "");
        check(failures, "or hue", home.hue, "");

        const deeper = RowsPlace.of([step("machine", "M", "machine"), step("cpu", "C", "cpu")]);
        check(failures, "a place's rows aren't home's", deeper.home, false);
        check(failures, "and wear the last step's glyph", deeper.glyph, "C");
        check(failures, "and its hue", deeper.hue, "cpu");
        check(failures, "a thing with no glyph gives none",
            RowsPlace.of([{ row: { name: "bare" } }]).glyph, "");

        // Stepping in tells the rows each value that changed, once.
        RowsPlace.tell(rows, [step("machine", "M", "machine")]);
        check(failures, "a step in tells its glyph, hue and that it left home", rows.told, 3);
        // A refresh makes held again, every step a new object, for the
        // same place: the rows are told nothing.
        rows.told = 0;
        RowsPlace.tell(rows, [step("machine", "M", "machine")]);
        check(failures, "a refresh in the same place tells the rows nothing", rows.told, 0);
        // A step beside, to a place with the same glyph, tells only its hue.
        RowsPlace.tell(rows, [step("desktop", "M", "desktop")]);
        check(failures, "a place alike but for its hue tells only that", rows.told, 1);
        // Back home tells them all again.
        rows.told = 0;
        RowsPlace.tell(rows, []);
        check(failures, "back home tells each", rows.told, 3);

        for (const failure of failures)
            console.log(`FAIL ${failure}`);
        console.log(`rows-place: ${failures.length} of the cases failed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

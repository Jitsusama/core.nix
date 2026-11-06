// A card's status line, the launcher's and held notices' alike, worked out
// in Qt's own JavaScript engine, the one the shell runs it in. The
// `a-card-says-how-it-stands-in-one-line` check copies
// modules/quickshell/StatusLine.js beside this file and runs
// `qml -platform offscreen status-line.qml`.
import QtQuick
import "StatusLine.js" as Line

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const failures = [];
        function holds(what, got, want) {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }

        // Each hint a letter a unit wide, the join two.
        const hints = ["d read", "y copy", ": commands", "Space ? keys", "\u21b5 go to"];
        const wide = hint => hint.length;
        const all = hints.reduce((sum, hint) => sum + hint.length, 0) + 4 * 2;
        holds("all that fit stay", Line.fitted(hints, wide, 2, all), hints);
        holds("the second gives way first", Line.fitted(hints, wide, 2, all - 1),
            ["d read", ": commands", "Space ? keys", "\u21b5 go to"]);
        holds("then the middle ones, one by one", Line.fitted(hints, wide, 2, 30),
            ["d read", "Space ? keys", "\u21b5 go to"]);
        holds("then the first, the keys list and Return staying",
            Line.fitted(hints, wide, 2, 20), ["Space ? keys", "\u21b5 go to"]);
        holds("and never fewer", Line.fitted(hints, wide, 2, 0), ["Space ? keys", "\u21b5 go to"]);
        holds("the hints are never changed in place", hints.length, 5);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            Qt.exit(1);
        }
        Qt.exit(0);
    }
}

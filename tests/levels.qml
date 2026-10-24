// How far a notice reaches by its sender's level, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-sender-keeps-to-its-level` check copies modules/quickshell/Levels.js
// beside this file and runs `qml -platform offscreen levels.qml`.
import QtQuick
import "Levels.js" as Levels

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
        // [what, the sender's level, whether the notice is critical, the level it gets]
        const cases = [
            ["a sender with no level shows a card", undefined, false, "card"],
            ["and its critical one is urgent", undefined, true, "urgent"],
            ["a sender set to count is counted, not shown", "count", false, "count"],
            ["and can't raise itself by being critical", "count", true, "count"],
            ["a sender kept to its history", "history", true, "history"],
            ["a sender set to a card shows its critical one as a card", "card", true, "card"],
            ["a sender let be urgent still shows a plain one as a card", "urgent", false, "card"],
            ["an unknown level is no level", "loud", true, "urgent"]
        ];
        for (const [what, level, critical, want] of cases) {
            const got = Levels.reached(level, critical);
            if (got !== want)
                failures.push(`${what}: ${got}, not ${want}`);
        }
        const shown = Levels.reach("card");
        if (!(shown.shown && shown.counted && !shown.urgent))
            failures.push(`a card shows and counts: ${JSON.stringify(shown)}`);
        const counted = Levels.reach("count");
        if (counted.shown || !counted.counted)
            failures.push(`a count counts unshown: ${JSON.stringify(counted)}`);
        const kept = Levels.reach("history");
        if (kept.shown || kept.counted)
            failures.push(`history neither shows nor counts: ${JSON.stringify(kept)}`);
        const urgent = Levels.reach("urgent");
        if (!(urgent.shown && urgent.counted && urgent.urgent))
            failures.push(`urgent shows, counts and stays: ${JSON.stringify(urgent)}`);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

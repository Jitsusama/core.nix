// The desktop's facts from the overview's pieces, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-piece-may-stand-for-several` check copies modules/quickshell/Facts.js
// beside this file and runs `qml -platform offscreen facts.qml`.
import QtQuick
import "Facts.js" as Facts

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

    function marks(facts) {
        return facts.map(fact => fact.entry.mark);
    }

    function run() {
        const failures = [];
        const check = (what, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        const entries = [
            { name: "battery", mark: "battery" },
            { name: "devices", mark: "device" },
            { name: "clock", mark: null }
        ];
        const battery = { title: "Battery" };
        const clock = { title: "Clock" };
        const headset = { key: "aa", title: "Headset" };
        const mouse = { key: "bb", title: "Mouse" };

        check("a piece is a fact, in the overview's order",
            marks(Facts.facts(entries, [battery, { each: [headset] }, clock])),
            ["battery", "device:aa", null]);
        check("a piece standing for several is a fact for each, marked by its key",
            marks(Facts.facts(entries, [battery, { each: [headset, mouse] }, clock])),
            ["battery", "device:aa", "device:bb", null]);
        check("and none while it stands for none",
            marks(Facts.facts(entries, [battery, { each: [] }, clock])),
            ["battery", null]);
        check("a piece not yet loaded is a fact with no piece",
            Facts.facts(entries, [battery, null, clock]).map(fact => fact.piece === null),
            [false, true, false]);
        const several = Facts.facts(entries, [battery, { each: [headset, mouse] }, clock]);
        check("each thing is its fact's piece",
            several.map(fact => fact.piece.title), ["Battery", "Headset", "Mouse", "Clock"]);
        check("its entry keeps the rest of the piece's",
            several.map(fact => fact.entry.name), ["battery", "devices", "devices", "clock"]);
        check("and the overview's own entry is left as it was", entries[1].mark, "device");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

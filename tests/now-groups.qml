// The order Now lists what's on in, run in Qt's own JavaScript engine, the
// one the shell runs it in. The `now-lists-by-group` check copies Now.js
// beside this file and runs `qml -platform offscreen now-groups.qml`. The
// rows are one of every group Now has, named as their pieces name them.
import QtQuick
import "Now.js" as Now

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

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function run() {
        const failures = [];
        const expect = (name, got, want) => {
            if (got !== want)
                failures.push(`${name}: "${got}", not "${want}"`);
        };

        // What wants Joel leads, then what reaches other people, what's
        // playing and the modes left on, A to Z within each.
        const every = [
            { group: "leftOn", name: "Keep Awake" },
            { group: "playing", name: "Playing" },
            { group: "onAir", name: "In a Call" },
            { group: "wants", name: "Waiting on You" },
            { group: "leftOn", name: "Do Not Disturb" },
            { group: "wants", name: "Notices" },
            { group: "onAir", name: "Camera" }
        ];
        expect("every group, in its place", names(Now.ordered(every)),
            "Notices, Waiting on You, Camera, In a Call, Playing, Do Not Disturb, Keep Awake");

        // A name is read as Joel reads it, its numbers by their value and
        // its case aside.
        expect("names as Joel reads them", names(Now.ordered([
            { group: "wants", name: "pi 10" },
            { group: "wants", name: "Pi 9" }
        ])), "Pi 9, pi 10");

        // A group Now doesn't know goes last, never ahead of what wants
        // Joel.
        expect("an unknown group goes last", names(Now.ordered([
            { group: "elsewhere", name: "Aardvark" },
            { group: "leftOn", name: "Keep Awake" },
            { group: "wants", name: "Waiting on You" }
        ])), "Waiting on You, Keep Awake, Aardvark");

        // The rows given stay as they were, since the launcher keeps them.
        const given = [{ group: "leftOn", name: "B" }, { group: "wants", name: "A" }];
        Now.ordered(given);
        expect("the rows given stay as they were", names(given), "B, A");

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

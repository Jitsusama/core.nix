// What the arrival line says, run in Qt's own JavaScript engine, the one
// the shell runs it in. The `the-notice-line-shows-what-still-waits` check
// copies modules/quickshell/NoticeLine.js beside this file and runs
// `qml -platform offscreen notice-line.qml`.
import QtQuick
import "NoticeLine.js" as Line

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

        const plain = { read: false, withdrawn: false, urgency: "normal" };
        const critical = { read: false, withdrawn: false, urgency: "critical" };
        const read = { read: true, withdrawn: false, urgency: "normal" };
        const withdrawn = { read: false, withdrawn: true, urgency: "normal" };

        // [what, the stack, the records by id, whether notices are held, the one shown]
        const choosing = [
            ["the front shows", [3, 1], { 1: plain, 3: plain }, false, 3],
            ["a read front gives way to the next", [3, 1], { 1: plain, 3: read }, false, 1],
            ["a withdrawn front gives way", [3, 1], { 1: plain, 3: withdrawn }, false, 1],
            ["one with no record yet waits for it", [3, 1], { 1: plain }, false, 1],
            ["nothing waiting shows nothing", [3, 1], { 1: read, 3: withdrawn }, false, -1],
            ["an empty stack shows nothing", [], {}, false, -1],
            ["held, a plain one shows nothing", [3], { 3: plain }, true, -1],
            ["held, a critical one behind shows", [3, 1], { 1: critical, 3: plain }, true, 1],
            ["held, a critical one read shows nothing", [1], { 1: Object.assign({}, critical,
                { read: true }) }, true, -1]
        ];
        for (const [what, shown, records, held, want] of choosing)
            holds(what, Line.chosen(shown, records, held), want);

        // [what, the one chosen, gathered, talking, critical, up]
        const standing = [
            ["a notice chosen is up", 3, false, false, false, true],
            ["none chosen is down", -1, false, false, false, false],
            ["the launcher showing the notices puts it down", 3, true, false, false, false],
            ["even a critical one goes while gathered", 3, true, false, true, false],
            ["the talk line puts a plain one down", 3, false, true, false, false],
            ["a critical one stays over the talk line", 3, false, true, true, true]
        ];
        for (const [what, chosen, gathered, talking, isCritical, want] of standing)
            holds(what, Line.up(chosen, gathered, talking, isCritical), want);

        const behind = { 1: plain, 2: read, 3: plain, 4: plain };
        holds("those behind still waiting are counted", Line.more([3, 1, 2, 4], behind, 3), 2);
        holds("none behind counts none", Line.more([3], { 3: plain }, 3), 0);
        holds("nothing chosen counts every one waiting", Line.more([3, 1], behind, -1), 2);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

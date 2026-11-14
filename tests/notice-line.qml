// What the arrival line says, and when a notice knocks, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-arrival-line-shows-what-still-waits` check copies
// modules/quickshell/NoticeLine.js, modules/quickshell/Knock.js and
// tests/lib/Expect.js beside this file and runs
// `qml -platform offscreen notice-line.qml`.
import QtQuick
import "Expect.js" as Expect
import "Knock.js" as Knock
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
        // The front as the notices daemon serves it (front.rs): the one in
        // front, the first critical one, and how many wait. Which one
        // that is, read or withdrawn ones left out, is the daemon's,
        // tested there (modules/notices). A notice replaced in place keeps
        // its id there (daemon.rs), so the line, which reads the one it
        // shows by id, shows the replacement where the first one stood.
        const front = (id, critical, waiting) => ({ id, critical, waiting });

        // [what, the front, whether notices are held, the one shown]
        const choosing = [
            ["the front shows", front(3, null, 2), false, 3],
            ["a critical one behind the front still lets the front show", front(3, 1, 2),
                false, 3],
            ["nothing waiting shows nothing", front(null, null, 0), false, -1],
            ["no front heard yet shows nothing", null, false, -1],
            ["a front missing its id shows nothing", {}, false, -1],
            ["held, a plain one shows nothing", front(3, null, 1), true, -1],
            ["held, the critical one shows", front(3, 1, 2), true, 1],
            ["held, a critical front shows", front(1, 1, 1), true, 1]
        ];
        for (const [what, served, held, want] of choosing)
            Expect.same(what, Line.chosen(served, held), want);

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
            Expect.same(what, Line.up(chosen, gathered, talking, isCritical), want);

        Expect.same("those behind still waiting are counted", Line.more(front(3, null, 3), 3), 2);
        Expect.same("none behind counts none", Line.more(front(3, null, 1), 3), 0);
        Expect.same("nothing chosen counts every one waiting",
            Line.more(front(3, null, 2), -1), 2);
        Expect.same("no front heard counts none", Line.more(null, -1), 0);

        // [what, chimes, the sound, held, now, the last knock, knocks]
        const knocking = [
            ["a message to Joel knocks", true, "knock.oga", false, 10000, 0, true],
            ["any other notice comes without a sound", false, "knock.oga", false, 10000, 0,
                false],
            ["no sound set knocks nothing", true, "", false, 10000, 0, false],
            ["held, it never knocks", true, "knock.oga", true, 10000, 0, false],
            ["a burst knocks once", true, "knock.oga", false, 10000, 9000, false],
            ["two seconds on, it knocks again", true, "knock.oga", false, 11000, 9000, true]
        ];
        for (const [what, chimes, sound, held, now, last, want] of knocking)
            Expect.same(what, Knock.knocks({ chimes, sound, held }, now, last), want);

        Qt.exit(0);
    }
}

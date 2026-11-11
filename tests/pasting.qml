// What Copies and Emoji say of the clipboard's history, as Pasting.js works
// it out, run in Qt's own JavaScript engine, the one the shell runs it in.
// The `the-clipboard-says-once-it-is-silent` check copies
// modules/quickshell/Pasting.js and tests/lib/Expect.js beside this file
// and runs `qml -platform offscreen pasting.qml`: nothing while silent
// under a second, "not answering" past it, nothing once it answers again,
// and a paste that never landed said for a minute, the silence first.
import QtQuick
import "Expect.js" as Expect
import "Pasting.js" as Said

QtObject {
    // A case that doesn't hold throws, naming it, and ends as a failure,
    // rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const said = { silent: "not answering", unpasted: "didn't paste" };
        const answering = { kept: true, lostAt: null, missedAt: null };
        const lost = at => Object.assign({}, answering, { lostAt: at });
        const missed = at => Object.assign({}, answering, { missedAt: at });

        Expect.same("an answering history says nothing", Said.says(answering, 5000, said), "");
        Expect.same("nothing under a second of silence, so a restart never blinks",
            Said.says(lost(5000), 5999, said), "");
        Expect.same("not answering once silent a second", Said.says(lost(5000), 6000, said),
            "not answering");
        Expect.same("cleared once it answers again", Said.says(answering, 60000, said), "");
        Expect.same("no history kept says nothing, its paste being Qt's",
            Said.says({ kept: false, lostAt: 0, missedAt: 0 }, 5000, said), "");

        Expect.same("a paste that never landed is said", Said.says(missed(5000), 5000, said),
            "didn't paste");
        Expect.same("and stops being said a minute on", Said.says(missed(5000), 65000, said), "");
        Expect.same("silence is said over a missed paste",
            Said.says({ kept: true, lostAt: 0, missedAt: 5000 }, 5000, said), "not answering");

        Qt.exit(0);
    }
}

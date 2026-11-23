// How the cat says its frames were shown (MarginCatShown.js): what a walk
// tells the journal, which frames are its own, which it asks niri of and
// which answers count, in Qt's own JavaScript engine, the one the shell
// runs it in. The `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat's scripts beside this file and runs `qml -platform
// offscreen margin-cat-shown.qml`.
import QtQuick
import "MarginCatShown.js" as Shown

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
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        const shown = Shown.walked([100, 116.7, 133.3, 150, 200], [1.2, 0.4, 3.04, 0.8], 1,
            { begun: 7, stepped: 6 });
        expect("a walk's frames as niri showed them, the gaps between them sorted",
            shown.said, {
                frames: 5, median: 16.7, p99: 50, slowest: 50, typical: 0.8, drawn: 3, unseen: 1,
                begun: 7, stepped: 6, held: 0
            });
        // Its steps beside its frames say whether it moved on each, or
        // was drawn again standing still (2026-11-13: Qt paced a 120 Hz
        // screen's steps at 60 where niri said 60 Hz).
        expect("as the journal says them", shown.line, "cat: a walk presented 5 of its 7 frames "
            + "a median 16.7 ms apart, the p99 50 ms, the slowest 50 ms, each drawn in 0.8 ms "
            + "typically and within 3 ms, 1 never shown, stepped 6 times");
        expect("a hop's frames are said as a walk's",
            Shown.walked([100, 116.7], [1], 0, { begun: 2, what: "hop" }).line
                .split(" of its")[0],
            "cat: a hop presented 2");
        expect("a walk nothing showed says so", Shown.walked([], [], 0, { begun: 0 }).said,
            {
                frames: 0, median: 0, p99: 0, slowest: 0, typical: 0, drawn: 0, unseen: 0,
                begun: 0, stepped: 0, held: 0
            });
        // Held under the launcher for 2 s, its gaps leave the hold out
        // rather than read it as one slow frame.
        const heldShown = Shown.walked([100, 116.7, 133.3, 2150, 2166.7], [1, 1, 1, 1, 1], 0,
            { begun: 5, stepped: 5, resumed: [3] });
        expect("a walk held under the launcher leaves the hold out of its gaps",
            [heldShown.said.slowest, heldShown.said.held], [16.7, 1]);
        expect("and says it was held", heldShown.line.endsWith(
            "stepped 5 times, held under the launcher once"), true);
        // Its own frames leave out those its surface began while held.
        expect("a walk never held counts every frame since it set off",
            Shown.begun(140, 100, 0, null), 40);
        expect("a walk held and let go leaves the hold's frames out",
            Shown.begun(140, 100, 6, null), 34);
        expect("a walk told while still held leaves the open hold out too",
            Shown.begun(140, 100, 6, 130), 24);
        // Every frame of a walk is asked of, but while it's held under
        // the card, and none once it's done.
        expect("a walk follows its frames", Shown.follows(true, true, false, false), true);
        expect("held under the card, it follows none",
            Shown.follows(true, true, true, false), false);
        expect("what his hand set off follows under the card",
            Shown.follows(true, true, true, true), true);
        expect("done, it follows none", Shown.follows(true, false, false, false), false);
        expect("done by his hand, it follows none", Shown.follows(true, false, true, true),
            false);
        expect("where frames can't be told of, it follows none",
            Shown.follows(false, true, false, false), false);
        // A walk done is told only once niri has answered for every
        // frame on its way as it sat, it having set off after frame 5
        // and the last begun as it sat being 40 (2026-11-19: told at
        // once, a walk on a busy builder said 131 of its 135 frames).
        expect("while it walks, every answer counts and none ends the wait",
            Shown.heard(12, 5, null), { counts: true, ends: false });
        expect("done, an answer before its last frame counts and waits for the rest",
            Shown.heard(38, 5, 40), { counts: true, ends: false });
        expect("the answer for its last frame counts and ends the wait",
            Shown.heard(40, 5, 40), { counts: true, ends: true });
        expect("an answer after its last frame isn't the walk's, but ends the wait",
            Shown.heard(41, 5, 40), { counts: false, ends: true });
        // A walk told early so the next could set off leaves its last
        // frames' answers to come after it, which aren't the next's.
        expect("while it walks, the walk before's late answer doesn't count",
            Shown.heard(5, 5, null), { counts: false, ends: false });
        expect("done, the walk before's late answer neither counts nor ends the wait",
            Shown.heard(4, 5, 40), { counts: false, ends: false });
        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

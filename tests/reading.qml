// How a long name slides, how words too long for their room give way, and
// how the field's path shares its room, a folder's path giving way first,
// run in Qt's own JavaScript engine, the one the shell runs it in. The `a-long-name-reads-at-one-pace` check copies
// Reading.js beside this file and runs `qml -platform offscreen reading.qml`.
import QtQuick
import "Reading.js" as Reading

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

    // Close enough, for sums of fractions.
    readonly property real near: 1e-9

    function run() {
        const failures = [];
        const expect = (name, ok, said) => {
            if (!ok)
                failures.push(`${name}: ${JSON.stringify(said)}`);
        };
        // A character twelve pixels across, read twelve a second.
        const pace = 144;
        for (const over of [12, 48, 600, 2400]) {
            const { seconds, curve } = Reading.slide(over, pace);
            const points = [[0, 0]];
            for (let at = 0; at < curve.length; at += 2)
                points.push([curve[at], curve[at + 1]]);
            // Each piece is a start, two handles and an end, as BezierSpline
            // reads them, and the last ends where the slide does.
            expect(`${over} px: three pieces`, curve.length === 18, curve);
            expect(`${over} px: ends at its end`,
                Math.abs(curve[16] - 1) < near && Math.abs(curve[17] - 1) < near, curve);
            // Time and ground only go forward, so the name never steps back.
            expect(`${over} px: only forward`,
                points.every((point, at) => at === 0
                    || (point[0] >= points[at - 1][0] - near
                        && point[1] >= points[at - 1][1] - near)), points);
            // It starts and stops at rest: flat at each end.
            expect(`${over} px: starts at rest`, points[1][1] === 0, points);
            expect(`${over} px: stops at rest`, Math.abs(points[8][1] - 1) < near, points);
            // Between the ramps it holds the pace, in pixels a second, and
            // each ramp meets it at that speed.
            const speed = (from, to) => (points[to][1] - points[from][1]) * over
                / ((points[to][0] - points[from][0]) * seconds);
            expect(`${over} px: holds the pace`, Math.abs(speed(3, 6) - pace) < 1e-6,
                speed(3, 6));
            expect(`${over} px: meets the pace speeding up`,
                Math.abs(speed(2, 3) - pace) < 1e-6, speed(2, 3));
            expect(`${over} px: meets the pace slowing`,
                Math.abs(speed(6, 7) - pace) < 1e-6, speed(6, 7));
            // A ramp's time is at most a fifth of a second.
            expect(`${over} px: short ramps`, points[3][0] * seconds <= 0.2 + near, points);
        }
        // A long slide takes the pace's time and one ramp more.
        expect("600 px at its pace", Math.abs(Reading.slide(600, pace).seconds - (600 / pace + 0.2))
            < near, Reading.slide(600, pace));
        // Nothing to slide takes no time.
        expect("nothing over", Reading.slide(0, pace).seconds === 0, Reading.slide(0, pace));
        // Words too long for their room give way at a whole word, in a
        // face ten pixels a character, as the field's monospace is.
        const width = words => words.length * 10;
        const cut = (words, room) => Reading.wholeWords(words, room, width);
        const missed = "Open-Meteo didn't answer";
        expect("whole where they fit", cut(missed, 240) === missed, cut(missed, 240));
        // "Open-Meteo didn't a\u2026" is what a cut by the letter left.
        expect("a word short", cut(missed, 230) === "Open-Meteo didn't\u2026", cut(missed, 230));
        expect("two words short", cut(missed, 170) === "Open-Meteo\u2026", cut(missed, 170));
        expect("not even a word", cut(missed, 100) === "", cut(missed, 100));
        // A separator left at the end goes before the ellipsis.
        const where = "Tokyo, Japan \u00b7 Open-Meteo";
        expect("no separator dangling", cut(where, 200) === "Tokyo, Japan\u2026", cut(where, 200));
        expect("no comma dangling", cut(where, 120) === "Tokyo\u2026", cut(where, 120));
        // The path's steps: the program's ask, a folder's path, the place.
        const steps = (room) => Reading.sharedSteps([100, 200, 60], [false, true, false], room, 40);
        const same = (one, other) => JSON.stringify(one) === JSON.stringify(other);
        expect("steps whole where they fit", same(steps(360), [100, 200, 60]), steps(360));
        // The path gives way before either other step is cut.
        expect("the path gives way first", same(steps(260), [100, 100, 60]), steps(260));
        // Down to its least, the others share what's left as they did.
        expect("then the others share", same(steps(140), [50, 40, 50]), steps(140));
        // With no step yielding, it's an even share, the shortest first.
        const even = Reading.sharedSteps([100, 200, 60], [false, false, false], 240, 40);
        expect("none yielding, shared evenly", same(even, [90, 90, 60]), even);
        // A side of items gives way whole, the least needed first, in the
        // order they're said, the one needed most kept however little room
        // (VKCF3Z, 2026-11-04: the tree, the address, then the state).
        const items = [{ said: "TESTQD", keep: 2 }, { said: "working", keep: 3 },
            { said: "$5.10 today", keep: 4 }, { said: "testqd-tree", keep: 0 }];
        const kept = room => Reading.wholeItems(items, room, width);
        const all = "TESTQD \u00b7 working \u00b7 $5.10 today \u00b7 testqd-tree";
        expect("items whole where they fit", kept(450) === all, kept(450));
        expect("the tree first", kept(430) === "TESTQD \u00b7 working \u00b7 $5.10 today",
            kept(430));
        expect("then the address", kept(290) === "working \u00b7 $5.10 today", kept(290));
        expect("then the state", kept(200) === "$5.10 today", kept(200));
        // The one needed most is said whole even where it doesn't fit: a
        // name gives way to it then, never a piece of it.
        expect("the cost whole", kept(50) === "$5.10 today", kept(50));
        expect("no items, nothing", Reading.wholeItems([], 100, width) === "",
            Reading.wholeItems([], 100, width));
        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            Qt.exit(1);
            return;
        }
        Qt.exit(0);
    }
}

// How a long name slides, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `a-long-name-reads-at-one-pace` check copies
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
        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            Qt.exit(1);
            return;
        }
        Qt.exit(0);
    }
}

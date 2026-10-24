// How the margin sets its glyphs, run in Qt's own JavaScript engine, the
// one the shell runs it in. The `the-margins-glyphs-stand-level` check
// copies Marks.js beside this file and runs `qml -platform offscreen
// marks.qml` over ink measured as TextMetrics measures it: from the
// baseline, its top negative.
import QtQuick
import "Marks.js" as Marks

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
        const near = (a, b) => Math.abs(a - b) < 1e-9;
        const expect = (name, got, want) => {
            if (!(typeof want === "number" ? near(got, want)
                : JSON.stringify(got) === JSON.stringify(want)))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };

        // Each glyph's ink, set to 8 high on the middle of a 12 high gap,
        // so its top is 2 down and its foot 2 up, whatever its size was.
        // [what, its ink at the size it was measured at]
        const glyphs = [
            ["a tall glyph", { x: 1, y: -14, width: 12, height: 16 }],
            ["a short one, as pi is", { x: 0.5, y: -6, width: 7, height: 6 }],
            ["a wide one, as the screen share is", { x: 0, y: -9, width: 20, height: 10 }],
            ["one that hangs below its baseline", { x: 2, y: -4, width: 8, height: 8 }]
        ];
        for (const [what, ink] of glyphs) {
            const set = Marks.fit(ink, 8, 6);
            // Where its ink lands once the font is scaled and its baseline
            // moved where fit says.
            const top = set.baseline + ink.y * set.scale;
            expect(`${what}: its height`, ink.height * set.scale, 8);
            expect(`${what}: its top`, top, 2);
            expect(`${what}: its foot`, top + ink.height * set.scale, 10);
            expect(`${what}: its left`, set.x + ink.x * set.scale, 0);
            expect(`${what}: its width`, set.width, ink.width * set.scale);
        }
        expect("a glyph with no ink takes no room",
            Marks.fit({ x: 0, y: 0, width: 0, height: 0 }, 8, 6),
            { scale: 1, x: 0, baseline: 6, width: 0 });

        // From the corner inward, on air, then what wants Joel, then what's
        // left on, each in the overview's order; drawn left to right, so
        // the corner's is last.
        const thing = (name, kind, order) => ({ name: name, kind: kind, order: order });
        expect("the corner holds what's on air",
            Marks.arranged([
                thing("awake", "leftOn", 60),
                thing("pi", "wants", 53),
                thing("camera", "onAir", 85),
                thing("caps", "leftOn", 75),
                thing("sharing", "onAir", 80),
                thing("battery", "wants", 30)
            ]).map(one => one.name),
            ["caps", "awake", "pi", "battery", "camera", "sharing"]);
        expect("nothing on, nothing drawn", Marks.arranged([]), []);

        // Colour says only how much it asks.
        const piece = (kind, now) => ({ kind: kind, now: now });
        expect("on air is strong", Marks.tone(piece("onAir", false)), "strong");
        expect("what wants Joel is strong", Marks.tone(piece("wants", false)), "strong");
        expect("what's left on is muted", Marks.tone(piece("leftOn", false)), "muted");
        expect("what needs him now is the alert", Marks.tone(piece("wants", true)), "alert");
        expect("left on but needed now is the alert too",
            Marks.tone(piece("leftOn", true)), "alert");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

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

        // Something on air that goes stays a while, muted, where it was, so
        // a camera let go is still seen to have been on; left on or
        // wanting Joel goes at once.
        const lit = (mark, kind) => ({ mark: mark, kind: kind, tone: "strong" });
        const camera = lit("camera", "onAir");
        const pi = lit("pi", "wants");
        let kept = Marks.lingers([], [camera, pi], [], { at: 1000, linger: 15000 });
        expect("a camera let go lingers, muted, and pi doesn't",
            kept.map(one => [one.mark, one.until, one.thing.tone, one.thing.kind]),
            [["camera", 16000, "muted", "onAir"]]);
        expect("what lingered is still the thing it was", camera.tone, "strong");
        kept = Marks.lingers(kept, [], [], { at: 9000, linger: 15000 });
        expect("it lingers till its time", kept.map(one => one.mark), ["camera"]);
        expect("then goes", Marks.lingers(kept, [], [], { at: 16000, linger: 15000 }), []);
        expect("taken again, it lingers no more",
            Marks.lingers(kept, [], [camera], { at: 2000, linger: 15000 }), []);
        expect("let go while it lingers, it lingers once",
            Marks.lingers(kept, [camera], [], { at: 5000, linger: 15000 }).map(one => one.until), [16000]);

        // What a notice will say waits a moment, so its glyph never shows
        // only to go as the card comes; the rest is drawn at once.
        const said = { mark: "agent", app: "pi" };
        const plain = { mark: "awake", app: "" };
        let waits = Marks.awaited([], [], [said, plain], { at: 1000, wait: 1000 });
        expect("pi's want waits, Keep Awake doesn't",
            waits, [{ mark: "agent", until: 2000 }]);
        expect("still lit, it keeps its time",
            Marks.awaited(waits, [said], [said], { at: 1500, wait: 1000 }),
            [{ mark: "agent", until: 2000 }]);
        expect("then it's drawn",
            Marks.awaited(waits, [said], [said], { at: 2000, wait: 1000 }), []);
        expect("gone, it waits no more", Marks.awaited(waits, [said], [], { at: 1500, wait: 1000 }),
            []);

        // Lag lasts while the same thing keeps the desktop or apps waiting.
        const waited = (worst, who, since) => ({ worst, who, since });
        expect("nothing waits, nothing lasts", Marks.lasted(waited("", "", null), null, 100), null);
        let lag = Marks.lasted(waited("memory", "apps", 40), null, 100);
        expect("a run lag-watch began counts from then", lag, { worst: "memory", since: 40 });
        expect("the same worst keeps its start",
            Marks.lasted(waited("memory", "apps", 40), lag, 110), { worst: "memory", since: 40 });
        expect("another worst starts again from now",
            Marks.lasted(waited("disk", "desktop", 40), lag, 120), { worst: "disk", since: 120 });
        expect("once it ends, it's gone", Marks.lasted(waited("", "", null), lag, 130), null);

        // Each mark stands in a square cell the gap's height, its drawing
        // half that, so the room above, below and either side is the same.
        // Nothing grows, so nothing ever reaches a window.
        expect("a cell the gap's height, its mark half", Marks.sized(12),
            { cell: 12, ink: 6, room: 3 });
        expect("and a gap of another size the same", Marks.sized(16),
            { cell: 16, ink: 8, room: 4 });

        // What's on points at the row the keys are on, or at the thing
        // stepped into, as its actions are; anything else points nowhere.
        const launcher = (open, collection, keys) => ({ open, collection, keys });
        expect("shut, What's on isn't asked", Marks.asked(launcher(false, "on", [])), false);
        expect("open on What's on, it is", Marks.asked(launcher(true, "on", [])), true);
        expect("open on another collection, it isn't",
            Marks.asked(launcher(true, "notices", ["on:camera"])), false);
        expect("the row the keys are on",
            Marks.pointed(launcher(true, "on", ["on:camera"])), "camera");
        expect("a thing stepped into, on one of its actions",
            Marks.pointed(launcher(true, "on", ["on:awake", "verb:on:awake:Let It Sleep"])),
            "awake");
        expect("an action alone says its thing",
            Marks.pointed(launcher(true, "on", ["verb:on:muted:Unmute It"])), "muted");
        expect("nothing selected points nowhere",
            Marks.pointed(launcher(true, "on", [""])), "");
        expect("shut points nowhere", Marks.pointed(launcher(false, "on", ["on:camera"])), "");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
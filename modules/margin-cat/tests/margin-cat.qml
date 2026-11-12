// Where the margin's cat sits and how big it's drawn, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat/MarginCat.js beside this file and runs
// `qml -platform offscreen margin-cat.qml`.
import QtQuick
import "MarginCat.js" as MarginCat

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
        // A screen 1,000 across, with no marks lit, and a cat 8 square
        // standing a cell of 1 clear of its column's border.
        const at = (column, more) => Object.assign({
            column, across: 1000, marks: 0, cell: 12, wide: 8, tall: 8, air: 1
        }, more ?? {});
        const column = (left, top, width) => ({ left, top, width });

        expect("with nothing focused it's nowhere", MarginCat.sits(at(null)), null);
        expect("it sits a cell above its column's top edge, over its middle",
            MarginCat.sits(at(column(100, 12, 400))), { x: 296, y: 3 });
        expect("fullscreen leaves no gap, so it's gone",
            MarginCat.sits(at(column(0, 0, 1000))), null);
        expect("a gap shallower than it is holds it nowhere",
            MarginCat.sits(at(column(100, 6, 400))), null);
        // The border under its feet is near its hue in both looks (1.19:1 and
        // 1.94:1), so with no air between them it melts into it.
        expect("a gap with no room for the air under it holds it nowhere",
            MarginCat.sits(at(column(100, 8, 400))), null);
        expect("a column half off the screen has it over the half that's seen",
            MarginCat.sits(at(column(-200, 12, 400))), { x: 96, y: 3 });
        expect("and never past the screen's edge",
            MarginCat.sits(at(column(996, 12, 400))), { x: 992, y: 3 });
        // Three marks of 12 centred on 500 span 482 to 518, and half a cell
        // either side keeps it off them: 476 to 524.
        expect("over the marks it sits beside them, on its column's side",
            MarginCat.sits(at(column(200, 12, 560), { marks: 3 })), { x: 468, y: 3 });
        expect("a column centred under them has it to their right",
            MarginCat.sits(at(column(250, 12, 500), { marks: 3 })), { x: 524, y: 3 });
        expect("marks far off leave it over its column",
            MarginCat.sits(at(column(0, 12, 200), { marks: 3 })), { x: 96, y: 3 });

        // Each of its eight cells a whole number of the screen's pixels,
        // about two thirds of the gap all told, and never none.
        expect("at 1.5 a gap of 12 gives cells of 2", MarginCat.cell(12, 1.5), 2);
        expect("at 1 of 1", MarginCat.cell(12, 1), 1);
        expect("at 2 of 2", MarginCat.cell(12, 2), 2);
        expect("a gap too small still draws it", MarginCat.cell(4, 1), 1);

        // A walk is nine strides, each on the screen's own pixels, its feet
        // touching the border on every other one and never on the last.
        const xs = walk => walk.map(stride => stride.x);
        expect("it trots across in nine even strides",
            xs(MarginCat.steps(0, 90, 1)), [10, 20, 30, 40, 50, 60, 70, 80, 90]);
        expect("and back the other way",
            xs(MarginCat.steps(90, 0, 1)), [80, 70, 60, 50, 40, 30, 20, 10, 0]);
        expect("its feet touch down on every other stride, never at the end",
            MarginCat.steps(0, 90, 1).map(stride => stride.dips),
            [true, false, true, false, true, false, true, false, false]);
        expect("where it already sits it doesn't walk", MarginCat.steps(40, 40.2, 1.5), []);
        const fractional = MarginCat.steps(3, 102, 1.5);
        expect("at 1.5 every stride lands on whole screen pixels",
            fractional.every(stride => Math.abs(stride.x * 1.5 - Math.round(stride.x * 1.5))
                < 1e-9), true);
        expect("and the last is where it's going", (fractional[fractional.length - 1] ?? {}).x, 102);
        expect("in nine", fractional.length, 9);

        // How engaged Joel is says how lively the cat may be (Joel, 2026-11-12:
        // "mild movements when i'm active, increeeasing as I'm less engaged").
        const heard = more => Object.assign({
            paused: false, idle: false, asleep: false, quiet: false
        }, more ?? {});
        expect("while he types it's mild", MarginCat.level(heard()), "typing");
        expect("ten seconds with no keys, a little more",
            MarginCat.level(heard({ paused: true })), "paused");
        expect("a minute with none, livelier",
            MarginCat.level(heard({ paused: true, idle: true })), "idle");
        expect("asleep it does nothing",
            MarginCat.level(heard({ paused: true, idle: true, asleep: true })), "asleep");
        expect("in Do Not Disturb it keeps still at any level",
            MarginCat.level(heard({ paused: true, idle: true, quiet: true })), "still");

        // The rest before each scene: about a minute while he types, half a
        // minute when he pauses, 20 to 40 s when he's left the keys.
        expect("typing rests 45 s at the least", MarginCat.rest("typing", 0), 45000);
        expect("and 75 s at the most", MarginCat.rest("typing", 1), 75000);
        expect("a pause rests 20 to 40 s",
            [MarginCat.rest("paused", 0), MarginCat.rest("paused", 1)], [20000, 40000]);
        expect("as does a minute with no keys",
            [MarginCat.rest("idle", 0), MarginCat.rest("idle", 1)], [20000, 40000]);
        expect("asleep there's no next scene", MarginCat.rest("asleep", 0.5), null);
        expect("nor in Do Not Disturb", MarginCat.rest("still", 0.5), null);

        // A scene is its beats, each a pose held, and it sits again after.
        const long = beats => beats.reduce((all, beat) => all + (beat.ms ?? 0), 0);
        const typed = [0, 0.4, 0.9].map(chance => MarginCat.scene("typing", chance));
        expect("while he types it's one pose and back",
            typed.map(beats => beats.length), [1, 1, 1]);
        expect("an ear flick, a tail swish or a blink",
            typed.map(beats => beats[0].pose), ["earFlick", "tailSwish", "blinking"]);
        expect("held a few hundred ms", typed.every(beats => long(beats) <= 500), true);
        const paused = [0, 0.5, 0.99].map(chance => MarginCat.scene("paused", chance));
        expect("a pause's scenes keep to the mild poses",
            paused.every(beats => beats.every(beat =>
                ["earFlick", "tailSwish", "blinking", "nothingHere"].includes(beat.pose))),
            true);
        expect("and last at most a second and a half",
            paused.every(beats => long(beats) > 0 && long(beats) <= 1500), true);
        expect("past a minute it strolls", MarginCat.scene("idle", 0)[0].walk, true);
        expect("or plays as when he pauses",
            MarginCat.scene("idle", 0.99).every(beat => beat.walk !== true), true);
        expect("asleep there's no scene", MarginCat.scene("asleep", 0), []);
        expect("nor in Do Not Disturb", MarginCat.scene("still", 0), []);
        // Eight poses a second at the most: no beat under 125 ms.
        expect("no beat is quicker than an eighth of a second",
            [...typed, ...paused].every(beats => beats.every(beat => beat.ms >= 125)), true);

        // Where a stroll goes: the gutter beside its column, between two of
        // the windows, sat as on a column's edge, and nowhere without one.
        const others = (...columns) => columns;
        expect("with no column beside it, there's nowhere to stroll",
            MarginCat.gutter(at(column(100, 12, 400), { others: others() })), null);
        expect("it strolls to the gutter on its right, over the gap's middle",
            MarginCat.gutter(at(column(100, 12, 400),
                { others: others(column(512, 12, 300)) })), { x: 502, y: 3 });
        expect("or its left where nothing's on the right",
            MarginCat.gutter(at(column(412, 12, 400),
                { others: others(column(100, 12, 300)) })), { x: 402, y: 3 });
        expect("never to a gutter off the screen",
            MarginCat.gutter(at(column(600, 12, 400),
                { others: others(column(1012, 12, 300)) })), null);
        expect("nor onto the margin's marks",
            MarginCat.gutter(at(column(100, 12, 394),
                { marks: 3, others: others(column(506, 12, 300)) })), null);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

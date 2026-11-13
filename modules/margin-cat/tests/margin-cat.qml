// Where the margin's cat sits and how big it's drawn, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat/MarginCat.js beside this file and runs
// `qml -platform offscreen margin-cat.qml`.
import QtQuick
import "MarginCat.js" as MarginCat
import "Play.js" as Play

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

        walks(expect);
        plays(expect);

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
        const MILD = ["earFlick", "tailSwish", "blinking", "nothingHere"];
        expect("a pause's scenes keep to the mild poses",
            paused.every(beats => beats.every(beat => MILD.includes(beat.pose))), true);
        expect("and last at most a second and a half",
            paused.every(beats => long(beats) > 0 && long(beats) <= 1500), true);
        expect("past a minute it strolls first", MarginCat.scene("idle", 0, 0)[0].walk, true);
        expect("then bats its yarn", MarginCat.scene("idle", 0.5, 1), [{ play: "yarn" }]);
        expect("then plays as when he pauses",
            MarginCat.scene("idle", 0.99, 2).every(beat => MILD.includes(beat.pose)), true);
        expect("then pounces", MarginCat.scene("idle", 0, 3), [{ play: "pounce" }]);
        expect("then rests in a mild one before it strolls again",
            [MarginCat.scene("idle", 0, 4).every(beat => MILD.includes(beat.pose)),
                MarginCat.scene("idle", 0, 5)[0].walk], [true, true]);
        expect("asleep there's no scene", MarginCat.scene("asleep", 0), []);
        expect("nor in Do Not Disturb", MarginCat.scene("still", 0), []);
        // Eight poses a second at the most: no beat under 125 ms.
        expect("no beat is quicker than an eighth of a second",
            [...typed, ...paused].every(beats => beats.every(beat => beat.ms >= 125)), true);

        // Where a stroll goes: the gutter beside its column, half niri's gap
        // (the cell, 12) past its edge, sat as on a column's edge.
        expect("it strolls to the gutter on its right, over the gap's middle",
            MarginCat.gutter(at(column(100, 12, 400))), { x: 502, y: 3 });
        expect("or its left when the right's is off the screen",
            MarginCat.gutter(at(column(600, 12, 396))), { x: 590, y: 3 });
        expect("never to a gutter off the screen",
            MarginCat.gutter(at(column(2, 12, 994))), null);
        expect("or its left when the right's is under the margin's marks",
            MarginCat.gutter(at(column(100, 12, 394), { marks: 3 })), { x: 90, y: 3 });
        expect("nor onto the marks on either side",
            MarginCat.gutter(at(column(494, 12, 12), { marks: 3 })), null);
        expect("nor where its column leaves no room above",
            MarginCat.gutter(at(column(100, 4, 400))), null);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // A walk of 100 cells of 1 to the right, thinking 200 ms: it turns,
    // thinks, crouches back, walks easing in and out with its strides stepped
    // by the ground it covers, overshoots, steps back and crouches as it
    // stops.
    function walks(expect) {
        const going = MarginCat.walk(100, 200, { cell: 1, ratio: 1, pause: 200 });
        const at = t => MarginCat.step(going, t);
        const beats = going.beats.map(beat => beat.name);
        expect("its beats, in order", beats,
            ["turns", "thinks", "crouches", "goes", "settles", "lands"]);
        const begins = name => going.beats.find(beat => beat.name === name).begins;
        const ends = name => going.beats.find(beat => beat.name === name).ends;
        expect("it turns its head the way it's going first, standing where it sat",
            [at(0).pose, at(0).x, at(0).facing], ["walkingB", 100, 1]);
        expect("it thinks a moment, still",
            [at(begins("thinks") + 100).beat, at(begins("thinks") + 100).x], ["thinks", 100]);
        expect("it thinks as long as it was given", ends("thinks") - begins("thinks"), 200);
        expect("it draws back a cell, crouched, before it sets off",
            [at(ends("crouches") - 1e-6).pose, Math.round(at(ends("crouches") - 1e-6).x)],
            ["pouncing", 99]);

        // Its walk, sampled each frame at 60 Hz.
        const frames = [];
        for (let t = begins("goes"); t < ends("goes"); t += 1000 / 60)
            frames.push(at(t));
        const xs = frames.map(frame => frame.x);
        expect("it only moves forward as it walks",
            xs.every((x, i) => i === 0 || x >= xs[i - 1]), true);
        const first = xs[1] - xs[0];
        const middle = Math.floor(xs.length / 2);
        const most = xs[middle + 1] - xs[middle];
        expect("it eases in, slow at first and fastest midway", first < most / 4, true);
        expect("and eases out", xs[xs.length - 1] - xs[xs.length - 2] < most / 4, true);
        expect("in its two strides", Array.from(new Set(frames.map(frame => frame.pose))).sort(),
            ["walkingA", "walkingB"]);
        expect("each stride a stretch of ground, never a frame's time",
            frames.every(frame => frame.pose === ["walkingA", "walkingB"][
                Math.floor((frame.x - 99) / going.stride) % 2]), true);
        const changes = frames.filter((frame, i) => i > 0 && frame.pose !== frames[i - 1].pose);
        expect("as many strides as its way holds", changes.length,
            Math.floor((201.5 - 99) / going.stride));
        expect("it walks on past its seat a cell and a half",
            Math.round(at(ends("goes") - 1e-6).x * 10) / 10, 201.5);
        expect("steps back onto it", [at(ends("settles") - 1e-6).beat,
            Math.round(at(ends("settles") - 1e-6).x * 10) / 10], ["settles", 200]);
        expect("and crouches as it stops, where it'll sit",
            [at(begins("lands")).pose, at(begins("lands")).x], ["pouncing", 200]);
        expect("then sits, done", [at(going.ends).done, at(going.ends).x, at(going.ends).pose],
            [true, 200, null]);
        expect("a way of 100 cells at a walking pace, about two seconds",
            Math.round(ends("goes") - begins("goes")), Math.round(102.5 / 50 * 1000));

        const back = MarginCat.walk(200, 100, { cell: 1, ratio: 1, pause: 0 });
        expect("back the other way it faces left", back.facing, -1);
        expect("drawing back to the right",
            Math.round(MarginCat.step(back, back.beats[1].ends - 1e-6).x), 201);
        expect("with no thought it doesn't stop to think",
            back.beats.map(beat => beat.name), ["turns", "crouches", "goes", "settles", "lands"]);
        expect("where it already sits, to the screen's pixel, it doesn't walk",
            MarginCat.walk(40, 40.2, { cell: 1, ratio: 1.5 }), null);
        const hurried = MarginCat.walk(150, 300, { cell: 1, ratio: 1, hurried: true, facing: 1 });
        expect("a walk begun on the way neither turns nor thinks nor crouches",
            hurried.beats.map(beat => beat.name), ["goes", "settles", "lands"]);
        expect("and sets off from where it is", MarginCat.step(hurried, 0).x, 150);
        const turned = MarginCat.walk(150, 50, { cell: 1, ratio: 1, hurried: true, facing: 1 });
        expect("one going back the way it came turns first", turned.beats[0].name, "turns");
        const slowed = MarginCat.walk(100, 200, { cell: 1, ratio: 1, pause: 200, slowdown: 2 });
        expect("the theme's slowdown stretches all of it", slowed.ends, going.ends * 2);
        const far = MarginCat.walk(0, 1000, { cell: 1, ratio: 1, pause: 0 });
        const goes = far.beats.find(beat => beat.name === "goes");
        expect("far, it hurries, never more than 120 cells a second",
            Math.round(goes.ends - goes.begins), Math.round(1002.5 / 120 * 1000));
        expect("its stride at most ten cells", far.stride, 10);
        const near = MarginCat.walk(0, 20, { cell: 2, ratio: 1, pause: 0 });
        expect("near, it ambles, its stride half its body", near.stride, 8);

        const shown = MarginCat.walked([100, 116.7, 133.3, 150, 200], [1.2, 0.4, 3.04, 0.8], 1);
        expect("a walk's frames as niri showed them, the gaps between them sorted",
            shown.said, { frames: 5, median: 16.7, p99: 50, slowest: 50, drawn: 3, unseen: 1 });
        expect("as the journal says them", shown.line, "cat: a walk presented 5 frames a median "
            + "16.7 ms apart, the p99 50 ms, the slowest 50 ms, each drawn within 3 ms, "
            + "1 never shown");
        expect("a play says it's a play", MarginCat.walked([0, 16], [1], 0, "the yarn").line,
            "cat: the yarn presented 2 frames a median 16 ms apart, the p99 16 ms, the "
            + "slowest 16 ms, each drawn within 1 ms, 0 never shown");
        expect("a walk nothing showed says so", MarginCat.walked([], [], 0).said,
            { frames: 0, median: 0, p99: 0, slowest: 0, drawn: 0, unseen: 0 });

        expect("facing right, a profile is drawn mirrored",
            MarginCat.turned(["#..", ".##"], 1), ["..#", "##."]);
        expect("facing left, as the theme draws it",
            MarginCat.turned(["#..", ".##"], -1), ["#..", ".##"]);
    }

    // The yarn and the pounce (Play.js), for a cat 8 cells of 1 across,
    // sitting at 100 on a screen 1,000 across, with 4 of room above it to
    // leap in and a thought of 200 ms.
    function plays(expect) {
        const room = Play.room({ x: 100, wide: 8, across: 1000, marks: 0, cell: 12 });
        expect("its room runs to the screen's edges", room, { left: 100, right: 892 });
        expect("and stops half a cell short of the marks on their side",
            [Play.room({ x: 100, wide: 8, across: 1000, marks: 3, cell: 12 }),
                Play.room({ x: 600, wide: 8, across: 1000, marks: 3, cell: 12 })],
            [{ left: 100, right: 368 }, { left: 76, right: 392 }]);
        const sat = more => Object.assign({ x: 100, cell: 1, room, hop: 4, pause: 200 }, more);
        const yarn = Play.play("yarn", sat());
        const at = t => Play.frame(yarn, t);
        const beat = name => yarn.beats.find(each => each.name === name);
        const during = (name, every) => {
            const frames = [];
            for (let t = beat(name).begins; t < beat(name).ends; t += every ?? 1)
                frames.push(at(t));
            return frames;
        };
        expect("the yarn's beats, in order", yarn.beats.map(each => each.name),
            ["rolls in", "notices", "eyes", "bats", "knocked", "wiggles", "leaps", "lands",
                "holds", "sends off"]);
        expect("it plays toward the more room",
            [yarn.way, Play.play("yarn", sat({ room: { left: 500, right: 20 } })).way], [1, -1]);
        expect("the yarn rolls in from twenty cells past its side", at(0).yarn.x, 128);
        expect("and comes to rest two cells short of its paws",
            Math.round(at(beat("rolls in").ends - 1e-6).yarn.x * 100) / 100, 110);
        const rolling = during("rolls in");
        expect("it turns as it rolls, toward the cat",
            [Array.from(new Set(rolling.map(frame => frame.yarn.pose))).sort(),
                rolling[1].yarn.facing], [["yarnRollingA", "yarnRollingB"], -1]);
        expect("each frame of its roll a stretch of ground, never a frame's time",
            rolling.every(frame => frame.yarn.pose === ["yarnRollingA", "yarnRollingB"][
                Math.floor((128 - frame.yarn.x) / 1.5) % 2]), true);
        expect("the cat sits as it sat while it comes",
            [at(100).pose, at(100).facing, at(100).x], ["nothingHere", 0, 100]);
        expect("then turns its head to it, ears up",
            [at(beat("notices").begins).pose, at(beat("notices").begins).facing],
            ["earFlick", 1]);
        expect("and eyes it as long as it thinks, the yarn lying still",
            [beat("eyes").ends - beat("eyes").begins, at(beat("eyes").begins).yarn.pose],
            [650, "yarn"]);
        expect("a bat sends it seven cells on",
            [at(beat("bats").begins).pose,
                Math.round(at(beat("knocked").ends - 1e-6).yarn.x * 100) / 100],
            ["batting", 117]);
        const wiggling = during("wiggles").map(frame => frame.x);
        const line = t => 100 - (1 - (1 - t) ** 3);
        const off = wiggling.map((x, i) => x - line(i / wiggling.length));
        expect("it draws back a cell crouched, wiggling either way of it",
            [during("wiggles")[0].pose, Math.min(...off) < -0.2, Math.max(...off) > 0.2,
                Math.round(at(beat("wiggles").ends - 1e-6).x * 100) / 100],
            ["pouncing", true, true, 99]);
        const leap = during("leaps");
        const lifts = leap.map(frame => frame.lift);
        expect("it leaps three cells high at the most, legs out",
            [Math.round(Math.max(...lifts) * 100) / 100, leap[0].pose], [3, "walkingA"]);
        expect("from the ground and back to it",
            [lifts[0], Math.round(at(beat("leaps").ends - 1e-6).lift * 100) / 100], [0, 0]);
        expect("no higher than the gap above it lets it", Math.round(Math.max(...during("leaps")
            .map(() => 0), ...Array.from({ length: 380 }, (_, t) => Play.frame(
            Play.play("yarn", sat({ hop: 1.5 })), beat("leaps").begins + t).lift)) * 100) / 100,
            1.5);
        expect("it lands crouched, its front paws on the yarn",
            [at(beat("lands").begins).pose, at(beat("lands").begins).x], ["pouncing", 111]);
        expect("holds it with a paw", at(beat("holds").begins).pose, "batting");
        const sent = during("sends off");
        expect("and sends it off the way it came, fading as it goes",
            [sent[0].yarn.opacity, Math.round(sent[sent.length - 1].yarn.opacity * 100) / 100,
                Math.round(sent[sent.length - 1].yarn.x * 10) / 10, sent[1].yarn.facing],
            [1, 0, 128, 1]);
        expect("it's done where it landed", [at(yarn.ends).done, at(yarn.ends).x, yarn.to],
            [true, 111, 111]);

        // Every millisecond of it, so no beat's end jumps it to the next.
        const all = Array.from({ length: Math.ceil(yarn.ends) }, (_, t) => at(t));
        const jumps = (of) => all.slice(1).reduce((most, frame, i) =>
            Math.max(most, Math.abs(of(frame) - of(all[i]))), 0);
        expect("nothing of it jumps: never a tenth of a cell in a millisecond",
            [jumps(frame => frame.x) < 0.1, jumps(frame => frame.lift) < 0.1,
                jumps(frame => frame.yarn?.x ?? 128) < 0.1], [true, true, true]);
        expect("each frame a pose of the theme's own",
            Array.from(new Set(all.map(frame => frame.pose))).sort(),
            ["batting", "earFlick", "nothingHere", "pouncing", "tailSwish", "walkingA"]);
        expect("the theme's slowdown stretches all of it",
            Play.play("yarn", sat({ slowdown: 2 })).ends, yarn.ends * 2);
        expect("with no room for the yarn, there's no play",
            Play.play("yarn", sat({ room: { left: 10, right: 20 } })), null);

        const pounce = Play.play("pounce", sat());
        expect("a pounce's beats, in order", pounce.beats.map(each => each.name),
            ["notices", "eyes", "wiggles", "leaps", "lands", "looks"]);
        const pounced = Array.from({ length: Math.ceil(pounce.ends) },
            (_, t) => Play.frame(pounce, t));
        expect("it leaps seven cells at nothing it's sure of, and looks",
            [pounce.to, pounced.every(frame => frame.yarn === null),
                Play.frame(pounce, pounce.ends - 1).pose], [107, true, "earFlick"]);
        expect("with no room for its leap, there's no pounce",
            Play.play("pounce", sat({ room: { left: 2, right: 6 } })), null);
    }
}

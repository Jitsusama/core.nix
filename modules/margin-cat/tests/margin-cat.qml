// Where the margin's cat sits and how big it's drawn, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat/MarginCat.js, MarginCatLedges.js, MarginCatRoutes.js,
// MarginCatWays.js, MarginCatFalls.js, MarginCatFloats.js, MarginCatPaths.js
// and MarginCatPlays.js beside this file and runs
// `qml -platform offscreen margin-cat.qml`.
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatPaths.js" as Paths
import "MarginCatPlays.js" as Plays
import "MarginCatWays.js" as Ways

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
        ways(expect);
        plays(expect, at, column);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // How lively the cat is, from how engaged Joel is, and what it plays
    // (Joel, 2026-11-12: "mild movements when i'm active, increeeasing as
    // I'm less engaged"; 2026-11-12: "play a bit more and not wait too
    // long. it's play should be a bit random, gentle, but playful").
    function plays(expect, at, column) {
        const heard = more => Object.assign({
            paused: false, idle: false, asleep: false, quiet: false
        }, more ?? {});
        expect("while he types it's mild", Plays.level(heard()), "typing");
        expect("ten seconds with no keys, a little more",
            Plays.level(heard({ paused: true })), "paused");
        expect("a minute with none, livelier",
            Plays.level(heard({ paused: true, idle: true })), "idle");
        expect("asleep it does nothing",
            Plays.level(heard({ paused: true, idle: true, asleep: true })), "asleep");
        expect("in Do Not Disturb it keeps still at any level",
            Plays.level(heard({ paused: true, idle: true, quiet: true })), "still");
        expect("as on a call, with a camera in use",
            Plays.level(heard({ paused: true, idle: true, onCamera: true })), "still");
        expect("as with the screen locked",
            Plays.level(heard({ paused: true, locked: true })), "still");
        expect("as where it's drawn on no screen, fullscreen or shared",
            Plays.level(heard({ paused: true, unseen: true })), "still");

        // Five minutes with no keys, as MarginCat.qml's timers take them:
        // paused at 10 s, idle at a minute and asleep at three, each change
        // drawing what comes next afresh, and each scene played out before
        // the next rest. Its chances are drawn the same way every run.
        const alone = more => {
            let seed = 7;
            const draw = () => {
                seed = (seed * 16_807) % 2_147_483_647;
                return seed / 2_147_483_647;
            };
            const changes = [10_000, 60_000, 180_000];
            const played = [];
            let t = 0;
            while (t < 300_000) {
                const level = Plays.level(heard(Object.assign({
                    paused: t >= changes[0], idle: t >= changes[1], asleep: t >= changes[2]
                }, more)));
                const coming = Plays.next(level, draw, played[played.length - 1] ?? "");
                const change = changes.find(when => when > t) ?? 300_000;
                if (coming === null || t + coming.after >= change) {
                    t = change;
                } else {
                    played.push(coming.scene.name);
                    t += coming.after
                        + coming.scene.beats.reduce((all, beat) => all + (beat.ms ?? 0), 0);
                }
            }
            return played;
        };
        const fullscreen = MarginCat.sits(at(column(0, 0, 1000)));
        expect("left five minutes it plays, then dozes", alone({}).length > 5, true);
        expect("but none in them with a camera in use", alone({ onCamera: true }), []);
        expect("nor under a fullscreen window, which leaves it on no screen",
            alone({ unseen: fullscreen === null }), []);
        expect("nor in Do Not Disturb", alone({ quiet: true }), []);

        // The rest before each scene: about a minute while he types, since
        // the keys are his; a few seconds once he pauses, and fewer once
        // he's left them, each drawn at random.
        expect("typing rests 45 s at the least", Plays.rest("typing", 0), 45000);
        expect("and 75 s at the most", Plays.rest("typing", 1), 75000);
        expect("a pause rests 5 to 12 s",
            [Plays.rest("paused", 0), Plays.rest("paused", 1)], [5000, 12000]);
        expect("a minute with no keys rests 3 to 8 s",
            [Plays.rest("idle", 0), Plays.rest("idle", 1)], [3000, 8000]);
        expect("asleep there's no next scene", Plays.rest("asleep", 0.5), null);
        expect("nor in Do Not Disturb", Plays.rest("still", 0.5), null);

        scenes(expect);

        // Where a short wander goes: along the margin from its seat, the
        // way it was drawn, or the other where that one's off its window,
        // off the screen or on the marks; nowhere where neither fits.
        const seat = { x: 296, y: 3 };
        const wanders = more => Plays.wander(at(column(100, 12, 400),
            Object.assign({ seat, step: 1, cells: 20, way: 1 }, more)));
        expect("it wanders its cells the way drawn", wanders(), { x: 316, y: 3 });
        expect("or the other way", wanders({ way: -1 }), { x: 276, y: 3 });
        expect("each cell its own", wanders({ step: 2 }), { x: 336, y: 3 });
        expect("the other way where its window ends",
            wanders({ seat: { x: 480, y: 3 } }), { x: 460, y: 3 });
        expect("nowhere off a window narrower than its wander",
            wanders({ seat: { x: 110, y: 3 }, column: column(100, 12, 30) }), null);
        expect("the other way where the screen ends",
            wanders({ seat: { x: 980, y: 3 }, column: column(700, 12, 300) }), { x: 960, y: 3 });
        expect("the other way where the marks are",
            wanders({ seat: { x: 460, y: 3 }, marks: 3 }), { x: 440, y: 3 });
        expect("nowhere where neither way fits",
            wanders({ seat: { x: 10, y: 3 }, across: 30 }), null);
        expect("nor with no seat", wanders({ seat: null }), null);
    }

    // Each scene, drawn from a run of chances (`draws`), as Math.random
    // would give them: what it is, its beats, each a pose held, and that
    // it sits again after.
    function scenes(expect) {
        const draws = (...chances) => {
            let i = 0;
            return () => chances[i++ % chances.length];
        };
        const long = beats => beats.reduce((all, beat) => all + (beat.ms ?? 0), 0);
        const inPlace = ["tailFlick", "batting", "stretch", "lookingAbout", "grooming"];
        const moves = ["hop", "pounce", "wander", "yarn"];
        const chances = [0, 0.2, 0.4, 0.6, 0.8, 0.99];

        const typed = chances.map(chance => Plays.scene("typing", draws(chance), ""));
        expect("while he types it's one pose and back",
            typed.map(played => played.beats.length), [1, 1, 1, 1, 1, 1]);
        expect("an ear flick, a tail swish or a blink",
            Array.from(new Set(typed.map(played => played.beats[0].pose))).sort(),
            ["blinking", "earFlick", "tailSwish"]);
        expect("held about 375 ms",
            typed.every(played => played.beats[0].ms >= 281 && played.beats[0].ms <= 469), true);

        const paused = chances.map(chance => Plays.scene("paused", draws(chance), ""));
        expect("paused, it plays in place, each of them",
            Array.from(new Set(paused.map(played => played.name))).sort(),
            inPlace.slice().sort());
        expect("never walking or hopping while he reads",
            paused.every(played => played.beats.every(beat => typeof beat.pose === "string"
                && beat.walk !== true && beat.hop !== true)), true);
        expect("each play short, under two and a half seconds",
            paused.every(played => long(played.beats) > 0 && long(played.beats) <= 2500), true);

        const again = inPlace.every(name => chances.every(chance =>
            Plays.scene("paused", draws(chance), name).name !== name));
        expect("never the same play twice running", again, true);

        const idle = chances.map(chance => Plays.scene("idle", draws(0.7, chance), ""));
        expect("idle, three in four of its plays are in place",
            idle.every(played => inPlace.includes(played.name)), true);
        const moved = chances.map(chance => Plays.scene("idle", draws(0.8, chance), ""));
        expect("and one in four a move",
            Array.from(new Set(moved.map(played => played.name))).sort(), moves);
        // How each moves is MarginCatGames.js's, tested beside it.
        expect("a hop is a play, where it sat",
            moved.find(played => played.name === "hop").beats, [{ play: "hop", pause: 0 }]);
        const yarn = moved.find(played => played.name === "yarn").beats;
        expect("the yarn is a play, then the walk back over its column",
            [yarn[0].play, yarn[1].walk, yarn[1].to], ["yarn", true, "column"]);
        expect("as a pounce is",
            moved.find(played => played.name === "pounce").beats.map(beat => beat.play ?? beat.to),
            ["pounce", "column"]);
        expect("each thinking 150 to 450 ms first",
            [Plays.scene("idle", draws(0.8, 0.9, 0), "").beats[0].pause,
                Plays.scene("idle", draws(0.8, 0.9, 1), "").beats[0].pause], [150, 450]);
        const wander = moved.find(played => played.name === "wander").beats;
        expect("a wander walks off, sits a moment, and walks back",
            [wander[0].to, wander[1].pose, wander[2].to], ["wander", "nothingHere", "column"]);
        expect("16 to 40 cells, either way",
            wander[0].cells >= 16 && wander[0].cells <= 40 && Math.abs(wander[0].way) === 1,
            true);

        // Each beat drawn a quarter shorter or longer than its own.
        const low = Plays.scene("paused", draws(0.1, 0, 0, 0, 0, 0, 0), "").beats;
        const high = Plays.scene("paused", draws(0.1, 0.999, 0.999, 0.999, 0.999, 0.999,
            0.999), "").beats;
        expect("its beats vary a quarter either way",
            low.every((beat, i) => Math.abs(high[i].ms / beat.ms - 1.25 / 0.75) < 0.05), true);
        // Eight poses a second at the most: no beat under 125 ms.
        expect("no beat is quicker than an eighth of a second",
            [...typed, ...paused, ...idle, ...moved].every(played => played.beats.every(beat =>
                beat.walk === true || beat.play !== undefined || beat.ms >= 125)), true);
        expect("asleep there's no scene", Plays.scene("asleep", draws(0), "").beats, []);
        expect("nor in Do Not Disturb", Plays.scene("still", draws(0), "").beats, []);

        // A play asked for by name, as a test asks for the one it watches,
        // comes next where its level has it, whatever the draw, and the
        // draws are taken as ever, so the rest of the scene varies alike.
        const asked = name => Plays.scene("idle", draws(0), "", { asked: name });
        expect("a play asked for comes next",
            [asked("hop").name, asked("wander").name, asked("yarn").name, asked("pounce").name,
                asked("grooming").name], ["hop", "wander", "yarn", "pounce", "grooming"]);
        expect("even just after the same one",
            Plays.scene("idle", draws(0), "hop", { asked: "hop" }).name, "hop");
        expect("but never a move while he reads",
            inPlace.includes(Plays.scene("paused", draws(0), "", { asked: "hop" }).name), true);
        expect("nor anything while he types",
            Plays.scene("typing", draws(0), "", { asked: "wander" }).beats.length, 1);
        expect("and the yarn asked for comes however lately it played",
            Plays.scene("idle", draws(0), "", { asked: "yarn", yarnIn: 60_000 }).name, "yarn");
        // How often the yarn comes is margin-cat-games.qml's, beside the yarn.
    }

    // A walk of 100 cells of 1 to the right, thinking 200 ms: it turns,
    // thinks, crouches back, walks easing in over its first stride and out
    // over its last, its strides stepped by the ground it covers, onto its
    // seat and no further, and crouches as it stops (Joel, 2026-11-14 17:19:
    // "i never want the kitty to skid into place, i always want to see its
    // paws moving when it is walking or running").
    function walks(expect) {
        const cat = more => Object.assign({ cell: 1, ratio: 1, wide: 8, tall: 8, air: 1 }, more);
        const on = x => ({ x, y: 0 });
        const going = Paths.walk(on(100), on(200), cat({ pause: 200 }));
        const at = t => Paths.frame(going, t);
        const beats = going.beats.map(beat => beat.name);
        expect("its beats, in order", beats,
            ["turns", "thinks", "crouches", "goes", "lands"]);
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
        // How far it goes in each tenth of a second wholly past its first
        // stride and short of its last.
        const tenths = xs.slice(6).map((x, i) => [xs[i], x]).filter(([from, to]) =>
            from >= 99 + going.stride && to <= 200 - going.stride).map(([from, to]) => to - from);
        expect("it eases in over its first stride, from a standstill", xs[1] - xs[0] < 1, true);
        expect("then goes at an even pace, never faster midway",
            Math.max(...tenths) - Math.min(...tenths) <= 2, true);
        expect("and eases out over its last", xs[xs.length - 1] - xs[xs.length - 2] < 1, true);
        // Each pair of legs apart two strides and gathered two, the hind pair
        // a stride behind (RSCH-20261113-24R2QV's rule 4), from the one a
        // pair on from its stand, stride B (WCJQSG b1034f36).
        const walk = ["walkingA", "walkingC", "walkingB", "walkingD"];
        const first = walk.indexOf("walkingB") + 1;
        expect("in its four strides, in turn, from D",
            frames.filter((frame, i) => i === 0 || frame.pose !== frames[i - 1].pose)
                .slice(0, 5).map(frame => frame.pose), [walk[3], ...walk]);
        expect("each stride a stretch of ground, never a frame's time",
            frames.every(frame => frame.pose === walk[
                (Math.floor((frame.x - 99) / going.stride) + first) % walk.length]), true);
        // A stride each 40 ms, A's from 40 and again from 200: stopping on
        // A, it steps on to C when A's been seen a SHOWN, to the ms; when it
        // hasn't, it steps into A early enough, at 199 for a walk to 232,
        // if D's been seen as long by then, or else stops on D, though not
        // on the lap before; it counts none of A's time from before its
        // strides began.
        const strideAt = (t, more) => MarginCat.striding(walk, Object.assign({
            stand: "walkingB", shown: 33, since: 0, joins: false,
            count: time => Math.floor(time / 40) }, more), t);
        expect("a walk starts a pair on from its stand, and stops at most a pair off it",
            [strideAt(0, { last: 235 }), strideAt(40, { last: 235 }),
                strideAt(220, { last: 235 }), strideAt(235, { last: 235 }),
                strideAt(300, { last: 235 }), strideAt(205, { last: 210 }),
                strideAt(210, { last: 210 }), strideAt(235, { last: 235, joins: true }),
                strideAt(275, { last: 275 }), strideAt(233, { last: 233 }),
                strideAt(232, { last: 232 }), strideAt(50, { last: 210 }),
                strideAt(232, { last: 232, since: 205 }), strideAt(199, { last: 232 }),
                strideAt(198, { last: 232 })],
            ["walkingD", "walkingA", "walkingA", "walkingC", "walkingC", "walkingD",
                "walkingD", "walkingA", "walkingC", "walkingC", "walkingC", "walkingA",
                "walkingD", "walkingA", "walkingD"]);
        // Set off from A, as from a walk begun again, it still stops into B:
        // ending on A it steps on to C, and ending on B it stays there.
        // Set off from D into A, too briefly to step on, it holds D, as it
        // does A when it set off in A and has yet to step.
        const setOff = (t, stand, more) => strideAt(t, Object.assign({ stand,
            stops: "walkingB" }, more));
        expect("set off from another stride, it's never both pairs off the one it stops into",
            [setOff(155, "walkingA", { last: 155 }), setOff(75, "walkingA", { last: 75 }),
                setOff(10, "walkingD", { last: 20 }), setOff(25, "walkingD", { last: 20 }),
                setOff(10, "walkingA", { last: 20, count: () => -1 }),
                setOff(20, "walkingA", { last: 20, count: () => -1 })],
            ["walkingC", "walkingB", "walkingD", "walkingD", "walkingA", "walkingC"]);
        const changes = frames.filter((frame, i) => i > 0 && frame.pose !== frames[i - 1].pose);
        expect("as many strides as its way holds, the last ending as it stops",
            changes.length, Math.ceil((200 - 99) / going.stride) - 1);
        expect("it walks onto its seat, never past it", Math.max(...xs) <= 200, true);
        const lastStride = frames.filter((frame, i) => i > 0 && frame.pose !== frames[i - 1].pose)
            .pop();
        expect("and in its last drawing goes no more than a stride and a cell",
            200 - lastStride.x <= going.stride + 1, true);
        expect("and crouches as it stops, where it'll sit",
            [at(begins("lands")).pose, at(begins("lands")).x], ["pouncing", 200]);
        expect("then sits, done", [at(going.ends).done, at(going.ends).x, at(going.ends).pose],
            [true, 200, null]);
        // Its eases cost time rather than quicken its pace (VKCF3Z d1d1ead2),
        // so a walk's pace is told between them.
        const eased = beat => (beat.ends - beat.begins) / (1 + beat.ramps[0] + beat.ramps[1]);
        expect("a short way at a walking pace, about two seconds",
            Math.round(eased(going.beats.find(beat => beat.name === "goes"))),
            MarginCat.CRUISES);

        const back = Paths.walk(on(200), on(100), cat({ pause: 0 }));
        expect("back the other way it faces left", back.facing, -1);
        expect("drawing back to the right",
            Math.round(Paths.frame(back, back.beats[1].ends - 1e-6).x), 201);
        expect("with no thought it doesn't stop to think",
            back.beats.map(beat => beat.name), ["turns", "crouches", "goes", "lands"]);
        expect("where it already sits, to the screen's pixel, it doesn't walk",
            Paths.walk(on(40), on(40.2), cat({ ratio: 1.5 })), null);
        const hurried = Paths.walk(on(150), on(300), cat({ hurried: true, facing: 1 }));
        expect("a walk begun on the way neither turns nor thinks nor crouches",
            hurried.beats.map(beat => beat.name), ["goes", "lands"]);
        expect("and sets off from where it is", Paths.frame(hurried, 0).x, 150);
        const turned = Paths.walk(on(150), on(50), cat({ hurried: true, facing: 1 }));
        expect("one going back the way it came turns first", turned.beats[0].name, "turns");
        const slowed = Paths.walk(on(100), on(200), cat({ pause: 200, slowdown: 2 }));
        expect("the theme's slowdown stretches all of it", slowed.ends, going.ends * 2);
        const far = Paths.walk(on(0), on(1000), cat({ pause: 0 }));
        const goes = far.beats.find(beat => beat.name === "goes");
        // Drawn back no further than the screen's left edge, it goes 1000.
        expect("far, it hurries, never more than 80 cells a second",
            Math.round(eased(goes)), Math.round(1000 / 80 * 1000));
        expect("at a run, at most three cells a drawing", far.stride, 3);
        const held = [];
        for (let t = goes.begins, run = null; t < goes.ends; t += 1) {
            const pose = Paths.frame(far, t).pose;
            if (run === null || pose !== run.pose) {
                held.push(run);
                run = { pose, begins: t };
            }
            run.ends = t + 1;
        }
        expect("each drawing of its run held two frames at 60 Hz", held.slice(2)
            .every(run => run.ends - run.begins >= 33), true);
        const near = Paths.walk(on(0), on(20), cat({ cell: 2, pause: 0 }));
        expect("near, it ambles, two cells a drawing", near.stride, 4);
        const shown = MarginCat.walked([100, 116.7, 133.3, 150, 200], [1.2, 0.4, 3.04, 0.8], 1,
            { begun: 7, stepped: 6 });
        expect("a walk's frames as niri showed them, the gaps between them sorted",
            shown.said, {
                frames: 5, median: 16.7, p99: 50, slowest: 50, typical: 0.8, drawn: 3, unseen: 1,
                begun: 7, stepped: 6
            });
        // Its steps beside its frames say whether it moved on each, or
        // was drawn again standing still (2026-11-13: Qt paced a 120 Hz
        // screen's steps at 60 where niri said 60 Hz).
        expect("as the journal says them", shown.line, "cat: a walk presented 5 of its 7 frames "
            + "a median 16.7 ms apart, the p99 50 ms, the slowest 50 ms, each drawn in 0.8 ms "
            + "typically and within 3 ms, 1 never shown, stepped 6 times");
        expect("a hop's frames are said as a walk's",
            MarginCat.walked([100, 116.7], [1], 0, { begun: 2, what: "hop" }).line
                .split(" of its")[0],
            "cat: a hop presented 2");
        expect("a walk nothing showed says so", MarginCat.walked([], [], 0, { begun: 0 }).said,
            {
                frames: 0, median: 0, p99: 0, slowest: 0, typical: 0, drawn: 0, unseen: 0,
                begun: 0, stepped: 0
            });

        // niri's own spring, as Joel's niri rides it between workspaces.
        const spring = { damping: 1, stiffness: 1600, epsilon: 0.001 };
        expect("a slide sets off still", MarginCat.slid(0, spring), 0);
        expect("and critically damped, it's a quarter way after 24 ms",
            Math.round(MarginCat.slid(24, spring) * 100) / 100, 0.25);
        expect("it never goes past its end", [10, 50, 100, 200, 400].every(t =>
            MarginCat.slid(t, spring) <= 1), true);
        expect("and slows to it, never back", [10, 50, 100, 200, 400].every((t, i, ts) =>
            i === 0 || MarginCat.slid(t, spring) > MarginCat.slid(ts[i - 1], spring)), true);
        expect("a looser spring overshoots, as niri's would",
            [60, 80, 100, 120].some(t =>
                MarginCat.slid(t, { damping: 0.5, stiffness: 1600 }) > 1), true);
        expect("it's done within its epsilon, about a quarter second",
            MarginCat.slides(spring), 231);
        expect("the theme's slowdown slows it as niri's clock",
            [MarginCat.slides(Object.assign({ slowdown: 2 }, spring)),
                MarginCat.slid(48, Object.assign({ slowdown: 2 }, spring))],
            [462, MarginCat.slid(24, spring)]);

        expect("a quarter turn clockwise", MarginCat.quartered(["#.", ".."], 1), [".#", ".."]);
        expect("and back", MarginCat.quartered(["#.", ".."], -1), ["..", "#."]);
        expect("a profile facing left faces up, a quarter turn clockwise",
            MarginCat.quartered(["...", "#..", "..."], 1), [".#.", "...", "..."]);
        expect("none leaves it as it is", MarginCat.quartered(["#.", ".."], 0), ["#.", ".."]);

        expect("facing right, a profile is drawn mirrored",
            MarginCat.turned(["#..", ".##"], 1), ["..#", "##."]);
        expect("facing left, as the theme draws it",
            MarginCat.turned(["#..", ".##"], -1), ["#..", ".##"]);
    }

    // Its ways through the world: a screen 1,248 by 800 with two columns,
    // a gap of 16 between them and above them, the right one two windows
    // stacked; its workspaces stacked 880 apart; and a cat of 8 cells of
    // 1, seated a cell above the tops (Joel, 2026-11-13: "it should stay
    // on top of the previous workspaces window ... climb up ...
    // pathfinding through the spaces between windows"; "when crossing
    // across gaps between windows, the kitty should jump").
    function ways(expect) {
        const spring = { damping: 1, stiffness: 1600, epsilon: 0.001 };
        const tiles = [
            { left: 16, top: 16, width: 600, height: 768, column: 0 },
            { left: 632, top: 16, width: 600, height: 376, column: 1 },
            { left: 632, top: 408, width: 600, height: 376, column: 1 },
            { left: 300, top: 300, width: 100, height: 100, column: null }
        ];
        const world = more => Object.assign({
            cell: 1, ratio: 1, wide: 8, tall: 8, air: 1, across: 1248, height: 800, facing: 1,
            spring, now: tiles, cols: Paths.columns(tiles, 0)
        }, more);
        const seat = x => ({ x, y: 7 });
        const cols = Paths.columns(tiles, 0);
        expect("each column once, its windows stacked top to bottom, none floating",
            cols.map(column => [column.left, column.right, column.top, column.bottom,
                column.rows.length]), [[16, 616, 16, 784, 1], [632, 1232, 16, 784, 2]]);
        // Each frame at 120 Hz of all of a way that's seen on the screen.
        const seen = way => {
            const frames = [];
            for (let t = 0; t <= way.ends; t += 1000 / 120) {
                const at = Paths.frame(way, t);
                if (at.y > -8 && at.y < 800 && at.x > -8 && at.x < 1248)
                    frames.push(Object.assign({ t }, at));
            }
            return frames;
        };
        const smooth = frames => frames.every((at, i) => i === 0 || frames[i - 1].beat === "rides"
            || Math.hypot(at.x - frames[i - 1].x, at.y - frames[i - 1].y) <= 4);

        const across = Paths.walk(seat(300), seat(900), world({ pause: 0 }));
        const names = across.beats.map(beat => beat.name);
        const leaps = across.beats.find(beat => beat.name === "leaps");
        expect("across a gap between columns it crouches, leaps and lands",
            names.slice(names.indexOf("leaps") - 1, names.indexOf("leaps") + 2),
            ["crouches", "leaps", "lands"]);
        expect("from the edge of one to the edge of the next",
            [leaps.from.x, leaps.to.x, leaps.to.y], [608, 632, 7]);
        const flight = seen(across).filter(at => at.beat === "leaps");
        expect("up and down again in an arc, never a glide",
            Math.min(...flight.map(at => at.y)) < 7 - 1, true);
        expect("as high as the room above has, never off the screen",
            flight.every(at => at.y >= 0), true);
        expect("seen in at least 7 frames at 60 Hz, and quick as a leap",
            [leaps.ends - leaps.begins >= 120, leaps.ends - leaps.begins <= 160], [true, true]);
        // It rises a third of what it crosses, at least 2 cells, held to
        // the screen; its pose and its height go by thirds: up as it
        // stretches, held at the top tucked, down as it reaches (VKCF3Z,
        // 69bb37d0).
        expect("a third of the 24 it crosses is 8, held to the screen's edge",
            Math.min(...flight.map(at => at.y)), 0);
        const arc = (from, to, ceiling) => {
            const hop = Ways.begun({ cell: 1, wide: 8, tall: 8, height: 800 }, from);
            Ways.across(hop, to, undefined, ceiling);
            const route = Ways.timed(hop);
            const frames = [];
            for (let t = 0; t < route.ends; t += 1000 / 120)
                frames.push(Ways.frame(route, t));
            return frames;
        };
        const thirds = frames => ["leaping", "tucked", "falling"].map(pose =>
            frames.filter(at => at.pose === pose));
        const open = arc({ x: 100, y: 400 }, { x: 124, y: 400 });
        expect("with room above, it rises 8 over 24", Math.min(...open.map(at => at.y)), 392);
        const [rising, topped, coming] = thirds(open);
        expect("a third of its frames in each pose, a frame either way",
            [rising, topped, coming].every(one => Math.abs(one.length - open.length / 3) <= 1),
            true);
        expect("rising as it stretches, held at the top tucked, falling as it reaches",
            [rising.every((at, i) => i === 0 || at.y <= rising[i - 1].y),
                topped.every(at => at.y === 392),
                coming.every((at, i) => i === 0 || at.y >= coming[i - 1].y),
                rising[0].y, coming[coming.length - 1].y > 392], [true, true, true, 400, true]);
        const high = arc({ x: 100, y: 2 }, { x: 118, y: 2 });
        expect("on the top row it still rises 2, to the screen's edge and no further",
            [Math.min(...high.map(at => at.y)), thirds(high)[1].every(at => at.y === 0)],
            [0, true]);
        expect("on a ledge under a row, no higher than the row's bottom",
            Math.min(...arc({ x: 100, y: 400 }, { x: 124, y: 400 }, 397).map(at => at.y)), 397);
        expect("a short gap still rises 2",
            Math.min(...arc({ x: 100, y: 400 }, { x: 103, y: 400 }).map(at => at.y)), 398);
        // Each pose a beat of the route shows, in order, and for how many ms.
        const posed = (route, name) => {
            const beat = route.beats.find(one => one.name === name);
            const shown = [];
            for (let t = beat.begins; t < beat.ends; t++) {
                const pose = Paths.frame(route, t).pose;
                if (shown.length && shown[shown.length - 1].pose === pose)
                    shown[shown.length - 1].ms += 1;
                else
                    shown.push({ pose, ms: 1 });
            }
            return shown;
        };
        const aloft = posed(across, "leaps");
        expect("rising, tucked at the top, then reaching down, never mid-stride",
            aloft.map(one => one.pose), ["leaping", "tucked", "falling"]);
        expect("each seen two frames at 60 Hz", aloft.every(one => one.ms >= 33), true);
        const top = across.beats.find(beat => beat.name === "leaps");
        // Level, its arc is highest halfway.
        expect("tucked at the top of its arc",
            [top.from.y, Paths.frame(across, (top.begins + top.ends) / 2).pose], [7, "tucked"]);
        expect("rising before it and falling after",
            [Paths.frame(across, top.begins).pose, Paths.frame(across, top.ends - 1).pose],
            ["leaping", "falling"]);
        // The shortest hop, level: a fifth of it is under two 60 Hz frames.
        const hop = Ways.begun({ cell: 1, wide: 8, tall: 8 }, { x: 100, y: 400 });
        Ways.leap(hop, { x: 110, y: 400 }, 0);
        expect("a hop too short to hold the tuck skips it rather than flash it",
            posed(Ways.timed(hop), "leaps").map(one => [one.pose, one.ms >= 33]),
            [["leaping", true], ["falling", true]]);
        expect("never a jump between two frames", smooth(seen(across)), true);
        expect("a walk along one column leaps nothing",
            Paths.walk(seat(100), seat(400), world({ pause: 0 })).beats
                .some(beat => beat.name === "leaps"), false);

        // Three marks of 16 centred on 624, from 600 to 648, each inked
        // from 4 to 12 down, and the cat stops half a mark's cell before
        // them (VKCF3Z 3d799798).
        const lit = Paths.marks(3, 1248, 16);
        expect("the marks, centred", [lit.from, lit.to], [600, 648]);
        const one = [{ left: 16, top: 16, width: 1216, height: 768, column: 0 }];
        const hopping = Paths.walk(seat(300), seat(900),
            world({ pause: 0, cols: Paths.columns(one, 0), marks: lit }));
        const crossing = hopping.beats.slice(hopping.beats.findIndex(beat => beat.name === "bats"));
        expect("past the marks it bats the first, then hops onto each, perching, and off",
            crossing.slice(0, 10).map(beat => beat.name), ["bats", "crouches", "hops", "perches",
                "hops", "perches", "hops", "perches", "hops", "lands"]);
        expect("batting from half a mark's cell before them",
            [crossing[0].from.x, crossing[0].pose], [584, "batting"]);
        const perched = crossing.filter(beat => beat.name === "perches");
        expect("one mark a hop, on the middle of each",
            perched.map(beat => beat.to.x + 4), [608, 624, 640]);
        expect("its paws on a mark's ink, only its head off the screen",
            perched.every(beat => beat.to.y + 8 > 4 && beat.to.y + 8 <= 12 && beat.to.y >= -4),
            true);
        const perching = perched.map(beat => beat.ends - beat.begins);
        expect("crouched there about 300 ms, never the same twice running",
            perching.every((ms, i) => ms >= 225 && ms <= 375 && (i === 0
                || Math.abs(ms - perching[i - 1]) > 1)), true);
        expect("crouched as it perches",
            Array.from(new Set(perched.map(beat => beat.pose))), ["pouncing"]);
        const off = crossing[9];
        expect("and off past them onto the top, crouched",
            [off.from.x, off.from.y, off.pose], [656, 7, "pouncing"]);
        const pace = (from, to) => Math.abs(to.to.x - from.from.x) / (to.ends - from.begins);
        const walked = hopping.beats.find(beat => beat.name === "goes");
        expect("across them no quicker than it walks, never a dash",
            pace(crossing[0], off) <= pace(walked, walked), true);
        expect("a walk that doesn't pass them hops onto none",
            Paths.walk(seat(100), seat(400), world({ pause: 0, marks: lit })).beats
                .some(beat => beat.name === "perches" || beat.name === "bats"), false);

        // A move between workspaces holds its ground through niri's slide,
        // on the glass as the bar is, and walks only once the slide
        // settles, from where it held: riding its window off the screen
        // brought it back in somewhere else (VKCF3Z 4138e938, 4d18ade4).
        const down = Paths.moves(seat(100), seat(900), world({ way: 1 }));
        const held = down.beats[0];
        expect("a move holds where it sat first, as long as the slide",
            [held.name, held.ends], ["rides", 231]);
        const sat = t => [Paths.frame(down, t).x, Paths.frame(down, t).y, Paths.frame(down, t).pose];
        expect("as it sat, still, the whole slide",
            [0, 10, 50, 100, 230].map(sat), [0, 10, 50, 100, 230].map(() => [100, 7, null]));
        const late = Paths.moves(seat(100), seat(900), world({ way: 1, ahead: 50 }));
        expect("told of the slide late, it holds only what's left of it",
            [Paths.frame(late, 180).beat === "rides", Paths.frame(late, 182).beat === "rides",
                Paths.frame(late, down.ends - 50).done], [true, false, true]);
        // On a screen drawing slowly, told of the slide later than it holds,
        // a move yet to take a step is still where it began: it finds its way
        // again as niri says the new columns, never cut short before it's
        // moved at all (Y2YBTS, 2026-11-14, a0cd2ad5: a builder drawing every
        // 112 ms, 249 ms behind a 231 ms hold).
        const slow = Paths.moves(seat(100), seat(900),
            world({ way: 1, frame: 112, ahead: MarginCat.behind(112, "slide") }));
        expect("a move told of its slide too late to hold, yet to step, finds its way again",
            [slow.ahead > held.ends, Paths.replans(slow, 0, false)], [true, true]);
        expect("stepped past its hold, it's cut short; still holding, it isn't",
            [Paths.replans(slow, 0, true), Paths.replans(down, 100, true),
                Paths.replans(down, 240, true)], [false, true, false]);
        // Its seat said only once the hold is over, as niri can on a
        // loaded machine, the way there begins where the hold left it,
        // then, never part way along it: cut into a walk, the move was
        // never its own (the-margin-cat-follows-the-keys-on-niri, a
        // builder drawing 87 ms apart, 2026-11-14).
        const resumed = Paths.resumed(down, 300);
        expect("its seat said late, its way there begins as it's said",
            [Paths.frame(resumed, 299).x, Paths.frame(resumed, 299).beat,
                Paths.frame(resumed, 300 + down.ends - 231).done], [100, "rides", true]);
        expect("its seat said as it holds, nothing of the move is moved",
            Paths.resumed(down, 100).ahead, down.ahead);
        const seatless = Paths.moves(seat(100), null, world({ way: 1 }));
        expect("with no seat yet, it holds on a little past the slide, then no more",
            [Paths.holdsOn(seatless, 231 + 200), Paths.holdsOn(seatless, 231 + 300)],
            [true, false]);
        const still = frames => frames.every((at, i) => i === 0
            || Math.hypot(at.x - frames[i - 1].x, at.y - frames[i - 1].y) <= 4);
        for (const [way, name] of [[1, "down"], [-1, "up"]]) {
            const move = Paths.moves(seat(100), seat(900), world({ way }));
            expect(`${name} a workspace, never off the screen, and on to its seat`,
                [seen(move).length > 0 && seen(move).every(at => at.y >= 0 && at.y + 8 <= 800),
                    Paths.frame(move, move.ends)],
                [true, { x: 900, y: 7, pose: null, turn: 0, facing: 1, beat: "sits",
                    done: true }]);
            expect(`${name}, all it shows is smooth, from the hold on`, still(seen(move)), true);
        }
        // Settled over a new window's face, it leaves by that face's
        // nearest edge, the shortest way, and never falls down through it.
        const faced = Paths.moves({ x: 948, y: 399 }, seat(900), world({ way: -1, now: one }));
        const out = faced.beats.find(beat => beat.name !== "rides"
            && Math.hypot(beat.to.x - beat.from.x, beat.to.y - beat.from.y) > 0);
        expect("over a face once it settles, out by its nearest edge first",
            [out.name, out.to.x, out.to.y], ["shoved", 1232, 399]);
        const inFace = at => at.x < 1231.5 && at.x + 8 > 16.5 && at.y < 783.5 && at.y + 8 > 16.5;
        expect("and in no face from there to its seat, never a fall",
            [seen(faced).filter(at => at.t >= out.ends).some(inFace),
                faced.beats.some(beat => beat.kind === "falls"), Paths.frame(faced, faced.ends).done],
            [false, false, true]);

        // Three columns: one window, two stacked with a gap at 300, and
        // two with one at 500 (Joel, 2026-11-13, by WCJQSG fbb80ff0: "i
        // might be on a window in the second row and it needs to travel
        // to it. or I am in a column on the left and change workspaces
        // where the last focused window is on the right").
        const three = [
            { left: 16, top: 16, width: 400, height: 768, column: 0 },
            { left: 432, top: 16, width: 400, height: 284, column: 1 },
            { left: 432, top: 316, width: 400, height: 468, column: 1 },
            { left: 848, top: 16, width: 384, height: 484, column: 2 },
            { left: 848, top: 516, width: 384, height: 268, column: 2 }
        ];
        const among = more => world(Object.assign({ pause: 0, now: three,
            cols: Paths.columns(three, 0) }, more));
        // Never gliding: each walk level, each climb straight up or down,
        // and every beat begun where the last one ended.
        const glides = way => way.beats.some((beat, i) =>
            (beat.name === "goes" && beat.from.y !== beat.to.y)
            || (beat.name === "climbs" && beat.from.x !== beat.to.x)
            || (i > 0 && Math.hypot(beat.from.x - way.beats[i - 1].to.x,
                beat.from.y - way.beats[i - 1].to.y) > 0.01));
        const gapsLeapt = way => way.beats.filter(beat => beat.name === "leaps"
            && Math.abs(beat.to.x - beat.from.x) >= 16).length;
        const ends = way => [Paths.frame(way, way.ends).x, Paths.frame(way, way.ends).y];
        const lower = { x: 532, y: 307 };
        const downTo = Paths.walk(seat(100), lower, among({}));
        expect("to a window under another it goes down a side and leaps the gap to it",
            [downTo.beats.some(beat => beat.name === "climbs" && beat.to.y > beat.from.y),
                gapsLeapt(downTo) > 0, ends(downTo), glides(downTo)],
            [true, true, [532, 307], false]);
        const under = Paths.walk(seat(532), lower, among({}));
        expect("from the top of its own column too, down past the window over it",
            [under !== null, under ? ends(under) : null, under ? glides(under) : null],
            [true, [532, 307], false]);
        const outOf = Paths.walk(lower, { x: 948, y: 507 }, among({}));
        expect("and from one to another", [ends(outOf), glides(outOf), gapsLeapt(outOf) > 0],
            [[948, 507], false, true]);
        expect("all of it smooth", [downTo, under, outOf].every(way => smooth(seen(way))), true);
        for (const [way, name] of [[1, "down"], [-1, "up"]]) {
            const sideways = Paths.moves(seat(100), seat(1100), among({ way }));
            expect(`${name} a workspace from the left to the right, it leaps each gap it meets`,
                [gapsLeapt(sideways) >= 2, ends(sideways), glides(sideways), smooth(seen(sideways))],
                [true, [1100, 7], false, true]);
        }
        const deep = Paths.moves(seat(100), { x: 948, y: 507 }, among({ way: 1 }));
        expect("down a workspace to a window under another, never gliding",
            [ends(deep), glides(deep), smooth(seen(deep))], [[948, 507], false, true]);

        // How it rides, falls and climbs back as the windows under it
        // change is margin-cat-rides.qml's.
        expect("with nothing on the new workspace it only holds",
            Paths.moves(seat(100), seat(100), world({ way: 1, now: [] })).beats
                .map(beat => beat.name), ["rides"]);
        // An empty workspace: it falls to the screen's floor, across only
        // the backdrop, and sits there; windows come, it climbs to its seat
        // by an edge, never up a face (VKCF3Z, 2026-11-14, e5a3ee44).
        const bare = Paths.walk(seat(100), { x: 100, y: 792 }, world({ pause: 0, hurried: true,
            now: [], cols: [], before: cols }));
        expect("its workspace emptied, it falls straight down to the floor and sits",
            [bare.beats.some(beat => beat.kind === "falls"), seen(bare).every(one => one.x === 100),
                Paths.frame(bare, bare.ends)],
            [true, true, { x: 100, y: 792, pose: null, turn: 0, facing: 1, beat: "sits",
                done: true }]);
        expect("reaching down all the way",
            posed(bare, "falls").map(one => one.pose), ["falling"]);
        const risen = Paths.walk({ x: 300, y: 792 }, seat(300), world({ pause: 0 }));
        const onFace = at => at.x < 615.5 && at.x + 8 > 16.5 && at.y < 783.5 && at.y + 8 > 16.5;
        expect("windows come, it climbs to its seat by an edge, never up a face",
            [risen.beats.some(beat => beat.name === "climbs"), seen(risen).some(onFace),
                Paths.frame(risen, risen.ends).x, Paths.frame(risen, risen.ends).y],
            [true, false, 300, 7]);
        expect("the theme's slowdown stretches all of a move",
            Math.round(Paths.moves(seat(100), seat(900), world({ way: 1, slowdown: 2 })).ends),
            Math.round(down.ends * 2));
    }
}

// Where the margin's cat sits and how big it's drawn, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat/MarginCat.js, MarginCatRoutes.js, MarginCatWays.js,
// MarginCatFalls.js, MarginCatPaths.js and MarginCatPlays.js beside this
// file and runs
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
        const asked = name => Plays.scene("idle", draws(0), "", name);
        expect("a play asked for comes next",
            [asked("hop").name, asked("wander").name, asked("yarn").name, asked("pounce").name,
                asked("grooming").name], ["hop", "wander", "yarn", "pounce", "grooming"]);
        expect("even just after the same one",
            Plays.scene("idle", draws(0), "hop", "hop").name, "hop");
        expect("but never a move while he reads",
            inPlace.includes(Plays.scene("paused", draws(0), "", "hop").name), true);
        expect("nor anything while he types",
            Plays.scene("typing", draws(0), "", "wander").beats.length, 1);
    }

    // A walk of 100 cells of 1 to the right, thinking 200 ms: it turns,
    // thinks, crouches back, walks easing in and out with its strides stepped
    // by the ground it covers, overshoots, steps back and crouches as it
    // stops.
    function walks(expect) {
        const cat = more => Object.assign({ cell: 1, ratio: 1, wide: 8, tall: 8, air: 1 }, more);
        const on = x => ({ x, y: 0 });
        const going = Paths.walk(on(100), on(200), cat({ pause: 200 }));
        const at = t => Paths.frame(going, t);
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
        // Each pair of legs apart two strides and gathered two, the hind pair
        // a stride behind (RSCH-20261113-24R2QV's rule 4).
        const walk = ["walkingA", "walkingC", "walkingB", "walkingD"];
        expect("in its four strides, in turn",
            frames.filter((frame, i) => i === 0 || frame.pose !== frames[i - 1].pose)
                .slice(0, 5).map(frame => frame.pose), [...walk, walk[0]]);
        expect("each stride a stretch of ground, never a frame's time",
            frames.every(frame => frame.pose === walk[
                Math.floor((frame.x - 99) / going.stride) % walk.length]), true);
        const changes = frames.filter((frame, i) => i > 0 && frame.pose !== frames[i - 1].pose);
        expect("as many strides as its way holds, the last ending as it stops",
            changes.length, Math.ceil((201.5 - 99) / going.stride) - 1);
        expect("it walks on past its seat a cell and a half",
            Math.round(at(ends("goes") - 1e-6).x * 10) / 10, 201.5);
        expect("steps back onto it", [at(ends("settles") - 1e-6).beat,
            Math.round(at(ends("settles") - 1e-6).x * 10) / 10], ["settles", 200]);
        expect("and crouches as it stops, where it'll sit",
            [at(begins("lands")).pose, at(begins("lands")).x], ["pouncing", 200]);
        expect("then sits, done", [at(going.ends).done, at(going.ends).x, at(going.ends).pose],
            [true, 200, null]);
        expect("a short way at a walking pace, about two seconds",
            Math.round(ends("goes") - begins("goes")), MarginCat.CRUISES);

        const back = Paths.walk(on(200), on(100), cat({ pause: 0 }));
        expect("back the other way it faces left", back.facing, -1);
        expect("drawing back to the right",
            Math.round(Paths.frame(back, back.beats[1].ends - 1e-6).x), 201);
        expect("with no thought it doesn't stop to think",
            back.beats.map(beat => beat.name), ["turns", "crouches", "goes", "settles", "lands"]);
        expect("where it already sits, to the screen's pixel, it doesn't walk",
            Paths.walk(on(40), on(40.2), cat({ ratio: 1.5 })), null);
        const hurried = Paths.walk(on(150), on(300), cat({ hurried: true, facing: 1 }));
        expect("a walk begun on the way neither turns nor thinks nor crouches",
            hurried.beats.map(beat => beat.name), ["goes", "settles", "lands"]);
        expect("and sets off from where it is", Paths.frame(hurried, 0).x, 150);
        const turned = Paths.walk(on(150), on(50), cat({ hurried: true, facing: 1 }));
        expect("one going back the way it came turns first", turned.beats[0].name, "turns");
        const slowed = Paths.walk(on(100), on(200), cat({ pause: 200, slowdown: 2 }));
        expect("the theme's slowdown stretches all of it", slowed.ends, going.ends * 2);
        const far = Paths.walk(on(0), on(1000), cat({ pause: 0 }));
        const goes = far.beats.find(beat => beat.name === "goes");
        // Drawn back no further than the screen's left edge, it goes 1001.5.
        expect("far, it hurries, never more than 120 cells a second",
            Math.round(goes.ends - goes.begins), Math.round(1001.5 / 120 * 1000));
        expect("its stride at most ten cells", far.stride, 10);
        const near = Paths.walk(on(0), on(20), cat({ cell: 2, pause: 0 }));
        expect("near, it ambles, its stride half its body", near.stride, 8);
        const shown = MarginCat.walked([100, 116.7, 133.3, 150, 200], [1.2, 0.4, 3.04, 0.8], 1,
            { begun: 7, stepped: 6 });
        expect("a walk's frames as niri showed them, the gaps between them sorted",
            shown.said, {
                frames: 5, median: 16.7, p99: 50, slowest: 50, drawn: 3, unseen: 1, begun: 7,
                stepped: 6
            });
        // Its steps beside its frames say whether it moved on each, or
        // was drawn again standing still (2026-11-13: Qt paced a 120 Hz
        // screen's steps at 60 where niri said 60 Hz).
        expect("as the journal says them", shown.line, "cat: a walk presented 5 of its 7 frames "
            + "a median 16.7 ms apart, the p99 50 ms, the slowest 50 ms, each drawn within 3 ms, "
            + "1 never shown, stepped 6 times");
        expect("a hop's frames are said as a walk's",
            MarginCat.walked([100, 116.7], [1], 0, { begun: 2, what: "hop" }).line
                .split(" of its")[0],
            "cat: a hop presented 2");
        expect("a walk nothing showed says so", MarginCat.walked([], [], 0, { begun: 0 }).said,
            {
                frames: 0, median: 0, p99: 0, slowest: 0, drawn: 0, unseen: 0, begun: 0,
                stepped: 0
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
        expect("long enough to be seen", leaps.ends - leaps.begins >= 160, true);
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

        // The window over its own resized, as niri does: it rides the top
        // it stands on, on niri's own spring, as the window's edge goes,
        // never across or through a window (Joel, 2026-11-13, by WCJQSG
        // 2bd7c7c6: "the kitten also needs to handle resizing windows").
        const ided = three.map((tile, i) => Object.assign({ id: i + 1 }, tile));
        const split = top => ided.map(tile => tile.column !== 2 ? tile : Object.assign({},
            tile, tile.id === 4 ? { height: top - 32 } : { top, height: 784 - top }));
        const resized = (now, to) => Paths.walk({ x: 948, y: 507 }, to, among({ hurried: true,
            cols: Paths.columns(now, 0), before: Paths.columns(ided, 0) }));
        for (const [top, name] of [[316, "shrunk, it rides up"], [616, "grown, it rides down"]]) {
            const rode = resized(split(top), { x: 948, y: top - 9 });
            expect(`the window over its own ${name} with its own window's top`,
                [rode.beats[0].name, rode.beats[0].to.y, Paths.frame(rode, 40).y,
                    seen(rode).every(one => one.x === 948)],
                ["rides", top - 9, 507 + (top - 516) * MarginCat.slid(40, spring), true]);
        }
        const heard = Paths.walk({ x: 948, y: 507 }, { x: 948, y: 307 }, among({ hurried: true,
            cols: Paths.columns(split(316), 0), before: Paths.columns(ided, 0), ahead: 30 }));
        expect("told of the resize late, it rides as far into niri's spring as the edge has gone",
            [Paths.frame(heard, 0).y, Math.round(heard.beats[0].ends)],
            [507 - 200 * MarginCat.slid(30, spring), MarginCat.slides(spring) - 30]);
        // Sitting, it hears only its own window's tile (24a136dde), so the
        // window over it is known by niri's row for its own, the second.
        const own = top => [Object.assign({ row: 2 }, split(top).find(tile => tile.id === 5))];
        const alone = Paths.walk({ x: 948, y: 507 }, { x: 948, y: 607 }, among({
            hurried: true, cols: Paths.columns(own(616), 0), before: Paths.columns(own(516), 0),
            ahead: 63, frame: 8.2 }));
        expect("the window over its own grown, heard alone and late, it rides flush, never in it",
            [0, 16, 64].map(t => [Paths.frame(alone, t).y, Paths.frame(alone, t).pose]),
            [0, 16, 64].map(t => [507 + 100 * MarginCat.slid(t + 63, spring), null]));
        const narrowed = ided.map(tile => tile.column !== 2 ? tile
            : Object.assign({}, tile, { width: 60 }));
        const left = resized(narrowed, { x: 874, y: 507 });
        expect("its column narrowed out from under it, it falls",
            [left.beats[0].name, left.beats.some(beat => beat.name === "rides")], ["falls", false]);

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

        // Heard late, a resize's ride catches niri's edge up without a jump (VKCF3Z,
        // 2026-11-13 a36e05f9): carried up within two frames in the pose it
        // had, left in the air a fall that sits flush by the time the edge
        // stops, never so far behind that what follows passes through it.
        expect("heard 25 ms and a frame late of a slide, and 30 ms more of a resize",
            [MarginCat.behind(1000 / 60, "slide"), MarginCat.behind(1000 / 60, "resize"),
                MarginCat.behind(1000 / 120, "slide"), MarginCat.behind(1000 / 120, "resize")],
            [42, 72, 33, 63]);
        const hz120 = 1000 / 120;
        const edgeAt = (dy, t) => 400 + dy * MarginCat.slid(t + 40, spring);
        const riding = (dy, more) => {
            const way = Ways.begun({ cell: 1, wide: 8, tall: 8 }, { x: 100, y: 400 });
            Ways.rides(way, { x: 100, y: 400 + dy }, Object.assign({ ms: 191, frame: hz120,
                edge: t => MarginCat.slid(t + 40, spring) }, more ?? {}));
            return Ways.timed(way);
        };
        const each = (route, ends) => {
            const all = [];
            for (let t = 0; t < ends; t++)
                all.push(Object.assign({ t }, Ways.frame(route, t)));
            return all;
        };
        const runs = frames => frames.reduce((all, one) => {
            const last = all[all.length - 1];
            if (last && last.pose === one.pose)
                last.ms += 1;
            else
                all.push({ pose: one.pose, ms: 1 });
            return all;
        }, []);
        const lifted = riding(-300);
        expect("carried up, it starts where it sat", Ways.frame(lifted, 0).y, 400);
        // On the GPU rig at 120 Hz niri drew a resize's frames 16.3 ms apart,
        // so its two frames are never shorter than two at 60 Hz.
        const hz60 = 1000 / 60;
        expect("halfway to the edge a 60 Hz frame in, though it walked at 120",
            Math.round(Ways.frame(lifted, hz60).y), Math.round((400 + edgeAt(-300, hz60)) / 2));
        expect("and on the edge two such frames in, and after",
            [2 * hz60, 3 * hz60, 100].map(t => Math.round(Ways.frame(lifted, t).y)),
            [2 * hz60, 3 * hz60, 100].map(t => Math.round(edgeAt(-300, t))));
        expect("in the pose it had", each(lifted, 191).every(one => one.pose === null), true);
        const dropped = riding(300, { away: true });
        const stops = each(dropped, 191).find(one => 300 - (edgeAt(300, one.t) - 400) < 1).t;
        expect("left in the air, it starts where it sat, and never sinks into the window",
            [Ways.frame(dropped, 0).y, each(dropped, 191).every(one =>
                one.y <= edgeAt(300, one.t) + 1e-6)], [400, true]);
        expect("it sits flush by the time the edge stops",
            Math.abs(Ways.frame(dropped, stops).y - edgeAt(300, stops)) < 0.5, true);
        const falling = runs(each(dropped, 191)).filter(one => one.pose !== null);
        expect("falling in the falling drawing, landing crouched, each seen two frames at 60 Hz",
            [falling.map(one => one.pose), falling.every(one => one.ms >= 33)],
            [["falling", "pouncing"], true]);
        const pressed = riding(300, { away: true, room: 20 });
        expect("never more than its room behind, before what follows passes through it",
            each(pressed, 191).every(one => edgeAt(300, one.t) - one.y <= 20 + 1e-6), true);
        expect("a gap under two cells closes with no pose",
            each(riding(1.5, { away: true }), 191).every(one => one.pose === null), true);

        // Its own motion lands on its art's own pixels, cells of 2 here,
        // carrying the part-cell a ride leaves it at (RSCH-20261113-24R2QV,
        // finding 1); the ride keeps niri's pixels.
        const stepped = Ways.begun({ cell: 2, wide: 16, tall: 16 }, { x: 101, y: 400 });
        Ways.rides(stepped, { x: 101, y: 413.3 }, { ms: 100, frame: hz120, edge: t => t / 100 });
        Ways.goes(stepped, 160, 413.3);
        Ways.leap(stepped, { x: 175, y: 380.7 }, 2);
        Ways.falls(stepped, { x: 175, y: 431 });
        const byItself = Ways.timed(stepped);
        const cells = v => Math.abs(v / 2 - Math.round(v / 2)) < 1e-6;
        const stepping = each(byItself, byItself.ends).map(one => Object.assign(one,
            { on: byItself.beats.find(beat => one.t < beat.ends) }));
        expect("each frame of its own a whole number of cells from where its beat starts or ends",
            stepping.filter(one => one.on.kind !== "rides").every(one => ["x", "y"].every(axis =>
                cells(one[axis] - one.on.from[axis]) || cells(one[axis] - one.on.to[axis]))),
            true);
        expect("never stepping back to get there",
            stepping.every((one, i) => i === 0 || one.x >= stepping[i - 1].x), true);
        expect("walking flush on the part-cell the ride left it at, and ending where it's going",
            [stepping.filter(one => one.beat === "goes").every(one => one.y === 413.3),
                Ways.frame(byItself, byItself.ends - 1e-6).y, Ways.frame(byItself, byItself.ends).y],
            [true, 431, 431]);
        expect("the ride on niri's own pixels",
            Math.round(Ways.frame(byItself, 50).y * 100), 40665);
    }
}

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

        walks(expect);
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
        expect("while he types it's mild", MarginCat.level(heard()), "typing");
        expect("ten seconds with no keys, a little more",
            MarginCat.level(heard({ paused: true })), "paused");
        expect("a minute with none, livelier",
            MarginCat.level(heard({ paused: true, idle: true })), "idle");
        expect("asleep it does nothing",
            MarginCat.level(heard({ paused: true, idle: true, asleep: true })), "asleep");
        expect("in Do Not Disturb it keeps still at any level",
            MarginCat.level(heard({ paused: true, idle: true, quiet: true })), "still");
        expect("as on a call, with a camera in use",
            MarginCat.level(heard({ paused: true, idle: true, onCamera: true })), "still");
        expect("as with the screen locked",
            MarginCat.level(heard({ paused: true, locked: true })), "still");
        expect("as where it's drawn on no screen, fullscreen or shared",
            MarginCat.level(heard({ paused: true, unseen: true })), "still");

        // The rest before each scene: about a minute while he types, since
        // the keys are his; a few seconds once he pauses, and fewer once
        // he's left them, each drawn at random.
        expect("typing rests 45 s at the least", MarginCat.rest("typing", 0), 45000);
        expect("and 75 s at the most", MarginCat.rest("typing", 1), 75000);
        expect("a pause rests 5 to 12 s",
            [MarginCat.rest("paused", 0), MarginCat.rest("paused", 1)], [5000, 12000]);
        expect("a minute with no keys rests 3 to 8 s",
            [MarginCat.rest("idle", 0), MarginCat.rest("idle", 1)], [3000, 8000]);
        expect("asleep there's no next scene", MarginCat.rest("asleep", 0.5), null);
        expect("nor in Do Not Disturb", MarginCat.rest("still", 0.5), null);

        scenes(expect);

        // Where a short wander goes: along the margin from its seat, the
        // way it was drawn, or the other where that one's off the screen or
        // on the marks; nowhere where neither fits.
        const seat = { x: 296, y: 3 };
        const wanders = more => MarginCat.wander(at(column(100, 12, 400),
            Object.assign({ seat, step: 1, cells: 20, way: 1 }, more)));
        expect("it wanders its cells the way drawn", wanders(), { x: 316, y: 3 });
        expect("or the other way", wanders({ way: -1 }), { x: 276, y: 3 });
        expect("each cell its own", wanders({ step: 2 }), { x: 336, y: 3 });
        expect("the other way where the screen ends",
            wanders({ seat: { x: 980, y: 3 } }), { x: 960, y: 3 });
        expect("the other way where the marks are",
            wanders({ seat: { x: 460, y: 3 }, marks: 3 }), { x: 440, y: 3 });
        expect("nowhere where neither way fits",
            wanders({ seat: { x: 10, y: 3 }, across: 30 }), null);
        expect("nor with no seat", wanders({ seat: null }), null);

        // A hop's rise, in its cells: none as it leaves and lands, its
        // height midway, as a thing thrown up falls.
        expect("a hop leaves and lands on the ground",
            [MarginCat.hopped(0, 400, 2), MarginCat.hopped(400, 400, 2)], [0, 0]);
        expect("and is highest midway", MarginCat.hopped(200, 400, 2), 2);
        expect("rising as it falls", MarginCat.hopped(100, 400, 2), MarginCat.hopped(300, 400, 2));
        expect("and never below the ground",
            [MarginCat.hopped(-10, 400, 2), MarginCat.hopped(410, 400, 2)], [0, 0]);
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
        const moves = ["hop", "wander"];
        const chances = [0, 0.2, 0.4, 0.6, 0.8, 0.99];

        const typed = chances.map(chance => MarginCat.scene("typing", draws(chance), ""));
        expect("while he types it's one pose and back",
            typed.map(played => played.beats.length), [1, 1, 1, 1, 1, 1]);
        expect("an ear flick, a tail swish or a blink",
            Array.from(new Set(typed.map(played => played.beats[0].pose))).sort(),
            ["blinking", "earFlick", "tailSwish"]);
        expect("held about 375 ms",
            typed.every(played => played.beats[0].ms >= 281 && played.beats[0].ms <= 469), true);

        const paused = chances.map(chance => MarginCat.scene("paused", draws(chance), ""));
        expect("paused, it plays in place, each of them",
            Array.from(new Set(paused.map(played => played.name))).sort(),
            inPlace.slice().sort());
        expect("never walking or hopping while he reads",
            paused.every(played => played.beats.every(beat => typeof beat.pose === "string"
                && beat.walk !== true && beat.hop !== true)), true);
        expect("each play short, under two and a half seconds",
            paused.every(played => long(played.beats) > 0 && long(played.beats) <= 2500), true);

        const again = inPlace.every(name => chances.every(chance =>
            MarginCat.scene("paused", draws(chance), name).name !== name));
        expect("never the same play twice running", again, true);

        const idle = chances.map(chance => MarginCat.scene("idle", draws(0.7, chance), ""));
        expect("idle, three in four of its plays are in place",
            idle.every(played => inPlace.includes(played.name)), true);
        const moved = chances.map(chance => MarginCat.scene("idle", draws(0.8, chance), ""));
        expect("and one in four a move",
            Array.from(new Set(moved.map(played => played.name))).sort(), moves);
        const hop = moved.find(played => played.name === "hop").beats;
        expect("a hop crouches, rises and lands crouched",
            hop.map(beat => [beat.pose, beat.hop === true]),
            [["pouncing", false], ["pouncing", true], ["pouncing", false]]);
        expect("a cell or two up", hop[1].cells >= 1 && hop[1].cells <= 2, true);
        const wander = moved.find(played => played.name === "wander").beats;
        expect("a wander walks off, sits a moment, and walks back",
            [wander[0].to, wander[1].pose, wander[2].to], ["wander", "nothingHere", "column"]);
        expect("16 to 40 cells, either way",
            wander[0].cells >= 16 && wander[0].cells <= 40 && Math.abs(wander[0].way) === 1,
            true);

        // Each beat drawn a quarter shorter or longer than its own.
        const low = MarginCat.scene("paused", draws(0.1, 0, 0, 0, 0, 0, 0), "").beats;
        const high = MarginCat.scene("paused", draws(0.1, 0.999, 0.999, 0.999, 0.999, 0.999,
            0.999), "").beats;
        expect("its beats vary a quarter either way",
            low.every((beat, i) => Math.abs(high[i].ms / beat.ms - 1.25 / 0.75) < 0.05), true);
        // Eight poses a second at the most: no beat under 125 ms.
        expect("no beat is quicker than an eighth of a second",
            [...typed, ...paused, ...idle, ...moved].every(played => played.beats.every(beat =>
                beat.walk === true || beat.ms >= 125)), true);
        expect("asleep there's no scene", MarginCat.scene("asleep", draws(0), "").beats, []);
        expect("nor in Do Not Disturb", MarginCat.scene("still", draws(0), "").beats, []);
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

        const shown = MarginCat.walked([100, 116.7, 133.3, 150, 200], [1.2, 0.4, 3.04, 0.8], 1,
            { begun: 7 });
        expect("a walk's frames as niri showed them, the gaps between them sorted",
            shown.said,
            { frames: 5, median: 16.7, p99: 50, slowest: 50, drawn: 3, unseen: 1, begun: 7 });
        expect("as the journal says them", shown.line, "cat: a walk presented 5 of its 7 frames "
            + "a median 16.7 ms apart, the p99 50 ms, the slowest 50 ms, each drawn within 3 ms, "
            + "1 never shown");
        expect("a hop's frames are said as a walk's",
            MarginCat.walked([100, 116.7], [1], 0, { begun: 2, what: "hop" }).line
                .split(" of its")[0],
            "cat: a hop presented 2");
        expect("a walk nothing showed says so", MarginCat.walked([], [], 0, { begun: 0 }).said,
            { frames: 0, median: 0, p99: 0, slowest: 0, drawn: 0, unseen: 0, begun: 0 });

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

        // A screen 800 tall, a workspace and its gap 880, a cat of 8
        // cells of 1 seated at 2 below the top.
        const moving = (way, more) => MarginCat.moves({ x: 100, y: 2 }, { x: 500, y: 2 },
            Object.assign({ way, step: 880, tall: 8, cell: 1, ratio: 1, facing: 1, spring },
                more ?? {}));
        const down = moving(1);
        const rides = down.beats[0];
        expect("a move rides its column first, as long as the slide",
            [rides.name, rides.ends], ["rides", 231]);
        expect("carried exactly as the world, up a workspace going down",
            [10, 50, 100].map(t => MarginCat.moved(down, t).y),
            [10, 50, 100].map(t => 2 - 880 * MarginCat.slid(t, spring)));
        expect("straight up its old column, as it sat", [10, 50, 100].map(t => {
            const where = MarginCat.moved(down, t);
            return [where.x, where.pose, where.turn, where.facing];
        }), [[100, null, 0, 1], [100, null, 0, 1], [100, null, 0, 1]]);
        expect("going down, it climbs down from above onto its seat",
            down.beats.map(beat => beat.name), ["rides", "climbs", "lands"]);
        const climbs = down.beats[1];
        const above = MarginCat.moved(down, climbs.begins);
        expect("from out of sight above its seat", [above.x, above.y], [500, -8]);
        expect("head down, a quarter turn back from its profile",
            [above.turn, above.facing], [-1, -1]);
        const steps = Array.from({ length: 20 }, (_, i) =>
            MarginCat.moved(down, climbs.begins + (climbs.ends - climbs.begins) * i / 20));
        expect("climbing only down, never back", steps.every((where, i) =>
            i === 0 || where.y > steps[i - 1].y), true);
        expect("in its two strides", Array.from(new Set(steps.map(where => where.pose))).sort(),
            ["walkingA", "walkingB"]);
        expect("at an amble over so short a way",
            Math.round(climbs.ends - climbs.begins), Math.round(10 / 40 * 1000));
        expect("it lands on its seat crouched, a moment", [down.beats[2].pose,
            MarginCat.moved(down, down.beats[2].begins).x,
            MarginCat.moved(down, down.beats[2].begins).y], ["pouncing", 500, 2]);
        expect("then sits, done", MarginCat.moved(down, down.ends),
            { x: 500, y: 2, pose: null, turn: 0, facing: 1, beat: "sits", done: true });

        // Going up, its old column falls away below it, so it lets go and
        // stays in the margin as the new workspace comes down under it
        // (VKCF3Z, 294b73a0).
        const nearBy = (x, more) => MarginCat.moves({ x: 100, y: 2 }, { x, y: 2 },
            Object.assign({ way: -1, step: 880, tall: 8, cell: 1, ratio: 1, facing: 1, spring },
                more ?? {}));
        const stepAway = nearBy(130);
        expect("going up, it stays where it sits as long as the slide",
            [stepAway.beats[0].name, stepAway.beats[0].ends], ["stays", 231]);
        expect("still, in its own pose, as the world slides under it", [10, 50, 100].map(t => {
            const where = MarginCat.moved(stepAway, t);
            return [where.x, where.y, where.pose, where.turn, where.facing];
        }), [[100, 2, null, 0, 1], [100, 2, null, 0, 1], [100, 2, null, 0, 1]]);
        expect("then a seat near enough it walks to, on the way it faces, at once",
            stepAway.beats.map(beat => beat.name), ["stays", "goes", "settles", "lands"]);
        expect("along the margin, never faster than a walk",
            Math.round(stepAway.beats[1].ends - stepAway.beats[1].begins),
            Math.round(31.5 / 40 * 1000));
        expect("and sits there, done", MarginCat.moved(stepAway, stepAway.ends),
            { x: 130, y: 2, pose: null, turn: 0, facing: 1, beat: "sits", done: true });
        expect("one behind it, it turns for first",
            nearBy(80).beats.map(beat => beat.name),
            ["stays", "turns", "goes", "settles", "lands"]);
        expect("one right where it is, it only stays",
            [nearBy(100).beats.map(beat => beat.name), nearBy(100).rests], [["stays"], false]);
        const acrossTheScreen = nearBy(500);
        expect("one farther than about 1.5 s it stays for, to rest and walk as ever",
            [acrossTheScreen.beats.map(beat => beat.name), acrossTheScreen.to,
                acrossTheScreen.seat, acrossTheScreen.rests],
            [["stays"], { x: 100, y: 2 }, { x: 500, y: 2 }, true]);
        const slowAway = nearBy(130, { slowdown: 2 });
        expect("the theme's slowdown stretches it, but brings no seat nearer",
            [slowAway.rests, Math.round(slowAway.ends)], [false, Math.round(stepAway.ends * 2)]);
        expect("the theme's slowdown stretches all of a move",
            Math.round(moving(1, { slowdown: 2 }).ends), Math.round(down.ends * 2));

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
}

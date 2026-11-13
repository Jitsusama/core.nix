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
        journeys(expect);
        plays(expect, at, column);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // Journeys thrown at it, the same each run: layouts of one to four
    // columns of one to three windows, scrolled or not, at Joel's laptop's
    // size and scale, and the keys shifting to any window of them, on the
    // same workspace or the one above or below. Every one must end on the
    // window's seat, never jump between two frames at 120 Hz, never go
    // into a window, the marks' ink or off the screen's sides (Joel,
    // 2026-11-13, by WCJQSG bd480983: "the cat should always be moving
    // from where it is to where my focus shifts, in a believable and
    // adorable way. the windows are the walls and roofs it is climbing or
    // jumping off of").
    function journeys(expect) {
        let state = 20261113;
        const random = () => {
            state = (state * 1103515245 + 12345) % 2147483648;
            return state / 2147483648;
        };
        const cell = 4 / 3;
        const at = { cell, ratio: 1.5, wide: 8 * cell, tall: 8 * cell, air: cell, across: 1276,
            height: 773, step: 789, facing: 1, pause: 0, marks: Paths.marks(3, 1276, 16),
            spring: { damping: 1, stiffness: 1600, epsilon: 0.001 } };
        const layout = () => {
            const tiles = [];
            let left = 16 - (random() < 0.3 ? Math.floor(random() * 500) : 0);
            const count = 1 + Math.floor(random() * 4);
            for (let c = 0; c < count; c++) {
                const width = 200 + Math.floor(random() * 700);
                const rows = 1 + Math.floor(random() * 3);
                const shares = [];
                for (let r = 0; r < rows; r++)
                    shares.push(random());
                const spare = 741 - 16 * (rows - 1) - 100 * rows;
                const all = shares.reduce((sum, share) => sum + share, 0);
                let top = 16;
                for (let r = 0; r < rows; r++) {
                    const height = r === rows - 1 ? 757 - top
                        : 100 + Math.floor((spare * shares[r]) / all);
                    tiles.push({ left, top, width, height, column: c, id: tiles.length });
                    top += height + 16;
                }
                left += width + 16;
            }
            return tiles;
        };
        const seatOn = tiles => {
            const tile = tiles[Math.floor(random() * tiles.length)];
            return MarginCat.sits({ column: tile, across: at.across, marks: 3, cell: 16,
                wide: at.wide, tall: at.tall, air: at.air });
        };
        const inside = (x, y, box) => x < box.left + box.width - 0.5
            && x + at.wide > box.left + 0.5 && y < box.top + box.height - 0.5
            && y + at.tall > box.top + 0.5;
        const lit = at.marks;
        const ink = { left: lit.from + 4, top: 4, width: lit.to - lit.from - 8, height: 8 };
        const wrong = [];
        // Every frame of `journey` a cat's, in none of `solid`'s windows
        // but where a window shoved it, and ending on `to`.
        const check = (journey, solid, said, to) => {
            const last = Paths.frame(journey, journey.ends);
            if (Math.abs(last.x - to.x) > 0.01 || Math.abs(last.y - to.y) > 0.01)
                wrong.push(`${said} ends at ${last.x},${last.y}`);
            // Up or down a side at about the pace it walks a top (Joel,
            // 2026-11-13 17:22, by WCJQSG fc3f716a: "Going up the side of
            // a window should be at the same speed roughly as walking on
            // top, it seems accelerated in the video"), each pace its
            // cells a second, of the beats seen.
            const seenOn = beat => Math.max(beat.from.y, beat.to.y) + at.tall > 0
                && Math.min(beat.from.y, beat.to.y) < at.height;
            const pace = beat => (Math.abs(beat.to.x - beat.from.x)
                + Math.abs(beat.to.y - beat.from.y)) / at.cell / ((beat.ends - beat.begins) / 1000);
            const walks = journey.beats.filter(beat => beat.kind === "goes" && seenOn(beat)
                && pace(beat) > 0).map(pace);
            for (const beat of journey.beats.filter(one => one.kind === "climbs" && seenOn(one)))
                if (walks.some(walked => Math.abs(pace(beat) - walked) > walked / 5))
                    wrong.push(`${said} climbs at ${Math.round(pace(beat))} cells a second, `
                        + `walking at ${walks.map(Math.round).join(", ")}`);
            let before = null;
            for (let t = 0; t <= journey.ends && wrong.length < 5; t += 1000 / 120) {
                const one = Paths.frame(journey, t);
                const shown = one.y + at.tall > 0 && one.y < at.height;
                if (one.beat !== "rides" && one.beat !== "shoved"
                    && solid.some(tile => inside(one.x, one.y, tile)))
                    wrong.push(`${said} is inside a window at ${t} ms, ${one.beat}`);
                else if (shown && one.beat !== "hops" && one.beat !== "perches"
                    && one.beat !== "shoved" && inside(one.x, one.y, ink))
                    wrong.push(`${said} is in the marks at ${t} ms, ${one.beat}`);
                else if (shown && (one.x < -0.01 || one.x + at.wide > at.across + 0.01))
                    wrong.push(`${said} is off the screen's side at ${t} ms, ${one.beat}`);
                else if (shown && before !== null && before.beat !== "rides"
                    && Math.hypot(one.x - before.x, one.y - before.y) > at.tall / 2)
                    wrong.push(`${said} jumps at ${t} ms, ${before.beat} to ${one.beat}`);
                before = shown ? one : null;
            }
        };
        let made = 0;
        for (let n = 0; n < 400 && made < 150; n++) {
            const way = [0, 1, -1][Math.floor(random() * 3)];
            const was = layout();
            const now = way === 0 ? was : layout();
            const from = seatOn(was);
            const to = seatOn(now);
            if (from === null || to === null)
                continue;
            const journey = way === 0
                ? Paths.walk(from, to, Object.assign({ cols: Paths.columns(now, 0) }, at))
                : Paths.moves(from, to, Object.assign({ way, was, now }, at));
            if (journey === null)
                continue;
            made += 1;
            const solid = now.concat(was.map(tile => Object.assign({}, tile,
                { top: tile.top - way * at.step })));
            const said = `${n} (${way}, ${JSON.stringify(from)} to ${JSON.stringify(to)})`;
            check(journey, solid, said, to);
            // And the windows changed under it somewhere on its way past
            // the slide, its way found again from where it's got to.
            const t = journey.beats[0].ends + random() * (journey.ends - journey.beats[0].ends);
            const here = Paths.frame(journey, t);
            const changed = changes(now, random);
            const then = seatOn(changed);
            const again = then === null ? null : Paths.walk({ x: here.x, y: here.y }, then,
                Object.assign({ cols: Paths.columns(changed, 0), hurried: true,
                    before: Paths.columns(now, 0) }, at));
            const starts = again === null ? here : Paths.frame(again, 0);
            if (Math.hypot(starts.x - here.x, starts.y - here.y) > 0.01)
                wrong.push(`${said} changed jumps from ${here.x},${here.y}`);
            if (again !== null)
                check(again, changed, `${said} changed at ${Math.round(t)} ms`, then);
        }
        expect("150 journeys thrown at it", made, 150);
        expect("each ends on its seat, every frame of it a cat's", wrong.slice(0, 5), []);
    }

    // `tiles` changed as niri changes them: a column grown or shrunk, a
    // split between two windows moved, a window closed or opened, or
    // the columns scrolled.
    function changes(tiles, random) {
        const moved = tiles.map(tile => Object.assign({}, tile));
        const by = span => Math.floor((random() - 0.5) * 2 * span);
        const pick = list => list[Math.floor(random() * list.length)];
        const tile = pick(moved);
        const own = moved.filter(one => one.column === tile.column);
        const right = (shift, column) => moved.filter(one => one.column > column)
            .forEach(one => { one.left += shift; });
        const kind = Math.floor(random() * 5);
        if (kind === 0) {
            const width = Math.max(100, tile.width + by(300));
            right(width - tile.width, tile.column);
            own.forEach(one => { one.width = width; });
        } else if (kind === 1 && own.length > 1) {
            const r = own.indexOf(tile) > 0 ? own.indexOf(tile) : 1;
            const shift = Math.min(Math.max(by(150), 60 - own[r - 1].height),
                own[r].height - 60);
            own[r - 1].height += shift;
            own[r].top += shift;
            own[r].height -= shift;
        } else if (kind === 2 && moved.length > 1) {
            const gone = own.indexOf(tile);
            moved.splice(moved.indexOf(tile), 1);
            if (own.length === 1)
                right(-tile.width - 16, tile.column);
            else if (gone > 0)
                own[gone - 1].height += tile.height + 16;
            else
                Object.assign(own[1], { top: tile.top, height: own[1].height + tile.height + 16 });
        } else if (kind === 3 && tile.height >= 216) {
            const half = Math.floor((tile.height - 16) / 2);
            moved.push(Object.assign({}, tile, { top: tile.top + half + 16,
                height: tile.height - half - 16, id: 1000 + moved.length }));
            tile.height = half;
            moved.sort((one, other) => one.column - other.column || one.top - other.top);
        } else {
            right(by(400), -1);
        }
        return moved;
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

        // A hop's rise, in its cells: none as it leaves and lands, its
        // height midway, as a thing thrown up falls.
        expect("a hop leaves and lands on the ground",
            [Plays.hopped(0, 400, 2), Plays.hopped(400, 400, 2)], [0, 0]);
        expect("and is highest midway", Plays.hopped(200, 400, 2), 2);
        expect("rising as it falls", Plays.hopped(100, 400, 2), Plays.hopped(300, 400, 2));
        expect("and never below the ground",
            [Plays.hopped(-10, 400, 2), Plays.hopped(410, 400, 2)], [0, 0]);
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
        const hop = moved.find(played => played.name === "hop").beats;
        expect("a hop crouches, rises and lands crouched",
            hop.map(beat => [beat.pose, beat.hop === true]),
            [["pouncing", false], ["pouncing", true], ["pouncing", false]]);
        expect("a cell up, never so high it reads as a leap",
            moved.filter(played => played.name === "hop").map(played => played.beats[1].cells),
            moved.filter(played => played.name === "hop").map(() => 1));
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
                beat.walk === true || beat.ms >= 125)), true);
        expect("asleep there's no scene", Plays.scene("asleep", draws(0), "").beats, []);
        expect("nor in Do Not Disturb", Plays.scene("still", draws(0), "").beats, []);

        // A play asked for by name, as a test asks for the one it watches,
        // comes next where its level has it, whatever the draw, and the
        // draws are taken as ever, so the rest of the scene varies alike.
        const asked = name => Plays.scene("idle", draws(0), "", name);
        expect("a play asked for comes next",
            [asked("hop").name, asked("wander").name, asked("grooming").name],
            ["hop", "wander", "grooming"]);
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
        expect("in its two strides", Array.from(new Set(frames.map(frame => frame.pose))).sort(),
            ["walkingA", "walkingB"]);
        expect("each stride a stretch of ground, never a frame's time",
            frames.every(frame => frame.pose === ["walkingA", "walkingB"][
                Math.floor((frame.x - 99) / going.stride) % 2]), true);
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
            cell: 1, ratio: 1, wide: 8, tall: 8, air: 1, across: 1248, height: 800, step: 880,
            facing: 1, spring, was: tiles, now: tiles, cols: Paths.columns(tiles, 0)
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
        expect("stretched out in the air, never mid-stride",
            Array.from(new Set(flight.map(at => at.pose))), ["leaping"]);
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

        const down = Paths.moves(seat(100), seat(900), world({ way: 1 }));
        const ride = down.beats[0];
        expect("a move rides its old column first, as long as the slide",
            [ride.name, ride.ends], ["rides", 231]);
        expect("carried exactly as the world, up off the screen going down",
            [10, 50, 100].map(t => Paths.frame(down, t).y),
            [10, 50, 100].map(t => 7 - 880 * MarginCat.slid(t, spring)));
        expect("as it sat", [Paths.frame(down, 50).x, Paths.frame(down, 50).pose], [100, null]);
        const late = Paths.moves(seat(100), seat(900), world({ way: 1, ahead: 50 }));
        expect("told of the slide late, it starts as far into it as niri's window has gone",
            [0, 100].map(t => Paths.frame(late, t).y),
            [50, 150].map(t => Paths.frame(down, t).y));
        expect("and gets there as much sooner", Paths.frame(late, down.ends - 50).done, true);
        expect("which is two and a half frames, as a GPU's niri showed at 60 and 120 Hz",
            [MarginCat.behind(1000 / 60), MarginCat.behind(1000 / 120)], [42, 21]);
        const unseen = way => way.beats.filter(beat => beat.name !== "rides"
            && Math.max(beat.from.y, beat.to.y) < -8)
            .reduce((all, beat) => all + beat.ends - beat.begins, 0);
        expect("out of sight it hurries, a third of a second all told",
            Math.round(unseen(down)) <= 360, true);
        const climbing = down.beats.find(beat => beat.name === "climbs");
        expect("down its old column's side head first, belly to it",
            [climbing.from.x, climbing.to.x, climbing.turn, climbing.facing,
                climbing.to.y > climbing.from.y],
            [616, 616, 1, 1, true]);
        const falls = seen(down).filter(at => at.beat === "drops");
        expect("it falls in from the screen's top", falls[0].y < 0, true);
        expect("falling ever faster", falls.every((at, i) => i < 2
            || at.y - falls[i - 1].y >= falls[i - 1].y - falls[i - 2].y - 1e-9), true);
        const landed = down.beats[down.beats.map(beat => beat.name).lastIndexOf("drops") + 1];
        expect("lands crouched on the column below", [landed.name, landed.from.y], ["lands", 7]);
        expect("then walks to its seat and sits there",
            Paths.frame(down, down.ends), { x: 900, y: 7, pose: null, turn: 0, facing: 1,
                beat: "sits", done: true });
        expect("all it shows is smooth", smooth(seen(down)), true);

        const up = Paths.moves(seat(700), seat(100), world({ way: -1 }));
        expect("going up, it rides its old column down off the screen",
            Paths.frame(up, 100).y, 7 + 880 * MarginCat.slid(100, spring));
        const climbs = up.beats.filter(beat => beat.name === "climbs");
        expect("leaps up onto a side of a column, then climbs it head first",
            [climbs[0].from.x, climbs[0].turn, climbs[0].facing], [616, 1, -1]);
        expect("up the screen", climbs.every(beat => beat.to.y < beat.from.y), true);
        const mantles = up.beats.find(beat => beat.name === "mantles");
        expect("and over its top onto it", [mantles.to.x, mantles.to.y, mantles.turn], [608, 7, 0]);
        // Up its whole height at the pace a long walk takes, as quick as
        // it walks a top and no quicker (Joel, 2026-11-13 17:22, by WCJQSG
        // fc3f716a, over b36c3fb6's two and a half seconds at most).
        const climbed = climbs.reduce((all, beat) => all + Math.abs(beat.to.y - beat.from.y), 0);
        const upFor = climbs.reduce((all, beat) => all + beat.ends - beat.begins, 0);
        expect("up its whole height at the pace it walks a long way",
            Math.round(climbed / world({ way: -1 }).cell / (upFor / 1000)), MarginCat.HURRIES);
        const bounding = seen(up).filter(at => at.beat === "climbs").map((at, i, all) =>
            i === 0 ? 0 : Math.abs(at.y - all[i - 1].y)).slice(2);
        expect("in bounds, surging and gripping, never at a lift's even pace",
            Math.max(...bounding) > 1.5 * Math.min(...bounding), true);
        expect("and sits on its seat", [Paths.frame(up, up.ends).x, Paths.frame(up, up.ends).done],
            [100, true]);
        expect("all it shows is smooth", smooth(seen(up)), true);
        const stacked = Paths.moves(seat(1100), seat(1000), world({ way: -1 }));
        const between = stacked.beats.filter(beat => beat.turn !== 0 && beat.name === "leaps");
        expect("up from below onto a side, and up a column of two windows across the gap",
            between.map(beat => [Math.round(beat.from.y), Math.round(beat.to.y)]),
            [[887, 801], [801, 776], [408, 384]]);

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
        const among = more => world(Object.assign({ pause: 0, was: three, now: three,
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
        const route = Paths.moves(seat(100), seat(1000), among({ way: -1 }));
        const sides = route.beats.filter(beat => beat.name === "climbs")
            .map(beat => beat.from.x);
        expect("up, it finds its way across more than one window",
            Array.from(new Set(sides)).length > 1, true);
        expect("leaping a gap between windows mid-climb",
            route.beats.some(beat => beat.name === "leaps" && Math.abs(beat.to.x - beat.from.x)
                >= 16 && beat.from.y > 100 && beat.from.y < 700), true);
        expect("and gets to its seat, never gliding", [ends(route), glides(route)],
            [[1000, 7], false]);
        expect("all of that smooth", smooth(seen(route)), true);
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
        const narrowed = ided.map(tile => tile.column !== 2 ? tile
            : Object.assign({}, tile, { width: 60 }));
        const left = resized(narrowed, { x: 874, y: 507 });
        expect("its column narrowed out from under it, it falls",
            [left.beats[0].name, left.beats.some(beat => beat.name === "rides")], ["falls", false]);

        expect("with nothing on the new workspace it only rides",
            Paths.moves(seat(100), seat(100), world({ way: 1, now: [] })).beats
                .map(beat => beat.name), ["rides"]);
        expect("the theme's slowdown stretches all of a move",
            Math.round(Paths.moves(seat(100), seat(900), world({ way: 1, slowdown: 2 })).ends),
            Math.round(down.ends * 2));
    }
}

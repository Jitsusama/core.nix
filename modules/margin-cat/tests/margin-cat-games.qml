// The margin cat's plays that move it (MarginCatGames.js): a hop, a pounce
// at nothing and its ball of yarn, by itself or by Joel's clicks, each a
// way of the walk's own beats, so
// they keep the rules a walk keeps (RSCH-20261113-24R2QV): whole cells, a
// leap that rises, tucks and falls, and no drawing seen for less than two
// frames at 60 Hz, and how often the yarn comes (MarginCatPlays.js). Run in
// Qt's own JavaScript engine, the one the shell
// runs it in: the `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat's scripts beside this file and runs `qml -platform
// offscreen margin-cat-games.qml`.
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatDrawings.js" as Drawings
import "MarginCatGames.js" as Games
import "MarginCatPlays.js" as Plays
import "Seated.js" as Seated

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
        rooms(expect);
        yarn(expect);
        rests(expect);
        turnedDown(expect);
        drawn(expect);
        pounce(expect);
        hop(expect);
        clicks(expect);
        drops(expect);
        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // Each millisecond of a play, as the screen would draw it.
    function frames(played) {
        return Array.from({ length: Math.ceil(played.ends) }, (_, t) => Games.frame(played, t));
    }

    // The frames of the beat named `name`, every one it has.
    function during(played, name) {
        return frames(played).filter(frame => frame.beat === name);
    }

    // The shortest any drawing of `of` holds, in ms, but the first and the
    // last, which the sitting cat holds on either side.
    function shortest(all, of) {
        const runs = [];
        for (const [t, frame] of all.entries()) {
            if (t === 0 || of(frame) !== of(all[t - 1]))
                runs.push(0);
            runs[runs.length - 1] += 1;
        }
        return Math.min(...runs.slice(1, -1));
    }

    // Where it may play from its seat: within its window's column and the
    // screen, and off the margin's marks, half a gap clear of them.
    function rooms(expect) {
        expect("its room runs to its column's ends", Games.room(Seated.sat()),
            { left: 100, right: 892 });
        expect("and no further than its window",
            Games.room(Seated.sat({ column: { left: 50, top: 30, width: 200 } })),
            { left: 50, right: 142 });
        // Three marks of 12 centred on 500 span 482 to 518; half a gap
        // either side, 476 to 524.
        expect("and stops half a gap short of the marks on their side",
            [Games.room(Seated.sat({ marks: 3 })),
                Games.room(Seated.sat({ marks: 3, seat: { x: 600, y: 20 } }))],
            [{ left: 100, right: 368 }, { left: 76, right: 392 }]);
        expect("nowhere without a seat", Games.room(Seated.sat({ seat: null })), null);
    }

    // The yarn rolls up to it, it bats it on, crouches, wiggles, leaps on
    // it, holds it and sends it off the way it came.
    function yarn(expect) {
        const played = Games.play("yarn", Seated.sat());
        const all = frames(played);
        const at = t => Games.frame(played, t);
        const beat = name => played.beats.find(each => each.name === name);
        const last = name => at(beat(name).ends - 1e-6);
        expect("the yarn's beats, in order", played.beats.map(each => each.name),
            ["waits", "notices", "eyes", "bats", "watches", "crouches", "wiggles", "wiggles",
                "wiggles", "wiggles", "leaps", "lands", "pins", "sends"]);
        const left = Games.play("yarn", Seated.sat({ seat: { x: 500, y: 20 },
            column: { left: 0, top: 30, width: 528 } }));
        expect("it plays toward the more room", [played.way, left.way], [1, -1]);
        expect("the yarn comes from just behind its column's edge, drawn only within it",
            [at(0).yarn.x, at(0).yarn.y, at(0).yarn.within], [1001, 20, { left: 0, right: 1000 }]);
        expect("whichever side it comes from",
            [Games.frame(left, 0).yarn.x, Games.frame(left, 0).yarn.within],
            [-9, { left: 0, right: 528 }]);
        const rolling = during(played, "waits");
        expect("and comes to rest two cells short of its paws", last("waits").yarn.x, 110);
        const near = Games.play("yarn", Seated.sat({ column: { left: 92, top: 30, width: 48 } }));
        expect("from far, it comes faster, at most a fifth of a cell a millisecond",
            [beat("waits").ends - beat("waits").begins, near.beats[0].ends], [8910, 1200]);
        expect("it turns as it rolls, toward the cat",
            [Array.from(new Set(rolling.map(frame => frame.yarn.pose))).sort(),
                rolling[1].yarn.facing], [["yarnRollingA", "yarnRollingB"], -1]);
        // Whether each drawing of a roll from `from` covers `every` cells.
        const turnsBy = (them, from, every) => them.every(frame => frame.yarn.pose
            === ["yarnRollingA", "yarnRollingB"][
                Math.max(0, Math.ceil(Math.abs(from - frame.yarn.x) / every - 1e-6) - 1) % 2]);
        expect("each drawing of its roll the same ground, two cells from near, seven from far",
            [turnsBy(during(near, "waits"), 141, 2), turnsBy(rolling, 1001, 7)], [true, true]);
        expect("the cat sits as it sat while it comes",
            [at(100).pose, at(100).facing, at(100).x], ["nothingHere", 0, 100]);
        expect("then turns its head to it, ears up",
            [at(beat("notices").begins).pose, at(beat("notices").begins).facing],
            ["earFlick", 1]);
        expect("and eyes it as long as it thinks, the yarn lying still",
            [beat("eyes").ends - beat("eyes").begins, at(beat("eyes").begins).yarn.pose,
                at(beat("eyes").begins).yarn.facing], [650, "yarn", -1]);
        expect("and lies, once batted, as it rolled", at(beat("crouches").begins).yarn.facing, 1);
        expect("a bat sends it seven cells on",
            [at(beat("bats").begins).pose, last("watches").yarn.x], ["batting", 117]);
        expect("it crouches a cell back",
            [at(beat("crouches").begins).pose, last("crouches").x], ["pouncing", 99]);
        const wiggling = played.beats.filter(each => each.name === "wiggles");
        expect("and wiggles a cell back and forth, crouched",
            [wiggling.map(each => at(each.begins).x), wiggling.map(each => at(each.begins).pose)],
            [[99, 98, 99, 98], ["pouncing", "pouncing", "pouncing", "pouncing"]]);
        const leaping = during(played, "leaps");
        expect("it leaps three cells high, rising and reaching down",
            [Math.min(...leaping.map(frame => frame.y)), leaping[0].pose,
                leaping[leaping.length - 1].pose], [17, "leaping", "falling"]);
        expect("and lands crouched, its front paws on the yarn",
            [at(beat("lands").begins).pose, at(beat("lands").begins).x, at(beat("lands").begins).y],
            ["pouncing", 111, 20]);
        expect("holds it with a paw", at(beat("pins").begins).pose, "batting");
        const sent = during(played, "sends");
        expect("and sends it back behind its column's edge",
            [sent[sent.length - 1].yarn.x, sent[1].yarn.facing], [1000, 1]);
        expect("it's done where it landed, the yarn gone",
            [at(played.ends).done, at(played.ends).x, played.to, at(played.ends).yarn],
            [true, 111, { x: 111, y: 20 }, null]);
        expect("the cat and the yarn each move in whole cells",
            all.every(frame => Number.isInteger(frame.x) && Number.isInteger(frame.y)
                && Number.isInteger(frame.yarn.x)), true);
        expect("no drawing of either shows for less than two frames at 60 Hz",
            [shortest(all, frame => frame.pose) >= 33,
                shortest(all, frame => frame.yarn.pose) >= 33], [true, true]);
        expect("the theme's slowdown stretches all of it",
            Games.play("yarn", Seated.sat({ slowdown: 2 })).ends, played.ends * 2);
        expect("with no room for the yarn, there's no play",
            Games.play("yarn", Seated.sat({ column: { left: 95, top: 30, width: 16 } })), null);
    }

    // The yarn, its costliest play, comes at most once in 15 minutes,
    // spaced at random, never twice running (VKCF3Z 63b08065): each idle
    // scene drawn from a run of chances (`draws`), as Math.random would
    // give them.
    function rests(expect) {
        const draws = (...chances) => {
            let i = 0;
            return () => chances[i++ % chances.length];
        };
        const long = beats => beats.reduce((all, beat) => all + (beat.ms ?? 0), 0);
        const chances = [0, 0.2, 0.4, 0.6, 0.8, 0.99];
        expect("the yarn rests 15 to 30 minutes once it's played",
            [Plays.restsFor("yarn", 0), Plays.restsFor("yarn", 1)], [15 * 60_000, 30 * 60_000]);
        expect("the puppy 20 to 40, and a play that never rests, none",
            [Plays.restsFor("puppy", 0), Plays.restsFor("puppy", 1), Plays.restsFor("hop", 1)],
            [20 * 60_000, 40 * 60_000, 0]);
        const lately = chances.map(chance => Plays.scene("idle", draws(0.8, chance), "",
            { restsIn: { yarn: 1 } }).name);
        expect("while it rests, its other moves, never the yarn",
            Array.from(new Set(lately)).sort(), ["hop", "pounce", "puppy", "wander"]);
        const bothRest = chances.map(chance => Plays.scene("idle", draws(0.8, chance), "",
            { restsIn: { yarn: 1, puppy: 1 } }).name);
        expect("while both rest, neither",
            Array.from(new Set(bothRest)).sort(), ["hop", "pounce", "wander"]);
        expect("its rest over, the yarn again", Plays.scene("idle", draws(0.8, 0.5), "",
            { restsIn: { yarn: 0 } }).name, "yarn");
        expect("one's rest begun keeps the other's, and what's left of each runs down by the clock",
            [Plays.restStarted({ yarn: 5 }, "puppy", 10, 0),
                Plays.restsLeft({ yarn: 100, puppy: 30 }, 40)],
            [{ yarn: 5, puppy: 1_200_010 }, { yarn: 60, puppy: -10 }]);
        let seed = 11;
        const random = () => {
            seed = (seed * 16_807) % 2_147_483_647;
            return seed / 2_147_483_647;
        };
        const played = { yarn: [], puppy: [] };
        let restsUntil = {};
        let clock = 0;
        let lastPlay = "";
        while (clock < 4 * 3_600_000) {
            const coming = Plays.next("idle", random, lastPlay,
                { restsIn: Plays.restsLeft(restsUntil, clock) });
            clock += coming.after;
            lastPlay = coming.scene.name;
            if (played[lastPlay]) {
                played[lastPlay].push(clock);
                restsUntil = Plays.restStarted(restsUntil, lastPlay, clock, random());
            }
            clock += long(coming.scene.beats) + (coming.scene.beats[0]?.play ? 17_000 : 0);
        }
        const apart = times => times.slice(1).map((at, i) => at - times[i]);
        const spaced = (times, fewest, least) => [times.length >= fewest,
            apart(times).every(gap => gap >= least),
            Math.max(...apart(times)) - Math.min(...apart(times)) > 5 * 60_000];
        expect("four idle hours, the yarn a few times, 15 minutes apart at the least, unevenly",
            spaced(played.yarn, 4, 15 * 60_000), [true, true, true]);
        expect("and the puppy a few times, 20 minutes apart at the least, unevenly",
            spaced(played.puppy, 3, 20 * 60_000), [true, true, true]);
    }

    // A play a cat turns down, still walking or held as its beat comes, is
    // neither spent nor rested from: the yarn asked for comes in a later
    // scene, and its rest starts only once a cat begins its play (a test on
    // niri waited a minute for a yarn its first scene's cat had turned
    // down); though an ask no cat takes for 20 s from its first refusal is
    // let go, however many scenes a test's pace brings in that time.
    function turnedDown(expect) {
        const yarn = { play: "yarn", pause: 200 };
        const back = { walk: true, to: "column" };
        const now = 100_000;
        const answering = { answers: true, asked: "yarn", refusedAt: null };
        const refused = Plays.handed(answering, yarn, false, now);
        expect("the yarn asked for, turned down, still asked, and not rested from",
            refused, { answers: false, asked: "yarn", refusedAt: now, rests: "" });
        expect("nor spent by the walk back no cat takes after it",
            Plays.handed(refused, back, false, now + 10), refused);
        expect("the yarn asked for, begun, spent, and its rest begun",
            Plays.handed(answering, yarn, true, now),
            { answers: false, asked: "", refusedAt: null, rests: "yarn" });
        expect("the puppy, begun, its own rest begun, and a pounce none",
            [Plays.handed(answering, { play: "puppy", pause: 200 }, true, now).rests,
                Plays.handed(answering, { play: "pounce", pause: 200 }, true, now).rests],
            ["puppy", ""]);
        expect("begun after it was turned down, spent and its refusal forgotten",
            Plays.handed(Object.assign({}, answering, { refusedAt: now }), yarn, true,
                now + 5_000), { answers: false, asked: "", refusedAt: null, rests: "yarn" });
        expect("a yarn drawn at random, begun, rests and leaves another play asked",
            Plays.handed({ answers: false, asked: "pounce", refusedAt: now }, yarn, true, now),
            { answers: false, asked: "pounce", refusedAt: now, rests: "yarn" });
        expect("a play asked for in place, spent by its first pose",
            Plays.handed({ answers: true, asked: "batting", refusedAt: null },
                { pose: "batting" }, true, now).asked, "");
        expect("asked again while one plays, kept through that one's walk back",
            Plays.handed({ answers: false, asked: "yarn", refusedAt: null }, back, true,
                now).asked, "yarn");
        let left = refused;
        const quick = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map(scene => {
            left = Plays.handed(Object.assign({}, left, { answers: true }), yarn, false,
                now + 300 * scene);
            return left.asked;
        });
        expect("turned down in ten scenes 300 ms apart, still asked", quick,
            Array(10).fill("yarn"));
        const late = [19_999, 20_000].map(after => Plays.handed(
            Object.assign({}, answering, { refusedAt: now }), yarn, false, now + after).asked);
        expect("turned down 20 s after its first refusal, the ask let go", late, ["yarn", ""]);
    }

    // The yarn drawn once a look (MarginCat.yarnLooks), as the screen draws
    // it, lights the same device pixels as the drawing it made afresh each
    // frame before, at a cell of 1 and of 2, every frame of a play either
    // way and at whatever it faces (VKCF3Z, f3ca0bb2), the theme's own
    // drawings (MarginCatDrawings.js, from modules/theme/glyphs.nix).
    function drawn(expect) {
        const lit = (drawing, cell) => drawing.reduce((all, row, y) => {
            for (let x = 0; x < row.length; x++)
                for (let n = 0; row[x] === "#" && n < cell * cell; n++)
                    all.push((y * cell + Math.floor(n / cell)) * 64 + x * cell + n % cell);
            return all;
        }, []).join(",");
        const afresh = yarn => MarginCat.turned(Drawings.pixels[yarn.pose] ?? [], yarn.facing);
        const once = yarn => MarginCat.yarnDrawn(MarginCat.yarnLooks(yarn), Drawings.pixels);
        const rolled = [Games.play("yarn", Seated.sat()), Games.play("yarn", Seated.sat({
            seat: { x: 500, y: 20 }, column: { left: 0, top: 30, width: 528 } }))]
            .reduce((all, played) => all.concat(frames(played)), [])
            .filter(frame => frame.yarn !== null).map(frame => frame.yarn);
        const facings = [1, -1, 0, -0, undefined];
        const posed = Object.keys(Drawings.pixels).reduce((all, pose) =>
            all.concat(facings.map(facing => ({ pose, facing }))), []);
        const differ = [...rolled, ...posed].filter(yarn => [1, 2].some(cell =>
            lit(afresh(yarn), cell) !== lit(once(yarn), cell)));
        expect("the yarn drawn once a look lights the pixels it lit drawn each frame",
            [rolled.length > 30_000, differ.slice(0, 3)], [true, []]);
        expect("and it lights some, each way it faces",
            [lit(once({ pose: "yarn", facing: 1 }), 2).length > 0,
                lit(once({ pose: "yarn", facing: 1 }), 1)
                    !== lit(once({ pose: "yarn", facing: -1 }), 1)], [true, true]);
        expect("none where there's no yarn", MarginCat.yarnDrawn(MarginCat.yarnLooks(null),
            Drawings.pixels), []);
    }

    // A pounce at nothing it's sure of: it eyes it, crouches, wiggles,
    // leaps seven cells, lands and looks about.
    function pounce(expect) {
        const played = Games.play("pounce", Seated.sat());
        const all = frames(played);
        expect("a pounce's beats, in order", played.beats.map(each => each.name),
            ["notices", "eyes", "crouches", "wiggles", "wiggles", "wiggles", "wiggles", "leaps",
                "lands", "looks"]);
        expect("it leaps seven cells on from where it sat, with no yarn, and looks",
            [played.to, all.every(frame => frame.yarn === null),
                Games.frame(played, played.ends - 1).pose], [{ x: 107, y: 20 }, true, "earFlick"]);
        expect("in whole cells, no drawing for less than two frames",
            [all.every(frame => Number.isInteger(frame.x) && Number.isInteger(frame.y)),
                shortest(all, frame => frame.pose) >= 33], [true, true]);
        expect("with no room for its leap, there's no pounce",
            Games.play("pounce", Seated.sat({ column: { left: 95, top: 30, width: 16 } })), null);
    }

    // Its yarn by a click (VKCF3Z 73c59942; Joel, 2026-11-12: "i want to be
    // able to also use my mouse to engage with my animals."): a click on
    // the cat rolls it in, and one on the yarn rolls it away from the
    // click, the cat walking after it and batting it on, then sitting.
    function clicks(expect) {
        const fetched = Games.fetch(Seated.sat());
        const at = (played, t) => Games.frame(played, t);
        expect("a click on the cat rolls its yarn in, noticed and eyed",
            fetched.beats.map(each => each.name), ["waits", "notices", "eyes"]);
        expect("from just behind its column's edge, never appearing at rest",
            [at(fetched, 0).yarn.x, at(fetched, 0).yarn.pose], [1001, "yarnRollingA"]);
        expect("and lies by its paws once it's done, the cat sitting where it sat",
            [at(fetched, fetched.ends + 5000).yarn, fetched.to, at(fetched, fetched.ends).pose],
            [{ x: 110, y: 20, pose: "yarn", facing: -1, within: { left: 0, right: 1000 } },
                { x: 100, y: 20 }, null]);
        expect("with no room to lie, no yarn",
            Games.fetch(Seated.sat({ column: { left: 95, top: 30, width: 16 } })), null);
        const lying = { x: 110, facing: -1 };
        const chased = chance => Games.chase(Seated.sat(), lying, -1, chance);
        const rested = played => at(played, played.ends + 5000).yarn.x;
        const followed = played => played.beats.find(each => each.name === "follows");
        const rolledTo = played => at(played, followed(played).ends - 1e-6).yarn.x;
        expect("a click on the yarn's near side rolls it four to eight cells away",
            [rolledTo(chased(0)), rolledTo(chased(1))], [114, 118]);
        const rolling = frames(chased(1)).filter(frame => frame.beat === "follows")
            .map(frame => frame.yarn.x);
        const half = Math.floor(rolling.length / 2);
        expect("slowing as it goes, never back",
            [rolling.every((x, i) => i === 0 || x >= rolling[i - 1]),
                rolling[half] - rolling[0] > rolling[rolling.length - 1] - rolling[half]],
            [true, true]);
        const played = chased(0);
        expect("the cat's eyes follow it, it walks up to it and bats it on",
            played.beats.map(each => each.name), ["follows", "goes", "bats", "watches"]);
        expect("two cells short of it, and the bat sends it three more",
            [played.to, rested(played), at(played, played.ends).done],
            [{ x: 104, y: 20 }, 117, true]);
        const short = Games.chase(Seated.sat({ column: { left: 0, top: 30, width: 125 } }), lying,
            -1, 1);
        expect("a cell short of its column's end, and with no room for a bat, it eyes it",
            [rolledTo(short), rested(short), short.beats.map(each => each.name)],
            [116, 116, ["follows", "goes", "eyes"]]);
        const back = Games.chase(Seated.sat(), { x: 130, facing: 1 }, 1, 0);
        expect("clicked on its far side, it rolls back toward the cat, and is batted away",
            [rolledTo(back), back.to, rested(back)], [126, { x: 116, y: 20 }, 129]);
        expect("never nearer the cat than two cells",
            rolledTo(Games.chase(Seated.sat(), { x: 113, facing: 1 }, 1, 1)), 110);
        expect("nor at all where it can't roll a cell",
            Games.chase(Seated.sat(), { x: 110, facing: 1 }, 1, 1), null);
        const all = [fetched, played, short, back].reduce((them, one) =>
            them.concat(frames(one)), []);
        expect("the cat never on the yarn", all.every(frame => frame.yarn === null
            || frame.x + 8 <= frame.yarn.x || frame.yarn.x + 8 <= frame.x), true);
        expect("in whole cells, no drawing of either for less than two frames at 60 Hz",
            [all.every(frame => Number.isInteger(frame.x) && Number.isInteger(frame.yarn.x)),
                [fetched, played, short, back].every(one =>
                    shortest(frames(one), frame => frame.pose) >= 33
                    && shortest(frames(one), frame => frame.yarn.pose) >= 33)], [true, true]);
    }

    // Let go of by Joel's hand (VKCF3Z, 9ead6710; Joel, 2026-11-12: "i
    // want to be able to also use my mouse to engage with my animals."):
    // it falls in its falling drawing, never rising, to the nearest edge
    // under it, or the floor, lands and flicks an ear, and sits there. A
    // window across 0 to 400 from 100 to 300, a cell of air over each top.
    function drops(expect) {
        const top = { left: 0, right: 400, top: 100, bottom: 300, rows: [{ top: 100, bottom: 300 }] };
        const lower = { left: 0, right: 400, top: 100, bottom: 500,
            rows: [{ top: 100, bottom: 300, id: 1 }, { top: 320, bottom: 500, id: 2 }] };
        const from = (x, y) => Seated.sat({ seat: { x, y }, air: 1 });
        const gap = Games.drop(from(100, 20), [top], []);
        const falling = during(gap, "falls");
        expect("let go over a window, it falls onto its top, lands and flicks an ear",
            [gap.beats.map(each => each.name), gap.to, gap.landsOn],
            [["falls", "lands", "looks"], { x: 100, y: 91 }, { floats: false, id: null }]);
        expect("falling the whole way, straight down and never up",
            [falling.every(frame => frame.pose === "falling"),
                falling.every(frame => frame.x === 100),
                falling.every((frame, i) => i === 0 || frame.y >= falling[i - 1].y)],
            [true, true, true]);
        expect("over a window's face, onto the next window's top, never up onto its own",
            [Games.drop(from(100, 150), [lower], []).to, Games.drop(from(100, 150), [lower], [])
                .landsOn, Games.drop(from(100, 150), [top], []).to],
            [{ x: 100, y: 311 }, { floats: false, id: 2 }, { x: 100, y: 592 }]);
        expect("two pixels under a top, it scrambles up nothing and falls to the floor",
            [Games.drop(from(100, 93), [top], []).to, Games.drop(from(100, 93), [top], []).landsOn],
            [{ x: 100, y: 592 }, null]);
        expect("over a floating window, onto its top",
            [Games.drop(from(100, 20), [top], [{ left: 50, right: 300, top: 60, bottom: 200,
                id: 9 }]).to, Games.drop(from(100, 20), [top], [{ left: 50, right: 300,
                top: 60, bottom: 200, id: 9 }]).landsOn], [{ x: 100, y: 51 }, { floats: true, id: 9 }]);
        expect("let go on a top, it only lands and flicks an ear",
            Games.drop(from(100, 91), [top], []).beats.map(each => each.name), ["lands", "looks"]);
        // From the top of the screen to its floor, at 60 Hz: a fall past a
        // third of the screen's height moves its first cell from rest
        // within 100 ms, and on every frame from 200 ms, when it falls a
        // cell a frame, never a cut, each step no more
        // than the fastest fall goes, 54 heights a second, and a cell for
        // whole cells.
        const long = Games.drop(from(100, 0), [], []);
        const ys = Array.from({ length: Math.floor(long.beats[0].ends / (1000 / 60)) + 1 },
            (_, i) => Games.frame(long, i * (1000 / 60)).y);
        const moving = ys.slice(Math.ceil(200 / (1000 / 60)));
        expect("a long fall moves every frame, as far each as the fastest fall goes",
            [ys.indexOf(1) * (1000 / 60) < 100, moving.every((y, i) => i === 0 || y > moving[i - 1]),
                moving.every((y, i) => i === 0 || y - moving[i - 1] <= 54 * 8 / 60 + 1),
                long.to.y],
            [true, true, true, 592]);
    }

    // A hop a cell straight up from a crouch, landing crouched where it sat
    // (VKCF3Z, 5e211585), as a leap's arc and drawings.
    function hop(expect) {
        const played = Games.play("hop", Seated.sat());
        const hopping = during(played, "hops");
        expect("a hop's beats, in order", played.beats.map(each => each.name),
            ["crouches", "hops", "lands"]);
        expect("a cell up and back where it sat, never along",
            [Math.min(...hopping.map(frame => frame.y)),
                hopping.every(frame => frame.x === 100), played.to], [19, true, { x: 100, y: 20 }]);
        expect("drawn as a leap is",
            hopping.every(frame => ["leaping", "tucked", "falling"].includes(frame.pose)), true);
        expect("crouched either side of it",
            [Games.frame(played, 0).pose, Games.frame(played, played.ends - 1).pose],
            ["pouncing", "pouncing"]);
        expect("and drawn level with no room above it, rather than off the screen",
            Math.min(...frames(Games.play("hop", Seated.sat({ seat: { x: 100, y: 0 } })))
                .map(frame => frame.y)), 0);
    }
}

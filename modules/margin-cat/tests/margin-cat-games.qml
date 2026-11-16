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
        drawn(expect);
        pounce(expect);
        hop(expect);
        clicks(expect);
        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // A cat 8 cells of 1 across, sitting at 100, 20 on a window across the
    // whole of a screen 1,000 by 600 with no marks lit, thinking 200 ms.
    function sat(more) {
        return Object.assign({
            seat: { x: 100, y: 20 }, column: { left: 0, top: 30, width: 1000 }, across: 1000,
            height: 600, marks: 0, gap: 12, cell: 1, wide: 8, tall: 8, pause: 200
        }, more ?? {});
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
        expect("its room runs to its column's ends", Games.room(sat()), { left: 100, right: 892 });
        expect("and no further than its window",
            Games.room(sat({ column: { left: 50, top: 30, width: 200 } })),
            { left: 50, right: 142 });
        // Three marks of 12 centred on 500 span 482 to 518; half a gap
        // either side, 476 to 524.
        expect("and stops half a gap short of the marks on their side",
            [Games.room(sat({ marks: 3 })),
                Games.room(sat({ marks: 3, seat: { x: 600, y: 20 } }))],
            [{ left: 100, right: 368 }, { left: 76, right: 392 }]);
        expect("nowhere without a seat", Games.room(sat({ seat: null })), null);
    }

    // The yarn rolls up to it, it bats it on, crouches, wiggles, leaps on
    // it, holds it and sends it off the way it came.
    function yarn(expect) {
        const played = Games.play("yarn", sat());
        const all = frames(played);
        const at = t => Games.frame(played, t);
        const beat = name => played.beats.find(each => each.name === name);
        const last = name => at(beat(name).ends - 1e-6);
        expect("the yarn's beats, in order", played.beats.map(each => each.name),
            ["waits", "notices", "eyes", "bats", "watches", "crouches", "wiggles", "wiggles",
                "wiggles", "wiggles", "leaps", "lands", "pins", "sends"]);
        const left = Games.play("yarn", sat({ seat: { x: 500, y: 20 },
            column: { left: 0, top: 30, width: 528 } }));
        expect("it plays toward the more room", [played.way, left.way], [1, -1]);
        expect("the yarn comes from just behind its column's edge, drawn only within it",
            [at(0).yarn.x, at(0).yarn.y, at(0).yarn.within], [1001, 20, { left: 0, right: 1000 }]);
        expect("whichever side it comes from",
            [Games.frame(left, 0).yarn.x, Games.frame(left, 0).yarn.within],
            [-9, { left: 0, right: 528 }]);
        const rolling = during(played, "waits");
        expect("and comes to rest two cells short of its paws", last("waits").yarn.x, 110);
        const near = Games.play("yarn", sat({ column: { left: 92, top: 30, width: 48 } }));
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
            Games.play("yarn", sat({ slowdown: 2 })).ends, played.ends * 2);
        expect("with no room for the yarn, there's no play",
            Games.play("yarn", sat({ column: { left: 95, top: 30, width: 16 } })), null);
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
            [Plays.yarnRests(0), Plays.yarnRests(1)], [15 * 60_000, 30 * 60_000]);
        const lately = chances.map(chance => Plays.scene("idle", draws(0.8, chance), "",
            { yarnIn: 1 }).name);
        expect("while it rests, its other moves, never the yarn",
            Array.from(new Set(lately)).sort(), ["hop", "pounce", "wander"]);
        expect("its rest over, the yarn again", Plays.scene("idle", draws(0.8, 0.6), "",
            { yarnIn: 0 }).name, "yarn");
        let seed = 11;
        const random = () => {
            seed = (seed * 16_807) % 2_147_483_647;
            return seed / 2_147_483_647;
        };
        const yarned = [];
        let clock = 0;
        let lastPlay = "";
        let yarnFrom = 0;
        while (clock < 4 * 3_600_000) {
            const coming = Plays.next("idle", random, lastPlay, { yarnIn: yarnFrom - clock });
            clock += coming.after;
            lastPlay = coming.scene.name;
            if (lastPlay === "yarn") {
                yarned.push(clock);
                yarnFrom = clock + Plays.yarnRests(random());
            }
            clock += long(coming.scene.beats) + (coming.scene.beats[0]?.play ? 17_000 : 0);
        }
        const apart = yarned.slice(1).map((at, i) => at - yarned[i]);
        expect("four idle hours, the yarn a few times, 15 minutes apart at the least, unevenly",
            [yarned.length >= 4, apart.every(gap => gap >= 15 * 60_000),
                Math.max(...apart) - Math.min(...apart) > 5 * 60_000], [true, true, true]);
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
        const rolled = [Games.play("yarn", sat()), Games.play("yarn", sat({
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
        const played = Games.play("pounce", sat());
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
            Games.play("pounce", sat({ column: { left: 95, top: 30, width: 16 } })), null);
    }

    // Its yarn by a click (VKCF3Z 73c59942; Joel, 2026-11-12: "i want to be
    // able to also use my mouse to engage with my animals."): a click on
    // the cat rolls it in, and one on the yarn rolls it away from the
    // click, the cat walking after it and batting it on, then sitting.
    function clicks(expect) {
        const fetched = Games.fetch(sat());
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
            Games.fetch(sat({ column: { left: 95, top: 30, width: 16 } })), null);
        const lying = { x: 110, facing: -1 };
        const chased = chance => Games.chase(sat(), lying, -1, chance);
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
        const short = Games.chase(sat({ column: { left: 0, top: 30, width: 125 } }), lying, -1,
            1);
        expect("a cell short of its column's end, and with no room for a bat, it eyes it",
            [rolledTo(short), rested(short), short.beats.map(each => each.name)],
            [116, 116, ["follows", "goes", "eyes"]]);
        const back = Games.chase(sat(), { x: 130, facing: 1 }, 1, 0);
        expect("clicked on its far side, it rolls back toward the cat, and is batted away",
            [rolledTo(back), back.to, rested(back)], [126, { x: 116, y: 20 }, 129]);
        expect("never nearer the cat than two cells",
            rolledTo(Games.chase(sat(), { x: 113, facing: 1 }, 1, 1)), 110);
        expect("nor at all where it can't roll a cell",
            Games.chase(sat(), { x: 110, facing: 1 }, 1, 1), null);
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

    // A hop a cell straight up from a crouch, landing crouched where it sat
    // (VKCF3Z, 5e211585), as a leap's arc and drawings.
    function hop(expect) {
        const played = Games.play("hop", sat());
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
            Math.min(...frames(Games.play("hop", sat({ seat: { x: 100, y: 0 } })))
                .map(frame => frame.y)), 0);
    }
}

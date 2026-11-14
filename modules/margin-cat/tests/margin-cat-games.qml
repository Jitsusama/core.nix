// The margin cat's plays that move it (MarginCatGames.js): a hop, a pounce
// at nothing and its ball of yarn, each a way of the walk's own beats, so
// they keep the rules a walk keeps (RSCH-20261113-24R2QV): whole cells, a
// leap that rises, tucks and falls, and no drawing seen for less than two
// frames at 60 Hz. Run in Qt's own JavaScript engine, the one the shell
// runs it in: the `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat's scripts beside this file and runs `qml -platform
// offscreen margin-cat-games.qml`.
import QtQuick
import "MarginCatGames.js" as Games

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
        pounce(expect);
        hop(expect);
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
        expect("the yarn comes from twenty cells past its side, unseen yet",
            [at(0).yarn.x, at(0).yarn.y, at(0).yarn.opacity], [128, 20, 0]);
        expect("and comes to rest two cells short of its paws, whole",
            [last("waits").yarn.x, last("waits").yarn.opacity], [110, 1]);
        const rolling = during(played, "waits");
        expect("it turns as it rolls, toward the cat",
            [Array.from(new Set(rolling.map(frame => frame.yarn.pose))).sort(),
                rolling[1].yarn.facing], [["yarnRollingA", "yarnRollingB"], -1]);
        expect("each drawing of its roll two cells of ground, never a moment's",
            rolling.every(frame => frame.yarn.pose === ["yarnRollingA", "yarnRollingB"][
                Math.floor((128 - frame.yarn.x) / 2) % 2]), true);
        expect("the cat sits as it sat while it comes",
            [at(100).pose, at(100).facing, at(100).x], ["nothingHere", 0, 100]);
        expect("then turns its head to it, ears up",
            [at(beat("notices").begins).pose, at(beat("notices").begins).facing],
            ["earFlick", 1]);
        expect("and eyes it as long as it thinks, the yarn lying still",
            [beat("eyes").ends - beat("eyes").begins, at(beat("eyes").begins).yarn.pose,
                at(beat("eyes").begins).yarn.facing], [650, "yarn", 0]);
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
        expect("and sends it off the way it came, fading as it goes",
            [Math.round(sent[0].yarn.opacity * 100) / 100,
                Math.round(sent[sent.length - 1].yarn.opacity * 100) / 100,
                sent[sent.length - 1].yarn.x, sent[1].yarn.facing], [1, 0, 127, 1]);
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

// The margin's puppy visiting the cat (Joel, 2026-11-12: "And maybe throw
// in a little puppy too?"), as MarginCatPuppy.js plays it, in Qt's own
// engine: it trots in from behind the edge of the cat's room, stands, bows,
// sits as the cat answers with a hop where it sits, and trots back, drawn
// only within the room, in whole cells, in its own drawings.
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatDrawings.js" as Drawings
import "MarginCatGames.js" as Games
import "MarginCatPuppy.js" as Puppy

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
        visits(expect);
        trots(expect);
        drawings(expect);
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

    // Each millisecond of a visit, the cat and the puppy as the screen
    // would draw them.
    function frames(played) {
        return Array.from({ length: Math.ceil(played.ends) }, (_, t) =>
            ({ cat: Games.frame(played, t), puppy: Puppy.frame(played, t) }));
    }

    // The visit's beats, the cat's answers and where the puppy is in each.
    function visits(expect) {
        const played = Puppy.visit(sat());
        const beat = name => played.beats.find(each => each.name === name);
        const puppyAt = t => Puppy.frame(played, t);
        const catAt = t => Games.frame(played, t);
        expect("the visit's beats, in order", played.beats.map(each => each.name),
            ["waits", "notices", "eyes", "crouches", "hops", "lands", "looks", "watches"]);
        expect("a puppy play, with no yarn", [played.kind, played.yarn], ["puppy", null]);
        const left = Puppy.visit(sat({ seat: { x: 500, y: 20 },
            column: { left: 0, top: 30, width: 528 } }));
        expect("it comes from the side with more room", [played.way, left.way], [1, -1]);
        expect("from just behind its column's edge, drawn only within it, facing the cat",
            [puppyAt(0).x, puppyAt(0).y, puppyAt(0).within, puppyAt(0).facing],
            [1001, 20, { left: 0, right: 1000 }, -1]);
        expect("whichever side it comes from",
            [Puppy.frame(left, 0).x, Puppy.frame(left, 0).facing, Puppy.frame(left, 0).within],
            [-9, 1, { left: 0, right: 528 }]);
        const stops = beat("notices").begins;
        expect("and stops two cells short of the cat, standing, as the cat notices it",
            [puppyAt(stops).x, puppyAt(stops).pose, catAt(stops).pose, catAt(stops).facing],
            [110, "puppyWalkingB", "earFlick", 1]);
        const eyes = beat("eyes");
        expect("it bows as the cat eyes it, as long as the cat thinks besides",
            [puppyAt(eyes.begins).pose, catAt(eyes.begins).pose, eyes.ends - eyes.begins],
            ["puppyBowing", "tailSwish", 850]);
        const hopping = frames(played).filter(frame => frame.cat.beat === "hops");
        expect("the cat answers with a hop where it sits, the puppy sitting",
            [Math.min(...hopping.map(frame => frame.cat.y)),
                hopping.every(frame => frame.cat.x === 100),
                hopping.every(frame => frame.puppy.pose === "puppySitting")], [19, true, true]);
        expect("and flicks an ear after, the puppy still sitting",
            [catAt(beat("looks").begins).pose, puppyAt(beat("looks").begins).pose],
            ["earFlick", "puppySitting"]);
        const going = frames(played).filter(frame => frame.cat.beat === "watches");
        expect("then it turns and trots back behind the edge as the cat watches",
            [going[1].puppy.facing, going[going.length - 1].puppy.x,
                going[going.length - 1].puppy.x >= going[going.length - 1].puppy.within.right],
            [1, 1000, true]);
        expect("and is gone once the visit's done, the cat where it sat",
            [puppyAt(played.ends), played.to, catAt(played.ends).done],
            [null, { x: 100, y: 20 }, true]);
        expect("the cat never leaves its seat",
            frames(played).every(frame => frame.cat.x === 100), true);
        expect("the theme's slowdown stretches all of it",
            Puppy.visit(sat({ slowdown: 2 })).ends, played.ends * 2);
        expect("with no room for it beside the cat, no visit",
            Puppy.visit(sat({ column: { left: 95, top: 30, width: 16 } })), null);
        expect("nor without a seat", Puppy.visit(sat({ seat: null })), null);
    }

    // Its trot: in whole cells, at the pace the cat walks a way that long,
    // a drawing for every two or three cells, in the cat's order, and none
    // shown for less than two frames at 60 Hz.
    function trots(expect) {
        const played = Puppy.visit(sat());
        const both = frames(played);
        const all = both.map(frame => frame.puppy);
        const waits = played.beats[0];
        expect("its trot in at the cat's hurry, 891 cells at 80 a second",
            waits.ends - waits.begins, 11137.5);
        const near = Puppy.visit(sat({ column: { left: 92, top: 30, width: 48 } }));
        expect("a short way at an amble, never quicker than the cat's",
            near.beats[0].ends - near.beats[0].begins, 31 / 40 * 1000);
        expect("in whole cells",
            all.every(puppy => puppy !== null && Number.isInteger(puppy.x)), true);
        const order = ["puppyWalkingA", "puppyWalkingC", "puppyWalkingB", "puppyWalkingD"];
        const coming = all.slice(0, Math.ceil(waits.ends));
        expect("a new drawing each three cells it covers, in the cat's order",
            coming.every(puppy => puppy.pose
                === order[Math.floor(Math.abs(1001 - puppy.x) / 3 + 1e-6) % 4]), true);
        expect("its strides all four drawings", Array.from(new Set(coming.map(puppy =>
            puppy.pose))).sort(), order.slice().sort());
        const runs = [];
        for (const [t, puppy] of all.entries()) {
            if (t === 0 || puppy.pose !== all[t - 1].pose)
                runs.push(0);
            runs[runs.length - 1] += 1;
        }
        expect("no drawing shows for less than two frames at 60 Hz",
            Math.min(...runs.slice(1, -1)) >= 33, true);
        const moved = both.filter((frame, t) => t > 0 && frame.puppy.x !== all[t - 1].x);
        expect("and moves only while it trots in and off",
            Array.from(new Set(moved.map(frame => frame.cat.beat))).sort(),
            ["notices", "waits", "watches"]);
    }

    // Each pose it takes is one of the theme's drawings, which the screen
    // turns the way it faces (MarginCat.yarnDrawn, as the yarn's).
    function drawings(expect) {
        const posed = new Set(frames(Puppy.visit(sat())).map(frame => frame.puppy?.pose));
        posed.delete(undefined);
        expect("every pose it takes is drawn",
            Array.from(posed).filter(pose => (Drawings.poses[pose] ?? []).length === 0), []);
        const looks = { pose: "puppyBowing", facing: 1 };
        expect("and turned the way it faces",
            MarginCat.yarnDrawn(MarginCat.yarnLooks(looks), Drawings.poses),
            MarginCat.turned(Drawings.poses.puppyBowing, 1));
    }
}

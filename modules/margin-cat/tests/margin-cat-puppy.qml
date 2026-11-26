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
        visits(expect);
        trots(expect);
        drawings(expect);
        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // Each millisecond of a visit, the cat and the puppy as the screen
    // would draw them.
    function frames(played) {
        return Array.from({ length: Math.ceil(played.ends) }, (_, t) =>
            ({ cat: Games.frame(played, t), puppy: Puppy.frame(played, t) }));
    }

    // The visit's beats, the cat's answers and where the puppy is in each.
    function visits(expect) {
        const played = Puppy.visit(Seated.sat());
        const visited = frames(played);
        const beat = name => played.beats.find(each => each.name === name);
        const puppyAt = t => Puppy.frame(played, t);
        const catAt = t => Games.frame(played, t);
        expect("the visit's beats, in order", played.beats.map(each => each.name),
            ["waits", "notices", "eyes", "crouches", "hops", "lands", "looks", "watches"]);
        expect("a puppy play, with no yarn", [played.kind, played.yarn], ["puppy", null]);
        const left = Puppy.visit(Seated.sat({ seat: { x: 500, y: 20 },
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
        const hopping = visited.filter(frame => frame.cat.beat === "hops");
        expect("the cat answers with a hop where it sits, the puppy sitting",
            [Math.min(...hopping.map(frame => frame.cat.y)),
                hopping.every(frame => frame.cat.x === 100),
                hopping.every(frame => frame.puppy.pose === "puppySitting")], [19, true, true]);
        expect("and flicks an ear after, the puppy still sitting",
            [catAt(beat("looks").begins).pose, puppyAt(beat("looks").begins).pose],
            ["earFlick", "puppySitting"]);
        const going = visited.filter(frame => frame.cat.beat === "watches");
        expect("then it turns and trots back behind the edge as the cat watches",
            [going[1].puppy.facing, going[going.length - 1].puppy.x,
                going[going.length - 1].puppy.x >= going[going.length - 1].puppy.within.right],
            [1, 1000, true]);
        expect("and is gone once the visit's done, the cat where it sat",
            [puppyAt(played.ends), played.to, catAt(played.ends).done],
            [null, { x: 100, y: 20 }, true]);
        expect("the cat never leaves its seat",
            visited.every(frame => frame.cat.x === 100), true);
        expect("the theme's slowdown stretches all of it",
            Puppy.visit(Seated.sat({ slowdown: 2 })).ends, played.ends * 2);
        // Its gap and itself, ten cells, take a column 23 across from 95,
        // the cat at 100 and 8 across.
        const roomed = width => Puppy.visit(Seated.sat({ column: { left: 95, top: 30, width } }));
        expect("with room beside the cat for just it and its gap, a visit; a cell less, none",
            [roomed(23)?.kind, roomed(22)], ["puppy", null]);
        expect("nor without a seat", Puppy.visit(Seated.sat({ seat: null })), null);
    }

    // Its trot: in whole cells, at the pace the cat walks a way that long,
    // a drawing for every two or three cells, in the cat's order, and none
    // shown for less than two frames at 60 Hz.
    function trots(expect) {
        const played = Puppy.visit(Seated.sat());
        const both = frames(played);
        const all = both.map(frame => frame.puppy);
        const waits = played.beats[0];
        expect("its trot in at the cat's hurry, 891 cells at 80 a second",
            waits.ends - waits.begins, 11137.5);
        const near = Puppy.visit(Seated.sat({ column: { left: 92, top: 30, width: 48 } }));
        expect("a short way at an amble, never quicker than the cat's",
            near.beats[0].ends - near.beats[0].begins, 31 / 40 * 1000);
        expect("in whole cells",
            all.every(puppy => puppy !== null && Number.isInteger(puppy.x)), true);
        const order = ["puppyWalkingA", "puppyWalkingC", "puppyWalkingB", "puppyWalkingD"];
        const coming = all.slice(0, Math.ceil(waits.ends));
        // Each drawing it takes on is the next in the cat's order, after a
        // whole stride of three cells; only the first, at the edge, may be
        // short.
        const strode = [];
        for (const [t, puppy] of coming.entries()) {
            if (t === 0 || puppy.pose !== coming[t - 1].pose)
                strode.push({ pose: puppy.pose, cells: new Set() });
            strode[strode.length - 1].cells.add(puppy.x);
        }
        const unordered = strode.filter((one, i) => i > 0 && order.indexOf(one.pose)
            !== (order.indexOf(strode[i - 1].pose) + 1) % 4);
        expect("a new drawing each three cells it covers, in the cat's order",
            [unordered.length, strode.slice(1).every(one => one.cells.size === 3)],
            [0, true]);
        // However long its way in, the drawing it arrives in shows for a
        // whole stride, any short one left behind the room's edge, where at
        // these widths strides measured from the edge left it 1 or 2 cells;
        // and it's the drawing before its standing, so one pair of legs
        // moves into the stand, never both, where its drawings taken in
        // turn from the edge had one of these widths arrive both pairs away
        // and two in the standing. From either side, since its cells round
        // the other way coming in from the left.
        const arrivals = [];
        const sides = width => [Seated.sat({ column: { left: 92, top: 30, width } }),
            Seated.sat({ seat: { x: 892, y: 20 }, column: { left: 908 - width, top: 30, width } })];
        for (const inward of [200, 201, 202, 203, 204, 205].flatMap(sides).map(Puppy.visit)) {
            const trotted = Array.from({ length: Math.ceil(inward.beats[0].ends) },
                (_, t) => Puppy.frame(inward, t));
            const arrives = trotted[trotted.length - 1].pose;
            let first = trotted.length - 1;
            while (first > 0 && trotted[first - 1].pose === arrives)
                first -= 1;
            const cellsShown = new Set(trotted.slice(first).map(puppy => puppy.x)).size;
            const stands = Puppy.frame(inward, inward.beats[1].begins).pose;
            if (cellsShown !== inward.puppy.stride || arrives !== "puppyWalkingC"
                || stands !== "puppyWalkingB")
                arrivals.push({ way: inward.way, cellsShown, arrives, stands });
        }
        expect("arriving in a whole stride, the one before its stand, however far it came",
            arrivals, []);
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
        // At every pace from an amble to a hurry, a stride rounded up from
        // what its pace covers in STRIDE_SECONDS lasts at least as long.
        const brief = [];
        for (let cells = 1; cells <= 2000; cells += 1) {
            const one = Puppy.trot(cells);
            const pace = cells / (one.ms / 1000);
            if ((one.stride / pace) * 1000 < 1000 / 30)
                brief.push(cells);
        }
        expect("at any pace, a drawing lasts two frames at 60 Hz", brief.slice(0, 3), []);
        const moved = both.map((frame, t) => Object.assign({ t }, frame))
            .filter(frame => frame.t > 0 && frame.puppy.x !== all[frame.t - 1].x);
        // Its last cell in, onto `near`, is drawn on the first frame of the
        // notice, and none after it.
        const noticed = played.beats.find(beat => beat.name === "notices");
        expect("and moves only while it trots in and off",
            [Array.from(new Set(moved.map(frame => frame.cat.beat))).sort(),
                moved.filter(frame => frame.cat.beat === "notices"
                    && frame.t > Math.ceil(noticed.begins)).length],
            [["notices", "waits", "watches"], 0]);
    }

    // Each pose it takes is one of the theme's drawings, which the screen
    // turns the way it faces (MarginCat.yarnDrawn, as the yarn's): the
    // same stride trotting off is the mirror of its trotting in.
    function drawings(expect) {
        const shown = frames(Puppy.visit(Seated.sat())).map(frame => frame.puppy);
        const posed = new Set(shown.map(puppy => puppy?.pose));
        posed.delete(undefined);
        expect("every pose it takes is drawn",
            Array.from(posed).filter(pose => (Drawings.poses[pose] ?? []).length === 0), []);
        const drawn = facing => MarginCat.yarnDrawn(MarginCat.yarnLooks(shown.find(puppy =>
            puppy?.facing === facing && puppy.pose === "puppyWalkingA")), Drawings.poses);
        const mirrored = drawn(-1).map(row => row.split("").reverse().join(""));
        expect("and turned the way it faces, off as it trots back",
            [JSON.stringify(drawn(1)) === JSON.stringify(mirrored),
                JSON.stringify(drawn(1)) === JSON.stringify(drawn(-1))], [true, false]);
    }
}

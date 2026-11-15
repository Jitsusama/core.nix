// The margin cat as the windows under it change, resized or closed, in
// Qt's own JavaScript engine, the one the shell runs it in: riding niri's
// spring with the edge it stands on, falling when that edge goes, and
// its own motion on its art's pixels around a ride. The
// `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat's scripts beside this file and runs
// `qml -platform offscreen margin-cat-rides.qml`.
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatPaths.js" as Paths
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
        resized(expect);
        closed(expect);
        ridden(expect);
        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // A screen 1,248 across and 800 down, a cat 8 square a cell of 1 clear
    // of what it stands on, so it stands 9 above a window's top, on three
    // columns: one window, two stacked with a gap at 300, and two with one
    // at 500, each window with its own id.
    function world() {
        const spring = { damping: 1, stiffness: 1600, epsilon: 0.001 };
        const three = [
            { left: 16, top: 16, width: 400, height: 768, column: 0 },
            { left: 432, top: 16, width: 400, height: 284, column: 1 },
            { left: 432, top: 316, width: 400, height: 468, column: 1 },
            { left: 848, top: 16, width: 384, height: 484, column: 2 },
            { left: 848, top: 516, width: 384, height: 268, column: 2 }
        ];
        const ided = three.map((tile, i) => Object.assign({ id: i + 1 }, tile));
        const among = more => Object.assign({
            cell: 1, ratio: 1, wide: 8, tall: 8, air: 1, across: 1248, height: 800, facing: 1,
            spring, pause: 0, now: three, cols: Paths.columns(three, 0)
        }, more);
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
        // Never gliding: each walk level, each climb straight up or down,
        // and every beat begun where the last one ended.
        const glides = way => way.beats.some((beat, i) =>
            (beat.name === "goes" && beat.from.y !== beat.to.y)
            || (beat.name === "climbs" && beat.from.x !== beat.to.x)
            || (i > 0 && Math.hypot(beat.from.x - way.beats[i - 1].to.x,
                beat.from.y - way.beats[i - 1].to.y) > 0.01));
        const ends = way => [Paths.frame(way, way.ends).x, Paths.frame(way, way.ends).y];
        return { spring, ided, among, seen, smooth, posed, glides, ends };
    }

    // The window over its own resized, as niri does: it rides the top it
    // stands on, on niri's own spring, as the window's edge goes, never
    // across or through a window (Joel, 2026-11-13, by WCJQSG 2bd7c7c6:
    // "the kitten also needs to handle resizing windows").
    function resized(expect) {
        const { spring, ided, among, seen } = world();
        const split = top => ided.map(tile => tile.column !== 2 ? tile : Object.assign({},
            tile, tile.id === 4 ? { height: top - 32 } : { top, height: 784 - top }));
        const rideTo = (now, to) => Paths.walk({ x: 948, y: 507 }, to, among({ hurried: true,
            cols: Paths.columns(now, 0), before: Paths.columns(ided, 0) }));
        for (const [top, name] of [[316, "shrunk, it rides up"], [616, "grown, it rides down"]]) {
            const rode = rideTo(split(top), { x: 948, y: top - 9 });
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
        const left = rideTo(narrowed, { x: 874, y: 507 });
        expect("its column narrowed out from under it, it falls",
            [left.beats[0].name, left.beats.some(beat => beat.name === "rides")], ["falls", false]);
    }

    // A row of its column closed under it, and the window over it grown
    // down over where it sat, as niri lays a column out again (Joel,
    // 2026-11-14 17:24: "when a row in a column is deleted, it seems to
    // hang out in the middle then walk across, that is not believable"):
    // at once, it falls ahead of that window's edge onto what's under
    // where the edge stops, lands, and climbs to its seat by an edge,
    // never shoved across a face, never gliding.
    function closed(expect) {
        const { ided, among, seen, smooth, posed, glides, ends } = world();
        const rowGone = ided.filter(tile => tile.id !== 3).map(tile => tile.id !== 2 ? tile
            : Object.assign({}, tile, { height: 768 }));
        const dropsOut = Paths.walk({ x: 532, y: 307 }, { x: 532, y: 7 }, among({
            hurried: true, closed: true, cols: Paths.columns(rowGone, 0),
            before: Paths.columns(ided, 0), ahead: 72, frame: 1000 / 60 }));
        const dropBeats = dropsOut.beats.map(beat => beat.name);
        expect("a row closed under it, it falls at once onto the floor, never shoved or thinking",
            [dropBeats[0], dropsOut.beats[0].to.y, dropBeats.includes("shoved"),
                dropBeats.includes("thinks")], ["rides", 792, false, false]);
        const dropping = seen(dropsOut).filter(at => at.beat === "rides");
        expect("from where it sat, only ever down, in the falling drawing, landing crouched",
            [Paths.frame(dropsOut, 0).y, dropping.every((at, i) => i === 0
                || at.y >= dropping[i - 1].y), dropBeats.includes("rides")
                ? posed(dropsOut, "rides").map(one => one.pose) : dropBeats,
                dropBeats.slice(1, 3)], [307, true, ["falling"], ["lands", "shakes"]]);
        const grownFace = at => at.x < 831.5 && at.x + 8 > 432.5 && at.y < 783.5
            && at.y + 8 > 16.5;
        const climbedOut = seen(dropsOut).filter(at => at.t >= dropsOut.beats[0].ends);
        expect("and from the floor to its seat by an edge, in no face, never gliding",
            [climbedOut.some(grownFace), dropBeats.includes("climbs"), glides(dropsOut),
                smooth(seen(dropsOut)), ends(dropsOut)], [false, true, false, true, [532, 7]]);

        // niri says the window over it grew before it says its own closed,
        // so for a moment its row is still there, under the grown window
        // (a nested niri, 2026-11-14): it falls at once all the same.
        const overlapped = ided.map(tile => tile.id !== 2 ? tile
            : Object.assign({}, tile, { height: 768 }));
        const fallsFirst = Paths.walk({ x: 532, y: 307 }, { x: 532, y: 7 }, among({
            hurried: true, on: 3, cols: Paths.columns(overlapped, 0),
            before: Paths.columns(ided, 0), ahead: 72, frame: 1000 / 60 }));
        expect("its row still listed under the window grown over it, it falls at once all the same",
            [fallsFirst.beats[0].name, fallsFirst.beats[0].to.y,
                fallsFirst.beats.some(beat => beat.name === "shoved"), ends(fallsFirst)],
            ["rides", 792, false, [532, 7]]);
        // Or it says its own closed first, the window over it not grown yet,
        // so nothing is where it sat: it rides down on niri's spring all the
        // same, ahead of the edge to come, never falling by weight under it.
        const closedFirst = Paths.walk({ x: 532, y: 307 }, { x: 532, y: 7 }, among({
            hurried: true, closed: true, on: 3,
            cols: Paths.columns(ided.filter(tile => tile.id !== 3), 0),
            before: Paths.columns(ided, 0), ahead: 72, frame: 1000 / 60 }));
        const firstBeats = closedFirst.beats.map(beat => beat.name);
        expect("its row closed before the window over it grows, it rides down ahead all the same",
            [firstBeats[0], closedFirst.beats[0].to?.y, firstBeats.includes("rides")
                ? posed(closedFirst, "rides").map(one => one.pose) : firstBeats,
                ends(closedFirst)], ["rides", 792, ["falling"], [532, 7]]);
        // Sitting, it heard only its own tile, niri's second in its column,
        // and hears nothing of the column yet: it rides down all the same.
        const ownRow = [Object.assign({ row: 2 }, ided.find(tile => tile.id === 3))];
        const aloneFirst = Paths.walk({ x: 532, y: 307 }, { x: 532, y: 7 }, among({
            hurried: true, closed: true, on: 3, cols: [],
            before: Paths.columns(ownRow, 0), ahead: 72, frame: 1000 / 60 }));
        const aloneBeats = aloneFirst.beats.map(beat => beat.name);
        expect("its row closed, heard alone, before the window over it grows, it rides down",
            [aloneBeats[0], aloneBeats.includes("rides")
                ? posed(aloneFirst, "rides").map(one => one.pose) : aloneBeats,
                aloneBeats.includes("shoved")], ["rides", ["falling"], false]);
        // The middle of three rows closed, niri moves the row below up
        // before the one over it grows (the GPU rig, 2026-11-15: it fell
        // through the row below's face to the floor, and climbed back):
        // it rides down onto the row below where it now stands.
        const rows = [
            { id: 6, left: 432, top: 16, width: 400, height: 234, column: 1 },
            { id: 7, left: 432, top: 266, width: 400, height: 234, column: 1 },
            { id: 8, left: 432, top: 516, width: 400, height: 268, column: 1 }
        ];
        const movedUp = [rows[0], Object.assign({}, rows[2], { top: 408, height: 376 })];
        const ontoBelow = Paths.walk({ x: 532, y: 257 }, { x: 532, y: 399 }, among({
            hurried: true, closed: true, on: 7, cols: Paths.columns(movedUp, 0),
            before: Paths.columns(rows, 0), ahead: 72, frame: 1000 / 60 }));
        const belowBeats = ontoBelow.beats.map(beat => beat.name);
        expect("the middle row closed, the row below moved up first, it rides down onto that row",
            [belowBeats.slice(0, 3), ontoBelow.beats[0].to?.y, belowBeats.includes("climbs"),
                ends(ontoBelow)], [["rides", "lands", "shakes"], 399, false, [532, 399]]);
        const opened = ided.concat([{ id: 9, left: 432, top: 500, width: 400, height: 284,
            column: 1 }]);
        const staysOn = Paths.walk({ x: 532, y: 307 }, { x: 532, y: 7 }, among({
            hurried: true, on: 3, cols: Paths.columns(opened, 0),
            before: Paths.columns(ided, 0) }));
        expect("a window just opened over its row a moment, before niri shrinks it, it stays on",
            staysOn.beats.filter(beat => beat.kind === "falls" || beat.kind === "rides")
                .map(beat => beat.name), []);
    }

    // Heard late, a resize's ride catches niri's edge up without a jump
    // (VKCF3Z, 2026-11-13 a36e05f9): carried up within two frames in the
    // pose it had, left in the air a fall that sits flush by the time the
    // edge stops, never so far behind that what follows passes through it.
    function ridden(expect) {
        const { spring } = world();
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

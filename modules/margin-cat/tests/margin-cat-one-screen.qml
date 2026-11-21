// The cat between workspaces, walked as if they were all on one screen
// (MarginCatStacked.js's moves()), in Qt's own JavaScript engine, the one the
// shell runs it in. The `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat's scripts beside this file and runs `qml -platform
// offscreen margin-cat-one-screen.qml`.
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatPaths.js" as Paths
import "MarginCatStacked.js" as Stacked

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
        for (const way of [1, -1])
            crosses(expect, way);
        down(expect);
        up(expect);
        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // A screen 1248 by 800, so niri stacks its workspaces 880 apart, a tenth
    // of its height between them (niri's monitor.rs, workspace_gap); on each
    // workspace two columns of one window, a gap between them from 616 to
    // 632; and a cat of 8 cells of 1, seated a cell above the tops.
    function world(more) {
        const tiles = [
            { left: 16, top: 16, width: 600, height: 768, column: 0, id: 1 },
            { left: 632, top: 16, width: 600, height: 768, column: 1, id: 2 }
        ];
        const below = tiles.map(tile => Object.assign({}, tile, { id: tile.id + 2 }));
        return Object.assign({
            cell: 1, ratio: 1, wide: 8, tall: 8, air: 1, across: 1248, height: 800, facing: 1,
            spring: { damping: 1, stiffness: 1600, epsilon: 0.001 }, way: 1, was: tiles,
            now: below
        }, more);
    }

    // Each frame of a move `every` ms, with where the slide has the
    // workspace it left (Paths.view), so where it is on its own ground
    // reads apart from the slide carrying it.
    function frames(route, every) {
        const view = Paths.view ?? (() => 0);
        const all = [];
        for (let t = 0; t <= route.ends; t += every) {
            const at = Paths.frame(route, t);
            all.push(Object.assign({ t, ground: at.y - view(route, t) }, at));
        }
        return all;
    }

    // A move down (`way` 1) or up (-1) between two workspaces alike, from
    // the top of one's left window to the other's right, and back.
    function crosses(expect, way) {
        const at = world({ way });
        const slides = MarginCat.slides(at.spring);
        const seen = one => one.y + 8 > 0 && one.y < 800 && one.x + 8 > 0 && one.x < 1248;
        const walks = ["walkingA", "walkingB", "walkingC", "walkingD"];
        // How far above the workspace it goes to its old one's ground is.
        const lift = 880 * way;
        const dir = way > 0 ? "down" : "up";
        // Joel, Sat 14 17:30: "when going up or down it should appear on
        // the screen from the position that would make sense for how it
        // would have to path if they were all on one screen. so the top of
        // the left would have to walk to the right, climb down the right
        // side, drop down and grab with its claws on the top of the window
        // below, climb up, and then walk over" (VKCF3Z fcd95edd).
        for (const [from, to, side] of [[100, 900, "left to right"],
            [1100, 100, "right to left"]]) {
            const name = `${dir} ${side}`;
            const move = Stacked.moves({ x: from, y: 7 }, { x: to, y: 7 }, at);
            const all = frames(move, 1000 / 120);
            // niri's slide takes the old tops off the screen, and all it
            // shows till then is its walk, or its turn, on the top it
            // stood on as the slide carries it.
            const sliding = all.filter(one => one.t < slides && seen(one));
            const carried = one => one.y + lift * MarginCat.slid(one.t, at.spring);
            expect(`${name}, it walks its top as niri slides it, paws moving`,
                sliding.filter(one => !(walks.includes(one.pose) || one.beat === "turns")
                    || Math.abs(carried(one) - 7) > 1e-6)
                    .map(one => [Math.round(one.t), one.pose, one.beat, carried(one)]), []);
            expect(`${name}, it goes off the screen with the workspace it left`,
                all.some(one => one.t < slides && (way > 0 ? one.y + 8 <= 0 : one.y >= 800)),
                true);
            const gone = all.findIndex(one => !seen(one));
            const back = all.findIndex((one, i) => i > gone && seen(one));
            const comes = all[back] ?? { t: Infinity };
            const beside = comes.x >= 616 && comes.x <= 624;
            // Joel, Sun 15 21:20: "when transitioning screens it should be
            // able to poke its head up and then come in"; 23:34: "the whole
            // kitty should not have to be rendered when you are seeing it
            // come on to your screen".
            if (way > 0)
                expect(`${name}, it comes in at the top over the gap, head first`,
                    [beside, comes.y < 0, comes.beat], [true, true, "peeks"]);
            else
                expect(`${name}, it comes in at the bottom beside a side, head first`,
                    [beside, comes.y > 792, comes.beat], [true, true, "peeks"]);
            const rows = one => way > 0 ? one.y + 8 : 800 - one.y;
            const peeking = all.filter((one, i) => i >= back
                && ["peeks", "peers", "looks"].includes(one.beat));
            expect(`${name}, no more of it than its head over the edge till it has looked`,
                [Math.max(...peeking.map(rows)), peeking.filter(one => rows(one) > 3).length],
                [3, 0]);
            const looking = peeking.filter(one => one.beat !== "peeks");
            expect(`${name}, its head over the edge, it looks about for 400 ms`,
                [looking.length > 0 && looking[looking.length - 1].t - looking[0].t >= 390,
                    [...new Set(looking.map(one => one.pose))]],
                [true, ["nothingHere", "earFlick"]]);
            expect(`${name}, its head comes over the edge with its paws moving`,
                peeking.filter(one => one.beat === "peeks" && !walks.includes(one.pose))
                    .map(one => [Math.round(one.t), one.pose]), []);
            expect(`${name}, peeking down from the gap its head is down, up it's upright`,
                [...new Set(peeking.map(one => one.turn !== 0))], [way > 0]);
            const on = move.beats[move.beats.findIndex(beat => beat.name === "looks") + 1];
            expect(`${name}, then it comes on with a drop or a leap up`,
                [on?.name, on?.turn], [way > 0 ? "drops" : "springs", 0]);
            expect(`${name}, out of sight no more than about two thirds of a second`,
                comes.t - (all[gone]?.t ?? 0) <= 700, true);
            // A beat strides on from the stride drawn before its chain,
            // found as the first beat its chain began with (MarginCatWays.js's
            // frame()), so a chain shifted to follow the drop still begins
            // at its own first beat's start, and no other chain's.
            expect(`${name}, every walk's chain begins where its own first beat does`,
                move.beats.filter((beat, i) => {
                    if ((beat.since ?? null) === null)
                        return false;
                    const chained = move.beats.findIndex(one => one.since === beat.since);
                    return move.beats[chained].begins !== beat.since
                        || move.beats.slice(chained, i + 1).some(one => one.since !== beat.since);
                }).map(beat => [beat.name, beat.since]), []);
            const names = move.beats.map(beat => beat.name);
            const grab = move.beats[names.indexOf(way > 0 ? "drops" : "springs") + 1];
            const caught = [grab?.name, grab?.to.y, grab?.to.x === 616 || grab?.to.x === 624];
            if (way > 0)
                expect(`${name}, the drop ends in a grab at a top's corner in its reach`,
                    caught, ["grabs", 8, true]);
            else
                expect(`${name}, the leap up ends hanging on a side's foot, never its underside`,
                    caught, ["grabs", 776, true]);
            expect(`${name}, then it climbs on and walks to its seat`,
                Paths.frame(move, move.ends), { x: to, y: 7, pose: null, turn: 0,
                    facing: Math.sign(to - from), beat: "sits", done: true });
            // Its own ground, the slide taken out, never jumps: no more
            // than half its body between two frames at 120 Hz.
            const shown = all.filter(seen);
            expect(`${name}, all it shows is smooth on its own ground`,
                shown.filter((one, i) => i > 0 && one.t - shown[i - 1].t < 9
                    && Math.hypot(one.x - shown[i - 1].x, one.ground - shown[i - 1].ground) > 4)
                    .map(one => [Math.round(one.t), one.beat]), []);
            // Its ground is laid over the new workspace's, the old one's
            // `lift` above it.
            const inFace = (one, tiles, dy) => tiles.some(tile =>
                one.x < tile.left + tile.width - 0.5 && one.x + 8 > tile.left + 0.5
                && one.ground < tile.top - dy + tile.height - 0.5
                && one.ground + 8 > tile.top - dy + 0.5);
            expect(`${name}, in no window's face, old or new`,
                shown.filter(one => inFace(one, at.was, lift) || inFace(one, at.now, 0))
                    .map(one => [Math.round(one.t), one.beat]), []);
            // Told of the slide late, as the shell is (MarginCat.behind),
            // its frame at `t` shows as niri's slide is `ahead` ms further
            // on, and the old top it stands on is where that slide has it.
            // A frame ahead of it at 120 Hz, going up, it showed 90 px into
            // its window's face (2026-11-15, take uw3). Once it peeks in
            // it's on the new workspace, which the face case holds.
            const told = Stacked.moves({ x: from, y: 7 }, { x: to, y: 7 },
                world({ way, ahead: 33, frame: 1000 / 120 }));
            expect(`${name}, told late, never ahead of niri's slide into its window`,
                frames(told, 1000 / 120).filter(one => one.t < slides && seen(one)
                    && one.t + told.ahead < told.drops
                    && one.y + 8 > 16 - lift * MarginCat.slid(one.t + 33, at.spring) + 0.5)
                    .map(one => [Math.round(one.t), one.beat]), []);
            // Each pose it shows held two frames at 60 Hz, as it goes off
            // and comes in too.
            const runs = [];
            for (const one of frames(move, 1).filter(seen)) {
                const last = runs[runs.length - 1];
                if (last && last.pose === one.pose && one.t - last.until <= 1)
                    last.until = one.t;
                else
                    runs.push({ pose: one.pose, from: one.t, until: one.t, beat: one.beat });
            }
            expect(`${name}, every pose it shows held two frames at 60 Hz`,
                runs.filter((one, i) => i > 0 && i < runs.length - 1
                    && one.until + 1 - one.from < 33 && runs[i + 1].from === one.until + 1)
                    .map(one => [one.pose, one.beat, Math.round(one.from)]), []);
        }
        // Its seat at the same spot on the screen as the one it leaves,
        // as with the same windows on both, it's still a way across the
        // gap, never a hold in place as the windows slide by.
        const same = Stacked.moves({ x: 900, y: 7 }, { x: 900, y: 7 }, at);
        expect(`${dir}, its seat at the same spot, it still goes off the screen and comes in`,
            [same.beats.some(beat => beat.name === (way > 0 ? "drops" : "springs")),
                frames(same, 1000 / 120).some(one => one.t < slides
                    && (way > 0 ? one.y + 8 <= 0 : one.y >= 800)),
                Paths.frame(same, same.ends).beat],
            [true, true, "sits"]);
    }

    // What only a move down does: its seat said again out of sight, and
    // told of it late or not at all.
    function down(expect) {
        const at = world({});
        const slides = MarginCat.slides(at.spring);
        // Its seat said again while it's out of sight, it finds its way
        // from where it set off, never walking on from above the screen.
        const again = Stacked.moves({ x: 100, y: 7 }, { x: 900, y: 7 }, at);
        const drops = again.beats.find(beat => beat.name === "peeks").begins;
        expect("its seat moved as it's out of sight, it finds its way again, till it peeks in",
            [Paths.replans(again, drops - 10, true), Paths.replans(again, drops + 10, true)],
            [true, false]);

        // A window alone below sits in the middle of the screen (niri's
        // always-center-single-column), its corners far from either side
        // above: it lands on the top under the side it dropped off, the
        // one landing VKCF3Z allows (fcd95edd), never on the floor.
        const alone = world({ now: [{ left: 324, top: 16, width: 600, height: 768, column: 0,
            id: 9 }] });
        const wide = Stacked.moves({ x: 900, y: 7 }, { x: 600, y: 7 }, alone);
        const lands = wide.beats[wide.beats.findIndex(beat => beat.name === "drops") + 1];
        expect("with no corner in its reach, the drop lands on the top under it, never the floor",
            [lands?.name, lands?.to.y, lands?.to.x >= 324 - 8 && lands?.to.x <= 924,
                Paths.frame(wide, wide.ends).beat],
            ["lands", 7, true, "sits"]);

        // Told where it's going only after the slide's begun (the screen's
        // plan with no seat yet), it goes off the top with its window.
        const blind = Stacked.moves({ x: 100, y: 7 }, null, at);
        const half = Paths.frame(blind, slides / 2);
        expect("down with nowhere yet, it rides its window off the top on niri's spring",
            [blind.beats.map(beat => beat.name), half.x,
                Math.round(half.y * 100), Math.round(blind.ends)],
            [["rides"], 100, Math.round((7 - 880 * MarginCat.slid(slides / 2, at.spring)) * 100),
                slides]);
        // Ridden off over an empty workspace, it falls to the floor from
        // just above the screen, never from all the way up the old one.
        expect("off the top with nowhere to sit, it drops in from just above the screen",
            [Stacked.dropsFrom(Paths.frame(blind, blind.ends), at),
                Stacked.dropsFrom({ x: 100, y: 7 }, at)],
            [{ x: 100, y: -24 }, { x: 100, y: 7 }]);
        // Up with nowhere yet, it rides its window off the bottom, never
        // held on the glass as its ground slides away.
        const held = world({ way: -1 });
        const rides = Stacked.moves({ x: 100, y: 7 }, null, held);
        const halfway = Paths.frame(rides, slides / 2);
        expect("up with nowhere yet, it rides its window off the bottom on niri's spring",
            [rides.beats.map(beat => beat.name), halfway.x, Math.round(halfway.y * 100),
                Math.round(Paths.frame(rides, rides.ends).y)],
            [["rides"], 100, Math.round((7 + 880 * MarginCat.slid(slides / 2, at.spring)) * 100),
                887]);
        // Ridden off the bottom over an empty workspace, it peeks in at
        // the bottom edge, springs onto the floor and lands.
        const floor = Stacked.risesOnto(Stacked.risesFrom({ x: 100, y: 887 }, at), at);
        expect("off the bottom with nowhere to sit, it peeks in at the bottom onto the floor",
            [floor.beats.map(beat => beat.name), floor.from.y, floor.to.y,
                Paths.frame(floor, floor.ends).beat],
            [["peeks", "peers", "looks", "springs", "lands"], 800, 792, "sits"]);
        const late = Stacked.moves({ x: 100, y: 7 }, { x: 900, y: 7 }, world({ ahead: 50 }));
        expect("told of the slide 50 ms late, it's as far up as niri's slide has gone",
            Math.round(Paths.frame(late, 0).y * 100),
            Math.round((7 - 880 * MarginCat.slid(50, at.spring)) * 100));
    }

    // Up from a window alone, which niri sets in the middle of the screen,
    // to two half columns: no head below is under a side above, so it walks
    // its top to under the crack between them and leaps straight up it
    // (VKCF3Z c9474cef). With no side over any top, it comes in on the
    // nearest side's foot, never shifted sideways where it's seen.
    function up(expect) {
        const alone = [{ left: 324, top: 16, width: 600, height: 768, column: 0, id: 9 }];
        const halves = world({ way: -1, was: alone });
        const crack = Stacked.moves({ x: 400, y: 7 }, { x: 900, y: 7 }, halves);
        const names = crack.beats.map(beat => beat.name);
        const springs = crack.beats[names.indexOf("springs")];
        const grab = crack.beats[names.indexOf("springs") + 1];
        expect("up from a top under the crack above, it leaps from there straight up it",
            [springs?.from.x, [616, 624].includes(springs?.to.x), grab?.name, grab?.to.y],
            [springs?.to.x, true, "grabs", 776]);
        expect("up from a top under the crack above, then it climbs on and walks to its seat",
            Paths.frame(crack, crack.ends).beat, "sits");
        // Told to go up while still under the screen on its way up from the
        // workspace below, it has no ground to leave: it comes up as the
        // walk brings it, where before it threw in Routes' route().
        const risen = Stacked.moves({ x: 400, y: 850 }, { x: 900, y: 7 }, halves);
        expect("up from under the screen, it comes up and walks to its seat",
            Paths.frame(risen, risen.ends).beat, "sits");

        const narrow = [{ left: 100, top: 16, width: 200, height: 768, column: 0, id: 9 }];
        const apart = [
            { left: 400, top: 16, width: 400, height: 768, column: 0, id: 10 },
            { left: 900, top: 16, width: 300, height: 768, column: 1, id: 11 }
        ];
        const none = world({ way: -1, was: narrow, now: apart });
        const far = Stacked.moves({ x: 200, y: 7 }, { x: 1000, y: 7 }, none);
        const fars = far.beats.map(beat => beat.name);
        const comes = far.beats[fars.indexOf("springs")];
        expect("up with no side over any top, it comes in at the nearest side's foot, unmoved",
            [fars.slice(0, fars.indexOf("unseen")).filter(name => name !== "turns"),
                comes?.to.x, comes?.to.y, far.beats[fars.indexOf("springs") + 1]?.name,
                Paths.frame(far, far.ends).beat],
            [[], 392, 776, "grabs", "sits"]);
    }
}

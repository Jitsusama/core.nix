// The cat between workspaces, walked as if they were all on one screen
// (MarginCatStacked.js's moves()), and carried by its window as niri moves it
// (MarginCatCarried.js), in Qt's own JavaScript engine, the one the shell runs
// it in. The `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat's scripts beside this file and runs `qml -platform
// offscreen margin-cat-one-screen.qml`.
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatCarried.js" as Carried
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
        along(expect);
        straddled(expect);
        scrolled(expect);
        movedOff(expect);
        backOnIt(expect);
        visitCutIt(expect);
        standsIt(expect);
        floorsIt(expect);
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
        // Its report says how far the slide still has its ground, so the
        // nested niri's check can take the way back from a place it's said:
        // the gap's 880 short of it as the ride sets off, none once niri's
        // slide is within its epsilon of done, when the frame leaves it unsaid.
        const saidOff = one => one.carried ?? 0;
        const offBy = t => {
            const slid = MarginCat.slid(t, at.spring);
            return slid >= 1 - at.spring.epsilon ? 0 : -880 * (1 - slid);
        };
        expect("each frame of its ride up, it says its ground is as far off as niri's slide has it",
            frames(rides, 1000 / 120)
                .filter(one => !(Math.abs(saidOff(one) - offBy(one.t)) <= 1e-6))
                .map(one => [Math.round(one.t), saidOff(one), offBy(one.t)]), []);
        expect("setting off a whole gap short, and done by the ride's end",
            [saidOff(Paths.frame(rides, 0)), saidOff(Paths.frame(rides, rides.ends))], [-880, 0]);
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
        // walk brings it, leaping up from under the screen (Paths' comesUp),
        // where before it threw in Routes' route().
        const risen = Stacked.moves({ x: 400, y: 850 }, { x: 900, y: 7 }, halves);
        const seated = Paths.frame(risen, risen.ends);
        const leapsUp = risen.beats.some(beat => beat.name === "leaps" && beat.from.y > 800
            && beat.to.y < 800);
        expect("up from under the screen, the walk brings it up to its seat",
            [risen.stacked === undefined, leapsUp, seated.x, seated.y, seated.beat],
            [true, true, 900, 7, "sits"]);

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

    // Off the screen's side, where the window it sat on took it as niri
    // scrolled the strip, it comes back in at that edge, head first, and
    // walks the tops to its seat (Joel, 2026-11-14 17:24: "focusing to a
    // window off the left or right edge"). niri lays the strip's columns
    // on past each edge, the gap between them its own, and the windows
    // there stand lower than the cat was drawn, so it comes in on theirs.
    function along(expect) {
        const strip = world().was.concat([
            { left: -600, top: 40, width: 600, height: 744, column: -1, id: 5 },
            { left: 1248, top: 40, width: 600, height: 744, column: 2, id: 6 }
        ]);
        const at = world({ cols: Paths.columns(strip, 0), pause: 300 });
        const walks = ["walkingA", "walkingB", "walkingC", "walkingD"];
        const seen = one => one.x + 8 > 0 && one.x < 1248;
        for (const [from, to, side] of [[2000, 312, "off the right"],
            [-900, 900, "off the left"]]) {
            const walk = Stacked.walkAlong({ x: from, y: 7 }, { x: to, y: 7 }, at);
            const all = frames(walk, 1000 / 120);
            const comes = all.find(seen);
            const shows = one => from > 0 ? 1248 - one.x : one.x + 8;
            expect(`${side}, it comes in at the edge on the top there, head first`,
                [seen(all[0]), comes?.beat, comes?.y, shows(comes) <= 3],
                [false, "peeks", 31, true]);
            const peeking = all.filter(one => ["peeks", "peers", "looks"].includes(one.beat));
            expect(`${side}, no more of it than its head over the edge till it has looked`,
                [Math.max(...peeking.map(shows)), peeking.filter(one => shows(one) > 3).length],
                [3, 0]);
            const looking = peeking.filter(one => one.beat !== "peeks");
            expect(`${side}, its head over the edge, it looks about for 400 ms`,
                [looking.length > 0 && looking[looking.length - 1].t - looking[0].t >= 390,
                    [...new Set(looking.map(one => one.pose))]],
                [true, ["nothingHere", "earFlick"]]);
            expect(`${side}, its head comes over the edge with its paws moving`,
                peeking.filter(one => one.beat === "peeks" && !walks.includes(one.pose))
                    .map(one => [Math.round(one.t), one.pose]), []);
            const on = all.findIndex(one => one.beat === "looks");
            const looked = walk.beats.findIndex(beat => beat.name === "looks");
            const leap = walk.beats[looked + 2];
            expect(`${side}, then it leaps the gap at the edge onto the window inside it`,
                [walk.beats.slice(looked + 1, looked + 4).map(beat => beat.name),
                    leap?.to.x, leap?.to.y],
                [["crouches", "leaps", "lands"], from > 0 ? 1224 : 16, 7]);
            expect(`${side}, then it comes on facing in, never thinking again or going back out`,
                [walk.beats.some(beat => beat.name === "thinks"),
                    all.slice(on).filter(one => !seen(one)).length,
                    [...new Set(all.slice(0, on).map(one => one.facing))]],
                [false, 0, [from > 0 ? -1 : 1]]);
            const seated = Paths.frame(walk, walk.ends);
            expect(`${side}, and walks the tops to its seat`, [seated.x, seated.y, seated.beat],
                [to, 7, "sits"]);
        }
        const partly = Stacked.walkAlong({ x: 1244, y: 7 }, { x: 312, y: 7 }, at);
        expect("partly over the edge, it walks on from where it's drawn, peeking at nothing",
            [Paths.frame(partly, 0).x, partly.beats.some(beat => beat.name === "peeks")],
            [1244, false]);
        expect("on the screen, it walks as any walk does",
            JSON.stringify(Stacked.walkAlong({ x: 100, y: 7 }, { x: 900, y: 7 }, at)),
            JSON.stringify(Paths.walk({ x: 100, y: 7 }, { x: 900, y: 7 }, at)));
    }

    // Off the right edge on a window partly on the screen, it walks that
    // window's top in to its gap and leaps only the gap, never the window's
    // whole top from the edge.
    function straddled(expect) {
        const strip = [
            { left: 16, top: 16, width: 600, height: 768, column: 0, id: 1 },
            { left: 632, top: 16, width: 400, height: 768, column: 1, id: 2 },
            { left: 1048, top: 16, width: 600, height: 768, column: 2, id: 3 }
        ];
        const at = world({ cols: Paths.columns(strip, 0), pause: 300 });
        const walk = Stacked.walkAlong({ x: 2000, y: 7 }, { x: 312, y: 7 }, at);
        const looked = walk.beats.findIndex(beat => beat.name === "looks");
        const leapt = walk.beats.findIndex((beat, i) => i > looked && beat.name === "leaps");
        expect("on a window partly on the screen, it walks to its gap, then leaps only the gap",
            [walk.beats[looked + 1]?.name, walk.beats[leapt]?.from.x, walk.beats[leapt]?.to.x],
            ["goes", 1048, 1024]);
        const seated = Paths.frame(walk, walk.ends);
        expect("and walks the tops on to its seat", [seated.x, seated.y, seated.beat],
            [312, 7, "sits"]);
        // The edge 8 into niri's gap of 16 between the window inside it and one
        // past it, drawn in the air below that window's top. The peek stands
        // over the gap, as the window past it is wholly out of sight, and the
        // leap is only the gap's.
        const gapped = [strip[0], Object.assign({}, strip[1], { width: 608 }),
            { left: 1256, top: 40, width: 600, height: 744, column: 2, id: 3 }];
        const across = Stacked.walkAlong({ x: 2000, y: 100 }, { x: 312, y: 7 },
            world({ cols: Paths.columns(gapped, 0), pause: 300 }));
        const peeked = across.beats.find(beat => beat.name === "peeks");
        const leap = across.beats[across.beats.indexOf(peeked) + 4];
        expect("the edge in a gap, it peeks at the top of the window past it, then leaps the gap",
            [peeked?.to.x, peeked?.to.y, across.beats.slice(across.beats.indexOf(peeked) + 3,
                across.beats.indexOf(peeked) + 6).map(beat => beat.name), leap?.to.x],
            [1245, 31, ["crouches", "leaps", "lands"], 1232]);
        // No window past the edge at all, it peeks in at its own height.
        const bare = Stacked.walkAlong({ x: 2000, y: 7 }, { x: 312, y: 7 },
            world({ cols: Paths.columns(strip.slice(0, 2), 0), pause: 300 }));
        const alone = Paths.frame(bare, bare.ends);
        expect("with no window past the edge, it peeks in at its own height, then goes to its seat",
            [bare.beats.find(beat => beat.name === "peeks")?.to.y, alone.x, alone.y, alone.beat],
            [7, 312, 7, "sits"]);
    }

    // How niri's scroll carries it as it waits for the keys to rest: by
    // as far as its window moved, drawn on niri's spring from where it was.
    function scrolled(expect) {
        const was = world().was;
        const now = was.map(tile => Object.assign({}, tile, { left: tile.left - 1240 }));
        expect("its window scrolled, it's carried as far",
            [MarginCat.carries(was, now, 2), MarginCat.carries(was, was, 2),
                MarginCat.carries(was, now, 7)], [{ x: -1240, y: 0 }, null, null]);
        const spring = world().spring;
        const ends = MarginCat.slides(spring);
        const lags = [0, ends / 4, ends / 2, ends].map(t =>
            MarginCat.lags({ x: 1240, y: 0 }, t, spring).x);
        expect("drawn where it was as niri sets off, nearer as it slides, there as it stops",
            [lags[0], lags[1] < 1240 && lags[1] > lags[2], Math.abs(lags[3]) < 1240 * 0.01],
            [1240, true, true]);
        // Set off 20 ms before it's heard, and turned back 10 ms before it's
        // heard 120 ms on, so 110 ms into the first ride.
        const first = Carried.rides(null, { x: 100, y: 7 }, { x: -1140, y: 7 },
            { at: 1000 - 20, spring });
        expect("set off at rest, from where it sat, as far into the scroll as niri says",
            [first.from, first.at, first.ends], [{ x: 1240, y: 0 }, 980, ends]);
        const turned = Carried.rides(first, { x: -1140, y: 7 }, { x: 100, y: 7 },
            { at: 1100 - 10, spring });
        expect("turned back mid-way, from where it was drawn as niri turned it, not now",
            [turned.from.x, turned.at], [-1240 + MarginCat.lags(first.from, 110, spring).x, 1090]);
        const after = Carried.rides(first, { x: -1140, y: 7 }, { x: 100, y: 7 },
            { at: 9000, spring });
        expect("set off again once the first has settled, from its place",
            [Math.abs(after.from.x + 1240) < 1240 * 0.01, after.from.y], [true, 0]);
        // Let go, it rides the window it landed on, never the one it sat on.
        expect("carried by its seat's window, a window it was let go on, or on the floor none",
            [Carried.carriedBy(null, 2), Carried.carriedBy({ floats: false, id: 7 }, 2),
                Carried.carriedBy({ floats: false, id: null }, 2)], [2, 7, null]);
    }

    // Carried off the screen's side, then moved to the workspace below, it's
    // out of sight as niri slides, then comes in at that edge head first over
    // the new workspace's windows, never walking them from where its old
    // window went, thousands of pixels off.
    function movedOff(expect) {
        const at = world({});
        const slides = MarginCat.slides(at.spring);
        const seen = one => one.x + 8 > 0 && one.x < 1248 && one.y + 8 > 0 && one.y < 800;
        for (const [from, to, side] of [[4436, 312, "off the right"],
            [-3000, 900, "off the left"]]) {
            const move = Stacked.moves({ x: from, y: 7 }, { x: to, y: 7 }, at);
            const all = frames(move, 1000 / 120);
            const comes = all.find(seen);
            expect(`moved ${side}, it holds out of sight as niri slides`,
                [move.beats[0]?.name, Math.round(move.beats[0]?.ends),
                    all.filter(one => one.t < slides && seen(one)).length],
                ["rides", Math.round(slides), 0]);
            expect(`moved ${side}, its seat said again, it finds its way till the slide ends`,
                [Paths.replans(move, slides - 10, true), move.stacked, move.from],
                [true, undefined, { x: from, y: 7 }]);
            expect(`moved ${side}, it comes in at that edge, head first`,
                [comes?.beat, from > 0 ? comes?.x >= 1245 : comes?.x <= -5],
                ["peeks", true]);
            const seated = Paths.frame(move, move.ends);
            expect(`moved ${side}, then it walks to its seat`,
                [seated.x, seated.y, seated.beat], [to, 7, "sits"]);
        }
    }

    // How it goes back to its seat as the keys come back to its column
    // (MarginCatCarried.js's backOn): a ride on niri's spring only while one
    // runs or from wholly off the screen, never for a drag, and a ride under
    // way left to run on where it's placed there already.
    function backOnIt(expect) {
        const at = { wide: 8, across: 1248, riding: false, scrolling: false };
        const seat = { x: 312, y: 7 };
        const given = more => Object.assign({}, at, more);
        const cases = [
            ["nowhere before, it sits there at once", null, at, "sits"],
            ["placed there, a ride under way runs on", seat, given({ riding: true }), "stays"],
            ["placed there within half a pixel, it stays", { x: 312.4, y: 7 }, at, "stays"],
            ["a ride under way, it rides on to its seat", { x: 900, y: 7 },
                given({ riding: true }), "rides"],
            ["a scroll told within its spring, it rides", { x: 900, y: 7 },
                given({ scrolling: true }), "rides"],
            ["wholly off the right, it rides back in", { x: 4436, y: 7 }, at, "rides"],
            ["wholly off the left, it rides back in", { x: -8, y: 7 }, at, "rides"],
            ["a pixel on the screen, no scroll, it sits there at once", { x: -7, y: 7 }, at,
                "sits"],
            ["a floating window dragged, it sits there at once", { x: 312, y: 100 }, at, "sits"],
            ["its column grown across, it walks there", { x: 300, y: 7 },
                given({ reshaped: true }), "walks"],
            ["held short of this seat by a scroll, it walks on to it", { x: 900, y: 7 },
                given({ held: true, scrolling: true }), "walks"],
            ["held short of it with a ride under way, it walks on still", { x: 900, y: 7 },
                given({ held: true, riding: true }), "walks"],
            ["held short, nowhere before, it sits there at once", null, given({ held: true }),
                "sits"],
            ["held short of another's seat, a scroll still rides it", { x: 900, y: 7 },
                given({ held: false, scrolling: true }), "rides"]
        ];
        for (const [name, place, told, want] of cases)
            expect(`back on its column, ${name}`, Carried.backOn(place, seat, told), want);
    }

    // The puppy's visit ended as niri carries the cat's window away
    // (MarginCatCarried.js's visitCut): to its seat on that window, from a
    // hop's air too, riding only niri's scroll or to wholly off the screen,
    // and with no seat, as far as niri carried it.
    function visitCutIt(expect) {
        const at = { wide: 8, across: 1248, scrolling: false };
        const scrolls = Object.assign({}, at, { scrolling: true });
        const home = { x: 900, y: 7 };
        const hopping = { x: 312, y: 2 };
        const by = { x: 588, y: 0 };
        const cases = [
            ["its window scrolled away, it rides to its seat", home, hopping, scrolls,
                { to: home, how: "rides" }],
            ["cut mid-hop as a drag carries it, it sits on its seat at once", home, hopping, at,
                { to: home, how: "sits" }],
            ["its window scrolled wholly off the right, it rides as far as niri carried it",
                null, { x: 1000, y: 7 }, at, { to: { x: 1588, y: 7 }, how: "rides" }],
            ["no seat, still on the screen as a drag carries it, it sits there", null,
                hopping, at, { to: { x: 900, y: 2 }, how: "sits" }],
            ["nowhere before, with no seat, it goes nowhere", null, null, scrolls, null]
        ];
        for (const [name, seated, place, told, want] of cases)
            expect(`a visit cut, ${name}`, Carried.visitCut(seated, place, by, told), want);
    }

    // Caught mid-walk as the window it walks to moves, it rides that
    // window only standing on its top, never from the air of a leap, a side
    // or another window's top.
    function standsIt(expect) {
        const size = { wide: 8, tall: 8, air: 1 };
        const tile = { id: 2, left: 400, top: 40, width: 380, height: 740 };
        const standing = { x: 500, y: 31 };
        const cases = [
            ["walking along its top", standing, true],
            ["within half a cell of its top", { x: 500, y: 31.4 }, true],
            ["its middle just over its left edge", { x: 397, y: 31 }, true],
            ["in the air of a leap onto it", { x: 500, y: 20 }, false],
            ["hanging on its side", { x: 392, y: 60 }, false],
            ["on another window's top at its height", { x: 300, y: 31 }, false]
        ];
        for (const [name, at, want] of cases)
            expect(`caught mid-walk, ${name}`, Carried.standsOnTop(at, tile, size), want);
        expect("caught mid-walk, with no tile for it, it doesn't stand",
            Carried.standsOnTop(standing, undefined, size), false);
        // It holds and rides that window only as niri scrolls the strip;
        // a drag or a column moved beside it, which no scroll draws, walks
        // it on, as does a leap's air, scrolled or not.
        const told = more => Object.assign({}, size, more);
        const caughtCases = [
            ["standing on its top as niri scrolls", standing, told({ scrolling: true }),
                "holds"],
            ["standing on its top as a drag carries it", standing, told({ scrolling: false }),
                "walks on"],
            ["in a leap's air as niri scrolls", { x: 500, y: 20 }, told({ scrolling: true }),
                "walks on"]
        ];
        for (const [name, at, given, want] of caughtCases)
            expect(`caught mid-walk, ${name}`, Carried.caught(at, tile, given), want);
        expect("caught mid-walk, with no tile for it, it walks on",
            Carried.caught(standing, undefined, told({ scrolling: true })), "walks on");
    }

    // Its windows gone, it comes to the floor: from off the screen's side,
    // at that edge from the floor past it, whether it rode its old window
    // off the top or the bottom, never to a seat out of sight.
    function floorsIt(expect) {
        const at = { height: 800, tall: 8, wide: 8, across: 1248 };
        const cases = [
            ["on the screen, it comes straight down", { x: 312, y: 7 },
                { off: 0, from: { x: 312, y: 7 }, to: { x: 312, y: 792 } }],
            ["off the right over the screen", { x: 4436, y: -30 },
                { off: 1, from: { x: 1248, y: 792 }, to: { x: 1240, y: 792 } }],
            ["off the right under it", { x: 4436, y: 887 },
                { off: 1, from: { x: 1248, y: 792 }, to: { x: 1240, y: 792 } }],
            ["off the left", { x: -3000, y: 7 },
                { off: -1, from: { x: -8, y: 792 }, to: { x: 0, y: 792 } }],
            ["a pixel on the left, it comes straight down", { x: -7, y: 7 },
                { off: 0, from: { x: -7, y: 7 }, to: { x: -7, y: 792 } }]
        ];
        for (const [name, place, want] of cases)
            expect(`to the floor, ${name}`, Stacked.floorsFrom(place, at), want);
    }
}

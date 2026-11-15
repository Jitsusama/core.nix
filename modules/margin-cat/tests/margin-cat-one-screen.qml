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
        down(expect);
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

    function down(expect) {
        const at = world({});
        const slides = MarginCat.slides(at.spring);
        const seen = one => one.y + 8 > 0 && one.y < 800 && one.x + 8 > 0 && one.x < 1248;
        const walks = ["walkingA", "walkingB", "walkingC", "walkingD"];
        // Joel, Sat 14 17:30: "when going up or down it should appear on
        // the screen from the position that would make sense for how it
        // would have to path if they were all on one screen. so the top of
        // the left would have to walk to the right, climb down the right
        // side, drop down and grab with its claws on the top of the window
        // below, climb up, and then walk over" (VKCF3Z fcd95edd).
        for (const [from, to, name] of [[100, 900, "left to right"],
            [1100, 100, "right to left"]]) {
            const move = Stacked.moves({ x: from, y: 7 }, { x: to, y: 7 }, at);
            const all = frames(move, 1000 / 120);
            // niri's slide takes the old tops off the screen in a frame
            // or two, and all it shows till then is its walk, or its turn,
            // on the top it stood on as the slide carries it.
            const sliding = all.filter(one => one.t < slides && seen(one));
            const carried = one => one.y + 880 * MarginCat.slid(one.t, at.spring);
            expect(`down ${name}, it walks its top as niri slides it up, paws moving`,
                sliding.filter(one => !(walks.includes(one.pose) || one.beat === "turns")
                    || Math.abs(carried(one) - 7) > 1e-6)
                    .map(one => [Math.round(one.t), one.pose, one.beat, carried(one)]), []);
            expect(`down ${name}, it goes off the top with the workspace it left`,
                all.some(one => one.t < slides && one.y + 8 <= 0), true);
            const gone = all.findIndex(one => !seen(one));
            const back = all.findIndex((one, i) => i > gone && seen(one));
            const comes = all[back] ?? { t: Infinity };
            expect(`down ${name}, it comes in at the top over the gap, mid-drop`,
                [comes.x >= 616 && comes.x <= 624, comes.y < 0, comes.pose, comes.beat],
                [true, true, "falling", "drops"]);
            expect(`down ${name}, out of sight no more than about two thirds of a second`,
                comes.t - (all[gone]?.t ?? 0) <= 700, true);
            const names = move.beats.map(beat => beat.name);
            const grab = move.beats[names.indexOf("drops") + 1];
            expect(`down ${name}, the drop ends in a grab at a top's corner in its reach`,
                [grab?.name, grab?.to.y, grab?.to.x === 616 || grab?.to.x === 624],
                ["grabs", 8, true]);
            expect(`down ${name}, then it climbs on and walks to its seat`,
                Paths.frame(move, move.ends), { x: to, y: 7, pose: null, turn: 0,
                    facing: Math.sign(to - from), beat: "sits", done: true });
            // Its own ground, the slide taken out, never jumps: no more
            // than half its body between two frames at 120 Hz.
            const shown = all.filter(seen);
            expect(`down ${name}, all it shows is smooth on its own ground`,
                shown.filter((one, i) => i > 0 && one.t - shown[i - 1].t < 9
                    && Math.hypot(one.x - shown[i - 1].x, one.ground - shown[i - 1].ground) > 4)
                    .map(one => [Math.round(one.t), one.beat]), []);
            const inFace = (one, tiles) => tiles.some(tile => one.x < tile.left + tile.width - 0.5
                && one.x + 8 > tile.left + 0.5 && one.ground < tile.top + tile.height - 0.5
                && one.ground + 8 > tile.top + 0.5);
            expect(`down ${name}, in no window's face, old or new`,
                shown.filter(one => inFace(one, one.t < slides ? at.was : at.now))
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
            expect(`down ${name}, every pose it shows held two frames at 60 Hz`,
                runs.filter((one, i) => i > 0 && i < runs.length - 1
                    && one.until + 1 - one.from < 33 && runs[i + 1].from === one.until + 1)
                    .map(one => [one.pose, one.beat, Math.round(one.from)]), []);
        }

        // Its seat said again while it's out of sight, it finds its way
        // from where it set off, never walking on from above the screen.
        const again = Stacked.moves({ x: 100, y: 7 }, { x: 900, y: 7 }, at);
        const drops = again.beats.find(beat => beat.name === "drops").begins;
        expect("its seat moved as it's out of sight, it finds its way again, till it drops in",
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
        const late = Stacked.moves({ x: 100, y: 7 }, { x: 900, y: 7 }, world({ ahead: 50 }));
        expect("told of the slide 50 ms late, it's as far up as niri's slide has gone",
            Math.round(Paths.frame(late, 0).y * 100),
            Math.round((7 - 880 * MarginCat.slid(50, at.spring)) * 100));
    }
}

// The margin cat's ways on and off floating windows, in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat's scripts beside this file and runs
// `qml -platform offscreen margin-cat-floats.qml`.
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatPaths.js" as Paths
import "MarginCatRoutes.js" as Routes

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
        // A screen 1,248 across and 800 down, a cat 8 square a cell of 1
        // clear of what it stands on, so it stands 9 above a window's top.
        // A window down the left, two stacked on the right with a gap of
        // 16 between them, and whatever floats in front of them.
        const tiles = [
            { left: 16, top: 16, width: 600, height: 768, column: 0 },
            { left: 632, top: 16, width: 600, height: 376, column: 1 },
            { left: 632, top: 408, width: 600, height: 376, column: 1 }
        ];
        const float = (left, top, width, height) =>
            ({ left, right: left + width, top, bottom: top + height, id: left * 1000 + top });
        const world = more => Object.assign({
            cell: 1, ratio: 1, wide: 8, tall: 8, air: 1, across: 1248, height: 800,
            facing: 1, pause: 0, cols: Paths.columns(tiles, 0), floats: [], floated: []
        }, more);
        const names = way => way.beats.map(beat => beat.name);
        const named = (way, name) => way.beats.find(beat => beat.name === name) ?? null;
        const ends = beat => beat ? [beat.from.x, beat.from.y, beat.to.x, beat.to.y] : null;
        const last = (way, name) => way.beats.filter(beat => beat.name === name).pop() ?? null;

        // Onto one from above: along the tops to over it, and straight
        // down onto it in front of the window behind, landing crouched a
        // beat longer from past a leap's height (VKCF3Z 26038726).
        const low = float(300, 300, 100, 100);
        const onto = Paths.walk({ x: 900, y: 7 }, { x: 346, y: 291 }, world({ floats: [low] }));
        expect("onto a floating window it drops straight down from the top over it",
            ends(named(onto, "falls")), [346, 7, 346, 291]);
        expect("and lands there, never walking through the air",
            names(onto).slice(-2), ["falls", "lands"]);
        const landed = last(onto, "lands");
        expect("landing a beat longer from past a leap's height",
            landed && Math.round(landed.ends - landed.begins), 2 * MarginCat.LANDS);

        // Off one whose focus went: up onto the nearest ledge within a
        // leap, the gap between the right's two windows, 12 above.
        const gapped = float(700, 420, 200, 200);
        const leapsOff = Paths.walk({ x: 760, y: 411 }, { x: 900, y: 7 },
            world({ floats: [gapped] }));
        expect("off it, it crouches and leaps straight up onto the ledge in reach",
            [names(leapsOff).slice(0, 3), ends(leapsOff.beats[1])],
            [["crouches", "leaps", "lands"], [760, 411, 760, 399]]);
        expect("and goes on up from there, never shoved",
            names(leapsOff).includes("shoved"), false);

        // With no ledge in a leap, off its end toward its seat, and down.
        const deep = float(700, 600, 200, 100);
        const dropsOff = Paths.walk({ x: 760, y: 591 }, { x: 300, y: 7 },
            world({ floats: [deep] }));
        expect("too far below a ledge, it walks to its end, hops off and falls",
            names(dropsOff).slice(0, 3), ["goes", "hops", "falls"]);
        expect("off the end nearer its seat, onto what's under",
            [ends(dropsOff.beats[0]), ends(dropsOff.beats[1]), dropsOff.beats[2].to.y],
            [[760, 591, 700, 591], [700, 591, 692, 591], 792]);
        expect("never leaping far up because a ledge is nearest",
            names(dropsOff).indexOf("leaps") < 0
                || dropsOff.beats[names(dropsOff).indexOf("leaps")].to.y > 591 - 32, true);

        // Closed from under it: off at once from where it stands.
        const closed = Paths.walk({ x: 760, y: 411 }, { x: 900, y: 7 },
            world({ floated: [gapped], hurried: true }));
        expect("its window closed, it leaps up at once, never thinking or walking first",
            [names(closed).slice(0, 3), ends(closed.beats[1])],
            [["crouches", "leaps", "lands"], [760, 411, 760, 399]]);

        const fallsOff = Paths.walk({ x: 760, y: 591 }, { x: 300, y: 7 },
            world({ floated: [deep], hurried: true }));
        expect("closed with no ledge in a leap, it falls straight down in front of what's behind",
            [names(fallsOff).slice(0, 2), ends(fallsOff.beats[0])],
            [["falls", "lands"], [760, 591, 760, 792]]);

        // Along the same one: on its top the whole way.
        const along = Paths.walk({ x: 310, y: 291 }, { x: 346, y: 291 }, world({ floats: [low] }));
        expect("along one floating window it stays on its top, never falling",
            [along.beats.every(beat => beat.to.y === 291), names(along).includes("falls"),
                names(along).includes("shoved")], [true, false, false]);

        // Onto one higher than the window under it: from that window's
        // top beside its side edge, so the arc never crosses its face.
        const under = [{ left: 16, top: 200, width: 400, height: 584, column: 0 }];
        const high = float(300, 170, 200, 100);
        const leapsOn = Paths.walk({ x: 100, y: 191 }, { x: 396, y: 161 },
            world({ cols: Paths.columns(under, 0), floats: [high] }));
        expect("onto one a leap above, it crouches beside its edge, leaps up it and over",
            [names(leapsOn).slice(names(leapsOn).indexOf("leaps") - 1,
                names(leapsOn).indexOf("leaps") + 3), ends(named(leapsOn, "leaps")),
                ends(named(leapsOn, "over"))],
            [["crouches", "leaps", "over", "lands"], [292, 191, 292, 162],
                [292, 162, 300, 161]]);
        expect("then walks its top to its seat",
            ends(last(leapsOn, "goes")), [300, 161, 396, 161]);

        // Past a leap above: up its side and over.
        const higher = float(300, 100, 200, 100);
        const climbsOn = Paths.walk({ x: 100, y: 191 }, { x: 396, y: 91 },
            world({ cols: Paths.columns(under, 0), floats: [higher] }));
        const climbed = named(climbsOn, "climbs");
        expect("onto one past a leap above, it climbs its side, belly to it, and over",
            [ends(climbed), climbed?.turn, names(climbsOn).includes("over")],
            [[292, 191, 292, 92], -1, true]);
        expect("never leaping further up than a leap",
            climbsOn.beats.filter(beat => beat.name === "leaps")
                .every(beat => beat.from.y - beat.to.y <= 32), true);

        // In front of the gap between two windows, from the floor: up
        // some way round it and onto it, never up the gap behind it,
        // through its face (the GPU rig, 2026-11-14).
        const bridging = float(560, 420, 200, 100);
        const fromFloor = Paths.walk({ x: 100, y: 792 }, { x: 646, y: 411 },
            world({ floats: [bridging] }));
        // A beat's whole span, from end to end, since a climb can start
        // under its face and end over it.
        const through = beat => Math.max(beat.from.x, beat.to.x) + 8 > bridging.left
            && Math.min(beat.from.x, beat.to.x) < bridging.right
            && Math.max(beat.from.y, beat.to.y) + 8 > bridging.top + 1
            && Math.min(beat.from.y, beat.to.y) < bridging.bottom;
        expect("from the floor, it gets onto its top",
            [fromFloor.beats[fromFloor.beats.length - 1].to.x,
                fromFloor.beats[fromFloor.beats.length - 1].to.y], [646, 411]);
        expect("never passing through its face",
            fromFloor.beats.filter(through).map(beat => beat.name), []);

        // Never drawn over a floating window's own face, in any frame of
        // a way on or off it (VKCF3Z 4125043c), since it's drawn over the
        // tiled windows there: a leap on from beside its edge that moves
        // across as it rises cut its corner for 230 ms (2026-11-16). Each
        // way's frames every 4 ms, as where its box lies over the face; a
        // closed one has none.
        const overFace = (way, it) => {
            const hits = [];
            for (let t = 0; t <= way.ends; t += 4) {
                const at = Paths.frame(way, t);
                if (at.x + 8 > it.left + 0.5 && at.x < it.right - 0.5
                    && at.y + 8 > it.top + 0.5 && at.y < it.bottom - 0.5)
                    hits.push(`${Math.round(t)} ms at ${at.x.toFixed(1)},${at.y.toFixed(1)}`);
            }
            return hits.slice(0, 3);
        };
        expect("never over its face, on or off it",
            [[onto, low], [leapsOff, gapped], [dropsOff, deep], [along, low],
                [leapsOn, high], [climbsOn, higher], [fromFloor, bridging]]
                .map(([way, it]) => overFace(way, it)),
            [[], [], [], [], [], [], []]);

        // Over open floor, with no window over it nor under its sides:
        // up its own side from the floor and over.
        const narrow = [{ left: 16, top: 16, width: 384, height: 768, column: 0 }];
        const sideOn = Paths.walk({ x: 100, y: 792 }, { x: 646, y: 411 },
            world({ cols: Paths.columns(narrow, 0), floats: [bridging] }));
        expect("with no ledge over it, it climbs its own side from the floor",
            ends(named(sideOn, "climbs")), [552, 792, 552, 412]);
        expect("never through its face", sideOn.beats.filter(through).map(beat => beat.name),
            []);

        // Its seat on a floating window on a new workspace: it holds
        // through the slide, then drops straight onto it as a walk does.
        const moved = Paths.moves({ x: 900, y: 7 }, { x: 346, y: 291 },
            world({ way: 1, now: tiles, floats: [low], spring: { damping: 1, stiffness: 1600,
                epsilon: 0.001 } }));
        expect("moved to a seat on one, it holds, then drops straight onto it",
            [names(moved)[0], ends(named(moved, "falls")), names(moved).slice(-2)],
            ["rides", [346, 7, 346, 291], ["falls", "lands"]]);

        // Drawn under the windows, but over them standing on a floating
        // one, which the tiled window behind would hide it on.
        const floating = { left: 300, top: 300, width: 400, floats: true };
        const tiled = { left: 16, top: 16, width: 600, floats: false };
        expect("on a floating window it's drawn over the tiled ones",
            MarginCat.above(floating, false), true);
        expect("leaving it, it goes under them on the first drawing",
            MarginCat.above(floating, true), false);
        expect("anywhere else it's drawn under the windows",
            [MarginCat.above(tiled, false), MarginCat.above(null, false)], [false, false]);

        // On its way onto or off one, over the tiled windows for each
        // leg that begins or ends on it or climbs its side, since it's in
        // front of the tiled window behind, which would hide it there and
        // have it pop onto the floating one (Joel, 2026-11-15 23:34: "the
        // whole kitty should not have to be rendered when you are seeing
        // it come on to your screen"); under them for every other leg.
        const drawn = (way, name) => way.beats.filter(beat => beat.name === name)
            .map(beat => Paths.frame(way, (beat.begins + beat.ends) / 2).aboard === true);
        const drawnOver = way => way.beats.map(beat =>
            Paths.frame(way, (beat.begins + beat.ends) / 2).aboard === true);
        expect("dropping onto a floating window, it's drawn over the tiled ones",
            [drawn(onto, "falls"), drawn(onto, "lands").pop()], [[true], true]);
        expect("but under them along the tops on its way there",
            drawn(onto, "goes").every(over => !over), true);
        expect("leaping off one, over them until it lands on the ledge",
            [drawnOver(leapsOff).slice(0, 2), drawnOver(leapsOff).slice(2)
                .every(over => !over)], [[true, true], true]);
        expect("hopping off its end, over them as it walks, hops and falls",
            [drawnOver(dropsOff).slice(0, 3), drawnOver(dropsOff).slice(3).every(over => !over)],
            [[true, true, true], true]);
        expect("closed, over them as it falls in front of what's behind",
            drawn(fallsOff, "falls"), [true]);
        expect("along one, over them the whole way", drawnOver(along).every(over => over), true);
        expect("climbing one's side and over onto it, over them",
            [drawn(climbsOn, "climbs"), drawn(climbsOn, "over")], [[true], [true]]);
        expect("moved to a seat on one, under them through the slide, over as it drops",
            [drawn(moved, "rides"), drawn(moved, "falls")], [[false], [true]]);
        const nowhereNear = Paths.walk({ x: 900, y: 7 }, { x: 100, y: 7 }, world());
        expect("with no floating window on its way, never over them",
            drawnOver(nowhereNear).some(over => over), false);
        expect("over them as it walks on a leg aboard, its seat anywhere",
            [MarginCat.above(tiled, true, true), MarginCat.above(null, true, true)], [true, true]);
        expect("and under them on any other leg, even leaving a floating one",
            [MarginCat.above(floating, true, false), MarginCat.above(tiled, false, false)],
            [false, false]);

        // A floating window on its way between tiled windows is a ledge
        // and sides as theirs are (WCJQSG ade5ce02: "it climbs onto, jumps
        // off and walks along floats as it does tiled ones"; VKCF3Z
        // 4125043c, slice 2). Across the tops it walks: up onto it from
        // beside its edge, along its top, and off its far end down onto
        // the tops again, never through its face nor hidden behind it.
        const straddles = float(500, 10, 250, 200);
        const overIt = Paths.walk({ x: 100, y: 7 }, { x: 900, y: 7 },
            world({ floats: [straddles] }));
        const endsAt = way => {
            const end = way.beats[way.beats.length - 1].to;
            return [end.x, end.y];
        };
        expect("one across the tops on its way, it walks over its top",
            [endsAt(overIt), overIt.beats.some(beat => beat.to.y === 1),
                overFace(overIt, straddles)], [[900, 7], true, []]);
        expect("up its edge from beside it, and off its far end down onto the tops",
            [ends(named(overIt, "over")), ends(named(overIt, "hops")),
                ends(named(overIt, "falls"))],
            [[492, 2, 500, 1], [742, 1, 750, 1], [750, 1, 750, 7]]);
        expect("over the tiled windows on it, and under them on the tops either side",
            [drawn(overIt, "over"), drawn(overIt, "falls"),
                drawnOver(overIt)[0], drawnOver(overIt)[drawnOver(overIt).length - 1]],
            [[true], [true], false, false]);

        // A step up: a column too high to leap onto from the one beside
        // it, with a floating window between that is a leap above the
        // lower and a leap below the higher. It leaps onto that and on up,
        // never climbing the higher's side.
        const steps = [
            { left: 16, top: 700, width: 400, height: 84, column: 0 },
            { left: 432, top: 640, width: 800, height: 144, column: 1 }
        ];
        const stone = float(300, 670, 120, 100);
        const upIt = Paths.walk({ x: 100, y: 691 }, { x: 900, y: 631 },
            world({ cols: Paths.columns(steps, 0), floats: [stone] }));
        expect("a floating window a leap up on its way is a step",
            [endsAt(upIt), upIt.beats.some(beat => beat.to.y === 661),
                names(upIt).includes("climbs"), overFace(upIt, stone)],
            [[900, 631], true, false, []]);
        const leapsUp = upIt.beats.find(beat => beat.from.y === 661 && beat.to.y === 631)
            ?? null;
        expect("from its end up onto the higher top across the gap",
            [ends(leapsUp), leapsUp
                && Paths.frame(upIt, (leapsUp.begins + leapsUp.ends) / 2).aboard === true],
            [[412, 661, 432, 631], true]);

        // The same way back, right to left: up its right edge, along,
        // and off its left end down onto the tops.
        const backOver = Paths.walk({ x: 900, y: 7 }, { x: 100, y: 7 },
            world({ floats: [straddles] }));
        expect("right to left, up its right edge and off its left end",
            [ends(named(backOver, "over")), ends(named(backOver, "hops")),
                ends(named(backOver, "falls")), endsAt(backOver), overFace(backOver, straddles)],
            [[750, 2, 742, 1], [500, 1, 492, 1], [492, 1, 492, 7], [100, 7], []]);

        // A step down: from its end across the gap onto a lower top, as
        // a leap onto a ledge, its own floating window behind it.
        const level = [
            { left: 16, top: 700, width: 400, height: 84, column: 0 },
            { left: 432, top: 700, width: 800, height: 84, column: 1 }
        ];
        const downIt = Routes.next({ kind: "float", c: 0, x: 412, y: 661, row: 0 },
            Paths.columns(level, 0), world({ floats: [stone] }));
        expect("from its end down across the gap onto the lower top",
            downIt.filter(one => one.stop.how === "leaps")
                .map(one => [one.stop.x, one.stop.y]), [[432, 691]]);

        // Off its end beside a column's edge, onto that column's top as a
        // fall does anywhere, not past its top to the floor.
        const nearEdge = float(500, 10, 128, 200);
        const offNear = Paths.walk({ x: 100, y: 7 }, { x: 900, y: 7 },
            world({ floats: [nearEdge] }));
        expect("off its end part over a column, onto that column's top",
            [offNear.beats.some(beat => beat.to.y > 7), endsAt(offNear)], [false, [900, 7]]);

        // Never a leap up onto one from a gap's ledge beside its edge,
        // which would rise into the window over the gap.
        const inGap = float(700, 380, 200, 200);
        const fromGap = Routes.next({ kind: "ledge", c: 1, x: 692, y: 399, row: 1 },
            Paths.columns(tiles, 0), world({ floats: [inGap] }));
        expect("from a gap's ledge, never a leap up into the window over it",
            fromGap.filter(one => one.stop.how === "leapsOn").length, 0);

        // Onto and off one whose top is 2 above the tiles', at the
        // screen's top: no arc to show, so it steps up and down within
        // its walk, paws moving, rather than glide at the edge in a
        // leap's pose (VKCF3Z 299fb911).
        const flush = float(700, 14, 200, 100);
        const stepsOn = Paths.walk({ x: 100, y: 7 }, { x: 800, y: 5 },
            world({ floats: [flush] }));
        const stepsOff = Paths.walk({ x: 800, y: 5 }, { x: 100, y: 7 },
            world({ floats: [flush] }));
        // Its beats from beside a floating window's edge to beside its far
        // one, not the gap's leap between columns. By it, it only walks:
        // never a crouch, a leap or a landing's crouch, and never up or
        // down more than `most` in one leg, 2 cells unless it's said.
        const by = (way, it, wide) => way.beats.filter(beat =>
            Math.max(beat.from.x, beat.to.x) >= it.left - (wide ?? 8)
                && Math.min(beat.from.x, beat.to.x) <= it.right);
        const walks = (way, it, most, wide) => by(way, it, wide).length > 0
            && by(way, it, wide).every(beat =>
                ["goes", "turns", "settles"].includes(beat.name)
                    && Math.abs(beat.to.y - beat.from.y) <= (most ?? 2));
        expect("onto and off one 2 above the tops, it steps within its walk, never leaping",
            [walks(stepsOn, flush), walks(stepsOff, flush)], [true, true]);
        expect("and gets where it's going",
            [ends(stepsOn.beats[stepsOn.beats.length - 1]).slice(2),
                ends(stepsOff.beats[stepsOff.beats.length - 1]).slice(2)], [[800, 5], [100, 7]]);
        expect("stepping on and off one 2 above, never over its face",
            [overFace(stepsOn, flush), overFace(stepsOff, flush)], [[], []]);
        // One cell more, with room over it for an arc: a leap on, a hop
        // off.
        const three = float(700, 13, 200, 100);
        const leapsBy = (way, it, wide) => by(way, it, wide).map(beat => beat.name)
            .filter(name => ["crouches", "leaps", "over", "hops", "falls"].includes(name));
        expect("3 above, with room for its arc, it leaps on and hops off",
            [leapsBy(Paths.walk({ x: 100, y: 7 }, { x: 800, y: 4 }, world({ floats: [three] })),
                three),
                leapsBy(Paths.walk({ x: 800, y: 4 }, { x: 100, y: 7 }, world({ floats: [three] })),
                    three)],
            [["crouches", "leaps", "over"], ["hops", "falls"]]);

        // A cell of 2 and a cat 16 square: 2 cells up is a step, 3 a leap.
        const bigTiles = [
            { left: 16, top: 48, width: 600, height: 736, column: 0 },
            { left: 632, top: 48, width: 600, height: 736, column: 1 }
        ];
        const big = it => world({ cell: 2, wide: 16, tall: 16, air: 2,
            cols: Paths.columns(bigTiles, 0), floats: [it] });
        const twoUp = float(700, 44, 200, 100);
        const threeUp = float(700, 42, 200, 100);
        const bigOn = Paths.walk({ x: 100, y: 30 }, { x: 800, y: 26 }, big(twoUp));
        const bigOff = Paths.walk({ x: 800, y: 26 }, { x: 100, y: 30 }, big(twoUp));
        expect("at a cell of 2, 2 cells up it steps on and off, getting where it's going",
            [walks(bigOn, twoUp, 4, 16), walks(bigOff, twoUp, 4, 16), endsAt(bigOn),
                endsAt(bigOff),
                overFace(bigOn, twoUp), overFace(bigOff, twoUp)],
            [true, true, [800, 26], [100, 30], [], []]);
        expect("and 3 cells up it leaps on and hops off",
            [leapsBy(Paths.walk({ x: 100, y: 30 }, { x: 800, y: 24 }, big(threeUp)), threeUp,
                16),
                leapsBy(Paths.walk({ x: 800, y: 24 }, { x: 100, y: 30 }, big(threeUp)),
                    threeUp, 16)],
            [["crouches", "leaps", "over"], ["hops", "falls"]]);

        // At the screen's top, as on the GPU rig: the tops stand 3 down
        // and a floating window across the gap stands 3 above the screen,
        // where a leap's arc would be off it, drawn held at the edge. It
        // walks up beside its edge, along its top and down off its far
        // end, never through its face (VKCF3Z 299fb911).
        const rigTiles = [
            { left: 16, top: 12, width: 600, height: 772, column: 0 },
            { left: 632, top: 12, width: 600, height: 772, column: 1 }
        ];
        const rig = it => world({ cols: Paths.columns(rigTiles, 0), floats: [it] });
        const atEdge = float(500, 6, 250, 200);
        const rigOver = Paths.walk({ x: 100, y: 3 }, { x: 900, y: 3 }, rig(atEdge));
        const rigBack = Paths.walk({ x: 900, y: 3 }, { x: 100, y: 3 }, rig(atEdge));
        expect("at the screen's top, a float with no room for an arc is walked over both ways",
            [walks(rigOver, atEdge, 8), walks(rigBack, atEdge, 8), endsAt(rigOver),
                endsAt(rigBack), overFace(rigOver, atEdge), overFace(rigBack, atEdge)],
            [true, true, [900, 3], [100, 3], [], []]);
        expect("up beside its edge and down beside its far one",
            by(rigOver, atEdge).map(ends).slice(1),
            [[492, 3, 492, -3], [492, -3, 500, -3], [500, -3, 742, -3],
                [742, -3, 750, -3], [750, -3, 750, 3], [750, 3, 900, 3]]);
        expect("over the tiled windows from its step up to its step down, under them either side",
            by(rigOver, atEdge).map(beat =>
                Paths.frame(rigOver, (beat.begins + beat.ends) / 2).aboard === true),
            [false, true, true, true, true, true, false]);
        // And onto a seat on it and off it again, as lands() and off() go.
        const seatOn = Paths.walk({ x: 100, y: 3 }, { x: 600, y: -3 }, rig(atEdge));
        const seatOff = Paths.walk({ x: 600, y: -3 }, { x: 100, y: 3 }, rig(atEdge));
        expect("at the screen's top, it walks onto a seat on it and off it again",
            [walks(seatOn, atEdge, 8), walks(seatOff, atEdge, 8), endsAt(seatOn),
                endsAt(seatOff), overFace(seatOn, atEdge), overFace(seatOff, atEdge)],
            [true, true, [600, -3], [100, 3], [], []]);
        // Up to its own height it walks; a cell taller and it keeps its
        // leap, held at the edge, rather than walk up a wall.
        const capped = float(500, 4, 250, 200);
        const pastCap = float(500, 3, 250, 200);
        expect("its own height up at the screen's top, it walks; a cell more, it leaps",
            [walks(Paths.walk({ x: 100, y: 3 }, { x: 900, y: 3 }, rig(capped)), capped, 8),
                leapsBy(Paths.walk({ x: 100, y: 3 }, { x: 900, y: 3 }, rig(pastCap)), pastCap)],
            [true, ["crouches", "leaps", "over", "crouches", "hops", "falls"]]);
        // Its apex is its first leap's, to a cell under its landing: on
        // the screen there, it leaps, though its landing stands at its top.
        const toTheTop = float(500, 9, 250, 200);
        expect("onto one standing at the screen's top, its arc still on it, it leaps",
            leapsBy(Paths.walk({ x: 100, y: 7 }, { x: 900, y: 7 }, world({ floats: [toTheTop] })),
                toTheTop).slice(0, 3),
            ["crouches", "leaps", "over"]);
        // Straight up off one onto a ledge standing above the screen.
        const highTiles = [
            { left: 16, top: 6, width: 600, height: 778, column: 0 },
            { left: 632, top: 6, width: 600, height: 778, column: 1 }
        ];
        const underHigh = float(700, 12, 200, 100);
        const upHigh = Paths.walk({ x: 800, y: 3 }, { x: 1000, y: -3 },
            world({ cols: Paths.columns(highTiles, 0), floats: [underHigh] }));
        expect("up off one onto a ledge above the screen's top, it walks",
            [walks(upHigh, underHigh, 8), ends(upHigh.beats[0])], [true, [800, 3, 800, -3]]);
        // At a cell of 2 its apex is 2 pixels over its first leap's end,
        // so 1 over the screen's top it walks, and 1 under it, it leaps.
        const highBig = [
            { left: 16, top: 24, width: 600, height: 760, column: 0 },
            { left: 632, top: 24, width: 600, height: 760, column: 1 }
        ];
        const bigHigh = it => world({ cell: 2, wide: 16, tall: 16, air: 2,
            cols: Paths.columns(highBig, 0), floats: [it] });
        const nearTop = float(500, 17, 250, 200);
        const roomTop = float(500, 19, 250, 200);
        expect("at a cell of 2, its apex over the screen's top it walks, under it it leaps",
            [walks(Paths.walk({ x: 100, y: 6 }, { x: 1000, y: 6 }, bigHigh(nearTop)),
                nearTop, 16, 16),
                leapsBy(Paths.walk({ x: 100, y: 6 }, { x: 1000, y: 6 }, bigHigh(roomTop)),
                    roomTop, 16).slice(0, 3)],
            [true, ["crouches", "leaps", "over"]]);

        // Straight up off one 2 under the tops onto them, and straight
        // down onto it from them: steps, never a leap or a fall.
        const sunk = float(700, 18, 200, 100);
        const upOff = Paths.walk({ x: 800, y: 9 }, { x: 1000, y: 7 }, world({ floats: [sunk] }));
        const downOn = Paths.walk({ x: 1000, y: 7 }, { x: 800, y: 9 }, world({ floats: [sunk] }));
        expect("2 under the tops, it steps straight up off it and down onto it",
            [walks(upOff, sunk), walks(downOn, sunk), ends(upOff.beats[0]),
                ends(downOn.beats[downOn.beats.length - 1])],
            [true, true, [800, 9, 800, 7], [800, 7, 800, 9]]);

        // One off its way changes nothing, along the tops nor by a route.
        const aside = float(300, 600, 100, 100);
        expect("a floating window off its way changes nothing",
            names(Paths.walk({ x: 100, y: 7 }, { x: 900, y: 7 }, world({ floats: [aside] }))),
            names(Paths.walk({ x: 100, y: 7 }, { x: 900, y: 7 }, world())));
        expect("nor on a way that takes a route",
            names(Paths.walk({ x: 100, y: 7 }, { x: 900, y: 399 }, world({ floats: [aside] }))),
            names(Paths.walk({ x: 100, y: 7 }, { x: 900, y: 399 }, world())));

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}

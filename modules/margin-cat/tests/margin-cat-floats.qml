// The margin cat's ways on and off floating windows, in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat's scripts beside this file and runs
// `qml -platform offscreen margin-cat-floats.qml`.
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatPaths.js" as Paths

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
        expect("onto one a leap above, it crouches beside its edge and leaps on",
            [names(leapsOn).slice(names(leapsOn).indexOf("leaps") - 1,
                names(leapsOn).indexOf("leaps") + 2), ends(named(leapsOn, "leaps"))],
            [["crouches", "leaps", "lands"], [292, 191, 300, 161]]);
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

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}

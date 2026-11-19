// Journeys over generated layouts, run in Qt's own JavaScript engine,
// the one the shell runs the cat in: the rules every way it finds must
// keep, whatever the windows, apart from margin-cat.qml's cases written
// by hand. The `the-margin-cat-sits-above-its-column` check copies
// modules/margin-cat's scripts beside this file and runs
// `qml -platform offscreen margin-cat-journeys.qml`.
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
        journeys(expect);
        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // Journeys thrown at it, the same each run: layouts of one to four
    // columns of one to three windows, scrolled or not, at Joel's laptop's
    // size and scale, and the keys shifting to any window of them, on the
    // same workspace or the one above or below. Every one must end on the
    // window's seat, never jump between two frames at 120 Hz, never go
    // into a window, the marks' ink or off the screen's sides (Joel,
    // 2026-11-13, by WCJQSG bd480983: "the cat should always be moving
    // from where it is to where my focus shifts, in a believable and
    // adorable way. the windows are the walls and roofs it is climbing or
    // jumping off of").
    function journeys(expect) {
        let state = 20261113;
        const random = () => {
            state = (state * 1103515245 + 12345) % 2147483648;
            return state / 2147483648;
        };
        const cell = 4 / 3;
        const at = { cell, ratio: 1.5, wide: 8 * cell, tall: 8 * cell, air: cell, across: 1276,
            height: 773, facing: 1, pause: 0, marks: Paths.marks(3, 1276, 16),
            spring: { damping: 1, stiffness: 1600, epsilon: 0.001 } };
        const layout = () => {
            const tiles = [];
            let left = 16 - (random() < 0.3 ? Math.floor(random() * 500) : 0);
            const count = 1 + Math.floor(random() * 4);
            for (let c = 0; c < count; c++) {
                const width = 200 + Math.floor(random() * 700);
                const rows = 1 + Math.floor(random() * 3);
                const shares = [];
                for (let r = 0; r < rows; r++)
                    shares.push(random());
                const spare = 741 - 16 * (rows - 1) - 100 * rows;
                const all = shares.reduce((sum, share) => sum + share, 0);
                let top = 16;
                for (let r = 0; r < rows; r++) {
                    const height = r === rows - 1 ? 757 - top
                        : 100 + Math.floor((spare * shares[r]) / all);
                    tiles.push({ left, top, width, height, column: c, id: tiles.length });
                    top += height + 16;
                }
                left += width + 16;
            }
            return tiles;
        };
        const seatOn = tiles => {
            const tile = tiles[Math.floor(random() * tiles.length)];
            return MarginCat.sits({ column: tile, across: at.across, marks: 3, cell: 16,
                wide: at.wide, tall: at.tall, air: at.air });
        };
        const inside = (x, y, box) => x < box.left + box.width - 0.5
            && x + at.wide > box.left + 0.5 && y < box.top + box.height - 0.5
            && y + at.tall > box.top + 0.5;
        const lit = at.marks;
        const ink = { left: lit.from + 4, top: 4, width: lit.to - lit.from - 8, height: 8 };
        const wrong = [];
        const climbs = [];
        // No drawing alone (RSCH-20261113-24R2QV, finding 4): outside a
        // ride, each pose it's seen in holds two frames at 60 Hz or more,
        // but its first and its last, which go on from and into what's
        // before and after the journey. And no skid (Joel, 2026-11-14 17:19:
        // "i never want the kitty to skid into place, i always want to see
        // its paws moving when it is walking or running"): walking, no
        // drawing held while it goes more than its stride and a cell, and
        // nothing backward in a drawing held as it walks, stops or turns;
        // climbing up or down a side the same (Joel, 2026-11-14 22:40: "the
        // kitten when crawling up or across window edges still often does
        // not look like its paws are moving").
        const walking = ["goes", "climbs", "turns", "thinks", "settles", "lands",
            "shakes"];
        const striding = ["goes", "climbs"];
        // And each change between two strides moves one pair of feet, never
        // both, nor a pair back but onto its stand as it holds there, drawn
        // as stride B, as margin-cat-floats.qml holds a walk alone to.
        const strideOrder = ["walkingA", "walkingC", "walkingB", "walkingD"];
        const standsIn = ["turns", "thinks", "settles", "grabs"];
        const stepsBadly = (before, one) => {
            const on = (strideOrder.indexOf(one.pose) - strideOrder.indexOf(before) + 4) % 4;
            return strideOrder.includes(before) && strideOrder.includes(one.pose)
                && (on % 2 === 0 || (on === 3
                    && !(one.pose === "walkingB" && standsIn.includes(one.beat))));
        };
        // A journey set off from `from`, a frame of another, is judged
        // from it, so the step across the change is held too.
        const posed = one => (one.y + at.tall > 0 && one.y < at.height && one.beat !== "rides"
            ? one.pose : undefined);
        const flashes = (journey, said, from) => {
            let run = from && { pose: posed(from), begins: 0, ends: 0, first: from, last: from,
                walks: walking.includes(from.beat), goes: striding.includes(from.beat) };
            const skids = () => {
                const went = Math.abs(run.last.x - run.first.x)
                    + Math.abs(run.last.y - run.first.y);
                if (run.walks && run.goes && went > journey.stride + at.cell + 0.01)
                    wrong.push(`${said} glides ${Math.round(went / at.cell)} cells in one `
                        + `${run.pose} at ${run.begins} ms, ${run.first.beat} to ${run.last.beat}`);
                if (run.walks && (run.last.x - run.first.x) * run.first.facing < -0.01)
                    wrong.push(`${said} slides back in one ${run.pose} at ${run.begins} ms`);
            };
            for (let t = 0; t <= journey.ends; t += 1) {
                const one = Paths.frame(journey, t);
                const pose = posed(one);
                if (run !== null && pose === run.pose) {
                    run.ends = t;
                    run.last = one;
                    run.walks = run.walks && walking.includes(one.beat);
                    run.goes = run.goes || striding.includes(one.beat);
                    continue;
                }
                const ms = run === null ? 0 : run.ends - run.begins + 1;
                if (run !== null && run.begins > 0 && run.pose && ms < 33)
                    wrong.push(`${said} shows ${run.pose} ${ms} ms at ${run.begins} ms, `
                        + `${run.first.beat} to ${run.last.beat}`);
                if (run !== null && run.pose)
                    skids();
                if (run !== null && stepsBadly(run.pose, { pose, beat: one.beat }))
                    wrong.push(`${said} skids from ${run.pose} into ${pose} at ${t} ms, `
                        + `${run.last.beat} to ${one.beat}`);
                run = { pose, begins: t, ends: t, first: one, last: one,
                    walks: walking.includes(one.beat), goes: striding.includes(one.beat) };
            }
        };
        // Every frame of `journey` a cat's, in none of `solid`'s windows
        // but where a window shoved it, and ending on `to`.
        const check = (journey, solid, said, to, from = null) => {
            flashes(journey, said, from);
            const last = Paths.frame(journey, journey.ends);
            if (Math.abs(last.x - to.x) > 0.01 || Math.abs(last.y - to.y) > 0.01)
                wrong.push(`${said} ends at ${last.x},${last.y}`);
            // Up or down a side at about the pace it walks a top (Joel,
            // 2026-11-13 17:22, by WCJQSG fc3f716a: "Going up the side of
            // a window should be at the same speed roughly as walking on
            // top, it seems accelerated in the video"), each pace its
            // cells a second, of the beats seen; each between the eases
            // over its first and last strides (MarginCatWays.js's timed).
            const seenOn = beat => Math.max(beat.from.y, beat.to.y) + at.tall > 0
                && Math.min(beat.from.y, beat.to.y) < at.height;
            const pace = beat => (Math.abs(beat.to.x - beat.from.x)
                + Math.abs(beat.to.y - beat.from.y)) / at.cell / ((beat.ends - beat.begins) / 1000);
            const cruises = beat => pace(beat) * (1 + beat.ramps[0] + beat.ramps[1]);
            const walks = journey.beats.filter(beat => beat.kind === "goes" && seenOn(beat)
                && pace(beat) > 0).map(cruises);
            for (const beat of journey.beats.filter(one => one.kind === "climbs" && seenOn(one)))
                if (walks.some(walked => Math.abs(cruises(beat) - walked) > walked / 5))
                    climbs.push(`${said} climbs at ${Math.round(cruises(beat))} cells a second, `
                        + `walking at ${walks.map(Math.round).join(", ")}`);
            // Every fall straight down onto what's under it, never across
            // the air (Joel, 2026-11-13 17:22, by WCJQSG fc3f716a: "it
            // would not be able to drop down near the center of a window
            // below").
            for (const beat of journey.beats.filter(one => one.kind === "falls"))
                if (Math.abs(beat.to.x - beat.from.x) > 0.01)
                    wrong.push(`${said} falls from ${Math.round(beat.from.x)} `
                        + `to ${Math.round(beat.to.x)}`);
            let before = null;
            for (let t = 0; t <= journey.ends && wrong.length < 5; t += 1000 / 120) {
                const one = Paths.frame(journey, t);
                const shown = one.y + at.tall > 0 && one.y < at.height;
                // Never gone from the screen, holding through a slide
                // rather than riding off it (VKCF3Z 4138e938); a hop off
                // the bar may still arc past its top.
                if (!shown)
                    wrong.push(`${said} is off the screen at ${t} ms, ${one.beat}`);
                else if (one.beat !== "rides" && one.beat !== "shoved"
                    && solid.some(tile => inside(one.x, one.y, tile)))
                    wrong.push(`${said} is inside a window at ${t} ms, ${one.beat}`);
                else if (shown && one.beat !== "hops" && one.beat !== "perches"
                    && one.beat !== "shoved" && inside(one.x, one.y, ink))
                    wrong.push(`${said} is in the marks at ${t} ms, ${one.beat}`);
                else if (shown && (one.x < -0.01 || one.x + at.wide > at.across + 0.01))
                    wrong.push(`${said} is off the screen's side at ${t} ms, ${one.beat}`);
                else if (shown && before !== null && before.beat !== "rides"
                    && Math.hypot(one.x - before.x, one.y - before.y) > at.tall / 2 + 1e-6)
                    wrong.push(`${said} jumps at ${t} ms, ${before.beat} to ${one.beat}`);
                before = shown ? one : null;
            }
        };
        let made = 0;
        for (let n = 0; n < 400 && made < 150; n++) {
            const way = [0, 1, -1][Math.floor(random() * 3)];
            const was = layout();
            const now = way === 0 ? was : layout();
            const from = seatOn(was);
            const to = seatOn(now);
            if (from === null || to === null)
                continue;
            const journey = way === 0
                ? Paths.walk(from, to, Object.assign({ cols: Paths.columns(now, 0) }, at))
                : Paths.moves(from, to, Object.assign({ way, now }, at));
            if (journey === null)
                continue;
            made += 1;
            const said = `${n} (${way}, ${JSON.stringify(from)} to ${JSON.stringify(to)})`;
            check(journey, now, said, to);
            // And the windows changed under it somewhere on its way past
            // the slide, its way found again from where it's got to, in
            // the stride it's drawn in as the screen sets it off
            // (MarginCatScreen.qml's walkFrom()).
            const t = journey.beats[0].ends + random() * (journey.ends - journey.beats[0].ends);
            const here = Paths.frame(journey, t);
            const changed = changes(now, random);
            const then = seatOn(changed);
            const again = then === null ? null : Paths.walk({ x: here.x, y: here.y }, then,
                Object.assign({ cols: Paths.columns(changed, 0), hurried: true,
                    before: Paths.columns(now, 0), drawnIn: here.pose ?? null,
                    drawnInto: here.into ?? null, drawnAtPace: here.atPace ?? false }, at,
                { facing: here.facing }));
            const starts = again === null ? here : Paths.frame(again, 0);
            if (Math.hypot(starts.x - here.x, starts.y - here.y) > 0.01)
                wrong.push(`${said} changed jumps from ${here.x},${here.y}`);
            if (again !== null)
                check(again, changed, `${said} changed at ${Math.round(t)} ms`, then, here);
        }
        expect("150 journeys thrown at it", made, 150);
        expect("each ends on its seat, every frame of it a cat's", wrong.slice(0, 5), []);
        expect("it climbs a side at about its walking pace, within a fifth",
            climbs.slice(0, 5), []);
    }

    // `tiles` changed as niri changes them: a column grown or shrunk, a
    // split between two windows moved, a window closed or opened, or
    // the columns scrolled.
    function changes(tiles, random) {
        const moved = tiles.map(tile => Object.assign({}, tile));
        const by = span => Math.floor((random() - 0.5) * 2 * span);
        const pick = list => list[Math.floor(random() * list.length)];
        const tile = pick(moved);
        const own = moved.filter(one => one.column === tile.column);
        const right = (shift, column) => moved.filter(one => one.column > column)
            .forEach(one => { one.left += shift; });
        const kind = Math.floor(random() * 5);
        if (kind === 0) {
            const width = Math.max(100, tile.width + by(300));
            right(width - tile.width, tile.column);
            own.forEach(one => { one.width = width; });
        } else if (kind === 1 && own.length > 1) {
            const r = own.indexOf(tile) > 0 ? own.indexOf(tile) : 1;
            const shift = Math.min(Math.max(by(150), 60 - own[r - 1].height),
                own[r].height - 60);
            own[r - 1].height += shift;
            own[r].top += shift;
            own[r].height -= shift;
        } else if (kind === 2 && moved.length > 1) {
            const gone = own.indexOf(tile);
            moved.splice(moved.indexOf(tile), 1);
            if (own.length === 1)
                right(-tile.width - 16, tile.column);
            else if (gone > 0)
                own[gone - 1].height += tile.height + 16;
            else
                Object.assign(own[1], { top: tile.top, height: own[1].height + tile.height + 16 });
        } else if (kind === 3 && tile.height >= 216) {
            const half = Math.floor((tile.height - 16) / 2);
            moved.push(Object.assign({}, tile, { top: tile.top + half + 16,
                height: tile.height - half - 16, id: 1000 + moved.length }));
            tile.height = half;
            moved.sort((one, other) => one.column - other.column || one.top - other.top);
        } else {
            right(by(400), -1);
        }
        return moved;
    }
}

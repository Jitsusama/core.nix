// How the launcher's list keeps its rows, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `a-row-kept-is-never-drawn-again`
// check copies modules/quickshell/launcher/RowsKept.js and tests/lib/Expect.js
// beside this file and runs it offscreen. Each case does the steps to a list
// as Launcher.qml's keep does them to its ListModel, and holds that they end
// on the rows listed now, drawing afresh only those it didn't hold.
import QtQuick
import "Expect.js" as Expect
import "RowsKept.js" as RowsKept

QtObject {
    // A case that doesn't hold throws, naming it, and ends as a failure,
    // rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // The rows the steps leave, and those they drew afresh, as ListModel's
    // remove, move and insert would.
    function done(was, now, most) {
        const rows = was.slice();
        const drawn = [];
        for (const step of RowsKept.steps(was, now, most)) {
            if (step.remove !== undefined) {
                rows.splice(step.remove, step.count);
            } else if (step.move !== undefined) {
                rows.splice(step.to, 0, ...rows.splice(step.move, 1));
            } else {
                rows.splice(step.insert, 0, ...step.keys);
                drawn.push(...step.keys);
            }
        }
        return { rows: rows.join(" "), drawn: drawn.join(" ") };
    }

    function run() {
        // [what, the rows held, the rows listed now, what's drawn afresh]
        const cases = [
            ["the same rows take no step", "a b c", "a b c", ""],
            ["a letter narrowing keeps the rows between", "a b c d e", "b d", ""],
            ["rows moving keep what they drew", "a b c d", "d b a c", ""],
            ["only a row new to the list is drawn", "a b c", "c x a", "x"],
            ["a first list draws every row", "", "a b", "a b"],
            ["and nothing listed leaves nothing", "a b c", "", ""],
            ["a key listed twice is two rows", "a b a", "a a", ""],
            ["and a second of it is drawn", "a b", "b a a", "a"],
            ["a list wholly new draws each row", "a b", "c d", "c d"]
        ];
        for (const [what, was, now, drawn] of cases) {
            const words = said => said.split(" ").filter(word => word !== "");
            Expect.same(what, done(words(was), words(now)), { rows: now, drawn });
        }
        Expect.same("the same rows take no step at all", RowsKept.steps(["a", "b"], ["a", "b"]),
            []);
        // A laying's rows are tallied by kind as each is laid, in place, so
        // a dear laying says whose rows it spent on.
        const tally = RowsKept.begunLaying(0).kinds;
        RowsKept.tallied(tally, "agent", 30.5);
        RowsKept.tallied(tally, "run", 2);
        RowsKept.tallied(tally, "agent", 59.5);
        Expect.same("a tally counts each kind's rows and their time", tally,
            { agent: { rows: 2, ms: 90 }, run: { rows: 1, ms: 2 } });
        Expect.same("a row begun in the laying took from its beginning",
            RowsKept.ownTime(5, 8, 3), 3);
        Expect.same("a row begun before the laying's last row took only since it",
            RowsKept.ownTime(1, 8, 6), 2);
        const ownNamed = RowsKept.begunLaying(0).kinds;
        RowsKept.tallied(ownNamed, "constructor", 4);
        Expect.same("a kind named as one of Object's own is tallied as any other",
            ownNamed.constructor, { rows: 1, ms: 4 });
        // A laying begun at 10 gives a row made ahead, from 4, only what it
        // took since; the next, begun at 11, only what it took since the
        // first was done at 13; and one finished in a part begun at 20,
        // none of the placing between, which the laying leaves out.
        const replayed = RowsKept.begunLaying(10);
        RowsKept.rowLaid(replayed, "run", 4, 13);
        RowsKept.rowLaid(replayed, "agent", 11, 15);
        RowsKept.restarted(replayed, 20);
        RowsKept.rowLaid(replayed, "run", 12, 22);
        Expect.same("a laying gives each row only its own time, part by part", replayed,
            { kinds: { run: { rows: 2, ms: 5 }, agent: { rows: 1, ms: 2 } }, since: 22 });
        // A tally says whose rows it spent on, rounded, most first then by
        // name, and how many of each it laid. Kinds that round alike go by
        // name, whatever their unrounded time: "on" took longer than "AI
        // spend" before rounding. The kinds are the providers' own, a word or
        // two each, as Joel's journal will say them.
        const dearKinds = { "AI spend": { rows: 2, ms: 20.2 }, program: { rows: 3, ms: 89.6 },
            on: { rows: 5, ms: 20.4 }, act: { rows: 4, ms: 0.4 } };
        Expect.same("a laying says whose rows it spent on, most first",
            RowsKept.saidByKind(dearKinds), ', by kind "program" 3 rows in 90 and'
                + ' "AI spend" 2 rows in 20 and "on" 5 rows in 20');
        Expect.same("a laying no kind spent a millisecond on says none",
            [RowsKept.saidByKind({ act: { rows: 9, ms: 0.49 } }), RowsKept.saidByKind(null)],
            ["", ""]);
        Expect.same("a kind's half a millisecond rounds to one, and is said",
            RowsKept.saidByKind({ act: { rows: 1, ms: 0.5 } }), ', by kind "act" 1 row in 1');
        // Each row held keeps its mark wherever the steps move it, and a
        // row put in is given the next, so a row reads what it shows by its
        // mark, which no other row's coming or going changes: read by its
        // place, each row a removal or a move shifted read another's, then
        // its own again, every binding on it run twice (the keys lab's
        // "co", 2026-11-15).
        let given = 10;
        const marking = RowsKept.marked([1, 2, 3, 4, 5],
            RowsKept.steps("a b c d e".split(" "), "d x b a y z".split(" ")), () => given++);
        Expect.same("each row keeps its mark, and a row put in is given the next", marking,
            { marks: [4, 10, 2, 1, 11, 12], given: [[10], [11, 12]] });
        Expect.same("a list put in afresh is given a mark a row",
            RowsKept.marked([], RowsKept.steps([], ["a", "b"]), () => given++).marks, [13, 14]);
        // [what, the rows held, the rows listed now, the steps]
        const moves = [
            ["a row falling to the end is one move", "a b c d e", "b c d e a",
                [{ move: 0, to: 4 }]],
            ["a row rising to the top is one move", "a b c d e", "e a b c d",
                [{ move: 4, to: 0 }]],
            ["a row falling partway is one move", "a b c d e", "b c d a e",
                [{ move: 0, to: 3 }]],
            ["rows falling past each other move once each", "a b c d e", "c d e b a",
                [{ move: 0, to: 4 }, { move: 0, to: 3 }]]
        ];
        for (const [what, was, now, steps] of moves) {
            Expect.same(what, RowsKept.steps(was.split(" "), now.split(" ")), steps);
        }
        const many = "a b c d e f g h".split(" ");
        const swapped = ["h", ...many.slice(1, 7), "a"];
        Expect.same("two rows swapping ends move no row between",
            RowsKept.steps(many, swapped).length, 2);
        Expect.same("and end where they're listed", done(many, swapped),
            { rows: swapped.join(" "), drawn: "" });
        Expect.same("rows new to the list go in as one step a run",
            RowsKept.steps("a b".split(" "), "x y a z b w v".split(" ")),
            [{ insert: 0, keys: ["x", "y"] }, { insert: 3, keys: ["z"] },
                { insert: 5, keys: ["w", "v"] }]);
        // A step into Emoji lists 1,924 rows the list didn't hold, which
        // went in one step each, each looked for among those held first.
        const emoji = Array.from({ length: 1924 }, (_, at) => `emoji:${at}`);
        Expect.same("a first list of many is one step", RowsKept.steps([], emoji),
            [{ insert: 0, keys: emoji }]);
        // "f" in Emoji ranks 838 of its rows, best first, which moved 550 of
        // them one at a time, each move reading the rows in view again.
        const narrowed = emoji.filter((_, at) => at % 16 < 7);
        const ranked = narrowed.map((key, at) => [key, (at * 7919) % narrowed.length])
            .sort((one, other) => one[1] - other[1])
            .map(([key]) => key);
        Expect.same("a reorder moving more rows than the card shows is listed afresh",
            RowsKept.steps(emoji, ranked, 8),
            [{ remove: 0, count: emoji.length }, { insert: 0, keys: ranked }]);
        Expect.same("and one moving no more keeps its rows",
            RowsKept.steps("a b c d e".split(" "), "e a b c d".split(" "), 1),
            [{ move: 4, to: 0 }]);
        Expect.same("and narrowing moves none, however many go",
            RowsKept.steps(emoji, narrowed, 8).filter(step =>
                step.move !== undefined || step.insert !== undefined), []);
        // A backspace widens what's listed again, which put each run of rows
        // between those held in one step, each shifting the rows after it.
        Expect.same("a widening putting in more runs than the card shows is listed afresh",
            RowsKept.steps(narrowed, emoji, 8),
            [{ remove: 0, count: narrowed.length }, { insert: 0, keys: emoji }]);
        Expect.same("and one putting in no more keeps its rows",
            RowsKept.steps("a b c".split(" "), "a x b y c".split(" "), 2),
            [{ insert: 1, keys: ["x"] }, { insert: 3, keys: ["y"] }]);
        // Rows all put in again stand from the list's start, so the launcher
        // puts the selection back in view; moved or narrowed, they stay put.
        Expect.same("rows listed afresh are said to be",
            RowsKept.whole(emoji, RowsKept.steps(emoji, ranked, 8)), true);
        Expect.same("and rows moved are not",
            RowsKept.whole("a b c d e".split(" "),
                RowsKept.steps("a b c d e".split(" "), "e a b c d".split(" "), 1)), false);
        Expect.same("nor rows narrowed",
            RowsKept.whole(emoji, RowsKept.steps(emoji, narrowed, 8)), false);
        Expect.same("nor a first list", RowsKept.whole([], RowsKept.steps([], emoji)), false);

        // A backspace back to a place's whole list lists the rows it listed
        // before, row for row, so it's that list again, and what each key
        // works out over every row of it is worked out once.
        const rowsOf = keys => keys.map(key => ({ key }));
        const whole = rowsOf(emoji);
        const first = rowsOf(narrowed);
        let lately = RowsKept.lately(RowsKept.lately([], whole), first);
        const back = whole.slice();
        Expect.same("a list listed lately is listed again",
            RowsKept.again(lately, back, first) === whole, true);
        Expect.same("but never as the list shown now",
            RowsKept.again(lately, whole.slice(), whole) === whole, false);
        const other = whole.slice();
        other[5] = { key: other[5].key };
        Expect.same("and a row made again makes it another list",
            RowsKept.again(lately, other, first) === other, true);
        let made = 0;
        const counting = () => {
            made += 1;
            return whole.map(row => row.key);
        };
        RowsKept.said(lately, whole, "keys", counting);
        lately = RowsKept.lately(lately, whole);
        Expect.same("what's worked out of a list listed again is kept",
            RowsKept.said(lately, whole, "keys", counting) === RowsKept.said(lately, whole,
                "keys", counting) && made === 1, true);
        Expect.same("and of a list not listed lately is worked out each time",
            RowsKept.said(lately, other, "keys", counting) !== RowsKept.said(lately, other,
                "keys", counting), true);
        for (let more = 0; more < 8; more += 1) {
            lately = RowsKept.lately(lately, rowsOf([`${more}`]));
        }
        Expect.same("only a few lists are kept", RowsKept.again(lately, back, first) === whole,
            false);

        // A list a place is kept on stays, however many come after it, as
        // warming keeps Emoji and then four more lists before its chord.
        lately = RowsKept.lately(RowsKept.lately([], first), whole, [whole]);
        RowsKept.said(lately, whole, "keys", counting);
        made = 0;
        for (let more = 0; more < 8; more += 1) {
            lately = RowsKept.lately(lately, rowsOf([`${more}`]), [whole]);
        }
        RowsKept.said(lately, whole, "keys", counting);
        Expect.same("a kept place's list is kept past the few newest", made, 0);
        Expect.same("and only it, beside the few newest", lately.length, 5);
        lately = RowsKept.lately(lately, rowsOf(["after"]), []);
        Expect.same("until no place is kept on it", lately.some(one => one.rows === whole),
            false);

        // Keys named as Object's own are rows like any other.
        const own = ["__proto__", "constructor", "toString", "hasOwnProperty", "0"];
        Expect.same("keys named as Object's own end where they're listed",
            done(own, own.slice().reverse().concat(["valueOf"])),
            { rows: own.slice().reverse().concat(["valueOf"]).join(" "), drawn: "valueOf" });
        Expect.same("and repeat as any key does",
            done(["__proto__", "a"], ["a", "__proto__", "__proto__"]),
            { rows: "a __proto__ __proto__", drawn: "__proto__" });

        // Working Emoji's steps out costs a few walks over its rows, never a
        // walk for each row: a Map or a Set in Qt's engine looks a key up by
        // walking every key it holds, which took Emoji's first letter 26 to
        // 39 ms and a narrowing 67 to 104, against a walk's 0.5 (2026-11-11). Each is held to a
        // multiple of one walk making and reading an object of its keys,
        // the fastest of five batches each, since a slower one is the
        // builder's other work or the collector.
        const fastest = work => {
            let least = Infinity;
            for (let batch = 0; batch < 5; batch += 1) {
                const began = Date.now();
                for (let round = 0; round < 10; round += 1) {
                    work();
                }
                least = Math.min(least, (Date.now() - began) / 10);
            }
            return least;
        };
        const walk = fastest(() => {
            const places = Object.create(null);
            emoji.forEach((key, at) => { places[key] = at; });
            return ranked.filter(key => places[key] !== undefined).length;
        });
        for (const [what, was, now] of [["a first letter", emoji, ranked],
            ["a narrowing", emoji, narrowed], ["a narrowing of a ranking", ranked,
                ranked.filter((_, at) => at % 11 === 0)]]) {
            const took = fastest(() => RowsKept.steps(was, now, 8));
            console.info(`${what} took ${took} ms against a walk's ${walk}`);
            Expect.same(`${what} takes ${took} ms, at most ten walks of ${walk} ms`,
                took <= 10 * walk + 1, true);
        }

        // Lists drawn from a seeded sequence (Park and Miller's), the same
        // every run, each ending where it's listed and drawing only the
        // rows it didn't hold.
        let seed = 7;
        const next = most => {
            seed = (seed * 48271) % 2147483647;
            return seed % most;
        };
        const keys = "a b c d e f g h i j".split(" ");
        const pickedFrom = () => Array.from({ length: next(9) }, () => keys[next(keys.length)]);
        for (let each = 0; each < 500; each += 1) {
            const was = pickedFrom();
            const now = pickedFrom();
            const ended = done(was, now);
            const held = was.slice();
            const fresh = now.filter(key => {
                const at = held.indexOf(key);
                if (at >= 0) {
                    held.splice(at, 1);
                }
                return at < 0;
            });
            Expect.same(`${was.join("")} to ${now.join("")} ends where it's listed`,
                ended.rows, now.join(" "));
            Expect.same(`${was.join("")} to ${now.join("")} draws only rows it didn't hold`,
                ended.drawn.split(" ").filter(key => key !== "").sort(), fresh.sort());
            // Held to as many moving rows, it ends where it's listed too,
            // and listed afresh or moving no more than it was held to.
            const most = next(3);
            const steps = RowsKept.steps(was, now, most);
            // A first list is put in as one run, as afresh puts it.
            const afresh = was.length === 0
                || steps.some(step => step.remove === 0 && step.count === was.length);
            Expect.same(`${was.join("")} to ${now.join("")} past ${most} ends where it's listed`,
                done(was, now, most).rows, now.join(" "));
            Expect.same(`${was.join("")} to ${now.join("")} moves no more than ${most}`,
                afresh || steps.filter(step => step.move !== undefined).length <= most, true);
            Expect.same(`${was.join("")} to ${now.join("")} puts in no more runs than ${most}`,
                afresh || steps.filter(step => step.insert !== undefined).length <= most, true);
        }

        splitting();
        Qt.exit(0);
    }

    // A step back out to a row well down a list put in afresh splits it at
    // the first row of the view it lands in, and only then (Launcher.qml's
    // keep): rows ten tall, or a heading of twenty first.
    function splitting() {
        const even = () => 10;
        const headed = at => at === 0 ? 20 : 10;
        // [what, the row back to, how far down the view stands from the first
        // row's top, how tall each is, from]
        const cases = [
            ["a view standing on a row's top starts at it", 16, 100, even, 10],
            ["a row cut by the view's top is its first", 16, 105, even, 10],
            ["the row back to, standing at the top, is the first", 16, 160, even, 16],
            ["a heading's height is counted", 5, 25, headed, 1],
            ["a view at the list's top isn't split", 4, 0, even, 0],
            ["nor a step back to the first row", 0, 0, even, 0],
            ["nor one onto the line", -1, 0, even, 0]
        ];
        for (const [what, back, down, tall, from] of cases)
            Expect.same(what, RowsKept.splitFrom(back, down, tall), from);
        const whole = count => [{ insert: 0, keys: Array.from({ length: count }, (_, at) => at) }];
        // The steps putting rows in, as keep() takes them from steps().
        const putting = (was, now) => RowsKept.steps(was, now, 8)
            .filter(one => one.remove === undefined);
        // [what, the steps putting rows in, how many rows the list holds, afresh]
        const puts = [
            ["one run of all the rows is the list afresh", whole(30), 30, true],
            ["a run short of the list isn't", whole(20), 30, false],
            ["nor a row moved alone", [{ move: 2, to: 0 }], 3, false],
            ["rows into an empty list are", putting([], ["a", "b", "c"]), 3, true],
            ["so are rows sharing none with those they replace",
                putting(["x", "y"], ["a", "b", "c"]), 3, true],
            ["nor rows keeping one of those before, put in around it",
                putting(["b"], ["a", "b", "c"]), 3, false],
            ["but the same rows turned round past most moving are",
                putting("abcdefghijk".split(""), "kjihgfedcba".split("")), 11, true]
        ];
        for (const [what, steps, count, afresh] of puts)
            Expect.same(what, RowsKept.allAfresh(steps, count), afresh);
    }
}

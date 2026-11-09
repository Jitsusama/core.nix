// How a long name slides, how words too long for their room give way, and
// how the field's path shares its room, a folder's path giving way first,
// run in Qt's own JavaScript engine, the one the shell runs it in. The `a-long-name-reads-at-one-pace` check copies
// Reading.js beside this file and runs `qml -platform offscreen reading.qml`.
import QtQuick
import "Reading.js" as Reading

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

    // Close enough, for sums of fractions.
    readonly property real near: 1e-9

    function run() {
        const failures = [];
        const expect = (name, ok, said) => {
            if (!ok)
                failures.push(`${name}: ${JSON.stringify(said)}`);
        };
        // A character twelve pixels across, read twelve a second.
        const pace = 144;
        for (const over of [12, 48, 600, 2400]) {
            const { seconds, curve } = Reading.slide(over, pace);
            const points = [[0, 0]];
            for (let at = 0; at < curve.length; at += 2)
                points.push([curve[at], curve[at + 1]]);
            // Each piece is a start, two handles and an end, as BezierSpline
            // reads them, and the last ends where the slide does.
            expect(`${over} px: three pieces`, curve.length === 18, curve);
            expect(`${over} px: ends at its end`,
                Math.abs(curve[16] - 1) < near && Math.abs(curve[17] - 1) < near, curve);
            // Time and ground only go forward, so the name never steps back.
            expect(`${over} px: only forward`,
                points.every((point, at) => at === 0
                    || (point[0] >= points[at - 1][0] - near
                        && point[1] >= points[at - 1][1] - near)), points);
            // It starts and stops at rest: flat at each end.
            expect(`${over} px: starts at rest`, points[1][1] === 0, points);
            expect(`${over} px: stops at rest`, Math.abs(points[8][1] - 1) < near, points);
            // Between the ramps it holds the pace, in pixels a second, and
            // each ramp meets it at that speed.
            const speed = (from, to) => (points[to][1] - points[from][1]) * over
                / ((points[to][0] - points[from][0]) * seconds);
            expect(`${over} px: holds the pace`, Math.abs(speed(3, 6) - pace) < 1e-6,
                speed(3, 6));
            expect(`${over} px: meets the pace speeding up`,
                Math.abs(speed(2, 3) - pace) < 1e-6, speed(2, 3));
            expect(`${over} px: meets the pace slowing`,
                Math.abs(speed(6, 7) - pace) < 1e-6, speed(6, 7));
            // A ramp's time is at most a fifth of a second.
            expect(`${over} px: short ramps`, points[3][0] * seconds <= 0.2 + near, points);
        }
        // A long slide takes the pace's time and one ramp more.
        expect("600 px at its pace", Math.abs(Reading.slide(600, pace).seconds - (600 / pace + 0.2))
            < near, Reading.slide(600, pace));
        // Nothing to slide takes no time.
        expect("nothing over", Reading.slide(0, pace).seconds === 0, Reading.slide(0, pace));
        // Words too long for their room give way at a whole word, in a
        // face ten pixels a character, as the field's monospace is.
        const width = words => words.length * 10;
        const cut = (words, room) => Reading.wholeWords(words, room, width);
        const missed = "Open-Meteo didn't answer";
        expect("whole where they fit", cut(missed, 240) === missed, cut(missed, 240));
        // "Open-Meteo didn't a\u2026" is what a cut by the letter left.
        expect("a word short", cut(missed, 230) === "Open-Meteo didn't\u2026", cut(missed, 230));
        expect("two words short", cut(missed, 170) === "Open-Meteo\u2026", cut(missed, 170));
        expect("not even a word", cut(missed, 100) === "", cut(missed, 100));
        // A separator left at the end goes before the ellipsis.
        const where = "Tokyo, Japan \u00b7 Open-Meteo";
        expect("no separator dangling", cut(where, 200) === "Tokyo, Japan\u2026", cut(where, 200));
        expect("no comma dangling", cut(where, 120) === "Tokyo\u2026", cut(where, 120));
        // A note gives way by whole words from its end with no ellipsis,
        // a separator left at its end going too.
        const line = "line 7 of the deploy, went by";
        const noteIn = (words, room) => Reading.wordsFromEnd(words, room, width);
        expect("a note whole where it fits", noteIn(line, 290) === line, noteIn(line, 290));
        expect("a note a word short", noteIn(line, 250) === "line 7 of the deploy",
            noteIn(line, 250));
        expect("a note to its first word", noteIn(line, 40) === "line", noteIn(line, 40));
        expect("a note with no room", noteIn(line, 30) === "", noteIn(line, 30));
        // A note of items gives way by whole items from its end, never
        // inside one (VKCF3Z, 2026-11-07: never "11.9 Mb/s\u2026").
        const note = "Example ISP \u00b7 370 Mb/s down \u00b7 11.9 Mb/s up";
        const noted = (words, room) => Reading.wholeItemsFromEnd(words, room, width);
        expect("a note whole where it fits", noted(note, 420) === note, noted(note, 420));
        expect("a note's last item gives way whole",
            noted(note, 410) === "Example ISP \u00b7 370 Mb/s down", noted(note, 410));
        expect("a note's first item alone", noted(note, 200) === "Example ISP", noted(note, 200));
        // And no ellipsis on a note: its first item, where it alone
        // doesn't fit, isn't said (VKCF3Z, 2026-11-08: never "found by\u2026").
        expect("its first item whole or not at all", noted(note, 100) === "", noted(note, 100));
        // A sentence is no list of items: it gives way at a whole word.
        expect("a sentence at a whole word", noted("Example ISP down", 100) === "Example\u2026",
            noted("Example ISP down", 100));
        // A run of names gives way by whole names from its end, ending on
        // the last that fits, never an ellipsis (VKCF3Z, 2026-11-08).
        const run = "kitty, 1Password, btop++, Calendar";
        const ran = (words, room) => Reading.wholeNamesFromEnd(words, room, width);
        expect("a run whole where it fits", ran(run, 340) === run, ran(run, 340));
        expect("a run's last name gives way whole", ran(run, 330) === "kitty, 1Password, btop++",
            ran(run, 330));
        expect("a run's first name alone", ran(run, 120) === "kitty", ran(run, 120));
        expect("a name whole or not at all", ran(run, 40) === "", ran(run, 40));
        // Words with no run in them give way as a note's items do.
        expect("a note in a run's place", ran(note, 410) === noted(note, 410), ran(note, 410));
        // A note whose first item doesn't fit says nothing, never a piece
        // of it with an ellipsis.
        expect("a note's first item whole or not at all", noted(note, 100) === "",
            noted(note, 100));
        // Only a run of names is parted at its commas: any other words
        // keep theirs, as a side's "370 Mb/s down, 20 up" does.
        const side = "370 Mb/s down, 20 up";
        expect("a comma in a note parts nothing", noted(side, 200) === side, noted(side, 200));
        expect("and gives way at a whole word", noted(side, 150) === "370 Mb/s down\u2026",
            noted(side, 150));
        // The path's steps: the program's ask, a folder's path, the place,
        // a step's mark between each, three characters wide. Each gives
        // way at whole names, never a piece of one (VKCF3Z, 2026-11-05:
        // "~\u2026e" and "~/gath\u2026g/more").
        const same = (one, other) => JSON.stringify(one) === JSON.stringify(other);
        const asked = [{ text: "Open the Notes", yields: false },
            { text: "~/src/core.nix/notes", yields: true }, { text: "Gathered", yields: false }];
        const crumbs = (steps, room) => Reading.crumbs(steps, room, width, 30);
        const said = (steps, room) => crumbs(steps, room).said;
        expect("steps whole where they fit", same(said(asked, 480),
            ["Open the Notes", "~/src/core.nix/notes", "Gathered"]), said(asked, 480));
        // A folder's path gives way first, its middle folders to one mark,
        // then all but its last, then all of it.
        expect("the path's middle first", same(said(asked, 400),
            ["Open the Notes", "~/\u2026/notes", "Gathered"]), said(asked, 400));
        expect("then all but its last", same(said(asked, 360),
            ["Open the Notes", "\u2026/notes", "Gathered"]), said(asked, 360));
        expect("then the whole path", same(said(asked, 320),
            ["Open the Notes", "\u2026", "Gathered"]), said(asked, 320));
        // Only then is the first step said in fewer words, and where not a
        // word of it fits, it joins the mark; where he is stays whole.
        expect("then the ask's last words", same(said(asked, 250),
            ["Open the\u2026", "\u2026", "Gathered"]), said(asked, 250));
        expect("then the ask in the mark", same(said(asked, 120), ["\u2026", "Gathered"]),
            said(asked, 120));
        expect("where he is whole", same(said(asked, 20), ["\u2026", "Gathered"]),
            said(asked, 20));
        // A path of one folder below home has no middle to give.
        const home = [{ text: "Open the Notes", yields: false },
            { text: "~/gathering", yields: true }, { text: "Gathered", yields: false }];
        expect("a short path whole or not at all", same(said(home, 300),
            ["Open the Notes", "\u2026", "Gathered"]), said(home, 300));
        // A folder's path where he is keeps its last folder's name, the
        // ask giving way before it does.
        const there = [{ text: "Open the Notes", yields: false },
            { text: "~/src/marking", yields: true }];
        expect("where he is keeps its folder", same(said(there, 260),
            ["Open the Notes", "\u2026/marking"]), said(there, 260));
        expect("the ask gives way before it", same(said(there, 210),
            ["Open the\u2026", "\u2026/marking"]), said(there, 210));
        expect("and joins the mark, the folder kept", same(said(there, 20),
            ["\u2026", "\u2026/marking"]), said(there, 20));
        // Steps between the first and the last fold into one mark, those
        // nearest the first going first.
        const deep = ["Files", "Documents", "Taxes 2025", "Receipts"]
            .map(text => ({ text, yields: false }));
        expect("the second step folds first", same(said(deep, 330),
            ["Files", "\u2026", "Taxes 2025", "Receipts"]), said(deep, 330));
        expect("then the next folds in with it", same(said(deep, 300),
            ["Files", "\u2026", "Receipts"]), said(deep, 300));
        expect("what's whole, step by step", same(crumbs(deep, 330).whole,
            [true, false, true, true]), crumbs(deep, 330).whole);
        expect("no steps, nothing", same(said([], 100), []), said([], 100));
        // Every text the path measures is listed before, so a surface can
        // draw each to measure it as it's drawn.
        const unlisted = [];
        for (const steps of [asked, home, there, deep]) {
            const listed = Reading.crumbTexts(steps);
            const noting = text => {
                if (!listed.includes(text) && !unlisted.includes(text))
                    unlisted.push(text);
                return width(text);
            };
            Reading.crumbsLeast(steps, noting, 30);
            for (let room = 0; room <= 500; room += 5)
                Reading.crumbs(steps, room, noting, 30);
        }
        expect("every text a path measures is listed", unlisted.length === 0,
            JSON.stringify(unlisted));
        // A side of items gives way whole, the least needed first, in the
        // order they're said, the one needed most kept however little room
        // (VKCF3Z, 2026-11-04: the tree, the address, then the state).
        const items = [{ said: "TESTQD", keep: 2 }, { said: "working", keep: 3 },
            { said: "$5.10 today", keep: 4 }, { said: "testqd-tree", keep: 0 }];
        const kept = room => Reading.wholeItems(items, room, width);
        const all = "TESTQD \u00b7 working \u00b7 $5.10 today \u00b7 testqd-tree";
        expect("items whole where they fit", kept(450) === all, kept(450));
        expect("the tree first", kept(430) === "TESTQD \u00b7 working \u00b7 $5.10 today",
            kept(430));
        expect("then the address", kept(290) === "working \u00b7 $5.10 today", kept(290));
        expect("then the state", kept(200) === "$5.10 today", kept(200));
        // The one needed most is said whole even where it doesn't fit: a
        // name gives way to it then, never a piece of it.
        expect("the cost whole", kept(50) === "$5.10 today", kept(50));
        expect("no items, nothing", Reading.wholeItems([], 100, width) === "",
            Reading.wholeItems([], 100, width));
        // An item naming a path, once the items it outlasts have gone
        // whole, gives it way from its middle, its last folder whole
        // (VKCF3Z, 2026-11-08).
        const shots = "~/Pictures/Screenshots";
        const pathed = [{ said: `in ${shots}`, keep: 2, path: shots },
            { said: "screenshot", keep: 1 }];
        const eyebrow = room => Reading.wholeItems(pathed, room, width);
        expect("a path whole where it fits", eyebrow(380) === `in ${shots} \u00b7 screenshot`,
            eyebrow(380));
        expect("the kind first", eyebrow(370) === `in ${shots}`, eyebrow(370));
        expect("then the path from its middle", eyebrow(200) === "in ~/\u2026/Screenshots",
            eyebrow(200));
        expect("then down to its last folder", eyebrow(170) === "in \u2026/Screenshots",
            eyebrow(170));
        expect("which is never lost", eyebrow(50) === "in \u2026/Screenshots", eyebrow(50));
        // A drawer's eyebrow given as words, not items, gives way as a
        // note does, by whole items from its end, never cut inside a word
        // as "IN ~/PICTURES/SCREENS\u2026" was (VKCF3Z, 2026-11-08); one
        // given as items, as they say.
        const light = { eyebrow: "on USB \u00b7 desk light" };
        const brow = (given, room) => Reading.eyebrowOf(given, room, width);
        expect("an eyebrow whole where it fits", brow(light, 300) === light.eyebrow,
            brow(light, 300));
        expect("an eyebrow's last item gives way whole", brow(light, 150) === "on USB",
            brow(light, 150));
        expect("one given as items gives way as they say",
            brow({ eyebrow: "", eyebrowItems: pathed }, 200) === "in ~/\u2026/Screenshots",
            brow({ eyebrow: "", eyebrowItems: pathed }, 200));
        expect("no eyebrow, nothing", brow(null, 300) === "", brow(null, 300));
        // An item naming a file gives it way from its middle too, its start
        // and its extension kept, a letter at a time (VKCF3Z, 2026-11-08).
        const filed = [{ said: "Nightwish", keep: 1 },
            { said: "second-take.mp3", keep: 2, file: "second-take.mp3" }];
        const filedIn = room => Reading.wholeItems(filed, room, width);
        expect("a file whole where it fits",
            filedIn(270) === "Nightwish \u00b7 second-take.mp3", filedIn(270));
        expect("the artist first", filedIn(200) === "second-take.mp3", filedIn(200));
        expect("then the file from its middle", filedIn(120) === "seco\u2026ake.mp3",
            filedIn(120));
        expect("down to a letter and its extension", filedIn(30) === "s\u2026.mp3",
            filedIn(30));
        // Styled, the kept items keep their words and take a dot in the
        // colour given between them, so a selected agent row reads
        // "working \u00b7 $4.20" with its dot muted (VKCF3Z, 2026-11-06), and
        // a word that reads as markup is written as it reads. The colour
        // is a stand-in, as the theme's role is given it in a row.
        const styled = Reading.styledItems(Reading.keptItems(items, 290, width), "dots");
        expect("kept items, styled, a muted dot between",
            styled === "working <font color=\"dots\">\u00b7</font> $5.10 today", styled);
        const marked = Reading.styledItems(["a<b", "c & d"], "dots");
        expect("markup in the words written as it reads",
            marked === "a&lt;b <font color=\"dots\">\u00b7</font> c &amp; d", marked);
        // An item given an ink of its own is drawn in it, the rest in the
        // side's, so Reopen Them All's "1 folder gone" reads in the alert's
        // colour and its "1 of 2" doesn't (VKCF3Z, 2026-11-07).
        const inked = Reading.styledItems(["1 of 2", "1 folder gone"], "dots", ["", "alert"]);
        expect("an item in its own ink", inked === "1 of 2 <font color=\"dots\">\u00b7</font> "
            + "<font color=\"alert\">1 folder gone</font>", inked);
        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            Qt.exit(1);
            return;
        }
        Qt.exit(0);
    }
}

// How an open launcher paces the refreshes its sources ask for, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `the-launcher-paces-its-refreshes` check copies
// modules/quickshell/launcher/Pacing.js beside this file and runs
// `qml -platform offscreen pacing.qml`, with launcher/Wanted.js. A burst of
// changes is taken a pace apart, and never two refreshes closer than that,
// however the first was asked for; a source's rows are wanted only where
// they're shown or searched, so a change nobody sees asks none; a key's
// act is said shown by the first frame begun after its refresh; and an
// opening says its parts slowest first, and weighs the foot's verbs only
// once the words or the face have changed, and each row's keys once for
// what's typed, the steps and the mode; an opening's stall is said by the
// work it ran into, each frame's rows laid out and its sync apart, and its
// sources' waking counts home found after its fill as a slice, which only
// home's fill leaves to one, searching home again but for a chord. What
// was presented is said by
// place, each change timed from the earliest line not yet drawn
// (launcher/Presented.js). A closing makes again only the spots of the
// things whose stamp moved (launcher/Spots.js). y's flash starts its
// time a turn after the frame that draws it, only for the newest yank
// (launcher/Flash.js). An opening the collector stalled says where the
// heap stood at the ask its felt record began with (launcher/Collector.js).
// Long because its cases
// share one check's qml run, so each part split from it would need a check
// and a run of its own.
import QtQuick
import "Collector.js" as Collector
import "Flash.js" as Flash
import "Makings.js" as Makings
import "Pacing.js" as Pacing
import "Presented.js" as Presented
import "Spots.js" as Spots
import "Thread.js" as Thread
import "Wanted.js" as Wanted

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

    function check(failures, what, got, want) {
        if (got !== want)
            failures.push(`${what}: got ${got}, wanted ${want}`);
    }

    // An opening says what its own work went on, slowest first, so the
    // line Joel's slow opening left names its cause (2026-11-10).
    function opening(failures) {
        check(failures, "the parts are said slowest first",
            Pacing.openingParts({ joining: 14.2, finding: 30.6, foot: 3 }),
            "finding 31, joining 14, foot 3");
        check(failures, "a part under a millisecond is left out",
            Pacing.openingParts({ joining: 5, before: 0.4 }), "joining 5");
        check(failures, "parts as long are said by name",
            Pacing.openingParts({ verbs: 2, foot: 2 }), "foot 2, verbs 2");
        check(failures, "an opening with none says so", Pacing.openingParts({ before: 0 }),
            "none over a millisecond");

        // Each opening walked every thing's verbs to weigh the foot's room.
        let weighings = 0;
        const weigh = words => {
            weighings += 1;
            return words.widest;
        };
        const words = { widest: "connect" };
        const face = {};
        const first = Pacing.verbRoom(null, words, face, weigh);
        check(failures, "the first opening weighs the verbs", first.widest, "connect");
        const again = Pacing.verbRoom(first, words, face, weigh);
        check(failures, "the next keeps what it weighed", again, first);
        check(failures, "so the verbs were weighed once", weighings, 1);
        const reread = { widest: "brighten" };
        check(failures, "words read again are weighed again",
            Pacing.verbRoom(again, reread, face, weigh).widest, "brighten");
        check(failures, "and so are they in another face",
            Pacing.verbRoom(again, words, {}, weigh) === again, false);
        check(failures, "each weighed once more", weighings, 3);
    }

    // One act in For You's 115 rows weighed every row's keys again, where
    // one row had changed (2026-11-10).
    function weighing(failures) {
        let weighed = 0;
        const weigh = row => {
            weighed += 1;
            return { keys: row.name };
        };
        const words = {};
        const face = {};
        const rows = [{ name: "a" }, { name: "b" }, { name: "c" }];
        const first = Pacing.weighing(null, "|find", words, face);
        rows.forEach(row => Pacing.weighedRow(first, row, weigh));
        check(failures, "the first rows are each weighed", weighed, 3);
        const again = Pacing.weighing(first, "|find", words, face);
        check(failures, "the same typed, steps, mode, words and face keep them",
            again.rows === first.seen, true);
        const changed = [rows[0], { name: "B" }, rows[2]];
        const said = changed.map(row => Pacing.weighedRow(again, row, weigh).keys).join("");
        check(failures, "a row changed is weighed, the rest kept", weighed, 4);
        check(failures, "each says its own keys", said, "aBc");
        check(failures, "and the count says so", again.weighed, 4);
        // Home's key is the same across every opening, so what it held of
        // a row replaced was held for the life of the shell.
        const next = Pacing.weighing(again, "|find", words, face);
        check(failures, "the next holds only the rows the last saw", next.rows.size, 3);
        check(failures, "so a row replaced is let go", next.rows.has(rows[1]), false);
        // The selected row is weighed afresh, as it says more, and kept so
        // it isn't weighed again once it's left.
        Pacing.keptRow(next, changed[1]);
        changed.filter(row => row !== changed[1]).forEach(row => Pacing.weighedRow(next, row,
            weigh));
        const left = Pacing.weighing(next, "|find", words, face);
        changed.forEach(row => Pacing.weighedRow(left, row, weigh));
        check(failures, "a row selected, then left, isn't weighed again", weighed, 4);
        for (const [what, kept] of [["typed or a mode", Pacing.weighing(again, "x|find", words,
            face)], ["words", Pacing.weighing(again, "|find", {}, face)],
            ["a face", Pacing.weighing(again, "|find", words, {})]]) {
            Pacing.weighedRow(kept, rows[0], weigh);
            check(failures, `another ${what} weighs again`, kept === again, false);
        }
        check(failures, "once each", weighed, 7);
        // Emoji's 1,924 rows were each weighed as it opened (2026-11-11).
        const named = (alike, picked) => {
            const listed = alike.map((like, at) => ({ name: `${at}`, keysAlike: like }));
            return Pacing.toWeigh(listed, listed[picked] ?? null).map(row => row.name).join(" ");
        };
        const emoji = Array.from({ length: 1924 }, () => "emoji");
        check(failures, "a run of rows alike is weighed once", named(emoji, -1), "0");
        check(failures, "and the selection among them as well", named(emoji, 1000), "0 1000");
        check(failures, "rows not said alike are each weighed", named(["", undefined, ""], -1),
            "0 1 2");
        check(failures, "a run begins again after another row",
            named(["emoji", "emoji", "", "emoji", "copy", "copy", "emoji"], -1), "0 2 3 4 6");
        check(failures, "an empty list weighs none", named([], -1), "");
        // A backspace back to Emoji's whole list weighed every row again,
        // though the list was the one shown before (2026-11-11).
        const mixed = ["emoji", "emoji", "", "emoji", "copy", "copy", "emoji", "emoji"]
            .map((like, at) => ({ name: `${at}`, keysAlike: like }));
        const at = Pacing.weighedAt(mixed);
        // Each row whose kind differs from the one before it, and the
        // selection, walked as the foot first walked them.
        const walked = picked => mixed.filter((row, place) => row === picked
            || row.keysAlike === "" || row.keysAlike !== mixed[place - 1]?.keysAlike);
        const kept = [null].concat(mixed).every(picked => Pacing.weighedWith(mixed, at, picked)
            .map(row => row.name).join(" ") === walked(picked).map(row => row.name).join(" "));
        check(failures, "where rows are weighed is kept whatever is selected", kept, true);
        check(failures, "and the selection goes in its place",
            Pacing.weighedWith(mixed, at, mixed[5]).map(row => row.name).join(" "), "0 2 3 4 5 6");
    }

    // Going home and back weighed every one of Windows' rows again on each
    // kept opening (2026-11-11).
    function weighingPlaces(failures) {
        let weighed = 0;
        const weigh = row => {
            weighed += 1;
            return { keys: row.name };
        };
        const words = {};
        const face = {};
        const made = names => names.map(name => ({ name }));
        const windows = made(["kitty", "chrome", "slack"]);
        const home = made(["windows", "apps"]);
        // Shows `rows` in the place `key`, as the foot weighs them, and
        // gives what's held after.
        const shown = (held, key, rows, place) => {
            const now = Pacing.weighingAmong(held, key, { words, face }, place);
            rows.forEach(row => Pacing.weighedRow(now[0], row, weigh));
            return now;
        };
        let held = shown([], "windows|", windows, true);
        held = shown(held, "|", home, true);
        check(failures, "each place's rows are weighed once", weighed, 5);
        held = shown(held, "windows|", windows, true);
        check(failures, "a place shown again weighs none", weighed, 5);
        const retitled = [windows[0], { name: "chrome, a new tab" }, windows[2]];
        held = shown(held, "|", home, true);
        held = shown(held, "windows|", retitled, true);
        check(failures, "a row that changed is weighed again, alone", weighed, 6);
        check(failures, "and the place lets go of the row it was",
            held[0].seen.has(windows[1]), false);
        held = shown(held, "windows|k", [windows[0]], false);
        held = shown(held, "|", home, true);
        held = shown(held, "windows|k", [windows[0]], false);
        check(failures, "a typed search isn't held past the next", weighed, 8);
        for (const key of ["a|", "b|", "c|"])
            held = shown(held, key, made([key]), true);
        check(failures, "no more than four places are held", held.length, 4);
        held = shown(held, "windows|", retitled, true);
        check(failures, "so one pushed out weighs again", weighed, 14);
    }

    // An opening is usable once the shell's thread answers its ticks on
    // time for a while, not at its first frame: Joel's 15:08 openings
    // logged 437 ms to their frame and showed him the launcher 7 s after
    // his chord (Joel, 2026-11-10).
    function felt(failures) {
        const pace = { every: 16, within: 17, calmFor: 100 };
        const tick = (from, times) => times.reduce((felt, at) => Pacing.feltTicked(felt, at,
            pace), from);
        const calm = tick(Pacing.feltBegun(0), [16, 32, 48, 64, 80, 96, 112]);
        check(failures, "a thread free from its first frame is usable from it", calm.usable, 0);
        const pegged = tick(Pacing.feltBegun(0), [16, 1500, 1516, 1532, 1548, 1564, 1580]);
        check(failures, "a thread held after its frame isn't usable before 100 ms on time",
            pegged.usable, -1);
        const freed = tick(pegged, [1596, 1612]);
        check(failures, "it's usable from when it came free", freed.usable, 1500);
        check(failures, "and its longest stall is said", freed.stall, 1484);
        check(failures, "with when it began", freed.stallAt, 16);
        const jittered = tick(Pacing.feltBegun(0), [30, 46, 62, 78, 94, 110]);
        check(failures, "a tick a frame late isn't a stall", jittered.usable, 0);
        check(failures, "once usable, a later stall changes nothing",
            tick(freed, [5000]).usable, 1500);
        collector(failures, pace);
        threaded(failures, pace);
        stalled(failures);
        framed(failures);
    }

    // What the collector took of an opening's longest stall and of all of
    // it, from its own count at each tick: profiles of Joel's openings
    // found 24 to 64 ms of collecting in each, unnamed (2026-11-14).
    function collector(failures, pace) {
        const ticks = [[16, 2], [32, 2], [90, 32], [106, 33], [122, 40], [138, 40]];
        const felt = ticks.reduce((was, [at, collected]) => Pacing.feltTicked(was, at, pace,
            { collected }), Pacing.feltBegun(0, { collected: 2 }));
        check(failures, "the collector's part of the longest stall is its count across it",
            felt.stallCollected, 30);
        check(failures, "a shorter gap's collecting isn't the stall's",
            Pacing.feltTicked(felt, 160, pace, { collected: 60 }).stallCollected, 30);
        check(failures, "it's said after the stall's parts, with the opening's",
            Collector.collectorSaid(felt.stallCollected, 38.4),
            "; the collector 30 ms of the stall, 38 of the opening");
        check(failures, "an opening the collector didn't reach says nothing of it",
            Collector.collectorSaid(0, 0.4), "");
        check(failures, "an opening's working says what the collector took of it",
            Collector.workingSaid(12.4), "; the collector 12 ms of its working");
        check(failures, "working the collector didn't reach says nothing of it",
            Collector.workingSaid(0.4), "");
        // Read once the room is left, so outside has the room or more and
        // the chunks the room, or more under Qt's own limit, or, as here,
        // less, their cap holding them nearer.
        const stoodAtAsk = { chunks: 300.4, collectsAt: 321.6, outside: 27.2,
            collectsOutsideAt: 59.2, collecting: false };
        check(failures, "a stall the collector ran in says where the heap stood at the ask",
            Collector.standingSaid(30, stoodAtAsk),
            "; at the ask, chunks 300 of 322 MB and outside 27 of 59 MB");
        check(failures, "and says a collection was under way as it was asked",
            Collector.standingSaid(30, Object.assign({}, stoodAtAsk, { collecting: true })),
            "; at the ask, chunks 300 of 322 MB and outside 27 of 59 MB, a collection under way");
        const stoodLater = Object.assign({}, stoodAtAsk, { chunks: 12 });
        const carried = Pacing.feltTicked(Pacing.feltBegun(0, { asked: stoodAtAsk }), 16, pace,
            { asked: stoodLater, collected: 0 });
        // The launcher hands its ticks no reading today; this keeps a
        // tick from taking one in place of the opening's.
        check(failures, "an opening's ticks keep the heap read as it began, not one handed later",
            Collector.standingSaid(30, carried.asked),
            Collector.standingSaid(30, stoodAtAsk));
        check(failures, "and one begun with none read says nothing of the heap",
            Collector.standingSaid(30, Pacing.feltBegun(0).asked), "");
        check(failures, "a stall the collector didn't run in says nothing of the heap",
            Collector.standingSaid(0.4, stoodAtAsk), "");
        check(failures, "nor does one with no standing read",
            Collector.standingSaid(30, null) + Collector.standingSaid(30, {}), "");
        check(failures, "a tick without a count keeps the last",
            Pacing.feltTicked(Pacing.feltBegun(0, { collected: 5 }), 16, pace).collected, 5);
    }

    // What the shell's thread did in an opening's longest stall, from its
    // clocks at each tick: run, wait for a CPU or, for the rest, sleep.
    // Joel's journal called a mean 39 ms of each stall unnamed, and the lab
    // couldn't show it (2026-11-15).
    function threaded(failures, pace) {
        const ticks = [[16, { ran: 5, waited: 1 }], [32, { ran: 10, waited: 2 }],
            [132, { ran: 50.4, waited: 40 }], [148, { ran: 60, waited: 41 }]];
        const felt = ticks.reduce((was, [at, thread]) => Pacing.feltTicked(was, at, pace,
            { thread }), Pacing.feltBegun(0, { thread: { ran: 0, waited: 0 } }));
        check(failures, "the thread's part of the longest stall is its clocks across it",
            JSON.stringify(felt.stallThread), JSON.stringify({ ran: 40.4, waited: 38 }));
        check(failures, "a shorter gap's isn't the stall's",
            Pacing.feltTicked(felt, 164, pace, { thread: { ran: 70, waited: 50 } }).stallThread
                .ran, 40.4);
        check(failures, "a tick without its clocks keeps the last",
            Pacing.feltTicked(felt, 164, pace).thread.ran, 60);
        check(failures, "it's said after the stall's parts, sleeping what neither took",
            Thread.said(felt.stall, felt.stallThread, "the stall"),
            "; its thread ran 40 ms of the stall, waited 38 for a CPU and slept 22");
        check(failures, "clocks read past the stall leave no sleep below nothing",
            Thread.said(20, { ran: 15, waited: 9 }, "the stall"),
            "; its thread ran 15 ms of the stall, waited 9 for a CPU and slept 0");
        check(failures, "an opening that never stalled says nothing of it",
            Thread.said(0, { ran: 0, waited: 0 }, "the stall"), "");
        const paging = [[16, { ran: 5, waited: 1, faulted: 3 }],
            [132, { ran: 20, waited: 2, faulted: 1206 }]];
        const pagedStall = paging.reduce((was, [at, thread]) => Pacing.feltTicked(was, at, pace,
            { thread }), Pacing.feltBegun(0, { thread: { ran: 0, waited: 0, faulted: 0 } }));
        check(failures, "the stall counts its thread's major page faults",
            pagedStall.stallThread.faulted, 1203);
        const unreadStall = [[16, { ran: 5, waited: 1, faulted: 3 }],
            [132, { ran: 20, waited: 2, faulted: -1 }]].reduce((was, [at, thread]) =>
            Pacing.feltTicked(was, at, pace, { thread }),
            Pacing.feltBegun(0, { thread: { ran: 0, waited: 0, faulted: 0 } }));
        check(failures, "a stall whose count wasn't read takes no count",
            unreadStall.stallThread.faulted, undefined);
        check(failures, "and says nothing of them",
            Thread.said(unreadStall.stall, unreadStall.stallThread, "the stall"),
            "; its thread ran 15 ms of the stall, waited 1 for a CPU and slept 100");
        const fellStall = [[16, { ran: 5, waited: 1, faulted: 9 }],
            [132, { ran: 20, waited: 2, faulted: 4 }]].reduce((was, [at, thread]) =>
            Pacing.feltTicked(was, at, pace, { thread }),
            Pacing.feltBegun(0, { thread: { ran: 0, waited: 0, faulted: 0 } }));
        check(failures, "nor one whose count went back", fellStall.stallThread.faulted,
            undefined);
        keyThreaded(failures);
    }

    // What a slow key's thread did while it searched: Joel's "↓" in Agents
    // took 121 ms searching, mid key repeat, with every move around it under
    // 6, and his "ca" waited 62 ms behind the shell's other work, neither
    // saying whether the thread ran, waited for a CPU or slept (2026-11-16).
    function keyThreaded(failures) {
        const spent = Thread.spent({ ran: 1000.25, waited: 40 }, { ran: 1003.5, waited: 48 });
        check(failures, "a key's part is its clocks across its searching",
            JSON.stringify(spent), JSON.stringify({ ran: 3.25, waited: 8 }));
        check(failures, "it's said of the key's searching",
            Thread.said(121, spent, "its searching"),
            "; its thread ran 3 ms of its searching, waited 8 for a CPU and slept 110");
        check(failures, "a key read without clocks says nothing of them",
            Thread.said(121, null, "its searching"), "");
        check(failures, "nor does one whose run clock went back",
            Thread.spent({ ran: 9, waited: 4 }, { ran: 5, waited: 4 }), null);
        check(failures, "nor one whose wait went back",
            Thread.spent({ ran: 5, waited: 9 }, { ran: 9, waited: 4 }), null);
        check(failures, "a key without a reading at its start has none",
            Thread.spent(null, { ran: 5, waited: 4 }), null);
        check(failures, "a wait that failed to read at the start isn't taken as none waited",
            Thread.spent({ ran: 5, waited: 0 }, { ran: 9, waited: 5000 }), null);
        check(failures, "nor one that failed at the end",
            Thread.spent({ ran: 5, waited: 40 }, { ran: 9, waited: 0 }), null);
        keyPaged(failures);
    }

    // The major page faults a key's thread took while it searched: Joel's
    // "s" slept 448 ms of 566 searching, in work that never waits on its
    // event loop, with memory short (2026-11-21).
    function keyPaged(failures) {
        const paged = Thread.spent({ ran: 5, waited: 40, faulted: 120 },
            { ran: 120, waited: 43, faulted: 1323 });
        check(failures, "a key's part counts its thread's major page faults",
            paged.faulted, 1203);
        check(failures, "they're said after its sleep",
            Thread.said(566, paged, "its searching"),
            "; its thread ran 115 ms of its searching, waited 3 for a CPU and slept 448"
                + ", with 1203 major page faults");
        check(failures, "one is said as one",
            Thread.said(20, { ran: 15, waited: 1, faulted: 1 }, "the stall"),
            "; its thread ran 15 ms of the stall, waited 1 for a CPU and slept 4"
                + ", with 1 major page fault");
        check(failures, "a count of none is said as none",
            Thread.said(20, { ran: 15, waited: 1, faulted: 0 }, "the stall"),
            "; its thread ran 15 ms of the stall, waited 1 for a CPU and slept 4"
                + ", with no major page faults");
        check(failures, "a count that wasn't read says nothing of them",
            Thread.said(20, { ran: 15, waited: 1 }, "the stall"),
            "; its thread ran 15 ms of the stall, waited 1 for a CPU and slept 4");
        check(failures, "a count that went back isn't taken as faults",
            Thread.spent({ ran: 5, waited: 4, faulted: 9 }, { ran: 9, waited: 5, faulted: 3 })
                .faulted, undefined);
        check(failures, "nor one that couldn't be read",
            Thread.spent({ ran: 5, waited: 4, faulted: -1 }, { ran: 9, waited: 5, faulted: 3 })
                .faulted, undefined);
    }

    // What held an opening's longest stall, by the work it ran into: Joel's
    // kept openings drew in a median 48 ms and stalled a median 73 ms after,
    // and the journal named only some of it (2026-11-13).
    function stalled(failures) {
        const spans = [{ name: "filling", from: 10, to: 70 },
            { name: "syncing", from: 70, to: 120 }, { name: "waking", from: 120, to: 121 },
            { name: "filling", from: 300, to: 310 }];
        check(failures, "a stall is said by what it ran into, most first",
            Pacing.stallSpent(spans, 20, 170), "filling 50, syncing 50, waking 1, 49 unnamed");
        check(failures, "one that ran into nothing named is all unnamed",
            Pacing.stallSpent(spans, 150, 250), "100 unnamed");
        const nested = [{ name: "filling", from: 10, to: 70 },
            { name: "the drawer", from: 30, to: 40 }];
        check(failures, "work inside other work is its own, not both",
            Pacing.stallSpent(nested, 0, 100), "filling 50, the drawer 10, 40 unnamed");
        check(failures, "one all named says nothing unnamed",
            Pacing.stallSpent(nested, 10, 70), "filling 50, the drawer 10");
        // Some of the open handler's laps beside the slices after its frame,
        // which say "waking" and "joining" for their own work (2026-11-21),
        // in the order the launcher pushes them: the handler's join, felt
        // inside its lap, then the lap, and "opening", begun before the
        // handler, once `open` has turned. A span inside a lap of its name
        // adds into the lap; that the launcher gives its join that name
        // (freshenAs) is no case's.
        const laps = [{ name: Pacing.lapName("beginning"), from: 1, to: 5 },
            { name: Pacing.lapName("waking"), from: 5, to: 25 },
            { name: Pacing.lapName("joining"), from: 26, to: 30 },
            { name: Pacing.lapName("joining"), from: 25, to: 31 },
            { name: "opening", from: 0, to: 40 },
            { name: "waking", from: 60, to: 80 },
            { name: "joining", from: 80, to: 90 }];
        check(failures, "an opening's laps are said apart from the slices after it",
            Pacing.stallSpent(laps, 0, 100), "opening's waking 20, waking 20, opening 10, "
                + "joining 10, opening's joining 6, opening's beginning 4, 30 unnamed");
    }

    // Each frame's own work, as spans: the rows it laid out and its sync,
    // since what Joel's openings called drawing, 23 ms of a stall, held both
    // and whatever the thread ran after the sync (2026-11-14).
    function framed(failures) {
        const said = noted => JSON.stringify(noted.spans);
        let noted = Pacing.feltFramed(Pacing.feltUnframed(), "row", 10, 12);
        noted = Pacing.feltFramed(noted.frame, "row", 14, 18);
        check(failures, "rows laid out are said only once their frame animates",
            said(noted), "[]");
        noted = Pacing.feltFramed(noted.frame, "animating", 20);
        check(failures, "a frame's rows are laid out from the first begun to the last made",
            said(noted), JSON.stringify([{ name: "laying out", from: 10, to: 18 }]));
        noted = Pacing.feltFramed(noted.frame, "synced", 45);
        check(failures, "its sync runs from its animating until the thread runs again",
            said(noted), JSON.stringify([{ name: "syncing", from: 20, to: 45 }]));
        check(failures, "and is said once",
            said(Pacing.feltFramed(noted.frame, "synced", 60)), "[]");
        const bare = Pacing.feltFramed(Pacing.feltUnframed(), "animating", 70);
        check(failures, "a frame that made no rows laid none out", said(bare), "[]");
        const next = Pacing.feltFramed(Pacing.feltFramed(bare.frame, "row", 72, 75).frame,
            "animating", 90);
        check(failures, "a row made after a frame animates is the next frame's",
            said(next), JSON.stringify([{ name: "laying out", from: 72, to: 75 }]));
    }

    // Agents says how its sessions stand at home, and People is named
    // there by who it lists (Joel, 2026-11-08: "weird lag can happen").
    function wanting(failures) {
        const agents = { saysAtHome: true };
        const people = { saysAtHome: false };
        const at = fields => Object.assign({ cataloguing: false, open: true, held: [],
            becoming: [], typing: false, entering: null, viewing: false }, fields);
        const wanted = (fields, source) => Wanted.wanted(at(fields), source);
        check(failures, "closed, no source's rows are wanted", wanted({ open: false }, people),
            false);
        check(failures, "walked from outside, every source's are",
            wanted({ open: false, cataloguing: true }, agents), true);
        check(failures, "at home, a place home names is", wanted({}, people), true);
        check(failures, "and one saying what it holds itself isn't", wanted({}, agents), false);
        check(failures, "what's typed at home searches every place",
            wanted({ typing: true }, agents), true);
        const inAgents = [{ row: { from: agents } }];
        check(failures, "held, its own are", wanted({ held: inAgents }, agents), true);
        check(failures, "and another place's aren't", wanted({ held: inAgents }, people), false);
        check(failures, "nor with something typed there",
            wanted({ held: inAgents, typing: true }, people), false);
        check(failures, "held deeper, in one of its rows, its own still are",
            wanted({ held: inAgents.concat([{ row: { from: null } }]) }, agents), true);
        check(failures, "an act of its own waiting to be agreed keeps them wanted elsewhere",
            wanted({ held: [{ row: { from: people } }], becoming: [{ from: agents }] }, agents),
            true);
        const asleep = { saysAtHome: false, asleep: true };
        check(failures, "as a kept opening fills in, one not yet woken isn't",
            wanted({}, asleep), false);
        check(failures, "what's typed searches every place, woken or not",
            wanted({ typing: true }, asleep), true);
        check(failures, "and one held is wanted though not woken",
            wanted({ held: [{ row: { from: asleep } }] }, asleep), true);
        const now = { saysAtHome: false };
        const read = fields => Wanted.readAt(at(fields), [agents, now, null]);
        check(failures, "rows home shows are read at home", read({}), true);
        check(failures, "and walked from outside", read({ open: false, cataloguing: true }), true);
        check(failures, "but not shut", read({ open: false }), false);
        check(failures, "nor opened straight into another place",
            read({ held: [{ row: { from: people } }] }), false);
        check(failures, "though in a place that reads them they are",
            read({ held: [{ row: { from: now } }] }), true);
        // A source a step holds anew, laying its rows out afresh while
        // unwanted, says as it's held only what the step read of it a moment
        // before, so it isn't made again: Joel's steps into Agents waited 16
        // to 25 ms for it (his journal, 2026-11-19 and 20).
        const afresh = { readsAfreshUnwanted: true };
        const alsoAfresh = { readsAfreshUnwanted: true };
        const keeps = { readsAfreshUnwanted: false };
        check(failures, "a step holds anew a source that laid its rows out afresh",
            Wanted.heldAnew(afresh, []), afresh);
        check(failures, "but not one that kept them unwanted, which says those made at last",
            Wanted.heldAnew(keeps, []), null);
        check(failures, "nor one a step already holds",
            Wanted.heldAnew(afresh, [{ row: { from: afresh } }]), null);
        check(failures, "nor a row from no source", Wanted.heldAnew(undefined, []), null);
        check(failures, "the source a step holds anew said only what the step read",
            Wanted.readByStep(afresh, afresh, undefined), true);
        check(failures, "another source's change as it's held is its own",
            Wanted.readByStep(alsoAfresh, afresh, undefined), false);
        check(failures, "as is the held source's refresh the launcher asks for",
            Wanted.readByStep(afresh, afresh, "a row agreed"), false);
        check(failures, "and any change with no step holding a source anew",
            Wanted.readByStep(afresh, null, undefined), false);
        check(failures, "nor opening into another place before its step is held",
            read({ entering: people }), false);
        check(failures, "though opening into one that reads them they are",
            read({ entering: now }), true);
        // As a kept opening fills in, the reader waits its turn to wake, and
        // reads them then, not in the turn the launcher opened in, nor in
        // the turn another reader wakes in.
        const dozing = { saysAtHome: true, asleep: true };
        const woken = { saysAtHome: true, asleep: false };
        const nowDozing = { saysAtHome: false, asleep: true };
        const readDozing = (fields, readers) => Wanted.readAt(at(fields), readers);
        check(failures, "at home, a reader not yet woken reads none",
            readDozing({}, [dozing, nowDozing]), false);
        check(failures, "not even once another reader home names has woken",
            readDozing({}, [dozing, now]), false);
        check(failures, "but once woken itself it does",
            readDozing({}, [woken, nowDozing]), true);
        check(failures, "as it does when something's typed",
            readDozing({ typing: true }, [dozing, nowDozing]), true);
        check(failures, "and held in it, it reads them though not woken",
            readDozing({ held: [{ row: { from: dozing } }] }, [dozing, nowDozing]), true);
        check(failures, "in another place that reads them, it waits to wake as at home",
            readDozing({ held: [{ row: { from: now } }] }, [dozing, now]), false);
        check(failures, "and reads them there once woken",
            readDozing({ held: [{ row: { from: now } }] }, [woken, now]), true);
        check(failures, "or once something's typed there",
            readDozing({ held: [{ row: { from: now } }], typing: true }, [dozing, now]), true);
        // A kept opening into Now passes through these before Now is held.
        check(failures, "opening into a place that reads them, it waits to wake too",
            readDozing({ entering: now }, [dozing, now]), false);
        check(failures, "and reads them once woken",
            readDozing({ entering: now }, [woken, now]), true);
        // Which sources wait their turn: any home reads, and of those that
        // say what they hold at home, only one reading rows of its own.
        check(failures, "a source home reads waits its turn to wake",
            Wanted.wakesInTurn({ saysAtHome: false }), true);
        check(failures, "one that says what it holds at home doesn't",
            Wanted.wakesInTurn({ saysAtHome: true, readsHome: false }), false);
        check(failures, "unless it reads rows home shows of its own",
            Wanted.wakesInTurn({ saysAtHome: true, readsHome: true }), true);
        const warming = { saysAtHome: false, warming: true };
        check(failures, "shut, one warming for home is", wanted({ open: false }, warming), true);
        check(failures, "and no other", wanted({ open: false }, people), false);
        check(failures, "nor one home doesn't name",
            wanted({ open: false }, { saysAtHome: true, warming: true }), false);
        check(failures, "opening into a place before its step is held, home's aren't",
            wanted({ entering: agents }, people), false);
        check(failures, "and the place's own are", wanted({ entering: agents }, agents), true);
        check(failures, "nor while it's shut",
            wanted({ open: false, entering: agents }, agents), false);
        // Over a viewer, its command line hides every row, so none is made
        // for it: Joel's two at 23:43 made 84 and 38 home rows, and took
        // 110 and 75 ms to their frames (2026-11-14).
        check(failures, "over a viewer, no place home names is",
            wanted({ viewing: true }, people), false);
        check(failures, "nor one warming for home", wanted({ viewing: true }, warming), false);
        check(failures, "walked from outside over one, every source's still are",
            wanted({ viewing: true, cataloguing: true }, people), true);
        check(failures, "and rows home shows aren't read over one",
            read({ viewing: true }), false);
    }

    // A kept opening's sources wake in the order home's rows read them,
    // a place's own first and then those it holds, then the rest.
    function waking(failures) {
        const power = { name: "power" };
        const disk = { name: "disk" };
        const on = { name: "on" };
        const people = { name: "people" };
        const quests = { name: "quests" };
        const clock = { name: "clock", saysAtHome: true };
        const agents = { name: "agents", saysAtHome: true, readsHome: true };
        const sources = { power, disk, on, people, quests, clock, agents };
        const places = {
            machine: { stands: "power", holds: ["disk", "battery"] },
            battery: { stands: "power", holds: [] }
        };
        const order = Wanted.wakingOrder(["on", "machine", "people", "gone"],
            id => places[id] ?? null, id => sources[id] ?? null,
            [quests, people, on, power, disk]);
        check(failures, "home's own come first, each once, then the rest",
            order.map(one => one.name).join(" "), "on power disk people quests");
        const inTurn = Wanted.wakingOrder(["clock", "agents"], () => null,
            id => sources[id] ?? null, [clock, agents]);
        check(failures, "one saying what it holds waits its turn only reading home's rows",
            inTurn.map(one => one.name).join(" "), "agents");

        // Home found again after the fill is a slice of its own, so its
        // time is a slice's, never the sources' first wait.
        const slices = [{ from: 20, took: 8.4 }, { from: 30, took: 3 }];
        const alone = Wanted.wakeTimes(null, slices);
        check(failures, "with no finding, the first waits from the fill", alone.waited, 20);
        check(failures, "and the longest slice is a source's", alone.longest, 8);
        const bySlice = Wanted.wakeTimes({ from: 2, took: 12.6 }, slices);
        check(failures, "found in a slice, the first waits from its end", bySlice.waited, 5);
        check(failures, "and the finding is the longest slice", bySlice.longest, 13);
        const byKey = Wanted.wakeTimes({ from: 0, took: 4 }, [{ from: 37, took: 8 }]);
        check(failures, "found by a key, the fallback's wait is after it", byKey.waited, 33);
        check(failures, "a finding shorter than a slice isn't the longest", byKey.longest, 8);
        // The find woke alone only where no slice ran behind its frame or
        // fallback, and a missing name never vouches for it.
        const after = [{ after: 13 }, { after: "the fallback for 4" }];
        check(failures, "behind frames and a fallback of their own, the find woke alone",
            Wanted.foundAlone(12, after), true);
        check(failures, "a slice behind the find's frame woke with it",
            Wanted.foundAlone(13, after), false);
        check(failures, "a slice behind the find's fallback woke with it",
            Wanted.foundAlone("the fallback for 4", after), false);
        check(failures, "a find with no frame named isn't vouched for",
            Wanted.foundAlone(null, after), false);
        check(failures, "nor one beside a slice with none",
            Wanted.foundAlone(12, after.concat([{}])), false);

        // Only home's fill, run by filled() with home kept, finds home
        // after its frame; a place's, or any other opening, in the open.
        const home = { byFill: true, intoPlace: false, kept: 8, shared: false };
        check(failures, "home's fill finds home after its frame",
            Wanted.findsLater(home), true);
        check(failures, "a place's fill finds it in the open",
            Wanted.findsLater(Object.assign({}, home, { intoPlace: true })), false);
        check(failures, "an open filled() didn't run finds it there",
            Wanted.findsLater(Object.assign({}, home, { byFill: false })), false);
        check(failures, "an opening with no home kept finds it there",
            Wanted.findsLater(Object.assign({}, home, { kept: 0 })), false);
        // Kept from before a share, home could name what the share hides (an
        // ask, a copy), so while one is on it's found before the frame.
        check(failures, "home's fill under a share finds home there",
            Wanted.findsLater(Object.assign({}, home, { shared: true })), false);
        check(failures, "as does one that never said whether it's shared",
            Wanted.findsLater(Object.assign({}, home, { shared: undefined })), false);
        // A chord draws home's kept card before it's filled in only while
        // nothing may see the screen, as the kept card predates any share.
        check(failures, "a chord with nothing sharing opens from the kept card",
            Wanted.opensKept(false), true);
        check(failures, "one under a share opens whole", Wanted.opensKept(true), false);
        check(failures, "as does one that never said", Wanted.opensKept(undefined), false);
        // A chord into a kept place draws its kept rows only while nothing may
        // see the screen, as they predate any share and a shut launcher never
        // makes them again: Super+V's would show a copy, Super+O's a waiting
        // ask. A place listed as it stands as it opens draws either way.
        const chord = { listed: false, asNow: false, named: false, led: false };
        const under = shared => Object.assign({}, chord, { shared: shared });
        check(failures, "Super+V with nothing sharing draws the copies kept",
            Wanted.keptPlaceListed(under(false)), "kept");
        check(failures, "Super+V, or For You asked for from outside, under a share opens whole",
            Wanted.keptPlaceListed(under(true)), "");
        check(failures, "as does a chord missing whether it's shared",
            Wanted.keptPlaceListed(Object.assign({}, chord)), "");
        check(failures, "Windows under a share lists its source as it stands",
            Wanted.keptPlaceListed(Object.assign(under(true), { listed: true })), "source");
        check(failures, "a mark into On under a share lists it as it stands",
            Wanted.keptPlaceListed(Object.assign(under(true), { asNow: true, named: true })),
            "asItStands");
        check(failures, "Super+N under a share lists what it's led to",
            Wanted.keptPlaceListed(Object.assign(under(true), { led: true })), "led");
        check(failures, "as does a place asked for by name",
            Wanted.keptPlaceListed(Object.assign(under(true), { named: true })), "led");
        check(failures, "a source that lists its place lists it, even asked to list now",
            Wanted.keptPlaceListed(Object.assign(under(true),
                { listed: true, asNow: true, named: true })),
            "source");
        // With nothing sharing, each listed afresh still lists so, so the share
        // decides only a place that isn't.
        check(failures, "Windows with nothing sharing lists its source as it stands",
            Wanted.keptPlaceListed(Object.assign(under(false), { listed: true })), "source");
        check(failures, "a mark into On with nothing sharing lists it as it stands",
            Wanted.keptPlaceListed(Object.assign(under(false), { asNow: true, named: true })),
            "asItStands");
        check(failures, "Super+N with nothing sharing lists what it's led to",
            Wanted.keptPlaceListed(Object.assign(under(false), { led: true })), "led");
        check(failures, "as does a place asked for by name with nothing sharing",
            Wanted.keptPlaceListed(Object.assign(under(false), { named: true })), "led");
        // A share begun while an opening shows what it kept, before its fill or
        // before home is found after it, fills it in or finds home at once, so
        // the rows from before the share go as it begins, not up to half a
        // second later as niri presents the kept frame.
        const midway = { unfilled: false, finding: false };
        const begun = (shared, at) => Object.assign({}, midway, at, { shared: shared });
        check(failures, "a share begun before the fill fills it in at once",
            Wanted.sharedMidOpening(begun(true, { unfilled: true })), "fill");
        check(failures, "one begun before home is found after it finds home at once",
            Wanted.sharedMidOpening(begun(true, { finding: true })), "find");
        check(failures, "before the fill, it fills, which finds home too",
            Wanted.sharedMidOpening(begun(true, { unfilled: true, finding: true })), "fill");
        check(failures, "an opening missing whether it's shared still fills",
            Wanted.sharedMidOpening(begun(undefined, { unfilled: true })), "fill");
        check(failures, "a share ending leaves the opening to its frame",
            Wanted.sharedMidOpening(begun(false, { unfilled: true, finding: true })), "");
        check(failures, "one begun with no opening under way does nothing",
            Wanted.sharedMidOpening(begun(true, {})), "");
        // Found after it, home is searched again by a slice, a key, a click
        // or a share, but a chord only joins, and a step or a word finds its
        // own. Only a chord is told apart, so these pin only that none of them
        // is ever told apart with it.
        const stands = { home: true, held: 0, typed: "" };
        for (const by of ["slice", "key", "click", "share"])
            check(failures, `found by a ${by}, home is searched again`,
                Wanted.findsAgain(by, stands), true);
        check(failures, "found by a chord, home is only joined",
            Wanted.findsAgain("chord", stands), false);
        check(failures, "found after a step, home is only joined",
            Wanted.findsAgain("key", Object.assign({}, stands, { held: 1 })), false);
        check(failures, "found after a word, home is only joined",
            Wanted.findsAgain("key", Object.assign({}, stands, { typed: "g" })), false);
        check(failures, "where the open wouldn't find home, it's only joined",
            Wanted.findsAgain("slice", Object.assign({}, stands, { home: false })), false);
    }

    // The source an id names is found asking the vocabulary once for the
    // id, never once for each source it's held against.
    function naming(failures) {
        const all = [{ name: "power" }, { name: "disk" }, { name: "power", late: true }];
        let asked = 0;
        let read = 0;
        const named = Wanted.sourceNamed(all, one => {
            read += 1;
            return one.name;
        }, id => {
            asked += 1;
            return id === "battery" ? "power" : id;
        });
        const found = ["battery", "disk", "gone"].map(id => named(id));
        check(failures, "each id names its source, the first of that name, or none",
            found.map(one => one === null ? "none" : one.name + (one.late ? " late" : ""))
                .join(" "), "power disk none");
        check(failures, "asking the vocabulary once for each id", asked, 3);
        check(failures, "and reading each source's name once", read, 3);
    }

    // A key's acts are each said shown by the first frame begun after its
    // refresh. At 7b2252da0 dd marked a notice read, and a source no row
    // showed changed just after, was taken for the key's, and refreshed as
    // the notice's frame began; held back with it, the notice's act waited
    // on a frame that source never drew, and was never said shown.
    function framing(failures) {
        const notices = { source: "notices" };
        const wifi = { source: "wifi" };
        const names = acts => acts.map(one => one.source).join(" ");
        const refreshed = Pacing.refreshedActs(Pacing.unframed(), [notices]);
        const swappedFirst = Pacing.frameSwapped(refreshed);
        check(failures, "a swap of a frame begun before the refresh shows nothing",
            names(swappedFirst.shown), "");
        const begun = Pacing.frameBegun(swappedFirst.acts);
        const late = Pacing.refreshedActs(begun, [wifi]);
        const swapped = Pacing.frameSwapped(late);
        check(failures, "the frame begun after the refresh shows it", names(swapped.shown),
            "notices");
        check(failures, "though another act was refreshed while it was drawn",
            names(swapped.acts.waiting), "wifi");
        check(failures, "which the next frame shows",
            names(Pacing.frameSwapped(Pacing.frameBegun(swapped.acts)).shown), "wifi");
    }

    // A daemon's changes are timed from the first line the draw shows,
    // and said a place a line as the launcher closes (Y2YBTS, 2026-11-11).
    function presenting(failures) {
        const first = Presented.heard({}, "quests", 1000);
        const later = Presented.heard(first, "quests", 1100);
        check(failures, "a change waiting on a draw is timed from its first line",
            later.quests, 1000);
        check(failures, "which an earlier line still moves back",
            Presented.heard(later, "quests", 900).quests, 900);
        const both = Presented.heard(later, "windows", 1200);
        check(failures, "each place waits apart", JSON.stringify(both),
            '{"quests":1000,"windows":1200}');
        check(failures, "a place drawn waits no more",
            JSON.stringify(Presented.drawn(both, "quests")), '{"windows":1200}');
        check(failures, "nor is what waited changed", JSON.stringify(both),
            '{"quests":1000,"windows":1200}');

        const took = Presented.took([{ from: 0, at: 30 }, { from: 0, at: 80.4 },
            { from: 10, at: 60 }]);
        check(failures, "a set's median", took.median, 50);
        check(failures, "its slowest", took.slowest, 80);
        check(failures, "and how many within 50 ms, 50 itself among them", took.within, 2);
        // The daily loop reads a day's p99 from the closings' lines, since
        // one slow key in a hundred is what the median hides and the
        // slowest overstates (7QEY1S, 2026-11-11 [fde359bb]): the nearest
        // rank, so of 200 keys the 198th.
        const hundreds = Presented.took(Array.from({ length: 200 },
            (_, at) => ({ from: 0, at: 200 - at })));
        check(failures, "a set's p99 is its nearest rank", hundreds.p99, 198);
        check(failures, "which for a few is the slowest", took.p99, 80);
        // Each key's own time, in the order pressed and whole, is what
        // health pools for the day's p99, since a p99 of each opening's
        // can't be pooled (1JQPPC, 2026-11-12 [e1f448f2]).
        check(failures, "each key's time is kept in the order pressed",
            JSON.stringify(Presented.eachTook([{ from: 10, at: 60 }, { from: 0, at: 30.6 }])),
            "[31,50]");
        const said = Presented.changesSaid([
            { place: "windows", from: 100, at: 130 },
            { place: "quests", from: 0, at: 70 },
            { place: "windows", from: 200, at: 290 }
        ]);
        check(failures, "a line a place, by name", said.join("\n"),
            "launcher: 1 change in quests presented a median 70 ms from the line, "
            + "the slowest 70 ms, the p99 70 ms, 0 within 50 ms\n"
            + "launcher: 2 changes in windows presented a median 30 ms from the line, "
            + "the slowest 90 ms, the p99 90 ms, 1 within 50 ms");
        check(failures, "none presented says nothing", Presented.changesSaid([]).length, 0);
        // Joel's 20 changes in machine took a median 36 ms, 6 past 50, and
        // nothing said whether they waited in the shell or after their
        // frame began (2026-11-11). Each part is said over the changes
        // stamped with it, as a key's is, and how long each waited on the
        // pace before its refresh began, which Windows' 190 ms in the shell
        // was (2026-11-12).
        const parted = Presented.changesSaid([
            { place: "machine", from: 0, heard: 5, refreshed: 7, begun: 15, at: 40 },
            { place: "machine", from: 100, heard: 102, refreshed: 122, begun: 130, at: 190 },
            { place: "machine", from: 200, at: 230 },
            { place: "quests", from: 0, at: 70 }
        ]);
        check(failures, "a change says where its time went, each part's median apart",
            parted.join("\n"),
            "launcher: 3 changes in machine presented a median 40 ms from the line, "
            + "the slowest 90 ms, the p99 90 ms, 2 within 50 ms: a median 2 ms before the "
            + "shell heard it, 2 waiting on the pace, 8 in the shell and 25 from its frame's "
            + "start\n"
            + "launcher: 1 change in quests presented a median 70 ms from the line, "
            + "the slowest 70 ms, the p99 70 ms, 0 within 50 ms");
        // A refresh due while home was still to be found waits for the find,
        // not the pace, so a place's line counts its changes held and says the
        // longest hold apart, one of them not held waiting on the pace to its
        // refresh (H6EDLC, Super+N, 2026-11-19). Each place is said its own
        // way, so a place with none held beside it says no hold.
        // The longest hold comes after a shorter one, so the first isn't it.
        const heldApart = Presented.changesSaid([
            { place: "machine", from: 100, heard: 102, held: 110, refreshed: 122, begun: 130,
                at: 190 },
            { place: "windows", from: 0, heard: 5, refreshed: 40, begun: 48, at: 70 },
            { place: "machine", from: 0, heard: 5, held: 10, refreshed: 40, begun: 48, at: 70 },
            { place: "machine", from: 200, heard: 204, refreshed: 214, begun: 220, at: 250 }
        ]);
        check(failures, "a change held for home's find says the hold apart from the pace",
            heldApart.join("\n"),
            "launcher: 3 changes in machine presented a median 70 ms from the line, "
            + "the slowest 90 ms, the p99 90 ms, 1 within 50 ms: a median 4 ms before the "
            + "shell heard it, 8 waiting on the pace, 8 in the shell and 30 from its frame's "
            + "start; 2 held while home was found, the longest 30 ms\n"
            + "launcher: 1 change in windows presented a median 70 ms from the line, "
            + "the slowest 70 ms, the p99 70 ms, 0 within 50 ms: a median 5 ms before the "
            + "shell heard it, 35 waiting on the pace, 8 in the shell and 22 from its frame's "
            + "start");
        // Held from when a refresh was first due while home was still to be
        // found, a change heard then held from when it was heard, and a place
        // heard of nothing held for nothing, and none while nothing holds, as
        // once home's found (Launcher.qml's refreshDue and refreshed).
        const heardOf = { machine: 5, windows: 8 };
        const firstHeld = Presented.heldFrom({}, heardOf,
            { places: ["machine", "quests"], at: 10, holding: true });
        const heldAgain = Presented.heldFrom(firstHeld, heardOf,
            { places: ["machine", "windows"], at: 30, holding: true });
        const heldNone = Presented.heldFrom(firstHeld, heardOf,
            { places: ["windows"], at: 40, holding: false });
        check(failures, "a hold holds those heard from when it was first due",
            [firstHeld, heldAgain, heldNone].map(one => JSON.stringify(one)).join(" "),
            '{"machine":10} {"machine":10,"windows":30} {"machine":10}');
        // An earlier line come late takes its place's change after the hold
        // began, so it's held from when it was heard, never a wait below none.
        const heardLate = Presented.changesSaid([
            { place: "machine", from: 0, heard: 20, held: 10, refreshed: 40, begun: 48, at: 70 }
        ]);
        check(failures, "a change heard after its hold began waits on no pace",
            heardLate.join("\n"),
            "launcher: 1 change in machine presented a median 70 ms from the line, "
            + "the slowest 70 ms, the p99 70 ms, 0 within 50 ms: a median 20 ms before the "
            + "shell heard it, 0 waiting on the pace, 8 in the shell and 22 from its frame's "
            + "start; 1 held while home was found, the longest 20 ms");
        // Joel's changes in Now missed 50 ms on both sides of the shell, and
        // nothing said which daemons' lines they came from, Now being made
        // from many (2026-11-11, 20:10). Each daemon is said with how many
        // it set going, most first, then by name.
        const whose = Presented.changesSaid([
            { place: "on", from: 0, at: 60, by: "wifi" },
            { place: "on", from: 100, at: 170, by: "machine" },
            { place: "on", from: 200, at: 260, by: "machine" },
            { place: "on", from: 300, at: 340 }
        ]);
        check(failures, "a change says whose line set it going", whose.join("\n"),
            "launcher: 4 changes in on presented a median 60 ms from the line, "
            + "the slowest 70 ms, the p99 70 ms, 1 within 50 ms, from machine 2, wifi 1");

        // Joel's 12:06 keys each came to their frame in 10 to 53 ms but
        // were presented a median 101 ms from the key, and nothing said
        // where the rest went (ZINAWZ, 2026-11-11).
        const keys = [
            { from: 0, heard: 60, begun: 80, at: 100 },
            { from: 1000, heard: 1010, begun: 1040, at: 1050 },
            { from: 2000, heard: 2005, begun: 2015, at: 2300 }
        ];
        check(failures, "keys say where their time went, each part's median apart",
            Presented.keysSaid("presented", keys, true),
            "launcher: 3 keys presented a median 100 ms from the key, the slowest 300 ms, "
            + "the p99 300 ms, 1 within 50 ms: a median 10 ms before the shell heard it, "
            + "20 in the shell and 20 from its frame's start");
        check(failures, "keys said landed say how long alone",
            Presented.keysSaid("landed", keys.slice(0, 1), false),
            "launcher: 1 keys landed a median 100 ms from the key, the slowest 100 ms, "
            + "the p99 100 ms, 0 within 50 ms");
        check(failures, "as do keys timed with no frame begun",
            Presented.keysSaid("presented", [{ from: 0, at: 70 }], true),
            "launcher: 1 keys presented a median 70 ms from the key, the slowest 70 ms, "
            + "the p99 70 ms, 0 within 50 ms");
        check(failures, "no keys say nothing", Presented.keysSaid("presented", [], true), "");

        // Joel's keys past 50 ms went from 11% to 60% the day he docked to a
        // 60 Hz screen, from the 120 Hz panel (16 Nov), and nothing said so.
        const docked = { from: 0, at: 54, screen: "DP-4", refresh: 1000 / 59.997 };
        check(failures, "keys say the screen they were shown on, and its rate",
            Presented.screensSaid([docked, docked]), "; on DP-4 at 60 Hz");
        // The launcher shuts as it moves screens, so one line holds one
        // today; should that change, each is still said once.
        check(failures, "each screen once, in turn, were they on two",
            Presented.screensSaid([docked, { screen: "eDP-1", refresh: 1000 / 120 }, docked]),
            "; on DP-4 at 60 Hz and eDP-1 at 120 Hz");
        check(failures, "a key whose frame niri dropped leaves its screen once",
            Presented.screensSaid([{ screen: "DP-4", refresh: 0 }, docked]), "; on DP-4 at 60 Hz");
        check(failures, "and the keys' line ends with it",
            Presented.keysSaid("presented", [docked], true)
                .endsWith("within 50 ms; on DP-4 at 60 Hz"), true);
        check(failures, "a screen niri gave no rate for is named alone",
            Presented.screensSaid([{ screen: "DP-4", refresh: 0 }]), "; on DP-4");
        check(failures, "and keys on no named screen say none",
            Presented.screensSaid([{ from: 0, at: 54 }]), "");

        // Joel's 235 drawers on 2026-11-11 were timed only from the work
        // begun for each to its frame, a median 14 ms, though each followed
        // its key by 50 ms and more and reached the screen 25 to 46 ms after
        // its frame (Joel, 20:28: "the drawer in the launcher and its time
        // to appear needs to be kept to the same performance requirements
        // as everything else"). Each is said from the key that selected its
        // row, as it's presented, with where its time went.
        const drawer = { key: "agent:Y2YBTS", place: "agents", from: 1000, followed: 1050,
            begun: 1060, at: 1090 };
        check(failures, "a drawer says its time from the key as it's presented",
            Presented.drawerSaid(drawer),
            'launcher: drawer for "agent:Y2YBTS" presented 90 ms from the key: a median '
            + "50 ms before it followed, 10 until its frame began and 30 from its "
            + "frame's start, the frame after");
        check(failures, "and says when it came in its key's own frame",
            Presented.drawerSaid(Object.assign({ came: Pacing.drawerCame(true) }, drawer))
                .endsWith("frame's start, in the key's frame"), true);
        // A slow drawer's own line, which reports off or on names its row
        // by a key a title may be in, so it stays one line, its key quoted
        // as the lab's ' took ' reads it.
        const slow = { making: 200, waiting: 40, drawing: 60, came: "the frame after" };
        check(failures, "a slow drawer's line says where its time went, its key quoted",
            Presented.drawerTook("window:a \"b\"\nlauncher: x", 300, slow, " (split)"),
            'launcher: drawer for "window:a \\"b\\"\\nlauncher: x" took 300 ms to its frame, '
            + "200 ms making (split), 40 waiting for its frame, 60 drawing it, the frame after");
        const drawers = Presented.drawersSaid([drawer,
            { key: "program:kitty", place: "programs", from: 0, followed: 5, begun: 15, at: 40 },
            { key: "agent:KP0L6A", place: "agents", from: 2000, followed: 2010, begun: 2020,
                at: 2040 },
            { key: "agent:16V6F4", place: "agents", from: 3000, at: 3045 }]);
        check(failures, "and at the close a line a place, by name, as changes are",
            drawers.join("\n"),
            "launcher: 3 drawers in agents presented a median 45 ms from the key, the slowest "
            + "90 ms, the p99 90 ms, 2 within 50 ms: a median 10 ms before it followed, 10 "
            + "until its frame began and 20 from its frame's start\n"
            + "launcher: 1 drawer in programs presented a median 40 ms from the key, the "
            + "slowest 40 ms, the p99 40 ms, 1 within 50 ms: a median 5 ms before it "
            + "followed, 10 until its frame began and 25 from its frame's start");
        check(failures, "none presented says nothing", Presented.drawersSaid([]).length, 0);
        // Ctrl+W l into Emoji's drawer laid all 1,833 lines before its
        // frame, and nothing timed it (2026-11-19). Going in says its time
        // from the key as it's presented, with how many lines that frame
        // laid, so a long lay is seen in the time it costs.
        check(failures, "going into a drawer says its time and the lines it laid first",
            Presented.intoSaid({ key: "place:emoji", from: 1000, at: 1031, laid: 38,
                screen: "DP-4", refresh: 1000 / 59.997 }),
            'launcher: into the drawer for "place:emoji" presented 31 ms from the key, '
            + "38 lines laid first; on DP-4 at 60 Hz");
        check(failures, "going in says the shell's share to its frame's swap",
            Presented.intoSaid({ key: "place:emoji", from: 1000, at: 1094, swapped: 1012.4,
                laid: 37 }),
            'launcher: into the drawer for "place:emoji" presented 94 ms from the key, '
            + "12 to the shell's swap, 37 lines laid first");
        check(failures, "and says one line as one",
            Presented.intoSaid({ key: "app:kitty", from: 0, at: 20, laid: 1 }),
            'launcher: into the drawer for "app:kitty" presented 20 ms from the key, '
            + "1 line laid first");
        // A row named by a window's title may hold a quote or a line break,
        // which the journal would read as the line's end or another line.
        check(failures, "going in quotes its key so a title can't forge a line",
            Presented.intoSaid({ key: 'win:a "b"\nlauncher: x', from: 0, at: 20, laid: 1 }),
            'launcher: into the drawer for "win:a \\"b\\"\\nlauncher: x" presented 20 ms '
            + "from the key, 1 line laid first");
        // JSON leaves the line and paragraph separators and the next-line
        // mark raw, and a reader splitting on any line end would split there.
        check(failures, "going in escapes every line end JSON leaves raw",
            Presented.intoSaid({ key: "win:a\u2028b\u2029c\u0085d", from: 0, at: 20, laid: 1 }),
            'launcher: into the drawer for "win:a\\u2028b\\u2029c\\u0085d" presented 20 ms '
            + "from the key, 1 line laid first");
        // Going back from the drawer's list of keys is a going-in under the
        // same row, which read as a second of Emoji's with another count.
        check(failures, "going in names the page it went into",
            Presented.intoSaid({ key: "place:emoji", page: "The Drawer's Keys", from: 0,
                at: 20, laid: 12 }),
            'launcher: into the drawer for "place:emoji" on "The Drawer\'s Keys" presented '
            + "20 ms from the key, 12 lines laid first");
        check(failures, "and quotes the page, as a window's title may name it",
            Presented.intoSaid({ key: "place:x", page: 'a "b"\nlauncher: x', from: 0, at: 20,
                laid: 1 }),
            'launcher: into the drawer for "place:x" on "a \\"b\\"\\nlauncher: x" presented '
            + "20 ms from the key, 1 line laid first");
        check(failures, "a drawer quotes its key too, so a title can't forge a line",
            Presented.drawerSaid(Object.assign({}, drawer, { key: 'win:a "b"\nlauncher: x' }))
                .startsWith('launcher: drawer for "win:a \\"b\\"\\nlauncher: x" presented 90 ms'),
            true);
        // j typed 0 to 480 ms after Ctrl+W l waited on the rest of Emoji's
        // run being laid (2026-11-19), so a key in a drawer says its time
        // from going in when it came soon after, which the drawer lab reads.
        check(failures, "a key in a drawer says its time from the key and from going in",
            Presented.keyInSaid({ act: "next", key: "place:emoji", from: 1040, into: 1000,
                at: 1131, screen: "DP-4", refresh: 1000 / 59.997 }),
            'launcher: next in the drawer for "place:emoji" presented 91 ms from the key, '
            + "131 from going in; on DP-4 at 60 Hz");
        check(failures, "and the shell's share to its frame's swap",
            Presented.keyInSaid({ act: "next", key: "place:emoji", from: 1040, into: 1000,
                at: 1131, swapped: 1058.4 }),
            'launcher: next in the drawer for "place:emoji" presented 91 ms from the key, '
            + "18 to the shell's swap, 131 from going in");
        // Two minutes into a drawer, its time from going in would read as
        // a fault, so a key long after says none; but while reports are on
        // every key says it, as the drawer lab reads it however long the
        // lay held the key.
        check(failures, "a key just inside a second of going in keeps its time from it",
            Presented.intoKept(1999, 1000, false), 1000);
        check(failures, "and one a second after keeps none",
            Presented.intoKept(2000, 1000, false), undefined);
        check(failures, "but one long after keeps it while reports are on",
            Presented.intoKept(121000, 1000, true), 1000);
        check(failures, "and a key that keeps none says none from going in",
            Presented.keyInSaid({ act: "previous", key: "place:emoji", from: 0, at: 60 }),
            'launcher: previous in the drawer for "place:emoji" presented 60 ms from the key');
        check(failures, "a key in a drawer quotes its key so a title can't forge a line",
            Presented.keyInSaid({ act: "next", key: 'x "y"\nlauncher: z', from: 0, at: 60 }),
            'launcher: next in the drawer for "x \\"y\\"\\nlauncher: z" presented 60 ms '
            + "from the key");
        // A window's row is keyed by its title, a mail's subject or a
        // document's name, which a slow key would write once per key.
        check(failures, "a window's key is said without its title unless reports are on",
            Presented.keyShown("window:org.gnome.Evolution:Re: my offer", false),
            "window:org.gnome.Evolution");
        check(failures, "as is one with an empty title",
            Presented.keyShown("window:kitty:", false), "window:kitty");
        check(failures, "a window key of one part, which may be a title, is said as window",
            Presented.keyShown("window:Inbox", false), "window");
        check(failures, "as is one that names no app, as niri's records leave it",
            Presented.keyShown("window::Inbox", false), "window");
        check(failures, "or one whose app was read as null",
            Presented.keyShown("window:null:Inbox", false), "window");
        check(failures, "or as undefined",
            Presented.keyShown("window:undefined:Inbox", false), "window");
        check(failures, "but an app whose id starts with a digit is said",
            Presented.keyShown("window:1Password:Vault", false), "window:1Password");
        check(failures, "as is one whose app holds a page's address, as a browser's may",
            Presented.keyShown("window:chrome-docs.google.com__document_d_1x_edit-Default:Plan",
                false), "window");
        check(failures, "and whole while they are",
            Presented.keyShown("window:kitty:~/src", true), "window:kitty:~/src");
        check(failures, "and the launcher's own ids are said whole",
            Presented.keyShown("place:emoji", false), "place:emoji");
        check(failures, "a category's too",
            Presented.keyShown("category:power", false), "category:power");
        check(failures, "and a collection's, as Emoji's row has",
            Presented.keyShown("collection:emoji", false), "collection:emoji");
        check(failures, "but any other key only by its kind, as a network's name is",
            Presented.keyShown("wifi:Harbour House", false), "wifi");
        check(failures, "a row named but with no key is said only as an answer with reports off",
            Presented.rowShown({ name: "Re: my offer" }, false), "answer");
        check(failures, "and its name while they're on",
            Presented.rowShown({ name: "Address" }, true), "Address");
        check(failures, "a key of no kind is said as nothing",
            Presented.keyShown("github.com", false), "");
        check(failures, "as is a name with a colon in it, which no kind is",
            Presented.keyShown("Call: Dr Smith", false), "");
        check(failures, "or one in lower case with a space",
            Presented.keyShown("dr smith: call", false), "");
        check(failures, "or a device's name, as a sound's trail has",
            Presented.keyShown("alsa_output.pci-0000_00_1f.3:by-ear", false), "");
        check(failures, "and a key that came as a number is read as text",
            Presented.keyShown(42, false), "");
        check(failures, "and said as text with reports on, so the line quotes it",
            Presented.rowShown({ key: 42 }, true), "42");
        check(failures, "a row with neither key nor name is said as a row",
            Presented.rowShown({}, false), "row");
        check(failures, "as is one missing, whether reports are on or off",
            Presented.rowShown(null, true), "row");
        check(failures, "so its row is said only as a row with reports off",
            Presented.rowShown({ key: "stopwatch", name: "Stopwatch" }, false), "row");
        check(failures, "and whole while they're on",
            Presented.rowShown({ key: "stopwatch", name: "Stopwatch" }, true), "stopwatch");
        check(failures, "a row's key is what's said of a row that has one",
            Presented.rowShown({ key: "window:kitty:~/src", name: "~/src" }, false),
            "window:kitty");
        // Joel's drawers took a median 54 ms on the 60 Hz screen and 33 on
        // the 120 Hz panel (2026-11-17), and his day's median followed
        // which he worked on. Each says its screen last, as keys do, so
        // what reads the words before it still reads them.
        const dockedScreen = { screen: "DP-4", refresh: 1000 / 59.997 };
        check(failures, "a drawer says the screen it was shown on, last",
            Presented.drawerSaid(Object.assign({}, drawer, dockedScreen)),
            'launcher: drawer for "agent:Y2YBTS" presented 90 ms from the key: a median '
            + "50 ms before it followed, 10 until its frame began and 30 from its "
            + "frame's start, the frame after; on DP-4 at 60 Hz");
        check(failures, "and after when it came in its key's own frame",
            Presented.drawerSaid(Object.assign({ came: Pacing.drawerCame(true) }, drawer,
                dockedScreen)).endsWith("in the key's frame; on DP-4 at 60 Hz"), true);
        check(failures, "and the close says each place's own",
            Presented.drawersSaid([Object.assign({}, drawer, dockedScreen),
                { key: "agent:KP0L6A", place: "agents", from: 2000, at: 2040 },
                { key: "program:kitty", place: "programs", from: 0, at: 30, screen: "eDP-1",
                    refresh: 1000 / 120 }]).join("\n"),
            "launcher: 2 drawers in agents presented a median 40 ms from the key, the "
            + "slowest 90 ms, the p99 90 ms, 1 within 50 ms: a median 50 ms before it "
            + "followed, 10 until its frame began and 30 from its frame's start; "
            + "on DP-4 at 60 Hz\n"
            + "launcher: 1 drawer in programs presented a median 30 ms from the key, the "
            + "slowest 30 ms, the p99 30 ms, 1 within 50 ms; on eDP-1 at 120 Hz");

        // Joel's drawers spent 45 to 48 ms from their frame's start to the
        // screen whatever they held, keys 12 to 34 and changes 6 to 12, and
        // nothing said whether the shell's swap or niri took it (KP0L6A,
        // 2026-11-12). Where the render thread timed the frame's swap, that
        // part is said to the swap and after it, over those it timed.
        check(failures, "a drawer says its frame's time to its swap and after it",
            Presented.drawerSaid(Object.assign({ swapped: 1064 }, drawer)),
            'launcher: drawer for "agent:Y2YBTS" presented 90 ms from the key: a median '
            + "50 ms before it followed, 10 until its frame began and 30 from its "
            + "frame's start (4 to its swap and 26 after it), the frame after");
        const swapped = Presented.drawersSaid([
            Object.assign({ swapped: 1062 }, drawer),
            { key: "agent:KP0L6A", place: "agents", from: 2000, followed: 2010, begun: 2020,
                swapped: 2030, at: 2040 },
            { key: "agent:16V6F4", place: "agents", from: 3000, followed: 3005, begun: 3010,
                swapped: null, at: 3045 }]);
        check(failures, "and at the close over those whose swap was timed", swapped.join("\n"),
            "launcher: 3 drawers in agents presented a median 45 ms from the key, the slowest "
            + "90 ms, the p99 90 ms, 2 within 50 ms: a median 10 ms before it followed, 10 "
            + "until its frame began and 30 from its frame's start (2 to its swap and 10 "
            + "after it)");
        check(failures, "as keys say theirs",
            Presented.keysSaid("presented", [Object.assign({ swapped: 82 }, keys[0]),
                keys[1], Object.assign({ swapped: 2020 }, keys[2])], true),
            "launcher: 3 keys presented a median 100 ms from the key, the slowest 300 ms, "
            + "the p99 300 ms, 1 within 50 ms: a median 10 ms before the shell heard it, "
            + "20 in the shell and 20 from its frame's start (2 to its swap and 18 after it)");
        const panel = { screen: "eDP-1", refresh: 1000 / 120 };
        check(failures, "and the screen they were on after all of it",
            Presented.keysSaid("presented", [Object.assign({ swapped: 82 }, keys[0], panel),
                Object.assign({}, keys[1], panel),
                Object.assign({ swapped: 2020 }, keys[2], panel)], true),
            "launcher: 3 keys presented a median 100 ms from the key, the slowest 300 ms, "
            + "the p99 300 ms, 1 within 50 ms: a median 10 ms before the shell heard it, "
            + "20 in the shell and 20 from its frame's start (2 to its swap and 18 after it); "
            + "on eDP-1 at 120 Hz");
        // As the launcher keeps them (keyPresented): a swap it didn't time
        // is null, one not reached 0 (frame-presentation.patch), and every
        // key says whether niri timed its press. A press's fraction is when
        // the shell handled it, so 1000.7 is said as niri stamped it, 1000.
        const clocked = [Object.assign({ swapped: 2020.04, stamped: true, later: 0 }, keys[2]),
            Object.assign({ swapped: null, stamped: true, later: 0 }, keys[1], { from: 1000.7 }),
            Object.assign({ swapped: 82.36, stamped: true, later: 0 }, keys[0])];
        check(failures, "each key says its press in whole ms, its swap and screen to a tenth",
            Presented.keysClockSaid(clocked),
            "launcher: 3 keys shown, each pressed, swapped and shown in monotonic ms: "
            + "0 82.4 100.0; "
            + "1000 - 1050.0; 2000 2020.0 2300.0");
        check(failures, "an untimed press, a swap not reached and a later frame are said",
            Presented.keysClockSaid([Object.assign({}, clocked[2], { stamped: false }),
                Object.assign({}, clocked[1], { swapped: 0, later: 1 }),
                Object.assign({}, clocked[0], { later: 2 })]),
            "launcher: 3 keys shown, each pressed, swapped and shown in monotonic ms: "
            + "- 82.4 100.0; 1000 - 1050.0, 1 frame of the shell's after its own; "
            + "2000 2020.0 2300.0, 2 frames of the shell's after its own");
        // How the launcher reads a key's record and the frame niri answered.
        const clockOf = (key, answered) => JSON.stringify(Presented.keyClockOf(key, answered));
        check(failures, "a key niri answered on its own frame, or a later one, says how many on",
            [clockOf({ frame: 7, stamped: true }, 7), clockOf({ frame: 7, stamped: true }, 9)]
                .join(" "),
            '{"stamped":true,"later":0} {"stamped":true,"later":2}');
        check(failures,
            "an untimed or unsaid press reads so, an unknown or earlier frame none later",
            [clockOf({ frame: 7, stamped: false }, 8), clockOf({}, 9),
                clockOf({ frame: 9, stamped: true }, 8)].join(" "),
            '{"stamped":false,"later":1} {"stamped":false,"later":0} {"stamped":true,"later":0}');
        check(failures, "and no keys say nothing of their clock", Presented.keysClockSaid([]), "");

        // Joel's openings waited a median 6 frames at 120 Hz for niri after
        // their GPU work was done (2026-11-15). Presentation times lie on the
        // vblanks, so the frames from the swap are counted back from the one
        // niri showed it at.
        const frame = { swapped: 1000, refresh: 8.333 };
        check(failures, "an opening says the frames from its swap to the screen",
            Presented.shownSaid(frame, 1022.6),
            "; 3 frames at 120 Hz from its swap to the screen"
            + " (swapped 1000.0, shown 1022.6 monotonic ms)");
        check(failures, "niri at the next vblank says one frame",
            Presented.shownSaid(frame, 1008.3),
            "; 1 frame at 120 Hz from its swap to the screen"
            + " (swapped 1000.0, shown 1008.3 monotonic ms)");
        // Said to a tenth of a ms, so it lines up with niri's log of its asks.
        check(failures, "and its swap and vblank in monotonic ms, to a tenth",
            Presented.shownSaid({ swapped: 123456.789, refresh: 8.333 }, 123481.04),
            "; 3 frames at 120 Hz from its swap to the screen"
            + " (swapped 123456.8, shown 123481.0 monotonic ms)");
        check(failures, "and counts at its own refresh",
            Presented.shownSaid({ swapped: 1000, refresh: 16.667 }, 1040),
            "; 3 frames at 60 Hz from its swap to the screen"
            + " (swapped 1000.0, shown 1040.0 monotonic ms)");
        check(failures, "and nothing where niri gave no refresh",
            Presented.shownSaid({ swapped: 1000 }, 1030), "");
        check(failures, "nor where the frame's swap wasn't timed",
            Presented.shownSaid({ refresh: 8.333 }, 1030), "");
    }

    // Joel's closing at 14:36 on 2026-11-11 made Sound's 111 rows and
    // Displays' 87 again for a change to one device or one screen, since a
    // collection was made whole once its source changed.
    function spotting(failures) {
        let makes = [];
        const make = thing => {
            makes.push(thing.key);
            return [`${thing.key}/scale`, `${thing.key}/refresh`];
        };
        const stampOf = thing => thing.stamp;
        const screens = [{ key: "left", stamp: "1x" }, { key: "right", stamp: "1x" }];
        const first = Spots.eachKept(undefined, screens, stampOf, make);
        check(failures, "the first making makes every thing", makes.join(), "left,right");
        check(failures, "and counts them", first.made, 2);
        check(failures, "its spots are each thing's, in order",
            Spots.spotsOf(screens, first.kept).join(),
            "left/scale,left/refresh,right/scale,right/refresh");

        makes = [];
        const moved = [{ key: "left", stamp: "1x" }, { key: "right", stamp: "1.5x" }];
        const second = Spots.eachKept(first.kept, moved, stampOf, make);
        check(failures, "a closing after one thing changed makes only it", makes.join(),
            "right");
        check(failures, "and keeps the other's spots as they were",
            second.kept.left === first.kept.left, true);

        makes = [];
        const gone = Spots.eachKept(second.kept, [moved[0]], stampOf, make);
        check(failures, "a thing gone makes nothing", makes.length, 0);
        check(failures, "and is let go", Object.keys(gone.kept).join(), "left");

        makes = [];
        Spots.eachKept(first.kept, screens, null, make);
        check(failures, "a source saying no stamp makes every thing, as before", makes.join(),
            "left,right");
        makes = [];
        Spots.eachKept(first.kept, [{ key: "left", stamp: null }], stampOf, make);
        check(failures, "and so does a thing whose stamp is null", makes.join(), "left");
    }

    // Joel's windows changes waited up to 190 ms in the shell while his
    // opening's sources woke, each behind another source's refresh a few
    // milliseconds dear, for the rest of a quarter second (2026-11-12,
    // 10:16 and 10:17); one that came alone took 13. The wait after a
    // refresh is three times what the last three cost, their median, so
    // the shell spends about a third of its time refreshing at most, and
    // never more than the pace. Sound's changes, which come from no key,
    // waited 58 ms after 12 ms refreshes and 255 after one dear one
    // (2026-11-12, 17:30 and 17:50).
    function costing(failures) {
        const every = 250;
        const cheap = Pacing.costed(Pacing.took(1000, Pacing.rested()), 8);
        const soon = Pacing.heard(cheap, 1020, every);
        check(failures, "a change after a cheap refresh waits three times its cost",
            soon.pace.due, 1024);
        check(failures, "and one past that is taken at once",
            Pacing.heard(cheap, 1025, every).now, true);
        const costs = ms => ms.reduce((pace, cost, at) =>
            Pacing.costed(Pacing.took(1000 + at * 300, pace), cost), Pacing.rested());
        const stalled = costs([8, 8, 80]);
        check(failures, "one dear refresh among cheap ones doesn't hold the next change long",
            Pacing.heard(stalled, stalled.at + 10, every).pace.due, stalled.at + 24);
        const dearer = costs([8, 80, 80]);
        check(failures, "but two of the last three dear do",
            Pacing.heard(dearer, dearer.at + 10, every).pace.due, dearer.at + 240);
        const past = costs([80, 80, 8, 8]);
        check(failures, "and only the last three count",
            Pacing.heard(past, past.at + 10, every).pace.due, past.at + 24);
        const dear = Pacing.costed(Pacing.took(1000, Pacing.rested()), 100);
        check(failures, "one after a dear refresh waits no more than the pace",
            Pacing.heard(dear, 1100, every).pace.due, 1250);
        const tiny = Pacing.costed(Pacing.took(1000, Pacing.rested()), 1);
        check(failures, "nor less than a frame, which would draw only the second",
            Pacing.heard(tiny, 1005, every).pace.due, 1016);
        check(failures, "a refresh keeps the last one's cost until its own is known",
            Pacing.took(2000, cheap).cost, 8);
        check(failures, "as does the timer ringing", Pacing.rang(soon.pace).cost, 8);
        check(failures, "a pace whose cost isn't known yet waits the whole pace",
            Pacing.heard(Pacing.took(1000, Pacing.rested()), 1100, every).pace.due, 1250);

        // A source changing every 2 ms for 3 s, as Slack's answers did, the
        // shell hearing nothing while it refreshes.
        const storm = cost => {
            let pace = Pacing.rested();
            let busy = 0;
            let spent = 0;
            let refreshes = 0;
            const refresh = at => {
                pace = Pacing.costed(Pacing.took(at, pace), cost);
                spent += cost;
                refreshes += 1;
                busy = at + cost;
            };
            for (let now = 1000; now < 4000; now += 2) {
                if (now < busy)
                    continue;
                if (pace.due !== null && pace.due <= now) {
                    pace = Pacing.rang(pace);
                    refresh(now);
                    continue;
                }
                const heard = Pacing.heard(pace, now, every);
                pace = heard.pace;
                if (heard.now)
                    refresh(now);
            }
            return { share: spent / 3000, each: 3000 / refreshes };
        };
        const cheapStorm = storm(10);
        check(failures, "a storm of cheap changes spends at most a third refreshing",
            cheapStorm.share <= 1 / 3 + 0.01, true);
        check(failures, "and shows them many times a second", cheapStorm.each <= 60, true);
        check(failures, "a storm of dear ones refreshes at most four times a second",
            storm(100).each >= every, true);
    }

    // y's flash is timed from the frame that draws it, a turn after the
    // frame, and only the newest yank's frame starts it, so an earlier yank
    // still waiting can't end a later flash before it's drawn (2026-11-18).
    function flashing(failures) {
        const turn = [];
        const frame = [];
        const drain = queue => {
            for (const then of queue.splice(0))
                then();
        };
        const started = [];
        let yanks = 0;
        const yank = () => {
            const at = ++yanks;
            Flash.fromItsFrame(then => turn.push(then), then => frame.push(then),
                () => at === yanks, () => started.push(at));
        };
        yank();
        check(failures, "a yank asks for no frame until its turn ends", frame.length, 0);
        drain(turn);
        check(failures, "its turn's end asks for its frame", frame.length, 1);
        drain(turn);
        drain(turn);
        check(failures, "turns alone start nothing before its frame", started.join(), "");
        drain(frame);
        check(failures, "nor does its frame, only the turn after", started.join(), "");
        drain(turn);
        check(failures, "the turn after its frame starts it", started.join(), "1");
        yank();
        drain(turn);
        drain(frame);
        yank();
        drain(turn);
        check(failures, "a yank between a frame and the turn after keeps that frame's flash"
            + " from starting", started.join(), "1");
        drain(frame);
        drain(turn);
        check(failures, "and only the newest yank's frame starts it", started.join(), "1,3");
    }

    // A drawer comes in its key's own frame where its kind made in under
    // 8 ms of late and no key is held, decided before it's made, never by
    // timing it in the frame; otherwise the frame after, so the highlight
    // never waits long on it (VKCF3Z, 2026-11-12, d4c660a6; Y2YBTS,
    // 2026-11-16, from Joel's For You drawers made in 4 to 7 ms).
    function drawers(failures) {
        check(failures, "a kind not yet made comes the frame after",
            Pacing.drawerJoinsKey({}, "home", false), false);
        let made = {};
        for (const making of [1, 2, 1])
            made = Pacing.drawerMade(made, "home", making);
        check(failures, "one whose kind makes in a millisecond or two comes in the key's frame",
            Pacing.drawerJoinsKey(made, "home", false), true);
        check(failures, "never while a key is held",
            Pacing.drawerJoinsKey(made, "home", true), false);
        check(failures, "another kind is decided by its own makings",
            Pacing.drawerJoinsKey(made, "agents", false), false);
        // For You's drawers made in 4 to 7 ms on Joel's machine, and 71 of
        // his 74 drawers on 16 November came the frame after, a median of
        // 72 ms from the key against his keys' 43.
        // Its rows' keys all begin on:, so they're one kind.
        const forYou = Pacing.drawerKind("on:waitingOnYou", { key: "on:waiting:want:KQL3KB" });
        let looks = {};
        for (const making of [6, 5, 7, 4])
            looks = Pacing.drawerMade(looks, forYou, making);
        check(failures, "one made in 4 to 7 ms, as For You's are, comes in the key's frame",
            Pacing.drawerJoinsKey(looks, forYou, false), true);
        // The line falls at 8: a median just under comes in the key's
        // frame, and one at 8 the frame after.
        check(failures, "a median of 7.5 ms comes in the key's frame",
            Pacing.drawerJoinsKey({ dear: [7, 8] }, "dear", false), true);
        check(failures, "and a median of 8 ms the frame after",
            Pacing.drawerJoinsKey({ dear: [8, 8, 8] }, "dear", false), false);
        // A place's rows are of many kinds: home's people and commands make
        // theirs in a millisecond, its programs' in 10 to 114, so one of
        // Slack's came in its key's frame after 49 ms making (2026-11-12).
        const person = { key: "person:ana@example.org" };
        const program = { key: "program:slack" };
        let home = {};
        for (const making of [0, 1, 0])
            home = Pacing.drawerMade(home, Pacing.drawerKind("home", person), making);
        check(failures, "a place's rows are kinds of their own",
            Pacing.drawerKind("home", person) === Pacing.drawerKind("home", program), false);
        check(failures, "so a program's first drawer at home comes the frame after",
            Pacing.drawerJoinsKey(home, Pacing.drawerKind("home", program), false), false);
        home = Pacing.drawerMade(home, Pacing.drawerKind("home", program), 49);
        check(failures, "as does one whose own were dear",
            Pacing.drawerJoinsKey(home, Pacing.drawerKind("home", program), false), false);
        check(failures, "while a person's still comes in the key's frame",
            Pacing.drawerJoinsKey(home, Pacing.drawerKind("home", person), false), true);
        check(failures, "and one place's kind isn't another's",
            Pacing.drawerKind("home", program) === Pacing.drawerKind("applications", program),
            false);
        check(failures, "a row with no key is its place's",
            Pacing.drawerKind("home", {}), "home");
        for (const making of [9, 13, 9, 12])
            made = Pacing.drawerMade(made, "home", making);
        check(failures, "one whose kind's median reaches 8 ms comes the frame after",
            Pacing.drawerJoinsKey(made, "home", false), false);
        for (let at = 0; at < 8; at++)
            made = Pacing.drawerMade(made, "home", 1);
        check(failures, "only its last eight count", made.home.length, 8);
        check(failures, "so a kind grown cheap comes in the key's frame again",
            Pacing.drawerJoinsKey(made, "home", false), true);
        const before = { home: [1] };
        Pacing.drawerMade(before, "home", 5);
        check(failures, "what was kept is left as it was", before.home.length, 1);
        check(failures, "its journal line says which it took",
            [true, false].map(joined => Pacing.drawerCame(joined)).join("; "),
            "in the key's frame; the frame after");
        resting(failures);
        kept(failures);
    }

    // The makings are kept across the shell's restarts, which emptied them:
    // Wi-Fi's drawers made in a median of 6 ms, yet 5 of the 6 from a key
    // came the frame after (F8TT8O, 2026-11-18).
    function kept(failures) {
        const wifi = Pacing.drawerKind("Network", { key: "wifi:harbour" });
        let made = {};
        for (const making of [6, 5, 7])
            made = Pacing.drawerMade(made, wifi, making);
        const read = Makings.read(JSON.stringify(made));
        check(failures, "a kind made cheaply before a restart comes in the key's frame after it",
            Pacing.drawerJoinsKey(read, wifi, false), true);
        check(failures, "text that isn't JSON can't be read",
            Makings.read("{\"Network wifi\": [6,"), null);
        check(failures, "an empty file keeps none and is no fault",
            JSON.stringify(Makings.read("")), "{}");
        check(failures, "nor a list in place of the kinds", Makings.read("[6, 5]"), null);
        check(failures, "nor a file past what's read",
            Makings.read(JSON.stringify({ home: [1], pad: "x".repeat(70000) })), null);
        // A kind named as every object's own is left out, rather than
        // throwing as it's read and losing every good kind with it.
        const named = Makings.read(
            "{\"__proto__\": [1], \"constructor\": [2], \"toString\": [3], \"home\": [4]}");
        check(failures, "a kind named as an object's own is left out",
            JSON.stringify(named), JSON.stringify({ home: [4] }));
        const odd = Makings.read(JSON.stringify({
            home: [1, "2", -3, null, 2],
            broken: "7",
            none: []
        }));
        check(failures, "only a kind's times in ms are kept", JSON.stringify(odd.home), "[1,2]");
        check(failures, "and a kind with none is no kind", Object.keys(odd).join(","), "home");
        const many = Makings.read(JSON.stringify({ home: [9, 9, 9, 9, 9, 1, 1, 1, 1, 1] }));
        check(failures, "a kind keeps its last eight", JSON.stringify(many.home),
            "[9,9,9,1,1,1,1,1]");
        // This shell's own come after what the file kept, as they're newer.
        const before = { home: [20, 20] };
        const joined = Makings.joined(before, { home: [1, 1, 1], agents: [3] });
        check(failures, "what this shell made follows what was kept",
            JSON.stringify(joined), JSON.stringify({ home: [20, 20, 1, 1, 1], agents: [3] }));
        check(failures, "and neither is changed", before.home.length, 2);
    }

    // A drawer a letter brought waits until the letters rest, longer than
    // the 100 to 150 ms Joel types them apart, so the next letter never
    // waits behind it: "calend" waited 272 ms behind Google Calendar's
    // first (2026-11-13, 06:14).
    function resting(failures) {
        const rest = Pacing.lettersRest();
        check(failures, "the rest outlasts the gap between Joel's letters", rest > 150, true);
        check(failures, "a letter just typed is still coming",
            Pacing.lettersComing(1000, 1000 + 150), true);
        check(failures, "until the rest has passed",
            Pacing.lettersComing(1000, 1000 + rest), false);
        check(failures, "and none are once an arrow has moved",
            Pacing.lettersComing(0, 100), false);
        check(failures, "its line says how long it rested, apart from its making",
            Pacing.drawerCame(false, 163.4), "after resting 163 ms for the letters");
        check(failures, "one that never rested says its frame as before",
            Pacing.drawerCame(true, 0), "in the key's frame");
        // Only a dear kind rests: on 17 November, 20 of Joel's 47 drawers
        // from 23:04 rested a median 166 ms before they followed, against 3
        // to 8 ms for the rest, though agents' drawers made in a median 4.
        check(failures, "a kind not yet made rests for the letters",
            Pacing.drawerRestsForLetters({}, "agent"), true);
        check(failures, "as does a dear one, as Google Calendar's first was",
            Pacing.drawerRestsForLetters({ program: [134, 12] }, "program"), true);
        check(failures, "while a cheap one follows its letter",
            Pacing.drawerRestsForLetters({ agent: [4, 3, 5] }, "agent"), false);
        check(failures, "and one at the line a drawer joins its key's frame under rests",
            Pacing.drawerRestsForLetters({ agent: [8, 8, 8] }, "agent"), true);
        check(failures, "its line says it followed its letter",
            Pacing.drawerCame(false, 0, true), "the frame after its letter");
    }

    function run() {
        const failures = [];
        const every = 250;
        drawers(failures);

        const first = Pacing.heard(Pacing.rested(), 1000, every);
        check(failures, "the first change is taken at once", first.now, true);
        const begun = Pacing.took(1000);

        const soon = Pacing.heard(begun, 1100, every);
        check(failures, "one a tenth of a second after waits", soon.now, false);
        check(failures, "until a pace after the last refresh began", soon.pace.due, 1250);

        const more = Pacing.heard(soon.pace, 1200, every);
        check(failures, "another while one waits joins it", more.now, false);
        check(failures, "and is due when the first was", more.pace.due, 1250);

        const rung = Pacing.rang(more.pace);
        check(failures, "once the timer rings nothing waits", rung.due, null);
        const late = Pacing.heard(Pacing.took(1250), 1600, every);
        check(failures, "a change a pace after the last refresh is taken at once", late.now,
            true);

        // The net's draws-still run at core 5a9d3fa6 saw two refreshes 37 ms
        // apart: its timer rang late under load, a second source's change
        // past the pace was taken at once beside it, and the timer, still
        // set, rang 37 ms after that for the next. A refresh taken another
        // way, as a key's own change or a daemon's answer is, takes the one
        // that waited with it.
        const waiting = Pacing.heard(Pacing.took(1000), 1100, every);
        check(failures, "one waits", waiting.pace.due, 1250);
        const elsewhere = Pacing.took(1356);
        check(failures, "a refresh taken while one waits takes the one that waited",
            elsewhere.due, null);
        const next = Pacing.heard(elsewhere, 1393, every);
        check(failures, "so the next change waits a pace from it", next.pace.due, 1606);

        // A key, or an act's refresh, is drawn by a frame begun after it: a
        // step just after Escape was timed by Escape's frame, under way as
        // its handler ran (keeps-pace under load, 2026-11-10).
        const typed = Pacing.refreshedActs(Pacing.unframed(), ["step"]);
        check(failures, "a swap of a frame begun before a key isn't the key's",
            Pacing.frameSwapped(typed).shown.length, 0);
        const drawing = Pacing.frameBegun(typed);
        check(failures, "a frame begun after it draws it",
            Pacing.frameSwapped(drawing).shown.join(), "step");
        const another = Pacing.refreshedActs(drawing, ["letter"]);
        const swapped = Pacing.frameSwapped(another);
        check(failures, "a key typed while that frame is drawn isn't drawn by it",
            swapped.shown.join(), "step");
        check(failures, "but waits for the next", swapped.acts.waiting.join(), "letter");
        check(failures, "which nothing yet draws", swapped.acts.framing.length, 0);

        // A change just after a key that acted is its own, shown at once;
        // one just after a key that only moved is no key's, and waits on the
        // pace, since Now's change each second and Agents' every few were
        // made again on Joel's arrows (Joel, 2026-11-08: "weird lag can
        // happen").
        const within = 50;
        check(failures, "a change 10 ms after an act is its own",
            Pacing.keyed(10, false, within, true), true);
        check(failures, "a change 10 ms after an arrow is no key's",
            Pacing.keyed(10, true, within, true), false);
        check(failures, "nor one 60 ms after an act",
            Pacing.keyed(60, false, within, true), false);
        // Only a place on screen as it changes is a key's: Runs changing
        // just after a key in Emoji, with none of its rows shown, was timed
        // as that key's act, and its refresh, drawing nothing, waited 631 ms
        // for a frame something else drew (Joel's journal, 2026-11-11).
        check(failures, "a change to a place not shown is no key's",
            Pacing.keyed(10, false, within, false), false);

        // Counted from the key's work done, not its press, so a handler
        // slowed past `within` before it acts keeps the change it made.
        check(failures, "a change within a key's work comes no time after it",
            Pacing.sinceKey(1080, 1000, 900), 0);
        check(failures, "so a slow handler's act is its own",
            Pacing.keyed(Pacing.sinceKey(1080, 1000, 900), false, within, true), true);
        check(failures, "one after the work is timed from its end",
            Pacing.sinceKey(1090, 1000, 1070), 20);
        check(failures, "and one long after it is no key's",
            Pacing.keyed(Pacing.sinceKey(1200, 1000, 1070), false, within, true), false);

        // A slow act's line says where its time went, each span once, so
        // the 534 ms the old line left out (agents, 595 ms, Joel's journal,
        // 14 Nov 09:39) is read as the key's work, the change's wait for
        // its turn, or the collector.
        const act = { key: "Return", name: "agents", by: "the shell", heard: 1000,
            at: 1002, done: 1534, begun: 1534, collected: 40 };
        check(failures, "a slow act says the key, its work, its turn and the collector",
            Pacing.actSaid(act, 1595, 1530, 440),
            "launcher: what Return changed in agents took 595 ms to its frame: its work "
            + "took 530 ms; the shell told it 2 ms after the key, its turn came 532 ms "
            + "later, its refresh began 0 ms after that and was drawn 61 ms after it; "
            + "the collector ran 400 ms of the 595");
        check(failures, "a key named by nothing is a key",
            Pacing.actSaid(Object.assign({}, act, { key: "" }), 1595, 1530, 40)
                .startsWith("launcher: what a key changed in agents took 595 ms"), true);
        check(failures, "and one that says what moved ends with it",
            Pacing.actSaid(Object.assign({}, act, { why: "its rows" }), 1595, 1530, 440)
                .endsWith("the collector ran 400 ms of the 595; what moved: its rows"), true);

        // What moved for a keyed act names each telling for its key, so
        // the For You case drawn as Return's act now and then (16 Nov)
        // says whether Now's rows moved, Now said so outright, or the
        // launcher refreshed it itself.
        check(failures, "a source's own word is what moved",
            Pacing.movedFor("", "its rows", undefined, 0), "its rows");
        check(failures, "the launcher's refresh names what for",
            Pacing.movedFor("", "", "a row taken back", 0), "a row taken back");
        check(failures, "and neither is one said outright",
            Pacing.movedFor("", "", undefined, 0), "said outright, no rows compared");
        const outright = Pacing.movedFor("", "", undefined, 0);
        check(failures, "a later telling through its rows follows, with when",
            Pacing.movedFor(outright, "its own rows as it stepped", undefined, 12.4),
            "said outright, no rows compared, then its own rows as it stepped 12 ms on");
        const twice = Pacing.movedFor(outright, "its rows", undefined, 3);
        check(failures, "and one said already isn't said again",
            Pacing.movedFor(twice, "its rows", undefined, 9), twice);
        check(failures, "nor the first again",
            Pacing.movedFor(twice, "", undefined, 9), twice);

        costing(failures);
        framing(failures);
        presenting(failures);
        wanting(failures);
        waking(failures);
        naming(failures);
        opening(failures);
        felt(failures);
        weighing(failures);
        weighingPlaces(failures);
        spotting(failures);
        flashing(failures);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

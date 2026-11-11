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
// what's typed, the steps and the mode. What was presented is said by
// place, each change timed from the earliest line not yet drawn
// (launcher/Presented.js). A closing makes again only the spots of the
// things whose stamp moved (launcher/Spots.js).
import QtQuick
import "Pacing.js" as Pacing
import "Presented.js" as Presented
import "Spots.js" as Spots
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
        const jittered = tick(Pacing.feltBegun(0), [30, 46, 62, 78, 94, 110]);
        check(failures, "a tick a frame late isn't a stall", jittered.usable, 0);
        check(failures, "once usable, a later stall changes nothing",
            tick(freed, [5000]).usable, 1500);
    }

    // Agents says how its sessions stand at home, and People is named
    // there by who it lists (Joel, 2026-11-08: "weird lag can happen").
    function wanting(failures) {
        const agents = { saysAtHome: true };
        const people = { saysAtHome: false };
        const at = fields => Object.assign({ cataloguing: false, open: true, held: [],
            becoming: [], typing: false, entering: null }, fields);
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
        check(failures, "nor opening into another place before its step is held",
            read({ entering: people }), false);
        check(failures, "though opening into one that reads them they are",
            read({ entering: now }), true);
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
    }

    // A kept opening's sources wake in the order home's rows read them,
    // a place's own first and then those it holds, then the rest.
    function waking(failures) {
        const power = { name: "power" };
        const disk = { name: "disk" };
        const on = { name: "on" };
        const people = { name: "people" };
        const quests = { name: "quests" };
        const sources = { power, disk, on, people, quests };
        const places = {
            machine: { stands: "power", holds: ["disk", "battery"] },
            battery: { stands: "power", holds: [] }
        };
        const order = Wanted.wakingOrder(["on", "machine", "people", "gone"],
            id => places[id] ?? null, id => sources[id] ?? null,
            [quests, people, on, power, disk]);
        check(failures, "home's own come first, each once, then the rest",
            order.map(one => one.name).join(" "), "on power disk people quests");
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
        const said = Presented.changesSaid([
            { place: "windows", from: 100, at: 130 },
            { place: "quests", from: 0, at: 70 },
            { place: "windows", from: 200, at: 290 }
        ]);
        check(failures, "a line a place, by name", said.join("\n"),
            "launcher: 1 change in quests presented a median 70 ms from the line, "
            + "the slowest 70 ms, 0 within 50 ms\n"
            + "launcher: 2 changes in windows presented a median 30 ms from the line, "
            + "the slowest 90 ms, 1 within 50 ms");
        check(failures, "none presented says nothing", Presented.changesSaid([]).length, 0);

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
            + "1 within 50 ms: a median 10 ms before the shell heard it, 20 in the shell "
            + "and 20 from its frame's start");
        check(failures, "keys said landed say how long alone",
            Presented.keysSaid("landed", keys.slice(0, 1), false),
            "launcher: 1 keys landed a median 100 ms from the key, the slowest 100 ms, "
            + "0 within 50 ms");
        check(failures, "as do keys timed with no frame begun",
            Presented.keysSaid("presented", [{ from: 0, at: 70 }], true),
            "launcher: 1 keys presented a median 70 ms from the key, the slowest 70 ms, "
            + "0 within 50 ms");
        check(failures, "no keys say nothing", Presented.keysSaid("presented", [], true), "");
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

    function run() {
        const failures = [];
        const every = 250;

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

        framing(failures);
        presenting(failures);
        wanting(failures);
        waking(failures);
        opening(failures);
        felt(failures);
        weighing(failures);
        weighingPlaces(failures);
        spotting(failures);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

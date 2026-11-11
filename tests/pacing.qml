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
// once the words or the face have changed.
import QtQuick
import "Pacing.js" as Pacing
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
            becoming: [], typed: "" }, fields);
        const wanted = (fields, source) => Wanted.wanted(at(fields), source);
        check(failures, "closed, no source's rows are wanted", wanted({ open: false }, people),
            false);
        check(failures, "walked from outside, every source's are",
            wanted({ open: false, cataloguing: true }, agents), true);
        check(failures, "at home, a place home names is", wanted({}, people), true);
        check(failures, "and one saying what it holds itself isn't", wanted({}, agents), false);
        check(failures, "what's typed at home searches every place",
            wanted({ typed: "ma" }, agents), true);
        const inAgents = [{ row: { from: agents } }];
        check(failures, "held, its own are", wanted({ held: inAgents }, agents), true);
        check(failures, "and another place's aren't", wanted({ held: inAgents }, people), false);
        check(failures, "nor with something typed there",
            wanted({ held: inAgents, typed: "ma" }, people), false);
        check(failures, "held deeper, in one of its rows, its own still are",
            wanted({ held: inAgents.concat([{ row: { from: null } }]) }, agents), true);
        check(failures, "an act of its own waiting to be agreed keeps them wanted elsewhere",
            wanted({ held: [{ row: { from: people } }], becoming: [{ from: agents }] }, agents),
            true);
        const asleep = { saysAtHome: false, asleep: true };
        check(failures, "as a kept opening fills in, one not yet woken isn't",
            wanted({}, asleep), false);
        check(failures, "what's typed searches every place, woken or not",
            wanted({ typed: "ma" }, asleep), true);
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
        const warming = { saysAtHome: false, warming: true };
        check(failures, "shut, one warming for home is", wanted({ open: false }, warming), true);
        check(failures, "and no other", wanted({ open: false }, people), false);
        check(failures, "nor one home doesn't name",
            wanted({ open: false }, { saysAtHome: true, warming: true }), false);
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
            Pacing.keyed(10, false, within), true);
        check(failures, "a change 10 ms after an arrow is no key's",
            Pacing.keyed(10, true, within), false);
        check(failures, "nor one 60 ms after an act", Pacing.keyed(60, false, within), false);

        // Counted from the key's work done, not its press, so a handler
        // slowed past `within` before it acts keeps the change it made.
        check(failures, "a change within a key's work comes no time after it",
            Pacing.sinceKey(1080, 1000, 900), 0);
        check(failures, "so a slow handler's act is its own",
            Pacing.keyed(Pacing.sinceKey(1080, 1000, 900), false, within), true);
        check(failures, "one after the work is timed from its end",
            Pacing.sinceKey(1090, 1000, 1070), 20);
        check(failures, "and one long after it is no key's",
            Pacing.keyed(Pacing.sinceKey(1200, 1000, 1070), false, within), false);

        framing(failures);
        wanting(failures);
        waking(failures);
        opening(failures);
        felt(failures);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

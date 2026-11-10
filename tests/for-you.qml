// How For You weighs what needs Joel, run in Qt's own JavaScript engine, the
// one the shell runs it in. The `for-you-weighs-what-needs-joel` check copies
// modules/quickshell/launcher/ForYou.js and Order.js beside this file and
// runs `qml -platform offscreen for-you.qml`. Each case is one of the design's
// rules (VKCF3Z, 2026-11-08): what blocks a session counts most, then what's
// due, what he can do now, its kind and how long it has waited; the heaviest
// leads and ties keep the order they came in; a to-do, or a look or a thing to
// try that blocks or is due, needs him now, and the rest stand in their kind's
// row, the oldest first among ties; a row's side says its strongest
// reason; the looks folder's files are read into what waits and what's
// history, a file that can't be read left out; and what other sources give
// For You, as Passwords' one to-do, is gathered into what waits and what's
// done, and weighed beside the rest.
import QtQuick
import "ForYou.js" as ForYou

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
        const expect = (what, got, wanted) => {
            if (JSON.stringify(got) !== JSON.stringify(wanted))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
        };
        const hour = 3600 * 1000;
        const now = Date.parse("2026-11-08T14:00:00Z");
        // Each part's points, by its id, as the weight's parts say them.
        const points = weighed => {
            const by = {};
            for (const part of weighed.parts)
                by[part.id] = part.points;
            return by;
        };
        const part = (thing, id) => points(ForYou.weigh(thing, now))[id];

        // Blocking: a session wanting Joel blocks one, itself, and each
        // more it blocks adds less than the first.
        expect("blocking one session", part({ kind: "todo", blocks: 1 }, "blocks"), 40);
        expect("blocking three sessions", part({ kind: "todo", blocks: 3 }, "blocks"), 70);
        expect("blocking none", part({ kind: "todo", blocks: 0 }, "blocks"), 0);

        // Due: past its deadline, within the hour, within the day, or later.
        const due = at => part({ kind: "todo", due: at }, "due");
        expect("past its deadline", due(now - 60 * 1000), 40);
        expect("due within the hour", due(now + 30 * 60 * 1000), 30);
        expect("due within the day", due(now + 5 * hour), 10);
        expect("due in three days", due(now + 72 * hour), 0);
        expect("due as an ISO time", due("2026-11-08T14:30:00Z"), 30);
        expect("no deadline", due(null), 0);

        // Waited: rising slowly with the hours, never past 20.
        const waited = hours => part({ kind: "look", since: now - hours * hour }, "waited");
        expect("waited no time", waited(0), 0);
        expect("waited an hour", waited(1), 5);
        expect("waited seven hours", waited(7), 15);
        expect("waited a thousand hours, capped", waited(1000), 20);
        expect("no time it began", part({ kind: "look" }, "waited"), 0);

        // Ready, and its kind.
        expect("ready now", part({ kind: "todo", ready: true }, "ready"), 20);
        expect("not ready", part({ kind: "todo", ready: false }, "ready"), 0);
        expect("a thing to do", part({ kind: "todo" }, "kind"), 10);
        expect("a thing to look at", part({ kind: "look" }, "kind"), 5);

        // The parts in their order, and the weight their whole sum.
        const item = { kind: "todo", blocks: 2, ready: true, due: now + 30 * 60 * 1000,
            since: now - hour };
        expect("each part in its order", ForYou.weigh(item, now).parts.map(one => one.id),
            ["blocks", "due", "ready", "kind", "waited"]);
        expect("an item's weight", ForYou.weigh(item, now).weight, 55 + 30 + 20 + 10 + 5);
        expect("a session's weight", ForYou.weigh({ kind: "todo", blocks: 1, since: now },
            now).weight, 50);
        expect("a look's weight", ForYou.weigh({ kind: "look", since: now - 7 * hour },
            now).weight, 20);

        // The heaviest first, those alike in the order they came.
        expect("heaviest first, ties as they came", ForYou.ordered([
            { name: "a", weight: 15 }, { name: "b", weight: 50 },
            { name: "c", weight: 15 }, { name: "d", weight: 50 }, { name: "e", weight: 70 }
        ]).map(one => one.name), ["e", "b", "d", "a", "c"]);

        // The bands: a to-do needs Joel now, and a look or a thing to try
        // only while it blocks a session or is due within the day; the rest
        // stand in their kind's own row (VKCF3Z, 2026-11-09).
        const band = (kind, thing) => {
            const bands = ForYou.banded([{ kind, thing, name: "it" }], now);
            return Object.keys(bands).filter(key => bands[key].length > 0);
        };
        expect("a to-do needs him now", band("do", { kind: "todo" }), ["now"]);
        expect("a look that blocks a session is lifted", band("look",
            { kind: "look", blocks: 1 }), ["now"]);
        expect("a look due within the day is lifted", band("look",
            { kind: "look", due: now + 5 * hour }), ["now"]);
        expect("a look overdue is lifted", band("look", { kind: "look", due: now - hour }),
            ["now"]);
        expect("a look that doesn't goes in To Look At", band("look",
            { kind: "look", since: now - 7 * hour }), ["look"]);
        expect("a look due in three days goes in To Look At", band("look",
            { kind: "look", due: now + 72 * hour }), ["look"]);
        expect("a thing to try that blocks is lifted", band("try",
            { kind: "look", blocks: 2 }), ["now"]);
        expect("a thing to try that doesn't goes in To Try", band("try",
            { kind: "look", since: now - hour }), ["try"]);
        expect("a kind with no row of its own needs him now", band("notice",
            { kind: "look" }), ["now"]);
        // Inside each, the heaviest first, those alike the oldest first,
        // then as they came, and each item keeps what its caller gave it.
        const looked = ForYou.banded([
            { kind: "look", thing: { kind: "look", since: now - hour }, name: "a" },
            { kind: "look", thing: { kind: "look", since: now - 1000 * hour }, name: "b" },
            { kind: "look", thing: { kind: "look", since: now - 1001 * hour }, name: "c" },
            { kind: "look", thing: { kind: "look" }, name: "d" },
            { kind: "look", thing: { kind: "look", since: now - hour }, name: "e" },
            { kind: "do", thing: { kind: "todo" }, name: "f" }
        ], now);
        expect("a band heaviest first, ties the oldest first", looked.look.map(one =>
            one.name), ["c", "b", "a", "e", "d"]);
        expect("each given its weight", looked.look.map(one => one.weight),
            [25, 25, 10, 10, 5]);
        expect("the rest in their own band", [looked.now.map(one => one.name),
            looked.try.length], [["f"], 0]);
        expect("no items, three empty bands", ForYou.banded(undefined, now),
            { now: [], look: [], try: [] });
        // Each item's time is read once, never at each of its sort's
        // comparisons, since To Try holds every landing with a line to try
        // that Joel hasn't read, its time as text, and each opening bands
        // them more than once. A library's Date is its own, so the reads are
        // counted at each item's time instead: one to weigh it, one to
        // order it.
        let timesRead = 0;
        const tries = Array.from({ length: 64 }, (_, at) => {
            const since = new Date(now - (at % 7) * hour).toISOString();
            const thing = { kind: "look", ready: true };
            Object.defineProperty(thing, "since", { enumerable: true, get: () => {
                timesRead += 1;
                return since;
            } });
            return { kind: "try", thing, name: `t${at}` };
        });
        const tried = ForYou.banded(tries, now);
        expect("a band's times read once each", timesRead, 2 * tries.length);
        expect("a band by time read as text, the oldest first", tried.try.slice(0, 2)
            .map(one => one.name), ["t6", "t13"]);

        // A look blocks the session that left it while that session wants
        // Joel, named by its session or its address, as the session's own
        // address, its presence's session or an address it answers to.
        const left = { session: "S1", address: "", since: now - hour };
        const wanting = presence => [{ address: "A9", presence: presence }];
        const lookBand = (one, sessions) => band("look", ForYou.lookThing(one, sessions));
        expect("a look whose session wants Joel is lifted",
            lookBand(left, wanting({ session: "S1" })), ["now"]);
        expect("as is one naming its session by an address it answers to",
            lookBand({ session: "", address: "A2" }, wanting({ addresses: ["A2"] })), ["now"]);
        expect("or by the session's own address",
            lookBand({ address: "A9" }, wanting({})), ["now"]);
        expect("a look whose session doesn't want Joel goes in To Look At",
            lookBand(left, wanting({ session: "S2", addresses: ["A2"] })), ["look"]);
        expect("as does one with no session named", lookBand({ session: "" },
            wanting({ session: "" })), ["look"]);
        expect("its thing counts each session it blocks, and keeps its times",
            ForYou.lookThing({ session: "S1", due: now + hour, since: now - hour },
                wanting({ session: "S1" }).concat(wanting({ session: "S1" }))),
            { kind: "look", blocks: 2, due: now + hour, since: now - hour });

        // What blocks a session: one whose want is an action its quest or
        // its file's address holds.
        const sessions = [
            { presence: { wants: { kind: "action" }, owns: ["QEST-1"], addresses: ["AAA"] } },
            { presence: { wants: { kind: "action" }, owns: [], addresses: ["BBB"] } },
            { presence: { wants: { kind: "question" }, owns: ["QEST-1"], addresses: ["CCC"] } },
            { presence: { wants: { kind: "action" }, owns: ["QEST-2"], addresses: ["DDD"] } }
        ];
        expect("the sessions an item blocks",
            ForYou.blocked({ quest: "QEST-1", from: "BBB" }, sessions), 2);
        expect("an item blocking none", ForYou.blocked({ quest: "QEST-9", from: "ZZZ" },
            sessions), 0);

        // The words the side and the weight are said in, as words.json
        // keeps them, and the clock read in UTC so the case holds anywhere.
        const say = {
            blocks: "blocks {count} session",
            blocksMany: "blocks {count} sessions",
            due: "due {time}",
            overdue: "overdue since {time}",
            waitedFor: "waiting {span}",
            spanMinutes: "{count} min",
            spanHours: "{count} h",
            spanDays: "{count} d",
            todo: "to do",
            look: "to look at",
            history: "{how} {time}",
            opened: "opened",
            cleared: "cleared",
            resolved: "resolved",
            clock: at => new Date(at).toISOString().slice(11, 16),
            ago: at => `${Math.round((now - at) / 60000)} min ago`,
            day: at => (now - at >= 24 * hour ? "yesterday" : "")
        };
        const reason = thing => ForYou.reason(thing, now, say);
        expect("blocking beats due", reason({ blocks: 2, due: now + hour, since: now - hour }),
            "blocks 2 sessions");
        expect("blocking one", reason({ blocks: 1 }), "blocks 1 session");
        expect("due beats when", reason({ due: now + hour, since: now - hour }), "due 15:00");
        expect("overdue", reason({ due: now - hour, since: now - 2 * hour }),
            "overdue since 13:00");
        expect("when, with nothing stronger", reason({ since: now - 12 * 60 * 1000 }),
            "12 min ago");
        expect("and before today its day, never a count of days",
            reason({ since: now - 30 * hour }), "yesterday");
        expect("nothing to say", reason({}), "");

        // The weight in words: the total, then each part that counts.
        expect("the weight said", ForYou.weightSaid(item, now, say),
            "120 \u00b7 blocks 2 sessions \u00b7 due 14:30 \u00b7 waiting 1 h");
        expect("a look's weight said, its kind left to its eyebrow", ForYou.weightSaid({
            kind: "look", since: now - 12 * 60 * 1000 }, now, say), "6 \u00b7 waiting 12 min");

        // The looks folder's files, each read whole or left out.
        const lookOf = fields => JSON.stringify(Object.assign({
            v: 1, id: "L1", at: "2026-11-08T10:00:00Z", session: "S1", kind: "pr",
            target: "https://github.com/o/r/pull/1", title: "Review the change",
            why: "it touches the launcher", state: "waiting"
        }, fields));
        const files = [
            { file: "a.json", text: lookOf({ id: "A", at: "2026-11-08T10:00:00Z" }) },
            { file: "bad.json", text: "{ not json" },
            { file: "b.json", text: lookOf({ id: "B", at: "2026-11-08T09:00:00Z" }) },
            { file: "v2.json", text: lookOf({ id: "V2", v: 2 }) },
            { file: "untitled.json", text: lookOf({ id: "U", title: "" }) },
            { file: "aimless.json", text: lookOf({ id: "T", target: undefined }) },
            { file: "odd.json", text: lookOf({ id: "K", kind: "song" }) },
            { file: "c.json", text: lookOf({ id: "C", state: "opened", how: "opened",
                resolvedAt: "2026-11-08T13:10:00Z" }) },
            { file: "d.json", text: lookOf({ id: "D", state: "cleared", how: "cleared by you",
                resolvedAt: "2026-11-08T13:30:00Z" }) },
            { file: "e.json", text: lookOf({ id: "E", state: "resolved", how: "merged",
                resolvedAt: "2026-11-08T14:02:00Z" }) },
            { file: "f.json", text: lookOf({ id: "F", state: "resolved",
                resolvedAt: "2026-11-08T12:00:00Z" }) },
            { file: "g.json.writing", text: lookOf({ id: "G" }) },
            { file: "null.json", text: "null" }
        ];
        const read = ForYou.looks(files);
        expect("those waiting, in the order they came", read.waiting.map(one => one.id),
            ["B", "A"]);
        expect("the rest, newest first", read.history.map(one => one.id), ["E", "D", "C", "F"]);
        expect("a look read whole", read.waiting[0].title, "Review the change");
        expect("a look's time as a number", read.waiting[0].since,
            Date.parse("2026-11-08T09:00:00Z"));

        // History's side: how it went and when.
        const side = id => ForYou.historySide(read.history.find(one => one.id === id), say);
        expect("one merged", side("E"), "merged 14:02");
        expect("one opened", side("C"), "opened 13:10");
        expect("one cleared by Joel", side("D"), "cleared by you 13:30");
        expect("one saying no how, by its state", side("F"), "resolved 12:00");

        // A look written again as it's done with, whole, with what it was.
        const opened = JSON.parse(ForYou.rewritten(read.waiting[0], "opened", "opened", now));
        expect("a look opened, written whole", [opened.v, opened.id, opened.state, opened.how,
            opened.resolvedAt, opened.title, opened.target, opened.since, opened.file],
            [1, "B", "opened", "opened", "2026-11-08T14:00:00.000Z", "Review the change",
                "https://github.com/o/r/pull/1", undefined, undefined]);

        // The to-dos Joel said were done, newest first, and none
        // he put off or that still waits.
        const identity = each => `${each.quest}:${each.id}`;
        const asked = [{ quest: "Q", id: "1" }, { quest: "Q", id: "2" }, { quest: "Q", id: "3" },
            { quest: "Q", id: "4" }];
        expect("the items done, newest first", ForYou.settledItems(asked,
            { done: { "Q:1": now - hour, "Q:2": now - 30 * 60 * 1000 },
                notNow: { "Q:3": { said: "", at: now } } }, identity)
            .map(one => [one.item.id, one.resolved]),
        [["2", now - 30 * 60 * 1000], ["1", now - hour]]);

        // What an item's owner said of it, as its drawer's facts say it:
        // a fact for each, "" where nothing says it (VKCF3Z, 2026-11-08).
        const ownSay = {
            moment: "{day} {time}",
            // When.js's day, its date where it isn't today's, in UTC here.
            day: (at, today) => new Date(at).toISOString().slice(0, 10)
                === new Date(today).toISOString().slice(0, 10) ? ""
                : `${new Date(at).getUTCDate()} Nov`,
            time: at => new Date(at).toISOString().slice(11, 16)
        };
        const owned = { quest: "QEST-1", deadline: "2026-11-08T15:00:00Z",
            at: "2026-11-09T09:30:00Z", where: { machine: "harbour" },
            owner: { address: "8IDU4E" },
            doneWhen: { check: "test -e /run/booted-system/sw/bin/ok" } };
        const figures = (item, told) => ForYou.figures(item,
            Object.assign({ now, window: "" }, told ?? {}), ownSay);
        const later = Object.assign({}, owned, { deadline: "2026-11-12T15:00:00Z" });
        // Later's rows aren't weighed, so their side says what they wait
        // for, never the deadline, even blocking nothing (Y2YBTS).
        expect("due for one under Later, days off, blocking nothing",
            figures(later).due, "12 Nov 15:00");
        expect("no due while the side says the deadline",
            figures(later, { sideSaysDue: true }).due, "");
        expect("no due while the weight says it, within the day", figures(owned).due, "");
        expect("set for another day, by its day", figures(owned).setFor, "9 Nov 09:30");
        expect("no deadline, no due", figures({}).due, "");
        expect("a moment as For You says each, today's its time alone", [
            ForYou.moment("2026-11-08T15:00:00Z", now, ownSay),
            ForYou.moment("2026-11-09T09:30:00Z", now, ownSay),
            ForYou.moment(null, now, ownSay)
        ], ["15:00", "9 Nov 09:30", ""]);
        expect("the machine as its owner names it", figures(owned).machine, "harbour");
        expect("where, the window it's done in", figures(owned, { window: "pi \u00b7 core" })
            .where, "pi \u00b7 core");
        expect("where, nowhere with no window open", figures(owned).where, "");
        expect("its owner's address, as written", figures(owned).address, "8IDU4E");
        expect("its check's command, as written", figures(owned).check,
            "test -e /run/booted-system/sw/bin/ok");
        expect("a session, how its quest stands said only where it isn't active",
            [ForYou.sessionSaid("pi \u00b7 core", "active"),
                ForYou.sessionSaid("pi \u00b7 core", "paused"),
                ForYou.sessionSaid("pi \u00b7 core", undefined), ForYou.sessionSaid("", "paused")],
            ["pi \u00b7 core", "pi \u00b7 core \u00b7 paused", "pi \u00b7 core", ""]);
        expect("one a person says is done has no check",
            figures({ doneWhen: { by: "person" } }).check, "");

        // Its mark: For You's own tray, never the person with a clock its
        // to-dos wear, emptied only when it holds nothing now or later
        // (VKCF3Z, 2026-11-08).
        expect("its mark holding one now", ForYou.markOf(1, 0), "forYou");
        expect("its mark holding only later", ForYou.markOf(0, 2), "forYou");
        expect("its mark holding nothing", ForYou.markOf(0, 0), "forYouEmpty");
        expect("never its to-dos' mark", [ForYou.markOf(0, 0), ForYou.markOf(3, 1)]
            .includes("waitsOnJoel"), false);

        // What other sources give For You (ForYou.js's given), as
        // Passwords gives its one to-do for what its last check found: each
        // to do weighed beside the rest, each done kept for History, and a
        // source giving nothing, or something neither, left out.
        const todo = { row: { name: "Change the Passwords the Check Found" },
            thing: { kind: "todo", since: now - 7 * hour, takes: 45, ready: true } };
        const clear = { row: { name: "Change the Passwords the Check Found" },
            done: { how: "checked clear", at: now - hour } };
        const sources = [{ forYou: [todo] }, {}, { forYou: null },
            { forYou: [clear, { row: { name: "neither" } }, null] }, null];
        const gathered = ForYou.given(sources);
        expect("those to do, gathered from every source", gathered.waiting.map(one =>
            one.row.name), ["Change the Passwords the Check Found"]);
        expect("those done, with how and when", gathered.done.map(one => [one.row.name,
            one.done.how, one.done.at]), [["Change the Passwords the Check Found",
            "checked clear", now - hour]]);
        expect("a done one's time, read from an ISO time", ForYou.given([{ forYou: [{
            row: {}, done: { how: "", at: "2026-11-08T13:00:00Z" } }] }]).done[0].done.at,
        now - hour);
        expect("no sources give nothing", ForYou.given(undefined), { waiting: [], done: [] });
        // Ready to do and waiting seven hours, the to-do weighs less than a
        // session waiting on Joel, which blocks, and more than a look.
        const passwords = ForYou.weigh(todo.thing, now).weight;
        expect("the passwords to-do's weight", passwords, 20 + 10 + 15);
        expect("weighed beside the rest", ForYou.ordered([
            { name: "look", weight: ForYou.weigh({ kind: "look", since: now - 7 * hour },
                now).weight },
            { name: "passwords", weight: passwords },
            { name: "session", weight: ForYou.weigh({ kind: "todo", blocks: 1, since: now },
                now).weight }
        ]).map(one => one.name), ["session", "passwords", "look"]);

        // How many each band holds, as a row's side says it.
        const bandWords = { toDo: "{count} to do", toLookAt: "{count} to look at",
            toTry: "{count} to try" };
        expect("every band, needed most first", ForYou.bandsRun({ now: 1, look: 44, try: 2 },
            bandWords).map(item => [item.said, item.keep]),
            [["1 to do", 3], ["44 to look at", 2], ["2 to try", 1]]);
        expect("a band holding none left out", ForYou.bandsRun({ now: 0, look: 44, try: 2 },
            bandWords).map(item => item.said), ["44 to look at", "2 to try"]);
        expect("a band not given counts none", ForYou.bandsRun({ try: 2 }, bandWords)
            .map(item => item.said), ["2 to try"]);
        expect("all empty, nothing", ForYou.bandsRun({ now: 0, look: 0, try: 0 }, bandWords),
            []);

        // What For You's drawer names as its heaviest.
        const lead = [{ said: "Answer the ask", keep: 1 }, { said: "2 min", keep: 2 },
            { said: "2 more", keep: 3 }];
        expect("what needs Joel by its title and minutes, never how many more",
            ForYou.heaviest(lead, "Read the plan"), "Answer the ask \u00b7 2 min");
        expect("one with no minutes by its title", ForYou.heaviest(lead.slice(0, 1), ""),
            "Answer the ask");
        expect("nothing to do, the heaviest look", ForYou.heaviest([], "Read the plan"),
            "Read the plan");
        expect("only things to try, none", ForYou.heaviest([], undefined), "");

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            // Qt.exit only asks the engine to stop once this returns.
            Qt.exit(1);
            return;
        }
        console.info("For You weighs what needs Joel");
        Qt.exit(0);
    }
}

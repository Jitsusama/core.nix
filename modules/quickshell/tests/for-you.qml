// How For You weighs what needs Joel, run in Qt's own JavaScript engine, the
// one the shell runs it in. The `for-you-weighs-what-needs-joel` check copies
// modules/quickshell/launcher/ForYou.js, LookForm.js, Looks.js and
// Order.js beside this file and runs `qml -platform offscreen for-you.qml`. Each case
// is one of the design's rules (VKCF3Z, 2026-11-08): what blocks a session
// counts most, then what's due, what he can do now, its kind and how long it
// has waited; the heaviest leads and ties keep the order they came in; a
// to-do, or a look or a thing to try that blocks or is due, needs him now,
// and the rest stand in their kind's row, the oldest first among ties; a
// row's side says its strongest reason; the looks folder's files are read
// into what waits and what's history, a file that can't be read left out,
// only one that changed parsed again, and one waiting three days gone quiet;
// what other sources give For You, as Passwords' one to-do, is gathered into
// what waits and what's done, and weighed beside the rest. How each stands
// under its cell is for-you-cells.qml's. Long because a look's cases, its
// second form's and its answers' among them, read the same records the
// weighing cases build, in the one run of the files its check copies;
// LookForm.js's own cases are the first to go to a file of their own if
// it grows past this.
import QtQuick
import "ForYou.js" as ForYou
import "LookForm.js" as LookForm
import "Looks.js" as Looks

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
        // What agents ask stands together in Agents Asking, never loose
        // among the to-dos, however it presses, once two ask (Joel,
        // 2026-11-18 10:18); one alone stands as itself, so Super+O,
        // Return, Return still reaches it (docs/design.md).
        const asks = (...others) => ForYou.banded([
            { kind: "agent", thing: { kind: "todo", blocks: 1, since: now - hour }, name: "a" },
            { kind: "agent", thing: { kind: "todo", blocks: 1, since: now - 9 * hour,
                due: now - hour }, name: "c" }
        ].concat(others), now);
        const placed = bands => [bands.now.map(one => one.name), bands.agent.map(one =>
            one.name), bands.agentAt];
        expect("two asks go in Agents Asking, even overdue, heaviest first",
            placed(asks()), [[], ["c", "a"], 0]);
        expect("one alone stands as itself", placed(ForYou.banded([{ kind: "agent",
            thing: { kind: "todo", blocks: 1 }, name: "a" }], now)), [["a"], [], 1]);
        // It stands where its heaviest weighs among what needs him now, so a
        // to-do blocking more still leads, as Reopen Them All does after a
        // boot, and a lighter one follows.
        expect("where its heaviest weighs", placed(asks(
            { kind: "do", thing: { kind: "todo", blocks: 9 }, name: "reopen" },
            { kind: "do", thing: { kind: "todo" }, name: "b" })),
            [["reopen", "b"], ["c", "a"], 1]);
        // A look that asks nothing stands To Read, never lifted, since it
        // asks nothing of him however long it's been (VKCF3Z, 2026-11-12).
        expect("a look that asks nothing goes in To Read", band("read",
            { kind: "look", since: now - hour }), ["read"]);
        expect("and is never lifted", band("read", { kind: "look", blocks: 1,
            due: now - hour }), ["read"]);
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
        expect("no items, five empty bands", ForYou.banded(undefined, now),
            { now: [], agent: [], agentAt: 0, look: [], try: [], read: [] });
        // The next moment time alone lifts a row into what needs him now, as
        // its due comes within the day, so For You bands again then: banded
        // only as its items changed, a look due tomorrow stayed in To Look
        // At past the hour it was due within the day (2026-11-14).
        const lifting = (kind, thing) => ({ kind, thing, name: "it" });
        const dueIn = hours => lifting("look", { kind: "look", due: now + hours * hour });
        expect("a look due in three days lifts a day before", ForYou.liftsAt([dueIn(72)], now),
            now + 48 * hour);
        const bandAt = (item, at) => {
            const bands = ForYou.banded([item], at);
            return Object.keys(bands).filter(key => bands[key].length > 0);
        };
        expect("banded a moment before, it still waits", bandAt(dueIn(72), now + 48 * hour - 1),
            ["look"]);
        expect("banded then, it needs him now", bandAt(dueIn(72), now + 48 * hour), ["now"]);
        expect("the soonest of several", ForYou.liftsAt([dueIn(72), lifting("try",
            { kind: "look", due: now + 30 * hour })], now), now + 6 * hour);
        expect("its due given as a time", ForYou.liftsAt([lifting("look", { kind: "look",
            due: new Date(now + 72 * hour).toISOString() })], now), now + 48 * hour);
        expect("one due within the day is lifted already", ForYou.liftsAt([dueIn(5)], now),
            null);
        expect("nothing due lifts nothing", ForYou.liftsAt([lifting("look",
            { kind: "look", since: now - hour })], now), null);
        expect("a to-do needs him now already", ForYou.liftsAt([lifting("do",
            { kind: "todo", due: now + 72 * hour })], now), null);
        expect("a look that asks nothing never lifts", ForYou.liftsAt([lifting("read",
            { kind: "look", due: now + 72 * hour })], now), null);
        expect("one that blocks a session is lifted already", ForYou.liftsAt([lifting("look",
            { kind: "look", blocks: 1, due: now + 72 * hour })], now), null);
        expect("no items, no lift", ForYou.liftsAt(undefined, now), null);
        // Each thing's times are read once in banding it, whatever its sort
        // weighs: read in the comparator, each ISO time was parsed again
        // for every pair, seconds of each opening on the laptop.
        const reads = { due: 0, since: 0 };
        const counted = at => {
            const thing = { kind: "look" };
            Object.defineProperty(thing, "since", { get: () => {
                reads.since += 1;
                return new Date(now - (at % 7) * hour).toISOString();
            } });
            Object.defineProperty(thing, "due", { get: () => {
                reads.due += 1;
                return null;
            } });
            return { kind: "look", thing, name: `${at}` };
        };
        const many = ForYou.banded(Array.from({ length: 50 }, (_, at) => counted(at)), now);
        expect("fifty things' times read once each", [reads.since, reads.due], [50, 50]);
        expect("read once, still the oldest first", many.look.slice(0, 3).map(one =>
            one.name), ["6", "13", "20"]);

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
            quiet: "went quiet",
            expired: "expired",
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
            { file: "null.json", text: "null" },
            { file: "h.json", text: lookOf({ id: "H", at: "2026-11-05T15:00:00Z" }) },
            { file: "o.json", text: lookOf({ id: "O", at: "2026-11-05T13:00:00Z" }) }
        ];
        const read = Looks.looks(files, {}, now);
        expect("those waiting, in the order they came", read.waiting.map(one => one.id),
            ["H", "B", "A"]);
        // One waiting past three days has gone quiet, in History from when it did, its file
        // left saying it waits (Joel, 2026-11-11; VKCF3Z's flows).
        expect("the rest, newest first, one gone quiet among them",
            read.history.map(one => one.id), ["E", "D", "C", "O", "F"]);
        const quiet = read.history.find(one => one.id === "O") ?? {};
        expect("one gone quiet, three days after it came", [quiet.state, quiet.resolved],
            ["quiet", Date.parse("2026-11-08T13:00:00Z")]);
        expect("a look read whole", read.waiting[0].title, "Review the change");
        expect("a look's time as a number", read.waiting[1].since,
            Date.parse("2026-11-08T09:00:00Z"));
        expect("none of them to read", read.reading, []);

        // A look that asks nothing is to read, not to look at, until it expires
        // 24 hours on into History, though no session has written it so yet; one
        // its session wrote expired, seen or replaced is in History (85PKB2,
        // 2026-11-12). One saying nothing of asking asks, as the harness's
        // older looks did.
        const aging = Looks.looks([
            { file: "r.json", text: lookOf({ id: "R", asks: false,
                expiresAt: "2026-11-09T13:00:00Z" }) },
            { file: "x.json", text: lookOf({ id: "X", asks: false,
                expiresAt: "2026-11-08T13:30:00Z" }) },
            { file: "y.json", text: lookOf({ id: "Y", asks: true }) },
            { file: "z.json", text: lookOf({ id: "Z", state: "expired",
                resolvedAt: "2026-11-08T12:00:00Z" }) },
            { file: "s.json", text: lookOf({ id: "S", state: "resolved", how: "seen",
                resolvedAt: "2026-11-08T11:00:00Z" }) }
        ], {}, now);
        expect("one asking nothing, to read", (aging.reading ?? []).map(one => one.id),
            ["R"]);
        expect("one asking, to look at", aging.waiting.map(one => one.id), ["Y"]);
        expect("one past its time, and those done, in History",
            aging.history.map(one => [one.id, one.state, one.resolved]),
            [["X", "expired", Date.parse("2026-11-08T13:30:00Z")],
                ["Z", "expired", Date.parse("2026-11-08T12:00:00Z")],
                ["S", "resolved", Date.parse("2026-11-08T11:00:00Z")]]);
        expect("one expired says so at its side", ForYou.historySide(aging.history[0], say),
            "expired 13:30");
        expect("an answer taken keeps those to read",
            Looks.taken(aging, { Y: { label: "Posted", at: now, taken: true } },
                label => label).reading?.map(one => one.id), ["R"]);

        // Read again keeping what was parsed, only a file that changed is
        // parsed again, and one gone is forgotten.
        const kept = {};
        const first = Looks.looks(files, kept, now);
        const changed = files.filter(one => one.file !== "e.json").map(one => one.file
            === "a.json" ? { file: one.file, text: lookOf({ id: "A", title: "Again" }) } : one);
        const again = Looks.looks(changed, kept, now);
        const byId = (them, id) => them.waiting.concat(them.history).find(one => one.id === id);
        expect("a file as it was, not parsed again", byId(again, "B") === byId(first, "B"), true);
        expect("a file that changed, parsed again", byId(again, "A").title, "Again");
        expect("a file gone, forgotten", ["e.json" in kept.by, "a.json" in kept.by], [false, true]);
        expect("a look gone quiet, kept as its file was read", kept.by["o.json"].look.state,
            "waiting");

        // A look's drawer says why it's there, then what Joel is asked and
        // his choices; an older look, saying no ask, gives only its why.
        const askingLook = Looks.looks([{ file: "q.json", text: lookOf({ id: "Q",
            ask: "Merge it, or say what to change." }) }], {}, now).waiting[0];
        expect("a look's words, its ask under its why", askingLook.words,
            "it touches the launcher\n\nMerge it, or say what to change.");
        expect("an older look's words, its why alone", read.waiting[1].words,
            "it touches the launcher");
        const wordsOf = (fields, id) => Looks.looks([{ file: `${id}.json`,
            text: lookOf(Object.assign({ id: id }, fields)) }], {}, now).waiting[0].words;
        expect("a look's words, its ask padded or only spaces, trimmed",
            [wordsOf({ ask: " Merge it? " }, "P"), wordsOf({ ask: "   " }, "W")],
            ["it touches the launcher\n\nMerge it?", "it touches the launcher"]);
        expect("a look's words, its why only spaces, or both, trimmed",
            [wordsOf({ why: "   ", ask: "Merge it?" }, "Y"),
                wordsOf({ why: " ", ask: " " }, "B")],
            ["Merge it?", ""]);

        // A look's choices are offered on its row, each answered by a steer to
        // the session that left it, or its quest's once that's ended, and shown
        // as given while the node may yet say it took it (85PKB2, 2026-11-12).
        const choosing = Looks.looks([{ file: "p.json", text: lookOf({ id: "P",
            address: "vp50pc", quest: "QEST-20261104-VP50PC", choices: [
                { value: "posted", label: "Posted" }, { value: "later" }, { label: "no value" },
                null, { value: "c" }, { value: "d" }, { value: "e" }] }) }], {}, now).waiting[0];
        expect("a look's choices, unlabelled ones by their value, four at most",
            (choosing.offers ?? []).map(one => [one.value, one.label]),
            [["posted", "Posted"], ["later", "later"], ["c", "c"], ["d", "d"]]);
        expect("an older look, offering none", read.waiting[1].offers, []);
        // A look said in its second form (85PKB2's harness #2008) says what
        // happened and why it matters in place of its why and ask, which its
        // harness made from them; what it presses, and what follows each
        // choice, from the choice itself, else what it wants back by label.
        // It asks Joel back only where it wants a choice or his own words, or
        // offers choices; its act is never a choice's or another act's name.
        const second = fields => Looks.looks([{ file: "s.json", text: lookOf(Object.assign({
            id: "S", why: "The bar keeps the branch. You couldn't see a cost.",
            ask: "Review It: Keep It, or Roll It Back.",
            happened: "The bar keeps the branch at narrow widths",
            matters: "At 76 columns you couldn't see what a session cost.",
            act: "Review It",
            choices: [{ value: "Keep It", label: "Keep It", follows: "It stays as it is." },
                { value: "Roll It Back", label: "Roll It Back" }],
            wants: { choices: [{ label: "Keep It", follows: "Not this one." },
                { label: "Roll It Back", follows: "The old bar comes back tonight." }] },
            about: { kind: "pr", target: "https://github.com/o/r/pull/1",
                title: "Review the change" }
        }, fields)) }], {}, now).waiting[0];
        const secondly = second({});
        expect("a second form's words, what happened then why it matters, never its ask",
            secondly.words, "The bar keeps the branch at narrow widths. At 76 columns you"
                + " couldn't see what a session cost.");
        expect("what follows each choice, its own, else what it wants back by label",
            secondly.offers.map(one => [one.label, one.follows]),
            [["Keep It", "It stays as it is."],
                ["Roll It Back", "The old bar comes back tonight."]]);
        expect("a choice saying nothing of what follows, follows with nothing",
            choosing.offers.map(one => one.follows), ["", "", "", ""]);
        expect("its words end as a sentence ends, after a closing quote or bracket too",
            second({ happened: "It said \u201cdone.\u201d", matters: "(It wasn't.)" }).words,
            "It said \u201cdone.\u201d (It wasn't.)");
        // A first form carrying the second form's fields, but no happened.
        const firstly = second({ happened: "" });
        expect("what it presses, in the second form only, never past four words or another's"
            + " name",
            [LookForm.actOf(secondly), LookForm.actOf(firstly), LookForm.actOf(read.waiting[1]),
                LookForm.actOf(second({ act: "keep it" })),
                LookForm.actOf(secondly, ["Clear It", "review it"]),
                LookForm.actOf(second({ act: "Read It All the Way" })),
                LookForm.actOf(second({ act: "Read the Whole Diff" })),
                LookForm.actOf(second({ act: " Review It " }))],
            ["Review It", "", "", "", "", "", "Read the Whole Diff", "Review It"]);
        expect("never another's name however it's spaced, stopped or hides what draws as"
            + " nothing",
            [LookForm.actOf(second({ act: "Keep It." })),
                LookForm.actOf(second({ act: "Keep  It" })),
                LookForm.actOf(second({ act: "Roll It Back\u2060" })),
                LookForm.actOf(second({ act: "Clear\u00a0It" }), ["Clear It"]),
                LookForm.actOf(second({ act: "Clear It\u200b!" }), ["Clear It"]),
                LookForm.actOf(second({ act: "Clear\u3164It" }), ["Clear It"]),
                LookForm.actOf(second({ act: "Cle\u034far It" }), ["Clear It"]),
                LookForm.actOf(second({ act: "Clear\u2800It" }), ["Clear It"]),
                LookForm.actOf(second({ act: "\u200b" })),
                LookForm.actOf(second({ act: "\u202etI raelC" }), ["Clear It"]),
                LookForm.actOf(second({ act: "\u2067ti weiveR\u2069" }))],
            ["", "", "", "", "", "", "", "", "", "", ""]);
        // A choice can't pass for one of the launcher's own acts either, since
        // pressing it to clear a look would answer it.
        expect("a choice named as one of the launcher's own acts poses, however written",
            [LookForm.poses("Clear It", ["Clear It", "Talk"]),
                LookForm.poses(" clear\u00a0it.", ["Clear It"]),
                LookForm.poses("Talk\u200b", ["Clear It", "Talk"]),
                LookForm.poses("Clear It\ufe0f", ["Clear It"]),
                LookForm.poses("Clear It\udb40\udc41", ["Clear It"]),
                LookForm.poses("Clear It\udb40\udd00", ["Clear It"]),
                LookForm.poses("Talk\u180b", ["Talk"]),
                LookForm.poses("Ta\u17b4lk", ["Talk"]),
                LookForm.poses("Clear It\ud834\udd73", ["Clear It"]),
                LookForm.poses("Talk\ud82f\udca0", ["Talk"]),
                LookForm.poses("Clear\u2065 It\ufff0", ["Clear It"]),
                LookForm.poses("Clear It\udb40\uddf0", ["Clear It"]),
                LookForm.poses("\u202etI raelC", ["Clear It"]),
                LookForm.poses("\u2067klaT\u2069", ["Talk"]),
                LookForm.poses("Keep It\u200f", ["Clear It"]),
                LookForm.poses("Keep It", ["Clear It", "Talk"]),
                LookForm.poses("Clear It Away", ["Clear It"]),
                LookForm.poses("\u202etI raelC", [])],
            [true, true, true, true, true, true, true, true, true, true, true, true, true,
                true, true, false, false, false]);
        // Explain It is told a terminal look's Open beside its cell's acts, by
        // the name its row shows, as that Open goes to the cell's window; any
        // other look's Open answers the look, so it is told none.
        const cellNames = ["Talk"];
        expect("Explain It is told a terminal look's Open by its row's name, after the cell's"
            + " acts, and no other look's",
            [LookForm.presses({ kind: "terminal" }, cellNames, "Review It"),
                LookForm.presses({ kind: "terminal" }, [], "Go To"),
                LookForm.presses({ kind: "page" }, ["Talk", "Go To"], "Open"), cellNames],
            [["Talk", "Review It"], ["Go To"], ["Talk", "Go To"], ["Talk"]]);
        expect("it asks back wanting a choice or his words, or offering choices, in the second"
            + " form only",
            [LookForm.asksBack(secondly), LookForm.asksBack(second({ wants: { words: true },
                choices: undefined })),
                LookForm.asksBack(second({ wants: undefined })),
                LookForm.asksBack(second({ choices: undefined })),
                LookForm.asksBack(second({ wants: undefined, choices: undefined })),
                LookForm.asksBack(second({ wants: { choices: [] }, choices: undefined })),
                LookForm.asksBack(firstly), LookForm.asksBack(choosing)],
            [true, true, true, true, false, false, false, false]);
        const secondBack = JSON.parse(Looks.rewritten(secondly, "opened", "opened", now));
        expect("written back, its second form as its session wrote it",
            [secondBack.happened, secondBack.act, secondBack.wants.choices[1].follows,
                secondBack.about.title, "follows" in secondBack.choices[1]],
            ["The bar keeps the branch at narrow widths", "Review It",
                "The old bar comes back tonight.", "Review the change", false]);
        // Who asks, in plain words, and what happens if Joel does nothing, as
        // its session writes them, else from what's known (Joel, 2026-11-19:
        // "i almost never know what I am supposed to do with it").
        const told = Looks.looks([{ file: "t.json", text: lookOf({ id: "T",
            asker: "The trunk disk incident", ask: "Merge it now, or hold it?",
            ifNothing: "It waits, unmerged, and the rescans go on." }) }], {}, now).waiting[0];
        const quest = { title: "Trunkd Disk Saturation During Compaction (INV-31016)" };
        expect("who asks, as its session says, else its quest's name without its id",
            [Looks.askerOf(told, quest), Looks.askerOf(read.waiting[1], quest),
                Looks.askerOf(read.waiting[1], { title: "Plain Words" }),
                Looks.askerOf(read.waiting[1], null)],
            ["The trunk disk incident", "Trunkd Disk Saturation During Compaction",
                "Plain Words", ""]);
        expect("what it asks and what happens if nothing, as written, else none",
            [Looks.askOf(told), Looks.ifNothingOf(told), Looks.askOf(read.waiting[1]),
                Looks.ifNothingOf(read.waiting[1])],
            ["Merge it now, or hold it?", "It waits, unmerged, and the rescans go on.", "", ""]);
        // Trimmed, as the sites' ask row reads them, so words of only spaces
        // say nothing and For You says what it knows in their place.
        const spaced = Looks.looks([{ file: "s.json", text: lookOf({ id: "S",
            ask: " Merge it? ", ifNothing: "   " }) }], {}, now).waiting[0];
        expect("what it asks and what happens if nothing, padded or only spaces, trimmed",
            [Looks.askOf(spaced), Looks.ifNothingOf(spaced)], ["Merge it?", ""]);
        const padded = Looks.looks([{ file: "t.json", text: lookOf({ id: "T",
            ask: "   ", ifNothing: " It waits. " }) }], {}, now).waiting[0];
        expect("what it asks only spaces, and what happens if nothing padded, trimmed",
            [Looks.askOf(padded), Looks.ifNothingOf(padded)], ["", "It waits."]);
        const toldBack = JSON.parse(Looks.rewritten(told, "opened", "opened", now));
        expect("a look written back keeps who asks, what, and if nothing, as its session wrote them",
            [toldBack.asker, toldBack.ask, toldBack.ifNothing],
            ["The trunk disk incident", "Merge it now, or hold it?",
                "It waits, unmerged, and the rescans go on."]);
        const posted = { value: "posted", label: "Posted" };
        const elsewhere = Object.assign({}, choosing, { address: "07HV3N" });
        expect("an answer, to the session that left it, by the address it goes by now",
            Looks.answer(Object.assign({}, elsewhere, { session: "s-p" }), posted,
                { address: "m0ved1", presence: { session: "s-p" } }, now),
            { address: "M0VED1", steer: { act: "answer", look: "P", value: "posted",
                from: "joel", at: "2026-11-08T14:00:00.000Z" } });
        expect("and to the cell the look names, by its own address",
            (Looks.answer(Object.assign({}, elsewhere, { cell: "CCCCCC" }), posted,
                { address: "CCCCCC", presence: { addresses: ["07HV3N"] } }, now) ?? {})
                .address, "CCCCCC");
        expect("and to the cell the look names, though it lists none of the look's names",
            (Looks.answer(Object.assign({}, elsewhere, { cell: "CCCCCC", session: "s-p" }), posted,
                { address: "CCCCCC", presence: { session: "s-q" } }, now) ?? {}).address,
            "CCCCCC");
        expect("a cell that only lists the look's address, at the look's own, never its own",
            (Looks.answer(elsewhere, posted, { address: "DDDDDD",
                presence: { quest: "QEST-2-XXXXXX", addresses: ["07HV3N"] } }, now) ?? {})
                .address, "07HV3N");
        expect("and to its quest once it's ended",
            (Looks.answer(elsewhere, posted, null, now) ?? {}).address, "VP50PC");
        expect("and to its quest where its cell is another that didn't leave it",
            (Looks.answer(elsewhere, posted, { address: "DDDDDD", presence: {} }, now) ?? {})
                .address, "VP50PC");
        expect("with nowhere to go, none", Looks.answer({ id: "N" }, posted, null, now), null);
        const given = { P: { label: "Posted", at: now } };
        expect("an answer given, while the node may yet say it took it, then not",
            [Looks.answering(choosing, given, now + 4 * 1000),
                Looks.answering(choosing, given, now + 6 * 1000),
                Looks.answering(choosing, {}, now)], ["Posted", "", ""]);
        // An answer nobody could be told, that the node refused or went before
        // saying it took, or that no word came for in 5 s (docs/design.md),
        // says so until he answers again, an hour on too, rather than its
        // choices just coming back (Joel, 2026-11-22: "to in any way follow
        // through").
        const failed = how => ({ P: { label: "Posted", at: now, failed: how } });
        const hourOn = now + 60 * 60 * 1000;
        expect("an answer that may not have gone, said so an hour on, and never as given",
            [Looks.unanswered(choosing, failed("nobody"), hourOn),
                Looks.unanswered(choosing, failed("untaken"), now),
                Looks.unanswered(choosing, failed("dropped"), hourOn),
                Looks.unanswered(choosing, given, now + 4 * 1000),
                Looks.unanswered(choosing, given, now + 6 * 1000),
                Looks.unanswered(choosing, given, hourOn),
                Looks.unanswered(choosing, { P: { label: "Posted", at: now, taken: true } },
                    now + 6 * 1000),
                Looks.answering(choosing, failed("nobody"), now),
                Looks.unanswered(Object.assign({}, choosing, { state: "resolved" }), given,
                    now + 6 * 1000)],
            ["nobody", "untaken", "dropped", "", "unheard", "unheard", "", "", ""]);
        // While no word of an answer has come its choices stay away, however
        // long, so a second can't follow it and the session hear both; any
        // word, the node going included, brings them back.
        expect("an answer out while no word of it came, never once said or taken",
            [Looks.awaiting(choosing, given),
                Looks.awaiting(choosing, failed("dropped")),
                Looks.awaiting(choosing, failed("unsent")),
                Looks.awaiting(choosing, failed("untaken")),
                Looks.awaiting(choosing, failed("nobody")),
                Looks.awaiting(choosing, { P: { label: "Posted", at: now, taken: true } }),
                Looks.awaiting(Object.assign({}, choosing, { state: "resolved" }), given),
                Looks.awaiting(choosing, {})],
            [true, false, false, false, false, false, false, false]);
        expect("when an answer's side next changes, at its 5 s, never once said or taken",
            [Looks.answersChange(given, now + 1000), Looks.answersChange(given, now + 6 * 1000),
                Looks.answersChange(failed("dropped"), now + 1000),
                Looks.answersChange({ P: { label: "Posted", at: now, taken: true } }, now),
                Looks.answersChange({}, now)],
            [4000, null, null, null, null]);
        const saying = label => `answered ${label}`;
        // Once the node takes it, the look leaves what waits for History at once
        // (VKCF3Z, 2026-11-12), though its file waits until its session writes it.
        const offering = { waiting: read.waiting.concat([choosing]), history: read.history };
        expect("an answer the node hasn't taken yet leaves it waiting",
            Looks.taken(offering, given, saying) === offering, true);
        const took = Looks.taken(offering, { P: { label: "Posted", at: now, taken: true } },
            saying);
        expect("one it took leaves what waits", took.waiting.map(one => one.id),
            ["H", "B", "A"]);
        const answered = took.history.findIndex(one => one.id === "P");
        expect("for History, by when it was answered, saying the answer",
            [took.history[answered - 1]?.id, took.history[answered + 1]?.resolved < now,
                ForYou.historySide(took.history[answered] ?? {}, say)],
            ["E", true, "answered Posted 14:00"]);
        expect("with the rest still there", took.history.length, offering.history.length + 1);
        expect("a look rewritten keeps its choices as its file gave them",
            JSON.parse(Looks.rewritten(choosing, "opened", "opened", now)).choices.length, 7);

        // History's side: how it went and when.
        const side = id => ForYou.historySide(read.history.find(one => one.id === id)
            ?? { state: "", since: null }, say);
        expect("one merged", side("E"), "merged 14:02");
        expect("one opened", side("C"), "opened 13:10");
        expect("one cleared by Joel", side("D"), "cleared by you 13:30");
        expect("one saying no how, by its state", side("F"), "resolved 12:00");
        expect("one gone quiet", side("O"), "went quiet 13:00");

        // A look written again as it's done with, whole, with what it was.
        const opened = JSON.parse(Looks.rewritten(read.waiting[1], "opened", "opened", now));
        expect("a look opened, written whole", [opened.v, opened.id, opened.state, opened.how,
            opened.resolvedAt, opened.title, opened.target, opened.since, opened.file,
            opened.words],
            [1, "B", "opened", "opened", "2026-11-08T14:00:00.000Z", "Review the change",
                "https://github.com/o/r/pull/1", undefined, undefined, undefined]);
        // The to-do daemon writes it the same, in its own key order and
        // at its own moment, which says nothing new of the look.
        const shown = Looks.rewritten(read.waiting[1], "cleared", "cleared", now);
        const written = (changes) => {
            const one = JSON.parse(shown);
            const told = {};
            for (const key of Object.keys(one).reverse())
                told[key] = one[key];
            return JSON.stringify(Object.assign(told, { resolvedAt: "2026-11-08T14:00:00.012Z" },
                changes));
        };
        expect("a look the daemon settled as shown is alike, written its own way",
            Looks.settledAlike(shown, written({})), true);
        expect("one it settled another way is not",
            Looks.settledAlike(shown, written({ how: "opened" })), false);
        expect("nor one whose session wrote it anew",
            Looks.settledAlike(shown, written({ title: "Review it again" })), false);
        expect("nor one with a word more", Looks.settledAlike(shown, written({ ask: "Why?" })),
            false);
        expect("nor what can't be read", Looks.settledAlike(shown, "{"), false);
        // The cell a look naming none is led by (Looks.js's cellNamed) is
        // never written as its own, as the daemon writes none.
        const unnamed = Object.assign({}, read.waiting[1], { cell: undefined,
            session: "01a1dd5c-1372-7422-ab6f-f984e1921aab" });
        expect("a look naming no cell is written naming none",
            "cell" in JSON.parse(Looks.rewritten(unnamed, "cleared", "cleared", now)), false);
        expect("a cell the look names is written as its own",
            JSON.parse(Looks.rewritten(Object.assign({}, unnamed, { cell: "BBBBBB" }),
                "cleared", "cleared", now)).cell, "BBBBBB");
        const offered = Looks.rewritten(choosing, "cleared", "cleared", now);
        const reordered = JSON.parse(offered);
        // Qt's engine has no Object.fromEntries, so each is turned by hand.
        const turned = choice => {
            const back = {};
            for (const key of Object.keys(choice).reverse())
                back[key] = choice[key];
            return back;
        };
        reordered.choices = reordered.choices.map(choice =>
            choice === null || typeof choice !== "object" ? choice : turned(choice));
        expect("its choices' keys ordered otherwise, it's alike all the way down",
            Looks.settledAlike(offered, JSON.stringify(reordered)), true);

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
        // What a look points at, short enough for one line, its whole
        // address in the drawer's nerdy bits (Y2YBTS, 2026-11-12: "i hate
        // gross line wraps").
        const pointing = { review: "{at} review", diff: "{at} diff" };
        expect("a change reads as its repository and number",
            [ForYou.pointsAt("pr", "https://github.com/shop/world/pull/2396409", pointing),
                ForYou.pointsAt("pr", "https://github.com/shop/world/pull/2396409/files",
                    pointing),
                ForYou.pointsAt("diff", "https://github.com/shop/world/pull/12/files", pointing)],
            ["world#2396409 review", "world#2396409 review", "world#12 diff"]);
        expect("a page reads as its site and its last step",
            [ForYou.pointsAt("page", "http://sites.localhost:8484/web/", pointing),
                ForYou.pointsAt("page", "https://example.org/a/b/report.html?x=1#top", pointing),
                ForYou.pointsAt("page", "https://example.org", pointing)],
            ["sites.localhost:8484/web", "example.org/report.html", "example.org"]);
        expect("a file reads as its name, and nothing as nothing",
            [ForYou.pointsAt("diff", "/home/skiff/notes/plan.md", pointing),
                ForYou.pointsAt("page", "", pointing)],
            ["plan.md diff", ""]);
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
        // A thing to try waits in To Try a day from when it landed, then
        // goes to History as not tried (VKCF3Z, 2026-11-12: 265 waited,
        // every one a day or older, stepped into 3 times in a week).
        const tries = [{ forYou: [
            { row: { name: "stale" }, thing: { kind: "try", since: now - 25 * hour } },
            { row: { name: "fresh" }, thing: { kind: "try", since: now - hour } },
            { row: { name: "to do" }, thing: { kind: "todo", since: now - 25 * hour } }
        ] }];
        const tried = ForYou.given(tries, now, "not tried");
        expect("a try past a day leaves To Try, a to-do stays", tried.waiting.map(one =>
            one.row.name), ["fresh", "to do"]);
        expect("and is in History as not tried, a day after it landed", tried.done.map(one =>
            [one.row.name, one.done.how, one.done.at]), [["stale", "not tried", now - hour]]);
        expect("with no time given, every try waits", ForYou.given(tries).waiting.length, 3);
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

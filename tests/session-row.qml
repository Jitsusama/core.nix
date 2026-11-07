// How the launcher says where a pi session stands and lays a list of them
// out, run in Qt's own JavaScript engine, the one the shell runs it in.
// The `the-launcher-lays-sessions-under-their-orchestrators` check copies
// modules/quickshell/launcher/SessionRow.js beside this file and runs
// `qml -platform offscreen session-row.qml`. Each list of sessions keeps
// one order of how they stand (needs you, working, in line, listening,
// paused, parked), and Agents sits each session under the one orchestrating it, a
// step in, an orchestration standing where its most urgent row would
// (QEST-20261026-15J7N3's agents-tree-look.md). A row says how it stands
// and what it spent at its right, and its drawer the rest.
import QtQuick
import "SessionRow.js" as SessionRow
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

    // Sessions said as "address:state>orchestrator", in the order Sessions
    // keeps them: "B:working>A" is B, working, orchestrated by A.
    function sessions(said) {
        return said.split(" ").filter(one => one !== "").map(one => {
            const [own, under] = one.split(">");
            const [address, state] = own.split(":");
            return { address: address, state: state, under: under ?? "" };
        });
    }

    // The laid-out list as it reads: each address, a dot for each step in.
    // A fold reads as how many of each state it holds, then its members.
    function read(list, laid) {
        return laid.map(one => `${".".repeat(one.depth)}${one.fold
            ? `(${one.counts.map(c => `${c.count} ${c.state}`).join(", ")}: ${read(list,
                one.members)})`
            : list[one.at].address}`).join(" ");
    }

    function run() {
        const failures = [];

        // [what, its presence, whether its want is Joel's, whether it waits
        // its turn in line, how it stands]
        const states = [
            ["one wanting Joel needs him", { busy: true, wants: { kind: "question" } }, true,
                false, "needs"],
            ["a want that isn't his reads as what it does", { busy: true, wants: {} }, false,
                false, "working"],
            ["one at work is working", { busy: true }, false, false, "working"],
            ["one waiting its turn is in line", { busy: true }, false, true, "inLine"],
            ["one idle is listening, never waiting", { busy: false }, false, false,
                "listening"],
            ["one the harness says is waiting is still only listening",
                { busy: false, state: "waiting" }, false, false, "listening"],
            ["one paused is paused", { state: "paused", busy: false }, false, true, "paused"],
            ["one paused that wants Joel needs him", { state: "paused" }, true, false, "needs"],
            ["one parked is parked, never listening", { state: "parked", busy: false }, false,
                false, "parked"],
            ["none at all is off", null, false, false, "off"]
        ];
        for (const [what, presence, asks, queued, want] of states) {
            const got = SessionRow.state(presence, asks, queued);
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        const order = ["paused", "parked", "listening", "off", "inLine", "needs", "working"]
            .sort((a, b) => SessionRow.rank(a) - SessionRow.rank(b)).join(" ");
        if (order !== "needs working inLine listening paused parked off")
            failures.push(`the one order of states: "${order}"`);

        // [what, the sessions in Sessions' order, how they're laid out]
        const trees = [
            ["sessions alone keep the order of how they stand",
                "A:listening B:needs C:working D:paused E:inLine",
                "B C E A D"],
            ["and within a state the order they came in", "A:working B:working", "A B"],
            ["a session follows its orchestrator, a step in",
                "A:working B:working C:working>A", "A .C B"],
            ["its orchestrator's orchestrator two steps in",
                "P:listening O:working>P B:working>O", "P .O ..B"],
            ["and never deeper than two", "P:working O:working>P B:working>O C:working>B",
                "P .O ..B ..C"],
            ["rows under one orchestrator keep the order of how they stand",
                "O:working A:listening>O B:needs>O C:working>O", "O .B .C .A"],
            ["one parked sits under its orchestrator, last",
                "O:parked A:parked>O B:listening>O X:paused", "O .B .A X"],
            ["an orchestration stands where its most urgent row would",
                "X:working O:listening A:needs>O", "O .A X"],
            ["and so does one under it, with its own",
                "O:working P:listening>O Q:working>O B:needs>P", "O .P ..B .Q"],
            ["one whose orchestrator isn't listed stands at the top, not in",
                "A:working B:working>GONE", "A B"],
            ["one naming itself is its own", "A:working>A", "A"],
            ["orchestrators naming each other are cut where they meet",
                "A:working>B B:working>A", "A .B"]
        ];
        for (const [what, said, want] of trees) {
            const list = sessions(said);
            const got = read(list, SessionRow.tree(list));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // Folded, an orchestration's members that only wait fold into one
        // entry after its own rows, saying each state it holds in the one
        // order; a lone one stays a row, and sessions under none never fold.
        const folds = [
            ["the waiting fold after those that move",
                "O:working A:listening>O B:working>O C:paused>O D:parked>O",
                "O .B .(1 listening, 1 paused, 1 parked: A C D)"],
            ["one waiting alone stays a row", "O:working A:listening>O B:working>O",
                "O .B .A"],
            ["one under it that moves keeps its orchestrator out of the fold",
                "O:listening P:listening>O Q:listening>O R:listening>O B:working>P",
                "O .P ..B .(2 listening: Q R)"],
            ["one that waits with its own that wait folds whole",
                "O:working P:listening>O Q:paused>O B:listening>P",
                "O .(2 listening, 1 paused: P .B Q)"],
            ["those under none never fold", "A:listening B:listening C:paused", "A B C"]
        ];
        for (const [what, said, want] of folds) {
            const list = sessions(said);
            const got = read(list, SessionRow.tree(list, true));
            if (got !== want)
                failures.push(`folded, ${what}: "${got}", not "${want}"`);
        }

        // Whom each row is under, and whether it orchestrates one listed.
        const list = sessions("O:working A:working>O B:working>GONE");
        const laid = SessionRow.tree(list);
        const said = laid.map(one => `${list[one.at].address}<${one.under}${one.orchestrates
            ? "*" : ""}`).join(" ");
        if (said !== "O<* A<O B<")
            failures.push(`whom each is under: "${said}"`);

        // A session's acts follow how it stands: one at work or in line can
        // be stopped, one paused resumed, one parked only woken or talked
        // to, with no window known to go to. One here closes, last; one
        // elsewhere can't, since its node's relay of a close is dropped.
        const acts = [
            ["working", true, true, "switch talk pause stop close"],
            ["working", true, false, "switch talk pause stop"],
            ["inLine", false, true, "talk pause stop close"],
            ["listening", true, true, "switch talk pause close"],
            ["needs", true, false, "switch talk pause"],
            ["paused", true, true, "switch talk resume close"],
            ["parked", false, true, "wake talk"]
        ];
        for (const [state, windowed, here, want] of acts) {
            const got = SessionRow.acts(state, windowed, here).join(" ");
            if (got !== want)
                failures.push(`the acts of one ${state}${here ? " here" : ""}: "${got}", `
                    + `not "${want}"`);
        }

        // What Close's ask says stays: its worktree or its folder, and its
        // quest's Needs You items by their count.
        const leaving = {
            stays: "its {kept} stays.",
            stay: "its {kept} stay.",
            tree: "worktree",
            folder: "folder",
            and: "and",
            needItem: "{count} Needs You item",
            needItems: "{count} Needs You items"
        };
        for (const [tree, needs, want] of [[true, 0, "its worktree stays."],
            [true, 1, "its worktree and 1 Needs You item stay."],
            [false, 2, "its folder and 2 Needs You items stay."],
            [false, 0, "its folder stays."]]) {
            const got = SessionRow.leaves(tree, needs, leaving);
            if (got !== want)
                failures.push(`what Close leaves of ${tree ? "a tree" : "a folder"} with `
                    + `${needs}: "${got}", not "${want}"`);
        }

        // A row says how it stands and what it spent today at its right,
        // the spend kept longest, and nothing more: its address and folder
        // are its drawer's (VKCF3Z, 2026-11-06: "working \u00b7 $4.20").
        const side = items => items.map(item => `${item.said}/${item.keep}`).join(" ");
        const sides = [
            ["its state, then its spend, kept longest", "working", "$4.20",
                "working/3 $4.20/4"],
            ["one that has spent nothing says its state alone", "listening", "",
                "listening/3"],
            ["one off says only what it spent", "", "$1", "$1/4"],
            ["a tier 2 figure leads, kept longest", "working", "$4.20",
                "84% full/5 working/3 $4.20/4", "84% full"]
        ];
        for (const [what, doing, spent, want, full] of sides) {
            const got = side(SessionRow.side(doing, spent, full));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // Its drawer's eyebrow says its address, where it works and since
        // when, and its facts don't say the address again.
        const told = {
            address: "8IDU4E",
            place: "core.nix",
            since: "since 21:52",
            machine: "",
            spent: "$4.20",
            named: id => id
        };
        const drawn = SessionRow.drawer("A Quest", { worksOn: "the row" }, told);
        if (drawn.eyebrow !== "8IDU4E \u00b7 core.nix \u00b7 since 21:52")
            failures.push(`the drawer's eyebrow: "${drawn.eyebrow}"`);
        // One paused, parked or closed never looks live: its glyph is muted,
        // every other a program's, as the live rows' are.
        const hues = ["needs", "working", "inLine", "listening", "paused", "parked", "off"]
            .map(state => `${state}:${SessionRow.hue(state)}`).join(" ");
        if (hues !== "needs:program working:program inLine:program listening:program"
            + " paused:muted parked:muted off:muted")
            failures.push(`the glyphs' roles by state: "${hues}"`);
        // A live one with a tier 2 figure takes the action's hue; one paused
        // stays muted, since it isn't live.
        const warned = ["working", "listening", "paused", "off"]
            .map(state => `${state}:${SessionRow.hue(state, true)}`).join(" ");
        if (warned !== "working:action listening:action paused:muted off:muted")
            failures.push(`the glyphs' roles warned: "${warned}"`);

        // Past 80% of its window its row says how full, in whole percent,
        // and nothing under it or where the record says no context.
        const figureWords = { full: "{share}% full", contextOf: "{used} of {limit}",
            turnTook: "{took} \u00b7 {out} out", firstToken: "{took} first token",
            tokensOf: "{sent} in \u00b7 {out} out", cacheRead: "{share}% read" };
        const fills = [[842, 1000, "84% full"], [799, 1000, ""], [null, 200000, ""],
            [5, 0, ""]].map(([used, limit, want]) => {
            const got = SessionRow.full({ context: { used, limit } }, figureWords);
            return got === want ? "" : `${used} of ${limit}: "${got}", not "${want}"`;
        }).filter(one => one !== "");
        if (SessionRow.full(undefined, figureWords) !== "")
            fills.push("one with no usage said it was full");
        for (const fill of fills)
            failures.push(`how full: ${fill}`);
        // Counts read as figures do, and times as they're said.
        const counts = [2, 999, 4100, 9999, 142400, 999600, 1000000, 1240000, 124620036]
            .map(count => SessionRow.tokens(count)).join(" ");
        if (counts !== "2 999 4.1k 10k 142k 1M 1M 1.2M 125M")
            failures.push(`tokens as figures: "${counts}"`);
        const times = [9284, 900, 38000, 125000].map(ms => SessionRow.took(ms)).join(", ");
        if (times !== "9.3 s, 0.9 s, 38 s, 2 min")
            failures.push(`turns' times: "${times}"`);
        // The drawer's figures by gen_ai's names, today's only on its day.
        const usage = {
            day: "2026-11-07",
            "gen_ai.request.model": "claude-opus-5-5",
            last: { "gen_ai.usage.output_tokens": 4100 },
            today: { "gen_ai.usage.input_tokens": 786, "gen_ai.usage.output_tokens": 272470,
                "gen_ai.usage.cache_read.input_tokens": 124620036,
                "gen_ai.usage.cache_creation.input_tokens": 1194960 },
            context: { used: 142000, limit: 200000 },
            turns: { today: 359, last: { duration_ms: 38000, first_token_ms: 900 } }
        };
        const figured = facts => facts.map(fact => `${fact.label}=${fact.value}`).join("; ");
        const onItsDay = figured(SessionRow.figures(usage, "2026-11-07", figureWords,
            id => id));
        const onItsDayWant = "model=claude-opus-5-5; context=142k of 200k; "
            + "last-turn=38 s \u00b7 4.1k out \u00b7 0.9 s first token; turns=359; "
            + "tokens=126M in \u00b7 272k out; cache=99.0% read";
        if (onItsDay !== onItsDayWant)
            failures.push(`the drawer's figures: "${onItsDay}", not "${onItsDayWant}"`);
        const dayAfter = figured(SessionRow.figures(usage, "2026-11-08", figureWords,
            id => id));
        const dayAfterWant = "model=claude-opus-5-5; context=142k of 200k; "
            + "last-turn=38 s \u00b7 4.1k out \u00b7 0.9 s first token";
        if (dayAfter !== dayAfterWant)
            failures.push(`the figures a day on: "${dayAfter}", not "${dayAfterWant}"`);
        if (SessionRow.figures(undefined, "2026-11-07", figureWords, id => id).length !== 0)
            failures.push("one with no usage had figures");
        // When a quest's session was last live: the harness's `at` where it
        // says one, else when a UUIDv7 id says it began, else never.
        const lives = [
            [{ id: "01a1b20b-e05f-7367-9461-074e26d23c53", at: "2026-11-07T05:00:00Z" },
                Date.parse("2026-11-07T05:00:00Z")],
            [{ id: "01a1b20b-e05f-7367-9461-074e26d23c53" }, 0x01a1b20be05f],
            [{ id: "01a1b20b-e05f-4367-9461-074e26d23c53" }, 0],
            [{ id: "not-a-uuid", at: "whenever" }, 0],
            [null, 0]
        ];
        for (const [resume, want] of lives) {
            const got = SessionRow.liveAt(resume);
            if (got !== want)
                failures.push(`last live from ${JSON.stringify(resume)}: ${got}, not ${want}`);
        }
        // The closed, the one last live first, those as live as each other
        // in the order given.
        const lastLive = SessionRow.lastLiveFirst([{ address: "OLDEST", at: 1 },
            { address: "NEVER", at: 0 }, { address: "NEWEST", at: 3 },
            { address: "TIEDA", at: 2 }, { address: "TIEDB", at: 2 }]).join(" ");
        if (lastLive !== "NEWEST TIEDA TIEDB OLDEST NEVER")
            failures.push(`the closed, last live first: "${lastLive}"`);
        // It names things from the world, so it's set as they're written.
        if (drawn.eyebrowNames !== true)
            failures.push("the drawer's eyebrow isn't said to name things");
        // Short of room, as the drawer gives it way (Reading.wholeItems), it
        // loses when it started first, then its repo, and never its address,
        // each whole, at a letter's width a letter.
        const eyebrowIn = room => Reading.wholeItems(drawn.eyebrowItems, room,
            words => words.length);
        for (const [room, want] of [[40, "8IDU4E \u00b7 core.nix \u00b7 since 21:52"],
            [30, "8IDU4E \u00b7 core.nix"], [10, "8IDU4E"], [3, "8IDU4E"]]) {
            if (eyebrowIn(room) !== want)
                failures.push(`the eyebrow in ${room}: "${eyebrowIn(room)}", not "${want}"`);
        }
        const facts = drawn.facts.map(fact => fact.label).join(" ");
        if (facts !== "works spent")
            failures.push(`the drawer's facts: "${facts}", not "works spent"`);
        // One whose record names no place, nor when it began, says its
        // address alone.
        const bare = SessionRow.drawer("A Quest", {}, Object.assign({}, told,
            { place: "", since: "" }));
        if (bare.eyebrow !== "8IDU4E")
            failures.push(`a bare drawer's eyebrow: "${bare.eyebrow}"`);

        // Return does the act its state makes harmless, always one of its
        // acts: Go To one at work, Talk to one listening, Resume one paused
        // and Wake one parked; with no window to go to, Talk.
        const returns = [
            ["working", true, "switch"],
            ["inLine", true, "switch"],
            ["needs", true, "switch"],
            ["working", false, "talk"],
            ["listening", true, "talk"],
            ["paused", true, "resume"],
            ["paused", false, "resume"],
            ["parked", false, "wake"]
        ];
        for (const [state, windowed, want] of returns) {
            const got = SessionRow.returns(state, windowed);
            if (got !== want || !SessionRow.acts(state, windowed, true).includes(got))
                failures.push(`Return on one ${state}${windowed ? "" : " unwindowed"}: `
                    + `"${got}", not "${want}"`);
        }

        // Together they say those needing Joel, those working, then the
        // day's dollars; with neither, how the rest stand in the fold's
        // order, the parked aside.
        const words = {
            workingCount: "{count} working",
            inLineCount: "{count} in line",
            needsCount: "1 needs you",
            needCount: "{count} need you",
            listening: "listening",
            paused: "paused",
            today: "{spent} today"
        };
        const summaries = [
            ["working working listening needs working", "$214",
                "1 needs you \u00b7 3 working \u00b7 $214 today"],
            ["needs needs paused", "", "2 need you"],
            ["paused listening parked listening listening", "$3.10",
                "3 listening, 1 paused \u00b7 $3.10 today"],
            ["listening inLine", "", "1 in line, 1 listening"],
            ["parked", "", ""],
            ["", "$0.40", "$0.40 today"],
            ["", "", ""]
        ];
        for (const [states, spent, want] of summaries) {
            const got = SessionRow.summary(states.split(" ").filter(one => one !== ""), spent,
                words);
            if (got !== want)
                failures.push(`together, ${states || "none"}: "${got}", not "${want}"`);
        }

        // What landed reads newest first, changes and what says what to
        // try, never a plan item ticked alone or one Joel has read, and
        // only a web link opens.
        const landing = (ref, at, more) => Object.assign({ ref: ref, at: at, what: ref }, more);
        const landers = [
            { address: "A", presence: { session: "a", landings: [
                landing("core.nix#2140", "2026-11-07T11:31:00Z",
                    { url: "https://github.com/o/core.nix/pull/2140" }),
                landing("PLAN-20261106-2ZPAI5#3", "2026-11-07T11:32:00Z"),
                landing("PLAN-20261106-2ZPAI5", "2026-11-07T11:32:30Z"),
                landing("PLAN-20261106-2ZPAI5#4", "2026-11-07T11:30:00Z",
                    { person: "try Stop twice" })] } },
            { address: "B", presence: { session: "b", landings: [
                landing("world#9", "2026-11-07T11:33:00Z", { url: "file:///etc/passwd" }),
                landing("core.nix#2138", "2026-11-07T11:20:00Z")] } },
            { address: "C", presence: { session: "c" } }
        ];
        const landedAs = rows => rows.map(one => `${one.session.address}:${one.ref}`
            + `${one.person === "" ? "" : "+try"}${one.url === "" ? "" : "+url"}`).join(" ");
        const unread = SessionRow.landed(landers, []);
        const unreadWant = "B:world#9 A:core.nix#2140+url A:PLAN-20261106-2ZPAI5#4+try "
            + "B:core.nix#2138";
        if (landedAs(unread) !== unreadWant)
            failures.push(`what landed: "${landedAs(unread)}", not "${unreadWant}"`);
        const once = SessionRow.landed(landers, [unread[1].key]);
        const onceWant = "B:world#9 A:PLAN-20261106-2ZPAI5#4+try B:core.nix#2138";
        if (landedAs(once) !== onceWant)
            failures.push(`with one read: "${landedAs(once)}", not "${onceWant}"`);
        const kept = SessionRow.stillRead(landers, [unread[1].key, "z gone#1 2026"]);
        if (kept.join() !== unread[1].key)
            failures.push(`what's kept read: ${JSON.stringify(kept)}`);
        // A landing's drawer says who landed it and when, what it is, and
        // what to try as its words, which the row's one line has no room
        // for; a fact nobody knows isn't said.
        const plate = SessionRow.landingDrawer(unread[2],
            { at: "11:30", session: "A Quest", named: id => id });
        const plateSays = `${plate.eyebrow} | ${plate.title} | `
            + `${plate.facts.map(fact => `${fact.label}=${fact.value}`).join(" ")} | ${plate.body}`;
        const plateWant = "A \u00b7 11:30 | PLAN-20261106-2ZPAI5#4 | landed=PLAN-20261106-2ZPAI5#4"
            + " session=A Quest | try Stop twice";
        if (plateSays !== plateWant)
            failures.push(`a landing's drawer: "${plateSays}", not "${plateWant}"`);
        const unsaid = SessionRow.landingDrawer(unread[3],
            { at: "", session: "", named: id => id });
        if (unsaid.eyebrow !== "B" || unsaid.facts.length !== 1 || unsaid.body !== "")
            failures.push(`a bare landing's drawer: ${JSON.stringify(unsaid)}`);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

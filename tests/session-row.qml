// How the launcher says where a pi session stands, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-says-how-each-session-stands` check copies
// modules/quickshell/launcher/SessionRow.js, SessionDrawer.js, Holders.js
// and Landed.js beside this file and runs
// `qml -platform offscreen session-row.qml`. Each list of sessions keeps
// one order of how they stand (needs you, working, in line, listening,
// paused, parked). How Agents lays them out, each under the one
// orchestrating it, is the agents daemon's (modules/agents/src/tree.rs),
// as is what a row says at its right; its drawer is still said here.
import QtQuick
import "SessionDrawer.js" as SessionDrawer
import "Holders.js" as Holders
import "SessionRow.js" as SessionRow
import "Landed.js" as Landed
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
            ["one parked that wants Joel is still parked, its want For You's",
                { state: "parked", wants: { kind: "question" } }, true, false, "parked"],
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
        // quest's to-dos by their count.
        const leaving = {
            stays: "its {kept} stays.",
            stay: "its {kept} stay.",
            tree: "worktree",
            folder: "folder",
            and: "and",
            toDo: "{count} to-do",
            toDos: "{count} to-dos"
        };
        for (const [tree, needs, want] of [[true, 0, "its worktree stays."],
            [true, 1, "its worktree and 1 to-do stay."],
            [false, 2, "its folder and 2 to-dos stay."],
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
            ["a tier 2 figure leads, kept longest, the state outlasting the money",
                "working", "$4.20", "84% full/5 working/4 $4.20/3", "84% full"]
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
        const drawn = SessionDrawer.drawer("A Quest", { worksOn: "the row" }, told);
        if (drawn.eyebrow !== "8IDU4E \u00b7 core.nix \u00b7 since 21:52")
            failures.push(`the drawer's eyebrow: "${drawn.eyebrow}"`);
        // Its hours ride on its own spend today as a spark, an orchestrator's
        // on Its Own, and what rebuilding its context cost follows, unsaid
        // when it's nothing.
        const hours = { shape: "spark", values: [0, 0.5, 1] };
        const sparked = given => SessionDrawer.drawer("A Quest", {},
            Object.assign({}, told, { hours: hours }, given)).facts
            .map(fact => fact.figure ? `${fact.label}=${fact.value}:${fact.figure.shape}`
                : `${fact.label}=${fact.value}`).join(" ");
        // An orchestrator's budget follows the whole of its spend, its own
        // and its sessions' among it.
        const sparks = [sparked({ rebuilt: "$1.20" }),
            sparked({ spent: "$14.20 in all", own: "$3.10", rebuilt: "", budget: "Mine" })]
            .join(" | ");
        if (sparks !== "spent=$4.20:spark rebuilt=$1.20 | "
            + "spent=$14.20 in all own=$3.10:spark budget=Mine")
            failures.push(`a drawer's hours: "${sparks}"`);
        // One paused, parked or closed never looks live: its glyph is muted,
        // every other a program's, as the live rows' are.
        const hues = ["needs", "working", "inLine", "listening", "paused", "parked", "off"]
            .map(state => `${state}:${SessionRow.hue(state)}`).join(" ");
        if (hues !== "needs:program working:program inLine:program listening:program"
            + " paused:muted parked:muted off:muted")
            failures.push(`the glyphs' roles by state: "${hues}"`);
        // A live one with a tier 2 figure takes the warning's hue; one paused
        // stays muted, since it isn't live.
        const warned = ["working", "listening", "paused", "off"]
            .map(state => `${state}:${SessionRow.hue(state, true)}`).join(" ");
        if (warned !== "working:warning listening:warning paused:muted off:muted")
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
        const onItsDayWant = "context=142k of 200k; "
            + "last-turn=38 s \u00b7 4.1k out \u00b7 0.9 s first token; model=claude-opus-5-5; "
            + "turns=359; "
            + "tokens=126M in \u00b7 272k out; cache=99.0% read";
        if (onItsDay !== onItsDayWant)
            failures.push(`the drawer's figures: "${onItsDay}", not "${onItsDayWant}"`);
        const dayAfter = figured(SessionRow.figures(usage, "2026-11-08", figureWords,
            id => id));
        const dayAfterWant = "context=142k of 200k; "
            + "last-turn=38 s \u00b7 4.1k out \u00b7 0.9 s first token; model=claude-opus-5-5";
        if (dayAfter !== dayAfterWant)
            failures.push(`the figures a day on: "${dayAfter}", not "${dayAfterWant}"`);
        if (SessionRow.figures(undefined, "2026-11-07", figureWords, id => id).length !== 0)
            failures.push("one with no usage had figures");
        // Context alone carries a figure, Meter's stroke of how full, warning
        // from 80%.
        const drawnOf = given => SessionRow.figures(given, "2026-11-07", figureWords, id => id)
            .filter(fact => fact.figure)
            .map(fact => `${fact.label}:${fact.figure.shape}:${fact.figure.values}:`
                + `${fact.figure.warn}`).join(" ");
        const strokes = [drawnOf(usage),
            drawnOf(Object.assign({}, usage, { context: { used: 168000, limit: 200000 } }))];
        if (strokes.join(" | ") !== "context:stroke:0.71:false | context:stroke:0.84:true")
            failures.push(`the drawer's strokes: "${strokes.join(" | ")}"`);
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
        // What it works on is words, said last, since it runs long.
        const facts = drawn.facts.map(fact => `${fact.label}${fact.words ? ":words" : ""}`)
            .join(" ");
        if (facts !== "spent works:words")
            failures.push(`the drawer's facts: "${facts}", not "spent works:words"`);
        // Its first screen holds what matters, in this order: what it's
        // doing, its step alone, what it needs Joel for, what it landed last,
        // what it spent, the budget it drew on, its figures, then its quest,
        // its machine and what its quest is about, that without the title
        // the drawer already says (Joel, 2026-11-07: "the agents side panel
        // generally just shows its title"; VKCF3Z, 2026-11-07; KP0L6A,
        // 2026-11-07, for the budget).
        const full = SessionDrawer.drawer("A Quest", {
            worksOn: "A Quest: what it's for, at length",
            stepDetail: { kind: "tool", name: "edit", detail: "/src/core.nix/Drawer.qml" },
            wants: { said: "which way?" },
            questInfo: { id: "QEST-1", title: "A Quest", status: "active" }
        }, Object.assign({}, told, {
            needs: "Sign In", landed: "core.nix#1 \u00b7 15:44",
            budget: "Your AI proxy token \u00b7 $24,349 of $40,000",
            machine: "there", figures: [{ label: "context", value: "84k of 200k" }]
        })).facts.map(fact => `${fact.label}=${fact.value}`).join("; ");
        const fullWant = "step=edit Drawer.qml; wants=which way? \u00b7 Sign In; "
            + "last-landing=core.nix#1 \u00b7 15:44; spent=$4.20; "
            + "budget=Your AI proxy token \u00b7 $24,349 of $40,000; context=84k of 200k; "
            + "task=QEST-1; machine=there; works=what it's for, at length";
        if (full !== fullWant)
            failures.push(`the drawer's first screen: "${full}", not "${fullWant}"`);
        // One with no step says no Doing, and a quest that isn't active says
        // how it stands.
        const still = SessionDrawer.drawer("A Quest", {
            questInfo: { id: "QEST-1", title: "A Quest", status: "paused" }
        }, told).facts.map(fact => `${fact.label}=${fact.value}`).join("; ");
        if (still !== "spent=$4.20; task=QEST-1 \u00b7 paused")
            failures.push(`a drawer with no step, its quest paused: "${still}"`);
        // A step says its tool and what it's on, a file by its name, and
        // nothing the harness cut short.
        const steps = [{ kind: "thinking" }, { kind: "tool", name: "bash", detail: "cd ~/a; f\u2026" },
            { kind: "tool", name: "quest", detail: "journal" }, null]
            .map(detail => SessionDrawer.step(detail)).join(", ");
        if (steps !== "thinking, bash, quest journal, ")
            failures.push(`a step: "${steps}"`);
        // One whose record names no place, nor when it began, says its
        // address alone.
        const bare = SessionDrawer.drawer("A Quest", {}, Object.assign({}, told,
            { place: "", since: "" }));
        if (bare.eyebrow !== "8IDU4E")
            failures.push(`a bare drawer's eyebrow: "${bare.eyebrow}"`);

        // What landed reads newest first, changes and what says what to
        // try, never a plan item ticked alone, and only a web link opens.
        // What says what to try waits in For You until Joel reads it, and
        // what he read goes to its History with when.
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
        const unread = Landed.all([], landers);
        const unreadWant = "B:world#9 A:core.nix#2140+url A:PLAN-20261106-2ZPAI5#4+try "
            + "B:core.nix#2138";
        if (landedAs(unread) !== unreadWant)
            failures.push(`what landed: "${landedAs(unread)}", not "${unreadWant}"`);
        const toTry = Landed.toTry(unread, [unread[1].key]);
        if (landedAs(toTry) !== "A:PLAN-20261106-2ZPAI5#4+try")
            failures.push(`to try: "${landedAs(toTry)}", not only the one saying what to`);
        if (Landed.toTry(unread, [unread[2].key]).length !== 0)
            failures.push("a landing read still waits to be tried");
        const readAt = { [unread[2].key]: 1000, [unread[0].key]: 2000, [unread[3].key]: "x" };
        const tried = Landed.triedRead(unread, readAt);
        if (tried.length !== 1 || tried[0].one !== unread[2] || tried[0].at !== 1000)
            failures.push(`what's read for History: ${JSON.stringify(tried.map(one => one.at))}`);
        const kept = Landed.stillRead([], landers, [unread[1].key, "z gone#1 2026"]);
        if (kept.join() !== unread[1].key)
            failures.push(`what's kept read: ${JSON.stringify(kept)}`);
        // What the harness keeps outlives its session: one landed by a
        // session now closed is listed by its address, with its quest,
        // repository and machine, and one both kept and still carried by
        // its session is listed once, read by the same key either way.
        const keeping = [
            landing("core.nix#2200", "2026-11-07T09:00:00Z", { session: "d", address: "D",
                quest: "QEST-20261106-KP0L6A", repo: "Jitsusama/core.nix", machine: "skiff" }),
            landing("core.nix#2138", "2026-11-07T11:20:00Z", { session: "b", address: "B" }),
            landing("PLAN-20261106-2ZPAI5#5", "2026-11-07T09:10:00Z", { session: "d" })
        ];
        const lasting = Landed.all(keeping, landers);
        const lastingWant = `${unreadWant} D:core.nix#2200`;
        if (landedAs(lasting) !== lastingWant)
            failures.push(`what's kept: "${landedAs(lasting)}", not "${lastingWant}"`);
        const closedOne = lasting[lasting.length - 1];
        if (closedOne.quest !== "QEST-20261106-KP0L6A" || closedOne.repo !== "Jitsusama/core.nix"
            || closedOne.machine !== "skiff")
            failures.push(`a kept landing's facts: ${JSON.stringify(closedOne)}`);
        const stillKept = Landed.stillRead(keeping, landers, [closedOne.key, unread[3].key]);
        if (stillKept.length !== 2)
            failures.push(`what's kept read with the harness's: ${JSON.stringify(stillKept)}`);
        const morning = Date.parse("2026-11-07T15:00:00Z");
        const todayCount = Landed.landedToday(keeping, morning);
        if (todayCount !== 2)
            failures.push(`landed today: ${todayCount}, not 2`);
        if (Landed.landedToday(keeping, Date.parse("2026-11-09T15:00:00Z")) !== 0)
            failures.push("a landing two days old counted as today's");
        // Landed's row says at its right how many landed today, never
        // how many are to read, which For You alone counts.
        const standing = { today: "{count} today", none: "none today" };
        const stood = [5, 0].map(today => Landed.stands(today, standing));
        if (JSON.stringify(stood) !== JSON.stringify(["5 today", "none today"]))
            failures.push(`Landed stands: ${JSON.stringify(stood)}`);
        // A landing's drawer says who landed it and that it's a landing,
        // what it is, when it landed first among its facts and its ref
        // last, named a change or an item by what it is, and what to try
        // as its words, which the row's one line has no room for; a fact
        // nobody knows isn't said.
        const drawerSays = drawn => `${drawn.eyebrow} | ${drawn.title} | `
            + `${drawn.facts.map(fact => `${fact.label}=${fact.value}`).join(" ")} | ${drawn.body}`;
        const plate = Landed.landingDrawer(unread[2], { kind: "landing",
            when: "Saturday 7 November, 11:30", session: "A Quest", named: id => id });
        const plateWant = "A \u00b7 landing | PLAN-20261106-2ZPAI5#4 | landed=Saturday 7 November,"
            + " 11:30 session=A Quest item=PLAN-20261106-2ZPAI5#4 | try Stop twice";
        if (drawerSays(plate) !== plateWant)
            failures.push(`a landing's drawer: "${drawerSays(plate)}", not "${plateWant}"`);
        const merged = drawerSays(Landed.landingDrawer(closedOne,
            { kind: "landing", when: "", session: "", named: id => id }));
        const mergedWant = "D \u00b7 landing | core.nix#2200 | task=QEST-20261106-KP0L6A"
            + " repo=Jitsusama/core.nix machine=skiff change=core.nix#2200 | ";
        if (merged !== mergedWant)
            failures.push(`a change's drawer: "${merged}", not "${mergedWant}"`);
        const unsaid = Landed.landingDrawer(unread[3],
            { kind: "", when: "", session: "", named: id => id });
        if (unsaid.eyebrow !== "B" || unsaid.facts.length !== 1 || unsaid.body !== "")
            failures.push(`a bare landing's drawer: ${JSON.stringify(unsaid)}`);
        // Its row says when it landed, its time today and its day before,
        // and its drawer the whole moment.
        const landedSaid = {
            clock: at => new Date(at).toISOString().slice(11, 16),
            day: at => at < Date.parse("2026-11-07T00:00:00Z") ? "yesterday" : "",
            date: () => "Saturday 7 November"
        };
        const whens = ["2026-11-07T11:19:00Z", "2026-11-06T09:00:00Z", ""]
            .map(at => Landed.landedWhen(at, landedSaid));
        const whensWant = [{ side: "11:19", fact: "Saturday 7 November, 11:19" },
            { side: "yesterday", fact: "Saturday 7 November, 09:00" }, { side: "", fact: "" }];
        if (JSON.stringify(whens) !== JSON.stringify(whensWant))
            failures.push(`when it landed: ${JSON.stringify(whens)}`);
        // Its row's side is worked out once for each moment a day, since
        // Landed lists over a thousand landings at every opening, and anew
        // once the day turns, when today's may read as yesterday.
        let worked = 0;
        const counted = Object.assign({}, landedSaid, { day: at => {
            worked += 1;
            return landedSaid.day(at);
        } });
        const landedSides = { day: "", sides: {} };
        const sided = ["2026-11-07T11:19:00Z", "2026-11-07T11:19:00Z", "2026-11-06T09:00:00Z", ""]
            .map(at => Landed.landedSide(landedSides, at, "2026-11-07", counted)).join(" | ");
        if (sided !== "11:19 | 11:19 | yesterday | " || worked !== 2)
            failures.push(`a landing's side, worked ${worked} times: "${sided}"`);
        Landed.landedSide(landedSides, "2026-11-07T11:19:00Z", "2026-11-08", counted);
        if (worked !== 3)
            failures.push(`a landing's side kept past its day: worked ${worked} times`);

        // Agents' Recent: those closed this boot, keyed by id at when they
        // ended, and never one Agents shows already: a closed orchestrator
        // or one running again by its address, one to reopen by its id.
        // When each ended is the node's number, read as it is, never a
        // time written out to be read back at each opening.
        const closed = [
            { id: "s-1", address: "A", title: "Recent, Once", machine: "here",
                folder: "tree", ended: 1794116460000,
                presence: { questInfo: { title: "Recent, Once" }, repo: "core.nix" } },
            { id: "s-2", address: "B", title: "notes", machine: "here", folder: "notes",
                quest: "QEST-B", ended: 1794115800000,
                presence: { questInfo: { title: "Write It Up" } } },
            { id: "s-3", address: "C", title: "C", machine: "here", folder: "x",
                ended: 1794114000000, presence: {} },
            { id: "s-4", address: "D", title: "D", machine: "here", folder: "y",
                ended: 1794110400000, presence: {} }
        ];
        const keyed = SessionRow.closedRecent(closed);
        const keyedSays = keyed.map(one => `${one.key}@${one.at}`).join(" ");
        const keyedWant = "session:s-1@1794116460000 session:s-2@1794115800000"
            + " session:s-3@1794114000000 session:s-4@1794110400000";
        if (keyedSays !== keyedWant)
            failures.push(`closed as Recent lists them: "${keyedSays}", not "${keyedWant}"`);
        const shownSays = SessionRow.shownElsewhere(closed, [{ id: "s-3" }], ["D", "Z"]).join();
        if (shownSays !== "session:s-3,session:s-4")
            failures.push(`closed shown elsewhere: "${shownSays}"`);
        // One whose quest a session works on again, at a new address, is
        // left out too, since reopening it would give the quest two.
        const questSays = SessionRow.shownElsewhere(closed, [], [], ["QEST-B", ""]).join();
        if (questSays !== "session:s-2")
            failures.push(`closed with its quest working again: "${questSays}"`);
        // Its note never says its name again: its quest's title where
        // that differs, else where it ran, its repository before its folder.
        const notes = closed.slice(0, 3).map(SessionRow.closedNote).join(" | ");
        const notesWant = "here \u00b7 core.nix | Write It Up | here \u00b7 x";
        if (notes !== notesWant)
            failures.push(`closed notes: "${notes}", not "${notesWant}"`);
        // When it closed, at its right: the time today, the day before it
        // otherwise.
        const closing = { closed: "closed", closedAt: "closed {time}",
            closedOn: "closed {day} {time}" };
        const ended = new Date(2026, 10, 7, 23, 41);
        const at = [new Date(2026, 10, 7, 23, 59), new Date(2026, 10, 8, 9, 5)]
            .map(now => SessionRow.closedAt(ended.getTime(), now, closing, () => "Sat"))
            .concat([SessionRow.closedAt(undefined, ended, closing, () => "Sat")]).join(" | ");
        if (at !== "closed 23:41 | closed Sat 23:41 | closed")
            failures.push(`when each closed: "${at}"`);

        // The sessions whose folder the disk guard measured, to talk to
        // about it: those at work on a quest's workspace, or working in a
        // tree or inside it, the one that worked last first (VKCF3Z,
        // 2026-11-08).
        const working = [
            { address: "EARLIER", folder: "kelp", changed: 10,
                presence: { quest: "REEF", cwd: "/home/ada/src/kelp" } },
            { address: "LAST", folder: "reef", changed: 30,
                presence: { quest: "REEF", cwd: "/home/ada/src/reef" } },
            // A session's folder is only its last part, so its record's
            // cwd is what says where it works.
            { address: "TREE", folder: "modules", changed: 20,
                presence: { questInfo: { id: "TIDE" }, cwd: "/home/ada/trees/tidepool/modules/" } }
        ];
        const owns = holder => Holders.owners(working, holder).map(one => one.address)
            .join(" ");
        const cases = [
            ["a quest's workspace, the session that worked last first",
                { path: "/home/ada/.cache/scratch/REEF", owner: { kind: "quest", name: "REEF" } },
                "LAST EARLIER"],
            ["a quest whose id its record names whole",
                { path: "/home/ada/.cache/scratch/TIDE", owner: { kind: "quest", name: "TIDE" } },
                "TREE"],
            ["a tree a session works inside",
                { path: "/home/ada/trees/tidepool", owner: { kind: "worktree", name: "tidepool" } },
                "TREE"],
            ["a folder only sharing its name's start",
                { path: "/home/ada/trees/tide", owner: { kind: "worktree", name: "tide" } }, ""],
            ["a quest no session is on",
                { path: "/home/ada/.cache/scratch/SKERRY",
                    owner: { kind: "quest", name: "SKERRY" } },
                ""],
            ["a cache", { path: "/home/ada/.cache/cargo", owner: { kind: "cache", name: "cargo" } },
                ""]
        ];
        for (const [what, holder, want] of cases) {
            if (owns(holder) !== want)
                failures.push(`${what}: talked to "${owns(holder)}", not "${want}"`);
        }

        // Who a talk goes to outlasts how it stands, so a narrow row keeps
        // "Listening Quest" whole rather than cutting it to "Quest".
        const talking = Holders.talkSide("Listening Quest", "listening");
        const keeps = Reading.keptItems(talking, 15, words => words.length).join(" | ");
        if (keeps !== "Listening Quest")
            failures.push(`a narrow talk row keeps "${keeps}", not its session's title`);
        const roomy = Reading.keptItems(talking, 40, words => words.length).join(" \u00b7 ");
        if (roomy !== "Listening Quest \u00b7 listening")
            failures.push(`a roomy talk row says "${roomy}"`);
        if (Holders.talkSide("Reef", "").length !== 1)
            failures.push("a talk row with no state says an empty item");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

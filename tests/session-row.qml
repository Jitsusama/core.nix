// How the launcher says where a pi session stands, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-says-how-each-session-stands` check copies
// modules/quickshell/launcher/SessionRow.js, SessionDrawer.js, Holders.js
// and Landed.js beside this file and runs
// `qml -platform offscreen session-row.qml`. Each list of sessions keeps
// one order of how they stand (needs you, working, in line, listening,
// paused, parked). How Agents lays them out, each under the one
// orchestrating it, is the agents daemon's (modules/agents/src/tree.rs),
// as are what a row says at its right and its drawer (drawer.rs), whose
// times alone are worded here.
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

        // The daemon's drawer is shown with its times worded by the clock:
        // when it began after its address and where it works, and when it
        // landed last after what it landed, alone where it said nothing
        // else. It names things from the world, so it's set as they're
        // written (modules/agents/src/drawer.rs says all the rest).
        const clock = { clock: at => `@${at}`, since: "since {at}" };
        const drawn = SessionDrawer.shown({
            title: "A Quest",
            eyebrow: [{ said: "8IDU4E", keep: 3 }, { said: "core.nix", keep: 2 },
                { at: { at: 2152, as: "since" }, keep: 1 }],
            facts: [
                { label: "Last Landing", value: "Serve rows \u00b7 core.nix#1",
                    at: { at: 1544, as: "clock" } },
                { label: "Spent Today", value: "$4.20" },
                { label: "Last Landing", value: "", at: { at: 1, as: "clock" } }
            ],
            body: ""
        }, clock);
        if (drawn.eyebrow !== "8IDU4E \u00b7 core.nix \u00b7 since @2152")
            failures.push(`the drawer's eyebrow: "${drawn.eyebrow}"`);
        const values = drawn.facts.map(fact => `${fact.label}=${fact.value}${fact.at ? "+at" : ""}`)
            .join("; ");
        const valuesWant = "Last Landing=Serve rows \u00b7 core.nix#1 \u00b7 @1544; "
            + "Spent Today=$4.20; Last Landing=@1";
        if (values !== valuesWant)
            failures.push(`the drawer's times: "${values}", not "${valuesWant}"`);
        // A landing given as its title and its change says when as an item
        // of its own, so its lines break only between them, each whole.
        const itemed = SessionDrawer.shown({ facts: [{ label: "Last Landing",
            value: "Serve rows \u00b7 core.nix#1", at: { at: 1544, as: "clock" },
            valueItems: [{ said: "Serve rows", whole: true },
                { said: "core.nix#1", whole: true }] }] }, clock).facts[0];
        const items = (itemed.valueItems ?? []).map(item => `${item.said}${item.whole ? "!" : ""}`)
            .join(" | ");
        if (items !== "Serve rows! | core.nix#1! | @1544!"
            || itemed.value !== "Serve rows \u00b7 core.nix#1 \u00b7 @1544")
            failures.push(`a landing's items: "${items}", its value "${itemed.value}"`);
        if (drawn.eyebrowNames !== true || drawn.title !== "A Quest")
            failures.push("the drawer's eyebrow isn't said to name things, or its title is lost");
        // Short of room, as the drawer gives it way (Reading.wholeItems), it
        // loses when it started first, then its repo, and never its address,
        // each whole, at a letter's width a letter.
        const eyebrowIn = room => Reading.wholeItems(drawn.eyebrowItems, room,
            words => words.length);
        for (const [room, want] of [[40, "8IDU4E \u00b7 core.nix \u00b7 since @2152"],
            [30, "8IDU4E \u00b7 core.nix"], [10, "8IDU4E"], [3, "8IDU4E"]]) {
            if (eyebrowIn(room) !== want)
                failures.push(`the eyebrow in ${room}: "${eyebrowIn(room)}", not "${want}"`);
        }
        // One whose record names no place, nor when it began, says its
        // address alone.
        const bare = SessionDrawer.shown({ eyebrow: [{ said: "8IDU4E", keep: 3 },
            { said: "", keep: 2 }], facts: [] }, clock);
        if (bare.eyebrow !== "8IDU4E")
            failures.push(`a bare drawer's eyebrow: "${bare.eyebrow}"`);
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
        // Landed's row says at its right how many landed today, never
        // how many are to read, which For You alone counts.
        const standing = { today: "{count} today", none: "none today" };
        const stood = [5, 0].map(today => Landed.stands(today, standing));
        if (JSON.stringify(stood) !== JSON.stringify(["5 today", "none today"]))
            failures.push(`Landed stands: ${JSON.stringify(stood)}`);
        // A landing's drawer is the daemon's (modules/agents/src/landed.rs),
        // shown as a session's is: when it landed worded as the whole
        // moment, alone as its fact's value, and its identifiers passed on
        // to be folded among the nerdy bits.
        const landingDrawn = SessionDrawer.shown({
            title: "Serve rows",
            eyebrow: [{ said: "A Quest", keep: 2 }, { said: "landing", keep: 1 }],
            facts: [{ label: "Landed", value: "", at: { at: 1130, as: "dated" } },
                { label: "Machine", value: "skiff" }],
            body: "try Stop twice",
            nerd: [{ label: "Change", value: "core.nix#2140" }]
        }, Object.assign({ dated: at => `Saturday 7 November, @${at}` }, clock));
        const landingFacts = landingDrawn.facts.map(fact => `${fact.label}=${fact.value}`);
        const landingNerd = landingDrawn.nerd.map(fact => `${fact.label}=${fact.value}`);
        const landingSays = [landingDrawn.eyebrow, landingFacts.join(" "), landingDrawn.body,
            landingNerd.join(" ")].join(" | ");
        const landingWant = "A Quest \u00b7 landing | Landed=Saturday 7 November, @1130"
            + " Machine=skiff | try Stop twice | Change=core.nix#2140";
        if (landingSays !== landingWant)
            failures.push(`a landing's drawer: "${landingSays}", not "${landingWant}"`);
        // Its row's side is when it landed, its time today and its day
        // before.
        const landedSaid = {
            clock: at => new Date(at).toISOString().slice(11, 16),
            day: at => at < Date.parse("2026-11-07T00:00:00Z") ? "yesterday" : ""
        };
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
        const going = [{ at: "2026-11-07T11:19:00Z", saying: "replying" },
            { at: "2026-11-07T11:19:00Z", saying: "" }, { at: "2026-11-07T11:19:00Z" }]
            .map(one => Landed.sideOf(one, landedSides, "2026-11-07", counted)).join(" | ");
        if (going !== "replying | 11:19 | 11:19")
            failures.push(`a landing's side while Joel's words go: "${going}"`);

        // Agents' Recent is the agents daemon's (modules/agents/src/recent.rs),
        // which lists, leaves out and notes each; when one closed, at its
        // right: the time today, the day before it
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

        // Only quests among the disk's holders are titled ahead, and each
        // is called by its title once known, else by its id (VKCF3Z).
        const reef = { path: "/home/ada/.cache/scratch/REEF",
            owner: { kind: "quest", name: "REEF" } };
        const tree = { path: "/home/ada/trees/tide",
            owner: { kind: "worktree", name: "worktree-tide" } };
        const other = { path: "/home/ada/junk", owner: { kind: "other", name: "junk" } };
        const held = Holders.quests([reef, tree, other]).join(" ");
        if (held !== "REEF")
            failures.push(`the quests holding the disk are "${held}", not "REEF"`);
        if (Holders.quests(undefined).length !== 0)
            failures.push("a guard with no holders names quests");
        const titles = { REEF: "Mend the reef" };
        const called = (holder, worded) => Holders.called(holder, worded, id => titles[id],
            "/home/ada");
        const calls = [
            ["a quest by its title", called(reef, "{name}'s workspace"),
                "Mend the reef's workspace"],
            ["a quest not titled yet by its id",
                Holders.called(reef, undefined, () => undefined, "/home/ada"), "REEF"],
            ["a worktree without its prefix", called(tree, undefined), "Tide"],
            ["another folder from home", called(other, undefined), "~/junk"]
        ];
        for (const [what, said, want] of calls) {
            if (said !== want)
                failures.push(`${what}: "${said}", not "${want}"`);
        }

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

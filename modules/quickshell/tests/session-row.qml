// How the launcher says where a pi session stands, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-says-how-each-session-stands` check copies
// modules/quickshell/launcher/SessionRow.js, SessionDrawer.js, Holders.js,
// Landed.js and ReopenTally.js beside this file and runs
// `qml -platform offscreen session-row.qml`. How each session stands, in
// the one order every list keeps (needs you, working, in line, listening,
// paused, parked), is the agents daemon's (modules/agents/src/standing.rs),
// as is how Agents lays them out, each under the one orchestrating it
// (modules/agents/src/tree.rs),
// as are what a row says at its right and its drawer (drawer.rs), whose
// times alone are worded here.
import QtQuick
import "SessionDrawer.js" as SessionDrawer
import "Holders.js" as Holders
import "SessionRow.js" as SessionRow
import "Landed.js" as Landed
import "ReopenTally.js" as ReopenTally
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

        // How a session stands, and where that comes in the one order, are
        // the agents daemon's alone (modules/agents/src/standing.rs), its
        // cases there.

        // A session's acts follow how it stands: one at work or in line can
        // be stopped, one paused resumed, one parked only woken or talked to, with no
        // window known to go to. One here closes, last; one elsewhere can't, since its
        // node's relay of a close is dropped. None offers Park: only the daemon's served
        // row does, since one not yet served may be parked already and would be woken by
        // it, as the node wakes a parked session for any steer.
        const acts = [
            ["working", true, true, "switch talk pause stop close"],
            ["working", true, false, "switch talk pause stop"],
            ["inLine", false, true, "talk pause stop close"],
            ["listening", true, true, "switch talk pause close"],
            ["needs", true, false, "switch talk pause"],
            ["paused", true, true, "switch talk resume close"],
            ["parked", false, true, "wake talk"],
            ["off", false, true, "talk pause close"],
            ["off", true, false, "switch talk pause"]
        ];
        for (const [state, windowed, here, want] of acts) {
            const got = SessionRow.acts(state, windowed, here).join(" ");
            if (got !== want)
                failures.push(`the acts of one ${state}${here ? " here" : ""}: "${got}", `
                    + `not "${want}"`);
        }

        // A row a session is behind but isn't its own, as a landing's,
        // offers Talk and Go To of its acts, in their order, and none that
        // steers or ends it: Go To only where it has a window, and one
        // parked only Talk, which wakes it, though a window be said of it.
        // One asking a question or a yes has no Talk, as What Needs Me gives
        // none (Now.qml's talkTo); one stuck, or parked, keeps it.
        const behind = [
            ["working", true, false, "switch talk"],
            ["listening", false, false, "talk"],
            ["paused", true, false, "switch talk"],
            ["parked", true, false, "talk"],
            ["needs", true, true, "switch"],
            ["needs", false, false, "talk"],
            ["parked", false, true, "talk"],
            // Asking its orchestrator, not Joel, it isn't his to answer,
            // yet words said to it still come after the answer.
            ["listening", true, true, "switch"]
        ];
        for (const [state, windowed, asking, want] of behind) {
            const got = SessionRow.behindActs(state, windowed, asking).join(" ");
            if (got !== want)
                failures.push(`the acts behind one ${state}${asking ? " asking" : ""}: `
                    + `"${got}", not "${want}"`);
        }
        // A question or a yes holds an ask open; a want of another kind,
        // or none, doesn't.
        const asking = [
            [{ wants: { kind: "question" } }, true],
            [{ wants: { kind: "approval" } }, true],
            [{ wants: { kind: "action" } }, false],
            [{ wants: null }, false],
            [undefined, false]
        ];
        for (const [presence, want] of asking) {
            if (SessionRow.asks(presence) !== want)
                failures.push(`asks of ${JSON.stringify(presence)}: not ${want}`);
        }
        // A row offers the acts the daemon serves, in their order, and marks
        // the one its Return does, which for one paused is Resume, not its
        // first; one it doesn't list leads, and with nothing served, the
        // shell's own list, Return on its first.
        const served = ids => ids.map(id => ({ key: `verb:agent:${id}` }));
        const offers = [
            ["one paused marks Resume", served(["switch", "talk", "resume", "close"]),
                "verb:agent:resume", ["talk"], "switch talk resume close", "resume"],
            ["one listening marks Talk after Go To", served(["switch", "talk", "pause"]),
                "verb:agent:talk", ["talk"], "switch talk pause", "talk"],
            ["one live offers Park as served",
                served(["switch", "talk", "pause", "park", "stop", "close"]),
                "verb:agent:talk", [], "switch talk pause park stop close", "talk"],
            ["one parked marks Wake", served(["wake", "talk"]), "verb:agent:wake", ["talk"],
                "wake talk", "wake"],
            ["its order is the daemon's, not the shell's", served(["talk", "pause"]),
                "verb:agent:talk", ["switch", "talk", "pause"], "talk pause", "talk"],
            ["what Return does leads when the row doesn't list it", served(["talk", "pause"]),
                "verb:agent:switch", ["talk"], "switch talk pause", "switch"],
            ["its other keys aren't a session's acts",
                [{ key: "verb:agents:landed:try" }, ...served(["talk"])], "verb:agent:talk", [],
                "talk", "talk"],
            ["nothing served, the shell's own, Return on its first", undefined, undefined,
                ["switch", "talk", "pause"], "switch talk pause", "switch"],
            // A daemon newer than the shell, as through a switch, serves acts
            // the shell can't make: they're left out, Return's too.
            ["an act the shell doesn't make is left out", served(["talk", "hush", "pause"]),
                "verb:agent:talk", [], "talk pause", "talk"],
            ["a Return the shell doesn't make marks its first", served(["talk", "pause"]),
                "verb:agent:hush", [], "talk pause", "talk"],
            ["only acts it can't make served, the shell's own", served(["hush"]),
                "verb:agent:hush", ["talk"], "talk", "talk"]
        ];
        // A row's own Return is read as its acts are, so an act the shell
        // doesn't make is none, rather than one sessionAct can't make.
        const keys = [
            ["park", "verb:agent:park", "park"], ["stop", "verb:agent:stop", "stop"],
            ["one the shell doesn't make", "verb:agent:hush", ""],
            ["another kind's", "verb:agents:park", ""], ["none", undefined, ""]
        ];
        for (const [what, key, want] of keys) {
            if (SessionRow.known(key) !== want)
                failures.push(`${what}: known "${SessionRow.known(key)}", not "${want}"`);
        }
        // The shell's own acts: before a row's held, one standing "off"'s,
        // with no Park; held with nothing the shell makes, Talk alone.
        const fallbacks = [
            ["unheld, its window here", false, true, true, "switch talk pause close"],
            ["unheld, elsewhere", false, false, false, "talk pause"],
            ["held, its window here", true, true, true, "talk"]
        ];
        for (const [what, held, windowed, here, want] of fallbacks) {
            const got = SessionRow.fallbackOf(held, windowed, here).join(" ");
            if (got !== want)
                failures.push(`fallback ${what}: "${got}", not "${want}"`);
        }
        // A held row's Return does the act its drawer marks, whatever it serves.
        const returnsOf = [
            ["one paused", served(["switch", "talk", "resume"]), "verb:agent:resume", "resume"],
            ["one the shell doesn't make", served(["talk", "pause"]), "verb:agent:hush",
                "talk"],
            ["one it doesn't list", served(["talk"]), "verb:agent:switch", "switch"],
            ["nothing it makes", served(["hush"]), "verb:agent:hush", "talk"]
        ];
        for (const [what, acts, returns, want] of returnsOf) {
            const got = SessionRow.returnOf(acts, returns);
            const marked = SessionRow.offered(acts, returns, ["talk"]).marked;
            if (got !== want || got !== marked)
                failures.push(`Return of ${what}: ${got}, marked ${marked}, not ${want}`);
        }
        // Pressed, an act is steered only while its row still offers it.
        const pressed = [
            ["Park on one still live", { acts: served(["talk", "park"]) }, "park", true],
            ["Park on one parked since", { acts: served(["wake", "talk"]) }, "park", false],
            ["Pause on one paused since", { acts: served(["talk", "resume"]) }, "pause", false],
            ["Park on a row not yet held", null, "park", true],
            ["Park on a row serving no acts", {}, "park", true]
        ];
        for (const [what, row, id, want] of pressed) {
            if (SessionRow.stillOffers(row, id) !== want)
                failures.push(`${what}: ${want ? "refused" : "steered"}`);
        }
        for (const [what, acts, returns, fallback, want, mark] of offers) {
            const { ids, marked } = SessionRow.offered(acts, returns, fallback);
            if (ids.join(" ") !== want || marked !== mark)
                failures.push(`${what}: "${ids.join(" ")}" marking ${marked}, `
                    + `not "${want}" marking ${mark}`);
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
        // one waiting on Joel the alert's, and every other a program's, as
        // the daemon's rows are (modules/agents/src/standing.rs's hue).
        const hues = ["needs", "working", "inLine", "listening", "paused", "parked", "off"]
            .map(state => `${state}:${SessionRow.hue(state)}`).join(" ");
        if (hues !== "needs:alert working:program inLine:program listening:program"
            + " paused:muted parked:muted off:muted")
            failures.push(`the glyphs' roles by state: "${hues}"`);
        // A live one with a tier 2 figure takes the warning's hue; one paused
        // stays muted, since it isn't live, and one waiting on Joel keeps the
        // alert's.
        const warned = ["needs", "working", "listening", "paused", "off"]
            .map(state => `${state}:${SessionRow.hue(state, true)}`).join(" ");
        if (warned !== "needs:alert working:warning listening:warning paused:muted off:muted")
            failures.push(`the glyphs' roles warned: "${warned}"`);
        // One waiting on Joel wears his mark in Agents; For You keeps the
        // mark and hue it'd wear not waiting, or with none served, its own.
        const kept = [
            { glyph: "waitsOnJoel", hue: "alert", mark: "shapeSquare", markHue: "warning" },
            { glyph: "agent", hue: "program", mark: undefined }
        ].map(made => JSON.stringify(SessionRow.unwaited(made))).join(" ");
        if (kept !== '{"glyph":"shapeSquare","hue":"warning"} {"glyph":"agent","hue":"program"}')
            failures.push(`For You's own marks: ${kept}`);

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
        // A session's last landing is folded under z a, out of its glance
        // (VKCF3Z, 2b03e43c), and still says when, worded as a fact's is.
        const foldedNerd = SessionDrawer.shown({ facts: [], nerd: [
            { label: "Session", value: "s-1" },
            { label: "Last Landing", value: "Serve rows \u00b7 core.nix#1",
                at: { at: 1544, as: "clock" } }] }, clock).nerd
            .map(bit => `${bit.label}=${bit.value}${bit.at ? "+at" : ""}`).join("; ");
        const foldedWant = "Session=s-1; Last Landing=Serve rows \u00b7 core.nix#1 \u00b7 @1544";
        if (foldedNerd !== foldedWant)
            failures.push(`the fold's times: "${foldedNerd}", not "${foldedWant}"`);
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

        // The sessions an arrow reaches next from the one drawn, whose
        // drawers Agents watches ahead with it: past a heading, and never a
        // row that isn't a session's, nor the drawn one itself.
        const listed = [{ key: "agent:TOP" }, { key: "heading:fold", heading: true },
            { key: "agent:ONE" }, { key: "agent:TWO" }, { key: "agents:spec" }];
        const headed = [{ key: "heading:a", heading: true }, { key: "agent:LAST" },
            { key: "heading:b", heading: true }, { key: "heading:c", heading: true }];
        const twice = [{ key: "agent:SAME" }, { key: "agent:SAME" }];
        const besides = [
            ["past a heading above and to a session below", listed, 2, "ONE", "TOP TWO"],
            ["none past the list's top", listed, 0, "TOP", "ONE"],
            ["a row not a session's is none", listed, 3, "TWO", "ONE"],
            ["found though another session's row is selected", listed, 0, "ONE", "TOP TWO"],
            ["found with no row selected", listed, -1, "ONE", "TOP TWO"],
            ["none for a session not listed", listed, 0, "GONE", ""],
            ["none past headings to either end", headed, 1, "LAST", ""],
            ["never the drawn session itself", twice, 0, "SAME", ""]
        ];
        for (const [what, rows, current, address, want] of besides) {
            const said = SessionDrawer.beside(rows, current, address).join(" ");
            if (said !== want)
                failures.push(`beside ${address}, ${what}: "${said}", not "${want}"`);
        }

        // What the foot says of an act refused: the session by its name, never
        // its address while a name is known, and the node's reason without
        // the address it opens with, nor anywhere else, where its name stands.
        const machineSaid = { untold: "not told, {why}", refused: "{act} refused, {why}",
            unanswered: "the node didn't answer, so it may not have been told",
            unansweredTo: "{act} had no answer from the node, so it may not be done" };
        const unheld = "isn't parked with a holder to wake it this boot";
        const refusals = [
            ["a Wake refused, by its row's name, the address left out", "Wake",
                `PARKED ${unheld}`, "PARKED", "Parked Quest",
                ["Parked Quest", `Wake refused, ${unheld}`]],
            ["the address left out whatever case the node wrote it in", "Pause",
                "parked is out of reach", "PARKED", "Parked Quest",
                ["Parked Quest", "Pause refused, is out of reach"]],
            ["its address within a reason said by its name", "Wake",
                "lighthouse is out of reach, so PARKED can't be told now", "PARKED", "Parked Quest",
                ["Parked Quest", "Wake refused, lighthouse is out of reach, so Parked Quest "
                    + "can't be told now"]],
            ["and as it's owned, past its own colon", "Wake", "PARKED: PARKED's holder left",
                "PARKED", "Parked Quest",
                ["Parked Quest", "Wake refused, Parked Quest: Parked Quest's holder left"]],
            ["nor one only opening with its address's letters", "Wake", "PARKEDS went away",
                "PARKED", "Parked Quest", ["Parked Quest", "Wake refused, PARKEDS went away"]],
            ["a reason's dollar signs said as they are", "Wake", "PARKED lost $& and $1",
                "PARKED", "Parked Quest", ["Parked Quest", "Wake refused, lost $& and $1"]],
            ["a name's dollar signs said as they are", "Wake", "so PARKED went", "PARKED",
                "Cost $& Quest", ["Cost $& Quest", "Wake refused, so Cost $& Quest went"]],
            ["no answer is the node's silence, never a refusal", "Wake", "", "PARKED",
                "Parked Quest",
                ["Parked Quest", "Wake had no answer from the node, so it may not be done"]],
            ["nor for words talked to it", "", "", "PARKED", "Parked Quest",
                ["Parked Quest", "the node didn't answer, so it may not have been told"]],
            ["words talked to it name no act", "", "PARKED is busy", "PARKED", "Parked Quest",
                ["Parked Quest", "not told, is busy"]],
            ["Talk is named as its act", "Talk", "PARKED is busy", "PARKED", "Parked Quest",
                ["Parked Quest", "Talk refused, is busy"]],
            ["its address only with no name known", "Wake", `PARKED ${unheld}`, "parked", "",
                ["PARKED", `Wake refused, ${unheld}`]]
        ];
        for (const [what, act, why, address, name, want] of refusals) {
            const said = SessionRow.refusal(machineSaid, { act, why, address, name });
            if (JSON.stringify(said) !== JSON.stringify(want))
                failures.push(`refusal, ${what}: ${JSON.stringify(said)}`);
        }
        // An act's name comes from the vocabulary, and one it doesn't name, as
        // For You's fill or a look's answer, is said as none, never its id.
        const vocabulary = { "verb:agent:wake": "Wake", "verb:agent:talk": "Talk" };
        const nameOf = id => vocabulary[id] ?? id;
        const actNames = [
            ["a known act by its name", "wake", "Wake"],
            ["Talk by its name", "talk", "Talk"],
            ["an act the vocabulary doesn't name is none", "fill", ""],
            ["nor a look's answer", "answer", ""],
            ["no act is none", "", ""],
            ["nor a missing one", undefined, ""]
        ];
        for (const [what, act, want] of actNames) {
            const said = SessionRow.actName(act, nameOf);
            if (said !== want)
                failures.push(`actName, ${what}: "${said}", not "${want}"`);
        }
        const unnamed = SessionRow.refusal(machineSaid, { act: SessionRow.actName("fill", nameOf),
            why: "PARKED is busy", address: "PARKED", name: "Parked Quest" });
        if (JSON.stringify(unnamed) !== JSON.stringify(["Parked Quest", "not told, is busy"]))
            failures.push(`refusal, an unnamed act said act-less: ${JSON.stringify(unnamed)}`);

        // A row drawn as woken is taken back only for a Wake, refused or never
        // answered, while it's still becoming woken; any other act refused
        // meanwhile leaves it be, and is said by refusal as itself.
        const takings = [
            ["a refused Wake, by the node's reason, its address said by name", "wake",
                "so PARKED went", true, "so Parked Quest went"],
            ["a Wake never answered, as no reason, said not woken yet", "wake", "", true, ""],
            ["never for a Pause refused meanwhile", "pause", "PARKED is busy", true, null],
            ["nor a Pause never answered", "pause", "", true, null],
            ["nor words talked to it", "", "PARKED is busy", true, null],
            ["nor a Wake whose row isn't becoming woken", "wake", "PARKED is busy", false, null],
            ["nor a Wake never answered whose row isn't becoming woken", "wake", "", false, null]
        ];
        for (const [what, act, why, held, want] of takings) {
            const back = SessionRow.takesBack(
                { act, why, address: "PARKED", name: "Parked Quest", held });
            if (back !== want)
                failures.push(`takesBack, ${what}: ${JSON.stringify(back)}`);
        }

        // What Agents does with each: a Wake taken back goes to becameNot as
        // itself, "" too, and anything else is said on the foot.
        const toldLauncher = [];
        const launcherStandIn = {
            becameNot: (key, why) => toldLauncher.push(["becameNot", key, why]),
            tellOnOpen: lines => toldLauncher.push(["tellOnOpen", lines])
        };
        const steers = [
            ["a Wake never answered, let go at once as no answer", "wake", "Wake", "",
                [["becameNot", "agent:PARKED", ""]]],
            ["a refused Wake, let go by the node's reason", "wake", "Wake", "so PARKED went",
                [["becameNot", "agent:PARKED", "so Parked Quest went"]]],
            ["a Pause never answered, said on the foot", "pause", "Pause", "",
                [["tellOnOpen", [["Parked Quest",
                    "Pause had no answer from the node, so it may not be done", "alert"]]]]]
        ];
        for (const [what, act, actName, why, want] of steers) {
            toldLauncher.length = 0;
            SessionRow.unsteered(launcherStandIn, machineSaid, { act, actName, why,
                address: "PARKED", name: "Parked Quest", called: "Parked Quest", held: true },
                "alert");
            if (JSON.stringify(toldLauncher) !== JSON.stringify(want))
                failures.push(`unsteered, ${what}: ${JSON.stringify(toldLauncher)}`);
        }

        const names = [
            ["its row's name first", "Parked Quest", "Its Quest", "PARKED", "Parked Quest"],
            ["its quest's title with no row served", undefined, "Its Quest", "PARKED", "Its Quest"],
            ["a blank name is none", "  ", "", "PARKED", "PARKED"],
            ["nothing at all is nothing", null, null, undefined, ""]
        ];
        for (const [what, served, title, address, want] of names) {
            const said = SessionRow.footName(served, title, address);
            if (said !== want)
                failures.push(`footName, ${what}: "${said}", not "${want}"`);
        }
        // As the shell hands a session over: one with a record by the title
        // its presence names, one started in pi's host with none yet by the
        // quest's name the launcher gave it, never its id.
        const titleOf = presence => presence.questInfo.title;
        const handed = [
            ["one with a record by its presence's title", { address: "QEST-X",
                name: "Kelp", presence: { questInfo: { title: "Reef" } } }, "Reef"],
            ["one with no record yet by the quest's name it was started on",
                { address: "QEST-X", machine: "", name: "Kelp" }, "Kelp"],
            ["one with neither by its address", { address: "QEST-X", machine: "" }, "QEST-X"]
        ];
        for (const [what, session, want] of handed) {
            const said = SessionRow.footNameOf(undefined, session, titleOf);
            if (said !== want)
                failures.push(`footNameOf, ${what}: "${said}", not "${want}"`);
        }

        // What Joel hears of a walk, as Sessions counts it (tests/places.qml
        // holds the count): one folder gone or many, then how many
        // reopened, in Agents' words, and nothing where a word is missing.
        const words = { folderGone: "folder gone", foldersGone: "folders gone",
            reopened: "reopened" };
        const toldOf = [
            ["one folder gone", { gone: 1, reopened: 2 }, words,
                '[["1","folder gone"],["2","reopened"]]'],
            ["many folders gone", { gone: 2, reopened: 1 }, words,
                '[["2","folders gone"],["1","reopened"]]'],
            ["no words", { gone: 1, reopened: 0 }, {}, '[["1",""],["0",""]]']
        ];
        for (const [what, now, said, want] of toldOf) {
            const got = JSON.stringify(ReopenTally.told(now, said));
            if (got !== want)
                failures.push(`told, ${what}: ${got}, not ${want}`);
        }

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

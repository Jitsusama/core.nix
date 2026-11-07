// pi's sessions, a collection: every pi running in a window, the one used
// last first, each named by the quest it has loaded and beside it its
// address, whether it's working, waiting on Joel or its turn in the
// machine's queue (Jobs.qml), what it spent today as spend-watch says
// (SpendWatch.qml), never while the screen is shared, and the folder it
// works in.
// Return goes to its window; the right arrow steps into what's known of it,
// each fact a row Return copies: its quest, its addresses, when it started,
// what it spent today and what of that its councils and fleets cost, the
// heavy work it has running or waiting, what it works on and what it
// hears. What runs, and
// how each is doing, is Sessions.qml's.
//
// Sessions on the realm's other machines are in the same list, drawn the
// same, their machine leading the note (VKCF3Z, 2026-10-29, G8 of
// mycelium's plan), in the one order every list of sessions keeps
// (SessionRow.js): those needing Joel, then those working, in line,
// listening and paused, then those parked, this machine's first in each,
// then the others by when each last changed. Go To joins one elsewhere
// through its node; one on a machine out of reach stays, dimmed, saying
// so, and does nothing. One parked has given its memory back until it's
// wanted (Sessions.parked), so Return wakes it in its own window, and
// talking to it wakes it too.
//
// Each session sits under the one orchestrating it, a step in, and two
// under that one's orchestrator, with no line or box: the step alone says
// it, and an orchestration stands where its most urgent row would
// (QEST-20261026-15J7N3's agents-tree-look.md). Only typing ranks, so
// while Joel types every row is at no depth, its orchestrator's address
// leading its note, as a row that arrives while the list is open is
// (Steady.js). An orchestrator says what its whole orchestration spent
// today, as spend-watch sums it by quest, and its drawer what of that was
// its own and its sessions'. Those closed stand at the tree's end,
// dimmed, the one last live first, and Return reopens one where it left
// off (VKCF3Z, 2026-11-07 [9a4b1dd0]), so the tree is the one place an
// orchestrator is listed.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Numbers.js" as Numbers
import "SessionRow.js" as SessionRow

Provider {
    id: provider

    what: Words.of("agents").what

    // Named, and found, as the vocabulary says (Words.qml): its tasks,
    // helpers and spend are its parts there.
    collection: Words.collection("agents")

    // In the one order of how they stand, each in the order Sessions keeps
    // within it (this machine's, the one used last first, then the others,
    // the one that changed last first), each under its orchestrator.
    // Made only while the launcher is open or its catalogue is walked from
    // outside, since every session's row reads the machine queue and the
    // session's presence, which change every few seconds; Home's count
    // reads Sessions itself. Nothing is kept from one making to the next,
    // so a row and its drawer say how the session stands now.
    readonly property var all: {
        const launcher = provider.launcher;
        if (!launcher || !(launcher.open || launcher.cataloguing))
            return [];
        // Those parked after, standing last in the one order, under their
        // orchestrators as the rest are.
        const running = Sessions.running.concat(Sessions.parked);
        // The orchestrators the harness lists, which say what their whole
        // orchestration spent even with none of theirs running.
        const orchestrators = source("orchestrators");
        const listed = (orchestrators?.listed ?? []).map(one => one.address);
        const laid = SessionRow.tree(running.map(session => ({
            address: session.address,
            under: session.presence?.orchestratedBy ?? "",
            state: stateOf(session)
        })), true);
        return rowsOf(laid, running, listed)
            .concat((orchestrators?.closed ?? []).map(one => closedRow(one, orchestrators)));
    }

    // A closed orchestrator's row: its glyph muted, as one paused is, its
    // name in text, "closed" at its right with what its whole orchestration
    // spent today when that's anything, its quest's title as its note, and
    // Return reopening it where it left off. One whose folder is gone says
    // so in alert, and starts afresh in its quest's start folder
    // (Orchestrators.qml's goneFrom), or, with none, steps in to its talks.
    function closedRow(one, orchestrators) {
        const family = one.quest && !Quiet.sharing ? SpendWatch.families[one.quest] ?? null
            : null;
        const items = SessionRow.side(word("closed"),
            family ? word("allOf", { spent: family.all }) : "");
        const gone = orchestrators.goneFrom(one);
        const row = member({
            thing: true,
            key: `orchestrator:${one.address}`,
            kind: SessionRow.KIND,
            name: one.short || one.title || one.address,
            also: Fuzzy.prepare(`${one.address} ${one.quest ?? ""} ${one.title ?? ""}`),
            glyph: Theme.glyph.agent,
            hue: SessionRow.hue("off"),
            side: items.map(item => item.said).join(" \u00b7 "),
            sideItems: items,
            note: one.title ?? "",
            depth: 0,
            verb: Words.verb("verb:orchestrator:reopen"),
            run: () => orchestrators.start(one),
            verbs: () => provider.closedActs(one, orchestrators)
        });
        if (gone !== null)
            Object.assign(row, {
                side: gone.side,
                sideItems: [{ said: gone.side, keep: 3 }],
                alerts: true,
                hue: "alert",
                verb: gone.afresh ? Words.verb("verb:orchestrator:afresh") : "more",
                run: gone.afresh ? gone.afresh : () => provider.launcher?.enter(row)
            });
        return row;
    }

    // A closed orchestrator's acts: Reopen, or Start Afresh where its
    // folder is gone, then its talks, newest first.
    function closedActs(one, orchestrators) {
        const act = (id, run) => thing({
            key: `verb:orchestrator:${id}`,
            verb: Words.verb(`verb:orchestrator:${id}`),
            name: Words.name(`verb:orchestrator:${id}`),
            glyph: Words.glyph(`verb:orchestrator:${id}`),
            hue: "action",
            run: run
        });
        const gone = orchestrators.goneFrom(one);
        const first = gone === null ? [act("reopen", () => orchestrators.start(one))]
            : gone.afresh ? [act("afresh", gone.afresh)] : [];
        return first.concat(orchestrators.talksOf(one.address));
    }
    // Said as every row it shows moves, the sessions' and those after
    // them, so a step held in Landed loses a landing as it's read.
    readonly property var everyRow: everything()
    onEveryRowChanged: changedTo(everyRow)

    // The rows of what SessionRow.tree laid out: a session's own, or the
    // fold of an orchestration's members that only wait, which says how
    // many of each state it holds, "22 listening, 3 paused", and whose
    // Return steps in to them (VKCF3Z, 2026-11-06). It holds their rows,
    // so typing still finds each.
    function rowsOf(laid, running, listed) {
        return laid.map(one => {
            if (!one.fold)
                return made(running[one.at], {
                    depth: one.depth,
                    orchestrates: one.orchestrates || listed.includes(running[one.at].address)
                });
            const held = rowsOf(one.members, running, listed);
            const row = thingRow({
                key: `agents:fold:${one.under}`,
                name: one.counts.map(each => `${each.count} ${word(each.state)}`).join(", "),
                glyph: Theme.glyph.agent,
                hue: "program",
                copiesNothing: true,
                depth: one.depth
            }, () => held);
            row.held = held;
            return row;
        });
    }

    // Every session's row, those folded too, as typing searches them.
    function sessionRows(rows) {
        return rows.reduce((every, row) => every.concat(row.held ? sessionRows(row.held)
            : [row]), []);
    }

    // How a session stands (SessionRow.js), from its presence, whether its
    // want is truly Joel's and whether it waits its turn in line.
    function stateOf(session) {
        return SessionRow.state(session.presence, Sessions.asks(session),
            inLine(session.presence) !== "");
    }

    // One of the vocabulary's words for a session, filled in.
    function word(key, fill) {
        let said = String((Words.of("agents").said ?? {})[key] ?? "");
        for (const name of Object.keys(fill ?? {}))
            said = said.replace(`{${name}}`, fill[name]);
        return said;
    }

    // What's said of a session elsewhere, in the vocabulary's words.
    function elsewhere(key, session, why) {
        return String((Words.of("agents:machine").said ?? {})[key] ?? "")
            .replace("{machine}", session.machine).replace("{why}", why ?? "");
    }

    // A session elsewhere that couldn't be joined or told is said in the
    // foot, by its address in the alert colour, where the act was pressed:
    // at once while the launcher's open, or else once it opens next, never
    // as a notice (VKCF3Z, 2026-11-07 [4a00987e]).
    Connections {
        target: Sessions

        function onUnjoined(session) {
            provider.launcher?.tellOnOpen([[session.address,
                provider.elsewhere(session.reachable ? "unheld" : "unreached", session),
                Theme.alert]]);
        }

        function onUnsteered(session, why) {
            provider.launcher?.tellOnOpen([[session.address,
                provider.elsewhere(why !== "" ? "untold" : "unanswered", session, why),
                Theme.alert]]);
        }

        function onToReopenChanged() {
            provider.lookForFolders();
        }
    }

    // Where a session's first job waiting for its turn stands in line.
    function inLine(presence) {
        const waiting = presence && presence.session
            ? Jobs.of(presence.session).find(job => job.state === "waiting") : undefined;
        return waiting ? word("inLine", { place: Numbers.ordinal(waiting.place) }) : "";
    }

    // What a session spent today as spend-watch says, and, for one that
    // `orchestrates`, its whole orchestration's as spend-watch sums its
    // quest and those under it, or nothing to say either by while sharing.
    function spendOf(presence, quest, place) {
        const spent = presence && presence.session && !Quiet.sharing
            ? SpendWatch.sessions[presence.session] ?? "" : "";
        const family = place?.orchestrates && quest !== "" && !Quiet.sharing
            ? SpendWatch.families[quest] ?? null : null;
        return { spent, family };
    }

    // A session's row, `depth` steps in under its orchestrator, and saying
    // what its whole orchestration spent when it `orchestrates`.
    function made(session, place) {
        const depth = place?.depth ?? 0;
        const presence = session.presence;
        const queued = inLine(presence);
        // A want that isn't truly Joel's (Sessions.asks) reads as what the
        // session does meanwhile.
        const asks = Sessions.asks(session);
        const state = SessionRow.state(presence, asks, queued !== "");
        const doing = state === "off" ? ""
            : state === "needs" ? Sessions.wanted(presence.wants)
            : state === "inLine" ? queued
            : word(state);
        const quest = presence && presence.owns
            ? presence.owns.find(owned => /^QEST-/.test(owned)) ?? "" : "";
        const { spent, family } = spendOf(presence, quest, place);
        const note = asks && presence.wants.said ? presence.wants.said
            : presence && presence.worksOn ? presence.worksOn : "";
        // Elsewhere, its machine leads the note; out of reach, it's all
        // the note says, and the row does nothing.
        const where = session.here ? ""
            : elsewhere(session.reachable ? "on" : "away", session);
        const noted = !session.reachable ? where
            : [where, note].filter(part => part !== "").join(" \u00b7 ");
        // Ranked flat, whom it's under leads.
        const orchestrator = presence?.orchestratedBy ?? "";
        const under = orchestrator !== "" && orchestrator !== session.address
            ? word("under", { address: orchestrator }) : "";
        // What it spent today, an orchestrator's whole orchestration's.
        const figure = family ? word("allOf", { spent: family.all }) : spent;
        // An orchestrator is found by what the harness lists it as too,
        // its short name and its quest's title, as it was in the place of
        // its own this tree replaced.
        const listedAs = place?.orchestrates ? (source("orchestrators")?.listed ?? [])
            .find(one => one.address === session.address) ?? null : null;
        const known = listedAs ? ` ${listedAs.short ?? ""} ${listedAs.title ?? ""}` : "";
        // A tier 2 figure, while it's true: its context past 80% full.
        const full = state === "off" ? ""
            : SessionRow.full(presence?.usage, Words.of("agents").said ?? {});
        const items = SessionRow.side(doing, figure, full);
        const row = member({
            thing: true,
            key: `agent:${session.address}`,
            kind: SessionRow.KIND,
            // The session is its window, which the windows and its quest
            // also find, and it says the most of the three, so it's the
            // one row left of them (Launcher.qml's compose).
            is: Sessions.isOf(session),
            rich: 2,
            name: session.title,
            // Its address, folder and quest's id, made ready to match, since
            // a member keeps its words as given.
            also: Fuzzy.prepare(`${session.address} ${session.folder} ${quest}${known}`),
            glyph: Theme.glyph.agent,
            hue: SessionRow.hue(state, full !== ""),
            side: items.map(item => item.said).join(" \u00b7 "),
            sideItems: items,
            note: noted,
            depth: depth,
            gone: !session.reachable,
            // Return does the act its state makes harmless (SessionRow.js's
            // returns), one of those its step in holds.
            returns: provider.sessionAct(session,
                SessionRow.returns(state, windowedOf(session))),
            verbs: () => provider.verbs(session, quest, spent, family),
            // What y takes: the first of its facts, as its verbs list them.
            operands: () => [quest !== "" ? copyOperand("fact:agent:quest", quest)
                : copyOperand("fact:agent:address",
                    (session.presence?.addresses ?? [session.address]).join(", "))],
            drawer: () => provider.drawerOf(session, quest, place)
        });
        // Itself as it reads ranked flat: at no depth, whom it's under
        // leading its note. It isn't named flat, which Quickshell's
        // JavaScript would read as the arrays' method it lacks.
        // Whom it's under leads what it says at its right too, given way
        // first, and found from the top its words after its name
        // (Launcher.qml's gathering), since its note is said only in the
        // foot (15J7N3, 2026-11-04).
        const ranked = [{ said: under, keep: 1 }].concat(items).filter(item => item.said !== "");
        row.ranked = () => Object.assign({}, row, {
            depth: 0,
            note: [under, noted].filter(part => part !== "").join(" \u00b7 "),
            side: ranked.map(item => item.said).join(" \u00b7 "),
            sideItems: ranked,
            belongs: under
        });
        return row;
    }

    // What a session's drawer says (SessionRow.js), read as it's shown:
    // for an orchestrator, its orchestration's spend, then its own and its
    // sessions'.
    function drawerOf(session, quest, place) {
        const presence = session.presence ?? {};
        // Read as it's shown, so spend-watch's next sum reaches an open
        // drawer even while the row stays as it was found.
        const { spent, family } = spendOf(presence, quest, place);
        // How its day went, as spend-watch says: its hours carry no dollars,
        // but what rebuilding its context cost is an amount, hidden while
        // the screen is shared.
        const day = SpendWatch.sessionsToday[presence.session ?? ""] ?? null;
        return SessionRow.drawer(session.title, presence, {
            hours: day?.graph ?? null,
            rebuilt: Quiet.sharing ? "" : String(day?.rebuilt ?? ""),
            spent: family ? word("allOf", { spent: family.all }) : spent,
            own: family ? family.own : "",
            under: family ? family.under : "",
            since: presence.owner?.startedAt
                ? word("since", { at: started(presence.owner.startedAt) }) : "",
            address: session.address,
            // Where it works: its repository, as its record names it,
            // else its folder.
            place: presence.repo || session.folder,
            machine: session.machine,
            named: id => Words.name(`agents:${id}`),
            // Its model, context, last turn and today's figures, as its
            // record's usage keeps them by gen_ai's names.
            figures: SessionRow.figures(presence.usage, Qt.formatDate(new Date(), "yyyy-MM-dd"),
                Words.of("agents").said ?? {}, id => Words.name(`agents:${id}`))
        });
    }

    // What the foot says once words have gone to a session.
    property string told: ""

    // A session's own acts, in the one order every session keeps
    // (SessionRow.js), each steering it through its inbox but Go To; an
    // orchestrator's row is its session's, with its talks after them.
    function sessionActs(session) {
        return SessionRow.acts(stateOf(session), windowedOf(session), session.here)
            .map(id => sessionAct(session, id));
    }

    // What Close leaves of a session: its worktree, where it works in a
    // folder under a trees folder, as the harness's and World's trees both
    // are cut, else its folder, and its quest's Needs You items, which stay
    // until each is done.
    function leaves(session) {
        const presence = session.presence ?? {};
        const cwd = String(presence.cwd ?? "");
        const inTree = cwd.includes("/trees/");
        const quest = presence.quest ?? presence.questInfo?.id ?? "";
        const needs = quest === "" ? 0
            : WaitingOnYou.items.filter(item => item.quest === quest).length;
        return SessionRow.leaves(inTree, needs, Words.of("agents").said ?? {});
    }

    // Whether a session has a window to go to: its own here, or one its
    // machine's node attaches elsewhere.
    function windowedOf(session) {
        return !!(session.window || session.kitty || (!session.here && session.attach));
    }

    // One of a session's acts, by its id in SessionRow.js's acts.
    function sessionAct(session, id) {
        const act = (id, fields) => thing(Object.assign({
            key: `verb:agent:${id}`,
            verb: Words.verb(`verb:agent:${id}`),
            name: Words.name(`verb:agent:${id}`),
            glyph: Words.glyph(`verb:agent:${id}`),
            hue: "action"
        }, fields));
        const steered = what => () => Sessions.steer(session.address, { act: what });
        const made = {
            switch: () => act("switch", { later: true, run: () => Sessions.goTo(session) }),
            talk: () => act("talk", {
                ask: Words.of("verb:agent:talk").ask ?? "",
                chip: session.title,
                state: () => provider.told,
                given: words => {
                    if (Sessions.talk(session.address, words))
                        provider.told = (Words.of("orchestrators").said ?? {}).sent
                            ?.replace("{name}", session.title) ?? "";
                }
            }),
            wake: () => act("wake", { run: () => Sessions.wake(session) }),
            pause: () => act("pause", { run: steered("pause") }),
            resume: () => act("resume", { run: steered("resume") }),
            // Stop ends the turn under way, which nothing puts back, so it's
            // an ending: Return writes :interrupt and a second runs it, as d
            // does. It doesn't ask, as :interrupt is guarded, since the turn
            // lost is the one stopped for going wrong.
            stop: () => act("stop", Object.assign({ run: steered("stop"),
                askedAs: session.address }, Words.ending("verb:agent:stop"))),
            // Close ends the session as /exit does, its window with it,
            // asked first, saying what stays (Confirm.question). The ask
            // names it by its address, since the step-in's path above
            // already says its title, which left no room for the ask.
            close: () => act("close", Object.assign({ run: steered("close"),
                leaves: leaves(session), askedAs: session.address },
                Words.ending("verb:agent:close")))
        };
        return made[id]();
    }

    // When a session started, as a clock says it, with its day when that
    // wasn't today.
    function started(at) {
        const when = new Date(at);
        const today = new Date().toDateString() === when.toDateString();
        return Qt.formatDateTime(when, today ? "HH:mm" : "ddd d MMM, HH:mm");
    }

    function verbs(session, quest, spent, family) {
        const presence = session.presence ?? {};
        // Each fact a part, named first (agents:task and the rest), saying
        // its value and copying it.
        const fact = (key, name, side) => copyRow({
            key: `fact:agent:${key}`,
            name: name,
            side: side,
            copy: side,
            glyph: Theme.glyph.agent
        });
        const named = (id, state) => Words.name(`agents:${id}`, state);
        const addresses = presence.addresses ?? [session.address];
        // What of its spend went on its councils and fleets, hidden with it,
        // each named by what it went on.
        const helpers = spent === "" ? []
            : (SpendWatch.helpers[presence.session] ?? [])
                .map(helper => fact(`on:${helper.on}`,
                    helper.on.charAt(0).toUpperCase() + helper.on.slice(1), helper.spent));
        // Its heavy work, what runs and then what waits, as the queue says.
        const jobs = presence.session ? Jobs.of(presence.session).map(job => copyRow({
            key: `fact:agent:job:${job.id}`,
            name: job.what,
            side: job.state === "waiting" ? `${Numbers.ordinal(job.place)} in line` : "running",
            glyph: Theme.glyph[job.state === "waiting" ? "queued" : "running"]
        })) : [];
        const facts = [
            quest !== "" ? fact("quest", named("task"), quest) : null,
            fact("address", named("address", addresses.length === 1 ? "" : "several"),
                addresses.join(", ")),
            presence.owner?.startedAt
                ? fact("started", named("started"), started(presence.owner.startedAt)) : null,
            family ? fact("spent", named("spent"), word("allOf", { spent: family.all }))
                : spent !== "" ? fact("spent", named("spent"), spent) : null,
            family ? fact("own", named("own"), family.own) : null,
            family ? fact("under", named("under"), family.under) : null
        ].concat(helpers, jobs, [
            presence.worksOn ? fact("works", named("works"), presence.worksOn) : null,
            (presence.groups ?? []).length > 0
                ? fact("hears", named("hears"), presence.groups.join(", ")) : null
        ]).filter(row => row !== null);
        // Its acts, then what's known of it, then what Joel said to it and
        // what came of each, newest first.
        const talks = source("orchestrators")?.talksOf(session.address) ?? [];
        return sessionActs(session).concat(facts, talks);
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of sessionRows(all)) {
            const byName = (row.prepared.mask & need) === need
                ? Fuzzy.match(row.prepared, words, false) : { score: -1 };
            const byWord = (row.also.mask & need) === need
                ? Fuzzy.match(row.also, words, false) : { score: -1 };
            const best = byName.score >= byWord.score ? byName : byWord;
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row });
        }
        const grade = Math.min(3, ...found.map(f => f.grade));
        const kept = found.filter(f => f.grade === grade).sort((a, b) => b.score - a.score);
        // Ranked, each reads flat, at no depth (the header says why).
        return { rows: kept.map(f => f.row.ranked ? f.row.ranked() : f.row), grade: grade };
    }

    // Starting a session, Agents' own last row, after the sessions running
    // (Joel, 2026-10-28: "i'm also not able to start an orchestrator agent
    // session from the launcher ... it is also a bit non-intuitive"): one
    // row listing the unfinished quests with no session, each the quest's
    // own row, whose Return starts one on it and whose step in holds Start
    // Here, then Orchestrate (Quests.qml), so whether it orchestrates is
    // an act, never a question asked first (VKCF3Z, 2026-11-07
    // [9a4b1dd0]). A closed orchestrator reopens on its own row. Found by
    // its name from anywhere, and folded, so home names the sessions,
    // never it.
    function source(name) {
        return (provider.launcher?.sources ?? []).find(one => one.settings.name === name) ?? null;
    }

    readonly property var start: {
        const quests = () => (source("quests")?.everything() ?? [])
            .filter(row => row.unfinished && row.verb !== "go to");
        return Object.assign(thingRow({
            key: "agents:start",
            name: Words.name("agents:start"),
            glyph: Theme.glyph.agent,
            hue: "action"
        }, quests), {
            folded: true
        });
    }

    // The sessions there are to reopen (Sessions.qml's toReopen): those the
    // last boot's end took away, gone on the boot they ran in or once put
    // off, and those other machines carried here, offered whatever the
    // boot until each runs here (VKCF3Z's design, [fce929da], [17db838c]).
    // One row after the sessions running while any are left, which Now
    // shows among what else wants Joel (WhatsOn.qml). Return steps in with
    // Reopen Them All first, so Return twice brings every one back, each in
    // its own window and folder; then each one, as a closed session,
    // reopening just it, the last boot's and then the carried, newest
    // first; then Not Now, which puts off only the last boot's, so it's
    // there only while any are. Its count is at its side, never in its
    // name, which would change as he reopens them, and its note says where
    // they came from.
    readonly property var toReopen: {
        const lost = Sessions.toReopen;
        if (lost.length === 0)
            return null;
        const said = Words.of("agents:to-reopen").said ?? {};
        const act = (id, fields) => thing(Object.assign({
            key: `verb:agents:to-reopen:${id}`,
            verb: Words.verb(`verb:agents:to-reopen:${id}`),
            name: Words.name(`verb:agents:to-reopen:${id}`),
            glyph: Words.glyph(`verb:agents:to-reopen:${id}`),
            hue: "action"
        }, fields));
        // Where one came from: the last boot, or the machine that carried
        // it; several machines are counted rather than listed.
        const carriedFrom = machines => String(said.carried ?? "").replace("{machine}",
            machines.length === 1 ? machines[0]
                : String(said.machines ?? "").replace("{count}", machines.length));
        const fromOf = one => one.carried === undefined ? String(said.lastBoot ?? "")
            : carriedFrom([one.carried.from]);
        const booted = Sessions.lastBoot.length;
        const machines = Sessions.carried.map(one => one.carried.from)
            .filter((from, at, all) => all.indexOf(from) === at);
        const note = machines.length === 0 ? String(said.lastBoot ?? "")
            : booted === 0 ? carriedFrom(machines)
            : String(said.both ?? "").replace("{boot}", booted)
                .replace("{carried}", Sessions.carried.length)
                .replace("{from}", carriedFrom(machines));
        const home = Sessions.home;
        const homely = folder => folder === home || folder.startsWith(`${home}/`)
            ? `~${folder.slice(home.length)}` : folder;
        // In its own folder only, never the home folder instead. One whose
        // folder is gone says so on its row, looked for as the launcher
        // opens (lookForFolders), and holds no Reopen (VKCF3Z, 2026-11-07
        // [4a00987e]); one whose folder went since it was listed is said
        // once the launcher opens next, with how the rest went, never as a
        // notice.
        const oneSaid = Words.of("verb:agents:to-reopen:one").said ?? {};
        const goneHere = one => one.machine === undefined && Sessions.folderGone(one.cwd);
        const reopen = one => Sessions.reopen(one, cwd => provider.launcher?.tellOnOpen([
            [homely(cwd), String(oneSaid.isGone ?? ""), Theme.alert]]));
        const goneCount = count => count === 1 ? String(said.goneOne ?? "")
            : String(said.goneMany ?? "").replace("{count}", count);
        const forget = one => act("forget", { run: () => Sessions.forgetReopen(one) });
        const unreopened = one => thingRow({
            key: `agents:to-reopen:${one.id}`,
            name: String(one.title || one.address || one.id),
            side: String(oneSaid.gone ?? "").replace("{cwd}", homely(one.cwd)),
            alerts: true,
            note: fromOf(one),
            glyph: Theme.glyph.agent,
            hue: "alert"
        }, () => [forget(one)]);
        const closed = one => goneHere(one) ? unreopened(one) : thing({
            thing: true,
            key: `agents:to-reopen:${one.id}`,
            name: String(one.title || one.address || one.id),
            // One held elsewhere says where, as a session there does.
            side: one.machine !== undefined ? provider.elsewhere("on", one) : homely(one.cwd),
            note: fromOf(one),
            verb: Words.verb("verb:agents:to-reopen:one"),
            glyph: Theme.glyph.agent,
            hue: "program",
            later: true,
            run: () => reopen(one)
        });
        const reopenable = lost.filter(one => !goneHere(one));
        const goneNow = lost.length - reopenable.length;
        // Reopen Them All reopens those that can be, its side counting them
        // when any can't: "58 of 60 \u00b7 2 folders gone". Each found gone
        // after is counted with how many reopened, and said once the
        // launcher opens next.
        const outcome = { asked: reopenable.length, gone: 0 };
        const tellAll = () => provider.launcher?.tellOnOpen([
            [String(outcome.gone), String(said[outcome.gone === 1 ? "folderGone"
                : "foldersGone"] ?? ""), Theme.alert],
            [String(outcome.asked - outcome.gone), String(said.reopened ?? "")]]);
        // Its count of what failed is drawn in the alert's colour, as the
        // row that failed is, the rest as a side reads (VKCF3Z, 2026-11-07).
        const there = String(said.there ?? "").replace("{there}", reopenable.length)
            .replace("{count}", lost.length);
        const themAll = () => act("all", {
            later: true,
            side: goneNow === 0 ? "" : `${there} \u00b7 ${goneCount(goneNow)}`,
            sideItems: goneNow === 0 ? [] : [{ said: there, keep: 2 },
                { said: goneCount(goneNow), keep: 3, hue: "alert" }],
            // Each back where it sat (Sessions.qml's reopenThemAll), a
            // carried one beside where Joel is, since it never sat here,
            // and then the windows that reopen themselves. Each is told to
            // carry on as it opens, so none waits to be poked; one reopened
            // alone from its own row isn't, since Joel is there.
            run: () => Sessions.reopenThemAll(reopenable, () => {
                outcome.gone += 1;
                tellAll();
            })
        });
        return thingRow({
            key: "agents:to-reopen",
            name: Words.name("agents:to-reopen"),
            side: String(said[lost.length === 1 ? "one" : "many"] ?? "")
                .replace("{count}", lost.length),
            note: note,
            glyph: Words.glyph("agents:to-reopen"),
            hue: "action"
        }, () => (reopenable.length === 0 ? [] : [themAll()]).concat(lost.map(closed),
            booted === 0 ? [] : [act("notNow", {
            run: () => Sessions.putOffLastBoot()
        })]));
    }

    // Whether each folder there is to reopen in is there, looked for as the
    // launcher opens and as the sessions to reopen change while it's open,
    // so each row says it before Return rather than trusting what the
    // shell found at its start.
    function lookForFolders() {
        if (provider.launcher?.open)
            Sessions.lookForFolders(Sessions.toReopen.filter(one => one.machine === undefined)
                .map(one => one.cwd));
    }
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            provider.lookForFolders();
        }
    }

    // What sessions landed that Joel hasn't read (SessionRow.js's landed):
    // one row while any is, Landed, its count at its side, which Now shows
    // among what wants him (VKCF3Z's design, tier 2) and Agents after the
    // sessions. A step in, each landing, newest first: what it is as its
    // name and its ref at the right in the mono; its drawer says who landed
    // it and when, and what to try, a sentence the row has no room for
    // beside its ref. Return goes to the change, or to its session where no
    // link to the change is known, and marks it read, as opening a notice or
    // a mail does: a mark, never an ending, since no Return ends or takes
    // away. Its own step in holds Open the Change, Go To the Session, Copy
    // What to Try and Forget It, its ending, which d does and u takes back,
    // as a notice's is. Read only from the sessions there are, so one that
    // ended takes what it landed with it.
    readonly property var landed: {
        const sessions = Sessions.running.concat(Sessions.parked);
        const unread = SessionRow.landed(sessions, Sessions.landingsRead);
        // The last read while the launcher holds it, it stays, empty, so
        // the step in can be said again with every landing dimmed, as a
        // row gone from any list is, rather than lost from under it; it
        // goes once the launcher lets it go.
        if (unread.length === 0 && !holdsLanded)
            return null;
        const read = one => Sessions.readLandings([one.key],
            SessionRow.stillRead(sessions, Sessions.landingsRead));
        const act = (id, fields) => thing(Object.assign({
            key: `verb:agents:landed:${id}`,
            verb: Words.verb(`verb:agents:landed:${id}`),
            name: Words.name(`verb:agents:landed:${id}`),
            glyph: Words.glyph(`verb:agents:landed:${id}`),
            hue: "action"
        }, fields));
        const windowed = one => Sessions.running.includes(one.session)
            && provider.windowedOf(one.session);
        const toChange = one => () => Niri.spawn(["xdg-open", one.url]);
        const toSession = one => () => Sessions.goTo(one.session);
        // Where Return takes it: the change, else its session, else nowhere
        // but read.
        const goes = one => one.url !== "" ? "change" : windowed(one) ? "session" : "";
        const parts = one => [
            one.url !== "" ? act("change", { later: true, run: toChange(one) }) : null,
            windowed(one) ? act("session", { later: true, run: toSession(one) }) : null,
            one.person !== "" ? copyRow({
                key: "verb:agents:landed:try",
                name: Words.name("verb:agents:landed:try"),
                copy: one.person,
                figure: false,
                glyph: Words.glyph("verb:agents:landed:try")
            }) : null,
            act("forget", Object.assign({
                stays: true,
                run: () => read(one),
                undo: () => Sessions.unreadLandings([one.key])
            }, Words.ending("verb:agents:landed:forget")))
        ].filter(row => row !== null);
        const landing = one => thing({
            thing: true,
            key: `agents:landed:${one.key}`,
            name: one.what,
            side: one.ref,
            drawer: () => SessionRow.landingDrawer(one, {
                at: one.at !== "" ? started(one.at) : "",
                session: one.session.title ?? "",
                named: id => Words.name(`agents:${id}`)
            }),
            verb: Words.verb(`verb:agents:landed:${goes(one) || "read"}`),
            glyph: Words.glyph("agents:landed"),
            hue: "action",
            later: goes(one) !== "",
            // Read where it goes nowhere, which keeps the launcher, as a
            // setting's change does, so the next landing is under the cursor.
            stays: goes(one) === "",
            run: () => {
                if (goes(one) === "change")
                    toChange(one)();
                else if (goes(one) === "session")
                    toSession(one)();
                read(one);
            },
            verbs: () => parts(one)
        });
        return thingRow({
            key: "agents:landed",
            name: Words.name("agents:landed"),
            side: unread.length > 0 ? String(unread.length) : "",
            glyph: Words.glyph("agents:landed"),
            hue: "action"
        }, () => unread.map(landing));
    }

    // Whether the launcher holds Landed open, apart, so a step anywhere else
    // remakes Landed only when this answer flips, never on every key.
    readonly property bool holdsLanded: !!provider.launcher && provider.launcher.open
        && (provider.launcher.held ?? []).some(step => step.row.key === "agents:landed")

    things: [toReopen, landed].filter(row => row !== null).concat([start])

    function everything() {
        return all.concat([toReopen, landed].filter(row => row !== null), [start]);
    }

    // How every session stands, running and parked, read from Sessions
    // itself, so it holds while the launcher is shut, as Home's row is read.
    function states() {
        return Sessions.running.concat(Sessions.parked).map(session => stateOf(session));
    }

    // How they stand together, "1 needs you \u00b7 3 working \u00b7 $214
    // today" (SessionRow.js), the day's dollars as spend-watch sums every
    // session's, never added up here, and none of them while the screen is
    // shared.
    function summary() {
        const spent = Quiet.sharing ? "" : String(SpendWatch.said?.agentsToday ?? "");
        return SessionRow.summary(states(), spent, Words.of("agents").said ?? {});
    }

    // What it holds, at its row's right wherever it's found, and what
    // Home's row says in place of naming its sessions (VKCF3Z,
    // 2026-11-06). Found by its name, "agents", its row is the answer, as
    // Processor's is for "cpu", and Return still opens it, since that's
    // what Joel types its name for.
    function counted() {
        return summary();
    }
    // The found row and Home's are said again as it moves, since the day's
    // dollars and sharing can change it while no session's row changes,
    // and the launcher joins what a place says only when it says it
    // changed (Y2YBTS, 2026-11-07).
    readonly property string standing: summary()
    onStandingChanged: changed()
    function homeSays() {
        return summary();
    }
}

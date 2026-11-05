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
// listening and paused, this machine's first in each, then the others by
// when each last changed. Go To joins one elsewhere through its node; one
// on a machine out of reach stays, dimmed, saying so, and does nothing.
//
// Each session sits under the one orchestrating it, a step in, and two
// under that one's orchestrator, with no line or box: the step alone says
// it, and an orchestration stands where its most urgent row would
// (QEST-20261026-15J7N3's agents-tree-look.md). Only typing ranks, so
// while Joel types every row is at no depth, its orchestrator's address
// leading its note, as a row that arrives while the list is open is
// (Steady.js). An orchestrator says what its whole orchestration spent
// today, as spend-watch sums it by quest, and its drawer what of that was
// its own and its sessions'.
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
        const running = Sessions.running;
        // The orchestrators the harness lists, which say what their whole
        // orchestration spent even with none of theirs running.
        const listed = (source("orchestrators")?.listed ?? []).map(one => one.address);
        const laid = SessionRow.tree(running.map(session => ({
            address: session.address,
            under: session.presence?.orchestratedBy ?? "",
            state: stateOf(session)
        })));
        return laid.map(one => made(running[one.at], {
            depth: one.depth,
            orchestrates: one.orchestrates || listed.includes(running[one.at].address)
        }));
    }
    onAllChanged: changedTo(all)

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

    // A session elsewhere that couldn't be joined or told is said in a
    // notice, since nothing opens to show it.
    Connections {
        target: Sessions

        function onUnjoined(session) {
            Niri.spawn(["notify-send", "--app-name=Sessions", "--", session.title,
                provider.elsewhere(session.reachable ? "unheld" : "unreached", session)]);
        }

        function onUnsteered(session, why) {
            Niri.spawn(["notify-send", "--app-name=Sessions", "--", session.title,
                provider.elsewhere(why !== "" ? "untold" : "unanswered", session, why)]);
        }
    }

    // Where a session's first job waiting for its turn stands in line.
    function inLine(presence) {
        const waiting = presence && presence.session
            ? Jobs.of(presence.session).find(job => job.state === "waiting") : undefined;
        return waiting ? word("inLine", { place: Numbers.ordinal(waiting.place) }) : "";
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
        const spent = presence && presence.session && !Quiet.sharing
            ? SpendWatch.sessions[presence.session] ?? "" : "";
        // An orchestrator's whole orchestration's, as spend-watch sums its
        // quest and those under it, or nothing to say it by.
        const family = place?.orchestrates && quest !== "" && !Quiet.sharing
            ? SpendWatch.families[quest] ?? null : null;
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
        const figure = family ? word("inAll", { spent: family.all })
            : spent !== "" ? word("today", { spent: spent }) : "";
        // What it says at its right, each item with how much it's needed,
        // so where room runs short they give way whole (Choice.qml): its
        // tree first, then its address, then its state, and what it cost
        // stays (VKCF3Z, 2026-11-04).
        const items = [{ said: session.address, keep: 2 }, { said: doing, keep: 3 },
            { said: figure, keep: 4 }, { said: session.folder, keep: 0 }]
            .filter(item => item.said !== "");
        const row = member({
            thing: true,
            key: `agent:${session.address}`,
            kind: SessionRow.KIND,
            // The session is its window, which the windows and its quest
            // also find, and it says the most of the three, so it's the
            // one row left of them (Launcher.qml's compose).
            is: Sessions.isOf(session),
            rich: 2,
            verb: "go",
            name: session.title,
            // Its address, folder and quest's id, made ready to match, since
            // a member keeps its words as given.
            also: Fuzzy.prepare(`${session.address} ${session.folder} ${quest}`),
            glyph: Theme.glyph.agent,
            hue: "program",
            side: items.map(item => item.said).join(" \u00b7 "),
            sideItems: items,
            note: noted,
            depth: depth,
            gone: !session.reachable,
            later: true,
            run: () => Sessions.goTo(session),
            verbs: () => provider.verbs(session, quest, spent, family),
            // What y takes: the first of its facts, as its verbs list them.
            operands: () => [quest !== "" ? copyOperand("fact:agent:quest", quest)
                : copyOperand("fact:agent:address",
                    (session.presence?.addresses ?? [session.address]).join(", "))],
            drawer: () => provider.drawerOf(session, quest, spent, family)
        });
        // Itself as it reads ranked flat: at no depth, whom it's under
        // leading its note. It isn't named flat, which Quickshell's
        // JavaScript would read as the arrays' method it lacks.
        // Whom it's under leads what it says at its right too, given way
        // after its tree, and found from the top its words after its name
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
    function drawerOf(session, quest, spent, family) {
        const presence = session.presence ?? {};
        return SessionRow.drawer(session.title, presence, {
            spent: family ? word("allOf", { spent: family.all }) : spent,
            own: family ? family.own : "",
            under: family ? family.under : "",
            started: presence.owner?.startedAt ? started(presence.owner.startedAt) : "",
            address: session.address,
            machine: session.machine,
            named: id => Words.name(`agents:${id}`)
        });
    }

    // What the foot says once words have gone to a session.
    property string told: ""

    // A session's own acts, in the one order every session keeps
    // (SessionRow.js), each steering it through its inbox but Go To; an
    // orchestrator's row offers the same ones (Orchestrators.qml).
    function sessionActs(session) {
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
            // Pausing and resuming change how the session stands, so from
            // the find Return lands on them first (Catalogue.changesState).
            pause: () => act("pause", { changes: true, run: steered("pause") }),
            resume: () => act("resume", { changes: true, run: steered("resume") }),
            stop: () => act("stop", { run: steered("stop") })
        };
        const windowed = !!(session.window || session.kitty || (!session.here && session.attach));
        return SessionRow.acts(stateOf(session), windowed)
            .map(id => made[id]());
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
        for (const row of all) {
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
    // row, then a choice of a session on a quest or one as an
    // orchestrator, each listing what it can start, the quests' and the
    // orchestrators' own rows, whose Return starts it. Found by its name
    // from anywhere, and folded, so home names the sessions, never it.
    function source(name) {
        return (provider.launcher?.sources ?? []).find(one => one.settings.name === name) ?? null;
    }

    readonly property var start: {
        const quests = () => (source("quests")?.everything() ?? [])
            .filter(row => row.unfinished && row.verb !== "go");
        const orchestrators = () => source("orchestrators")?.startable ?? [];
        const choice = (id, note, rows) => thingRow({
            key: `agents:start:${id}`,
            name: Words.name(`agents:start:${id}`),
            note: note,
            glyph: Theme.glyph.agent,
            hue: "action"
        }, rows);
        return Object.assign(thingRow({
            key: "agents:start",
            name: Words.name("agents:start"),
            glyph: Theme.glyph.agent,
            hue: "action"
        }, () => [
            choice("quest", "a session working on one of your quests", quests),
            choice("orchestrator", "a session that runs a quest and the quests under it",
                orchestrators)
        ]), {
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
        // In its own folder only; when that's gone, a notice says so and
        // nothing opens, never one in the home folder instead.
        const gone = String((Words.of("verb:agents:to-reopen:one").said ?? {}).gone ?? "");
        const unopened = (one, cwd) => Sessions.sayUnopened(
            String(one.title || one.address || one.id), gone.replace("{cwd}", homely(cwd)));
        const reopen = one => Sessions.reopen(one, cwd => unopened(one, cwd));
        const closed = one => thing({
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
        return thingRow({
            key: "agents:to-reopen",
            name: Words.name("agents:to-reopen"),
            side: String(said[lost.length === 1 ? "one" : "many"] ?? "")
                .replace("{count}", lost.length),
            note: note,
            glyph: Words.glyph("agents:to-reopen"),
            hue: "action"
        }, () => [act("all", {
            later: true,
            // Each back where it sat (Sessions.qml's reopenThemAll), a
            // carried one beside where Joel is, since it never sat here,
            // and then the windows that reopen themselves. Each is told to
            // carry on as it opens, so none waits to be poked; one reopened
            // alone from its own row isn't, since Joel is there.
            run: () => Sessions.reopenThemAll(lost, unopened)
        })].concat(lost.map(closed), booted === 0 ? [] : [act("notNow", {
            run: () => Sessions.putOffLastBoot()
        })]));
    }

    things: toReopen ? [toReopen, start] : [start]

    function everything() {
        return all.concat(toReopen ? [toReopen] : [], [start]);
    }

    function counted() {
        // As the launcher's foot says it (pieces/Pi.qml).
        const wanting = Sessions.wanting.length;
        return [`${Sessions.running.length} running`,
            Sessions.working > 0 ? `${Sessions.working} working` : "",
            wanting > 0 ? `${wanting} ${wanting === 1 ? "wants" : "want"} you` : ""]
            .filter(part => part !== "").join(", ");
    }
}

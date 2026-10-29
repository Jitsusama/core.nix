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

    // Those that truly want Joel first, then the one used last.
    readonly property var all: Sessions.wanting
        .concat(Sessions.running.filter(session => !Sessions.wanting.includes(session)))
        .map(session => made(session))
    onAllChanged: changedTo(all)

    // Where a session's first job waiting for its turn stands in line.
    function inLine(presence) {
        const waiting = presence && presence.session
            ? Jobs.of(presence.session).find(job => job.state === "waiting") : undefined;
        return waiting ? `${Numbers.ordinal(waiting.place)} in line` : "";
    }

    function made(session) {
        const presence = session.presence;
        const queued = inLine(presence);
        // A want that isn't truly Joel's (Sessions.asks) reads as what the
        // session does meanwhile.
        const asks = Sessions.asks(session);
        const doing = presence === null ? ""
            : asks ? Sessions.wanted(presence.wants)
            : queued !== "" ? queued
            : presence.busy ? "working" : "waiting";
        const quest = presence && presence.owns
            ? presence.owns.find(owned => /^QEST-/.test(owned)) ?? "" : "";
        const spent = presence && presence.session && !Quiet.sharing
            ? SpendWatch.sessions[presence.session] ?? "" : "";
        return member({
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
            side: [session.address, doing, spent !== "" ? `${spent} today` : "", session.folder]
                .filter(part => part !== "").join(" \u00b7 "),
            note: asks && presence.wants.said ? presence.wants.said
                : presence && presence.worksOn ? presence.worksOn : "",
            later: true,
            run: () => Sessions.goTo(session),
            verbs: () => provider.verbs(session, quest, spent),
            drawer: () => provider.drawerOf(session, quest, spent)
        });
    }

    // What a session's drawer says (SessionRow.js), read as it's shown.
    function drawerOf(session, quest, spent) {
        const presence = session.presence ?? {};
        return SessionRow.drawer(session.title, presence, {
            spent: spent,
            started: presence.owner?.startedAt ? started(presence.owner.startedAt) : "",
            address: session.address,
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
            pause: () => act("pause", { run: steered("pause") }),
            resume: () => act("resume", { run: steered("resume") }),
            stop: () => act("stop", { run: steered("stop") })
        };
        const windowed = !!(session.window || session.kitty);
        return SessionRow.acts(SessionRow.stands(session.presence), windowed)
            .map(id => made[id]());
    }

    // When a session started, as a clock says it, with its day when that
    // wasn't today.
    function started(at) {
        const when = new Date(at);
        const today = new Date().toDateString() === when.toDateString();
        return Qt.formatDateTime(when, today ? "HH:mm" : "ddd d MMM, HH:mm");
    }

    function verbs(session, quest, spent) {
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
            spent !== "" ? fact("spent", named("spent"), spent) : null
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
        return { rows: kept.map(f => f.row), grade: grade };
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

    things: [start]

    function everything() {
        return all.concat([start]);
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

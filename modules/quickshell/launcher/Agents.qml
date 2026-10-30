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
// mycelium's plan): those wanting Joel first, then those working, then
// those listening, this machine's first in each, then the others by when
// each last changed. Go To joins one elsewhere through its node; one on a
// machine out of reach stays, dimmed, saying so, and does nothing.
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

    // Those that truly want Joel first, then those working, then those
    // listening, each in the order Sessions keeps: this machine's, the one
    // used last first, then the others, the one that changed last first.
    // Made only while the launcher is open or its catalogue is walked from
    // outside, since every session's row reads the machine queue and the
    // session's presence, which change every few seconds; Home's count
    // reads Sessions itself.
    readonly property var all: {
        const launcher = provider.launcher;
        if (!launcher || !(launcher.open || launcher.cataloguing))
            return [];
        const rank = session => Sessions.wanting.includes(session) ? 0
            : SessionRow.stands(session.presence) === "busy" ? 1 : 2;
        return Sessions.running.map((session, at) => ({ session: session, at: at }))
            .sort((a, b) => rank(a.session) - rank(b.session) || a.at - b.at)
            .map(one => made(one.session));
    }
    onAllChanged: changedTo(all)

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
        const note = asks && presence.wants.said ? presence.wants.said
            : presence && presence.worksOn ? presence.worksOn : "";
        // Elsewhere, its machine leads the note; out of reach, it's all
        // the note says, and the row does nothing.
        const where = session.here ? ""
            : elsewhere(session.reachable ? "on" : "away", session);
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
            note: !session.reachable ? where
                : [where, note].filter(part => part !== "").join(" \u00b7 "),
            gone: !session.reachable,
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
            pause: () => act("pause", { run: steered("pause") }),
            resume: () => act("resume", { run: steered("resume") }),
            stop: () => act("stop", { run: steered("stop") })
        };
        const windowed = !!(session.window || session.kitty || (!session.here && session.attach));
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

    // The sessions the last boot's end took away (Sessions.qml's
    // lastBoot), one row after the sessions running while any are left,
    // and gone on the boot they ran in or once put off (VKCF3Z's design,
    // [fce929da]). Now shows this same row among what else wants Joel
    // (WhatsOn.qml). Return steps in with Reopen Them All first, so
    // Return twice brings every one back, each in its own window and
    // folder; then each one, as a closed session, reopening just it; then
    // Not Now. Its count is in its note, never its name, which would
    // change as he reopens them.
    readonly property var lastBoot: {
        const lost = Sessions.lastBoot;
        if (lost.length === 0)
            return null;
        const act = (id, fields) => thing(Object.assign({
            key: `verb:agents:last-boot:${id}`,
            verb: Words.verb(`verb:agents:last-boot:${id}`),
            name: Words.name(`verb:agents:last-boot:${id}`),
            glyph: Words.glyph(`verb:agents:last-boot:${id}`),
            hue: "action"
        }, fields));
        const home = Sessions.home;
        const homely = folder => folder === home || folder.startsWith(`${home}/`)
            ? `~${folder.slice(home.length)}` : folder;
        // In its own folder only; when that's gone, a notice says so and
        // nothing opens, never one in the home folder instead.
        const gone = String((Words.of("verb:agents:last-boot:one").said ?? {}).gone ?? "");
        const unopened = (one, cwd) => Sessions.sayUnopened(
            String(one.title || one.address || one.id), gone.replace("{cwd}", homely(cwd)));
        const reopen = one => Sessions.reopen(one, cwd => unopened(one, cwd));
        const closed = one => thing({
            key: `agents:last-boot:${one.id}`,
            name: String(one.title || one.address || one.id),
            // One held elsewhere says where, as a session there does.
            side: one.machine !== undefined ? provider.elsewhere("on", one) : homely(one.cwd),
            verb: Words.verb("verb:agents:last-boot:one"),
            glyph: Theme.glyph.agent,
            hue: "program",
            later: true,
            run: () => reopen(one)
        });
        return thingRow({
            key: "agents:last-boot",
            name: Words.name("agents:last-boot"),
            side: ((Words.of("agents:last-boot").said ?? {})[lost.length === 1 ? "one" : "many"]
                ?? "").replace("{count}", lost.length),
            glyph: Words.glyph("agents:last-boot"),
            hue: "action"
        }, () => [act("all", {
            later: true,
            // Each back where it sat (Sessions.qml's reopenAll).
            run: () => Sessions.reopenAll(lost, unopened)
        })].concat(lost.map(closed), [act("notNow", {
            run: () => Sessions.putOffLastBoot()
        })]));
    }

    things: lastBoot ? [lastBoot, start] : [start]

    function everything() {
        return all.concat(lastBoot ? [lastBoot] : [], [start]);
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

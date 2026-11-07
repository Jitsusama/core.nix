// pi's runs, a collection: the fleets, councils, judges and other review
// rounds of the last day, and any still running, as the realm's mycelium
// node serves them (kind `run`, which the harness publishes as each
// changes, and lets go a day after it ends), told every change as it
// comes rather than read again as the launcher opens. Those running come
// first, then the newest. A review round is named for the change it read,
// a fleet for what it's about; each wears how it stands, and says its
// kind, how far along it is or what it cost (never while the screen is
// shared), how long it ran, and the session that started it, while that
// one runs. One the node still says runs whose session has gone wears
// failure's mark, since nothing is left to finish it.
// Return goes to that session's window, or copies the run's id once it's
// gone; the right arrow steps into its participants, each saying how it
// stands, its model and its cost, whose Return reads its answer in a
// terminal once it has one.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    // Runs, Agents' one word for them (Words.qml), a part of Agents: each
    // a running of subagents, a fleet, a council or a judge's round.
    what: Words.of("runs").what

    collection: Words.collection("runs")

    // The node's records of runs (mycelium's PROTOCOL.md, The Daemon
    // Interface), watched whole and then told each change as only what
    // differs. What was held stays held while the node isn't answering, as
    // while a switch restarts it.
    readonly property var runKind: runLink.item
    LazyLoader {
        id: runLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "run"
            changesDiffer: true
            answersWhole: true
        }
    }
    // Each run's record as the node serves it, by its key.
    property var records: {
        runKind?.revision;
        return Object.values(runKind?.held ?? {});
    }

    // The session that started each, by its own id, which a run's record
    // names as its `parent`: its address. Said as text first, which
    // changes only when they do. A parked one is still its session.
    readonly property string holding: {
        const by = {};
        for (const session of Sessions.running)
            by[session.presence.session ?? ""] = { address: session.address, live: true };
        for (const session of Sessions.parked ?? []) {
            const id = session.presence.session ?? "";
            if (!by[id])
                by[id] = { address: session.address, live: false };
        }
        delete by[""];
        return JSON.stringify(by);
    }
    readonly property var started: JSON.parse(holding)

    // The runs, each as the rows are made from it: its bare id, its kind,
    // what it's about, the session it's of, and its participants. The
    // node's record keeps its own `kind`, so a run's own is read from its
    // id, which the harness writes as fleet.<id> or review.<round>-<time>.
    readonly property var runs: records.map(record => {
        const id = String(record.id ?? "");
        const bare = id.replace(/^[a-z]+\./, "");
        const round = bare.replace(/-.*$/, "");
        return {
            id: bare,
            key: record.key ?? `run:${id}`,
            kind: id.startsWith("fleet.") ? "fleet" : kinds[round] ? round : "review",
            about: record.about ?? "",
            parent: record.parent ?? "",
            started: record.since ?? "",
            participants: (record.members ?? []).map(member => ({
                id: member.id ?? "",
                state: member.state ?? "",
                model: member.model ?? "",
                cost: Number(member["pi.cost.usd"] ?? 0),
                started: member.since ?? "",
                ended: member.ended ?? "",
                answer: member.answer ?? ""
            }))
        };
    })

    // Each kind by Agents' own word for it: a fleet or a council is several
    // subagents at once.
    readonly property var kinds: ({
        fleet: "fleet",
        council: "council",
        judge: "judge",
        critique: "critique",
        audit: "audit",
        stack: "stack"
    })

    function running(state) {
        return ["queued", "starting", "running", "idle"].includes(state);
    }

    function failed(state) {
        return ["failed", "errored", "timeout", "idle-timeout", "soft-deadline", "gone"]
            .includes(state);
    }

    // How a run or participant stands, as a glyph's meaning.
    function mark(states) {
        if (states.some(running))
            return "running";
        if (states.some(failed))
            return "failed";
        if (states.every(state => state === "complete"))
            return "done";
        return "cancelled";
    }

    // How long, from its start to its end or to now.
    function lasted(from, to) {
        const begun = Date.parse(from);
        if (isNaN(begun))
            return "";
        const ended = to ? Date.parse(to) : Date.now();
        // Said as every other length of time is: "1 h 5 min".
        const minutes = Math.max(0, Math.round((ended - begun) / 60000));
        const hours = Math.floor(minutes / 60);
        return minutes < 1 ? "under a minute"
            : minutes < 60 ? `${minutes} min`
            : minutes % 60 === 0 ? `${hours} h` : `${hours} h ${minutes % 60} min`;
    }

    function dollars(amount) {
        return amount >= 10 ? `$${amount.toFixed(0)}` : `$${amount.toFixed(2)}`;
    }

    function ending(run) {
        const ends = run.participants.map(p => p.ended).filter(at => at !== "").sort();
        return ends.length > 0 ? ends[ends.length - 1] : "";
    }

    function cost(participants) {
        return participants.reduce((sum, p) => sum + (p.cost ?? 0), 0);
    }

    // Each finished run's row as last made, by its id, with what it was
    // made from: its record, its session and whether the screen is shared.
    // A running one's time makes every reading's text differ, so each
    // opening made every run's row again, 279 on Joel's shell with only a
    // few running (2026-11-03); a finished one's row is now kept while
    // nothing it says changed. A plain object, so keeping one notifies
    // nothing.
    readonly property var kept: ({})

    // A run as its session leaves it: one the node says runs whose session
    // is gone has nothing left to finish it, so its participants still at
    // work are gone. Read where the rows are, so a session coming or going
    // makes them once.
    function settled(run, sessions) {
        if (sessions[run.parent] || !run.participants.some(p => running(p.state)))
            return run;
        return Object.assign({}, run, { participants: run.participants.map(p => (running(p.state)
            ? Object.assign({}, p, { state: "gone" }) : p)) });
    }

    // Running first, then the newest.
    readonly property var all: {
        const sorted = runs.map(run => settled(run, started)).sort((a, b) => {
            const live = run => Number(run.participants.some(p => running(p.state)));
            return live(b) - live(a) || String(b.started).localeCompare(String(a.started));
        });
        const sharing = Quiet.sharing;
        const sessions = started;
        const next = {};
        const rows = sorted.map(run => {
            if (run.participants.some(p => running(p.state)))
                return made(run);
            const address = sessions[run.parent]?.live ? sessions[run.parent].address : "";
            const from = [JSON.stringify(run), address, sharing].join("\n");
            const before = provider.kept[run.id];
            next[run.id] = before && before.from === from ? before : { from: from, row: made(run) };
            return next[run.id].row;
        });
        for (const id of Object.keys(provider.kept))
            delete provider.kept[id];
        Object.assign(provider.kept, next);
        return rows;
    }
    onAllChanged: changedTo(all)

    // A review round by the change it read, as "council on core.nix#380",
    // and a fleet by what it's about, its kind beside it.
    function named(run) {
        return run.kind === "fleet" ? run.about || run.id
            : run.about ? `${kinds[run.kind] ?? run.kind} on ${run.about}` : run.id;
    }

    // The session that started it, while that one runs.
    function sessionOf(run) {
        const session = started[run.parent];
        return session?.live ? session : undefined;
    }

    function made(run) {
        const session = sessionOf(run);
        const states = run.participants.map(p => p.state);
        const live = states.some(running);
        const done = states.filter(state => !running(state)).length;
        const spent = cost(run.participants);
        const row = member({
            thing: true,
            key: run.key,
            verb: session ? "go to" : "copy",
            // With no session to go to, it copies its id for want of one, as
            // a job's row does, so Return goes into its parts instead
            // (Launcher.qml's copiesOnly) and y copies.
            figure: session ? undefined : false,
            name: named(run),
            live: live,
            glyph: Theme.glyph[mark(states)],
            hue: "program",
            side: [run.kind !== "fleet" && run.about ? "" : (kinds[run.kind] ?? run.kind),
                live ? `${done} of ${states.length} done` : "",
                !live && spent > 0 && !Quiet.sharing ? dollars(spent) : "",
                lasted(run.started, live ? "" : ending(run)),
                session ? session.address : ""]
                .filter(part => part !== "").join(" \u00b7 "),
            note: run.kind !== "fleet" && run.about ? run.id : "",
            later: true,
            now: !session,
            run: () => provider.go(run),
            verbs: () => provider.verbs(run)
        });
        // Its id and kind, made ready to match when first searched, as its
        // name is.
        let also = null;
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => also ?? (also = Fuzzy.kept(`${run.id} ${run.kind}`))
        });
        return row;
    }

    function go(run) {
        const session = sessionOf(run);
        const ongoing = session
            ? Sessions.running.find(one => one.address === session.address) : undefined;
        if (ongoing)
            Sessions.goTo(ongoing);
        else
            Quickshell.clipboardText = run.id;
    }

    // A participant's answer, the last words of its result, read in a
    // terminal, in less by name, since an agent's PAGER is often cat, which
    // would close it at once.
    function showAnswer(run, participant) {
        Niri.spawn(["kitty", "--title", `${named(run)} \u00b7 ${participant.id}`, "sh", "-c",
            "jq -r '.finalAssistantText // \"\"' \"$1\" | less", "sh", participant.answer]);
    }

    function verbs(run) {
        const session = sessionOf(run);
        const rows = run.participants.map(p => thing({
            key: `participant:${run.id}:${p.id}`,
            name: p.id,
            glyph: Theme.glyph[mark([p.state])],
            hue: "program",
            side: [p.state === "complete" ? "" : p.state,
                p.model.replace(/^.*\//, ""),
                !running(p.state) && p.cost && !Quiet.sharing ? dollars(p.cost) : "",
                lasted(p.started, running(p.state) ? "" : p.ended)]
                .filter(part => part !== "").join(" \u00b7 "),
            verb: p.answer ? "read" : "close",
            later: true,
            run: () => {
                if (p.answer)
                    provider.showAnswer(run, p);
            }
        }));
        return (session ? [thing({
            key: "verb:run:switch",
            verb: Words.verb("verb:run:switch"),
            name: Words.name("verb:run:switch"),
            side: session.address,
            glyph: Words.glyph("verb:run:switch"),
            hue: "action",
            later: true,
            run: () => provider.go(run)
        })] : []).concat(rows);
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
        const kept = found.filter(f => f.grade === grade)
            .sort((a, b) => Number(b.row.live) - Number(a.row.live) || b.score - a.score);
        return { rows: kept.map(f => f.row), grade: grade };
    }

    function everything() {
        return all.slice();
    }

    function counted() {
        if (all.length === 0)
            return "empty";
        const live = all.filter(row => row.live).length;
        return [live > 0 ? `${live} running` : "", `${all.length} today`]
            .filter(part => part !== "").join(", ");
    }

}

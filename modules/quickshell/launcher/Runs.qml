// pi's runs, a collection: the fleets, councils, judges and other review
// rounds of the last day, and any still running, as pi-runs reads them
// (runs.rs), asked again each time the launcher opens. Those running come
// first, then the newest. Each is named for what it read, if a review
// round read a change, and otherwise by its id; it wears how it stands,
// and says its kind, how far along it is or what it cost (never while the
// screen is shared), how long it ran, and the session that started it,
// while that one runs.
// Return goes to that session's window, or copies the run's id once it's
// gone; the right arrow steps into its participants, each saying how it
// stands, its model and its cost, whose Return reads its answer in a
// terminal once it has one.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    // Subagents, Agents' one word for them (Words.qml), a part of Agents.
    what: Words.of("runs").what

    collection: Words.collection("runs")

    // As pi-runs wrote them, and the text they were read from, so the same
    // text read again changes nothing.
    property var runs: []
    property string written: ""

    // The session each pi process is, by its pid: its address and window.
    // Said as text first, which changes only when they do.
    readonly property string holding: {
        const by = {};
        for (const session of Sessions.running) {
            const pid = session.presence.owner ? session.presence.owner.pid : null;
            if (pid && !by[pid])
                by[pid] = { address: session.address };
        }
        return JSON.stringify(by);
    }
    readonly property var started: JSON.parse(holding)

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
        return state === "starting" || state === "running" || state === "idle";
    }

    function failed(state) {
        return state === "failed" || state === "timeout" || state === "idle-timeout" || state === "gone";
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

    // Running first, then the newest.
    readonly property var all: runs.slice().sort((a, b) => {
        const live = run => Number(run.participants.some(p => running(p.state)));
        return live(b) - live(a) || String(b.started).localeCompare(String(a.started));
    }).map(run => made(run))
    onAllChanged: changedTo(all)

    function named(run) {
        return run.about ? `${kinds[run.kind] ?? run.kind} on ${run.about}` : run.id;
    }

    function made(run) {
        const session = started[run.pid];
        const states = run.participants.map(p => p.state);
        const live = states.some(running);
        const done = states.filter(state => !running(state)).length;
        const spent = cost(run.participants);
        const row = member({
            thing: true,
            key: `run:${run.id}`,
            verb: session ? "go" : "copy",
            name: named(run),
            live: live,
            glyph: Theme.glyph[mark(states)],
            hue: "program",
            side: [run.about ? "" : (kinds[run.kind] ?? run.kind),
                live ? `${done} of ${states.length} done` : "",
                !live && spent > 0 && !Quiet.sharing ? dollars(spent) : "",
                lasted(run.started, live ? "" : ending(run)),
                session ? session.address : ""]
                .filter(part => part !== "").join(" \u00b7 "),
            note: run.about ? run.id : run.cwd,
            later: true,
            now: !session,
            run: () => provider.go(run),
            verbs: () => provider.verbs(run)
        });
        // Its id, kind and folder, made ready to match when first searched,
        // as its name is.
        let also = null;
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => also ?? (also = Fuzzy.kept(`${run.id} ${run.kind} ${run.cwd}`))
        });
        return row;
    }

    function go(run) {
        const session = started[run.pid];
        const ongoing = session
            ? Sessions.running.find(one => one.address === session.address) : undefined;
        if (ongoing)
            Sessions.goTo(ongoing);
        else
            Quickshell.clipboardText = run.id;
    }

    // A participant's answer, read in a terminal, in less by name, since
    // an agent's PAGER is often cat, which would close it at once.
    function showAnswer(run, participant) {
        Niri.spawn(["kitty", "--title", `${named(run)} \u00b7 ${participant.id}`, "sh", "-c",
            "pi-runs answer \"$1\" | less", "sh", participant.answer]);
    }

    function verbs(run) {
        const session = started[run.pid];
        const rows = run.participants.map(p => thing({
            key: `participant:${run.id}:${p.id}`,
            name: p.id,
            glyph: Theme.glyph[mark([p.state])],
            hue: "program",
            side: [running(p.state) ? (p.activity || p.state) : (p.state === "complete" ? "" : p.state),
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

    Process {
        id: reading
        command: ["pi-runs", Sessions.state]
        stdout: StdioCollector {
            id: collected

            onStreamFinished: {
                const text = collected.text;
                if (text === provider.written)
                    return;
                try {
                    const said = JSON.parse(text);
                    if (Array.isArray(said)) {
                        provider.written = text;
                        provider.runs = said;
                    }
                } catch (error) {
                    // Half a reading, or none: what was read before stands.
                }
            }
        }
    }

    function look() {
        reading.running = true;
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                provider.look();
        }
    }

    Component.onCompleted: look()
}

// Joel's timers, a collection in Desktop beside the clipboard, and the
// typed command that starts one: "timer 10m tea" answers "start tea, 10
// min", and Return starts it, the launcher staying while the answer turns
// into that timer counting down. The timers daemon keeps them and rings
// each as a notice itself (timers/src/main.rs), so a timer rings with the
// launcher closed and through a restart of the shell; HeldTimers.qml holds
// what it says, and Timing.js reads what's typed and words the time left.
//
// A timer's row says its time left as a clock reads it, counting each
// second while the launcher shows it, and how it stands below: what it
// counts and when it ends, paused, or when it rang. Return pauses a
// running one and resumes a paused one, the launcher staying, and sets a
// rung one going again, since that ends nothing; d ends it, as End It
// does inside, and u sets it going again with what it had left. Inside
// are Pause or Resume, Again, Add a Minute, End It and Copy the Time Left
// (Y2YBTS, 2026-11-07, for VKCF3Z).
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Timing.js" as Timing

Provider {
    id: provider

    what: "timer"

    collection: Words.collection("timers")

    examples: ["timer 10m tea"]

    // What a timer's rows say, in the vocabulary's words.
    readonly property var said: Words.of("timer").said ?? {}
    readonly property var units: ({
        hours: said.hours ?? "h",
        minutes: said.minutes ?? "min",
        seconds: said.seconds ?? "s"
    })
    readonly property string start: Words.command("command:timer").start ?? "timer "

    // Every timer's row as it reads now, made afresh each time, since
    // what each says counts on.
    function rows() {
        return HeldTimers.timers.map(record => provider.made(record));
    }

    // A clock's time of day, as the launcher says one: "20:42".
    function clockAt(at) {
        return Qt.formatTime(new Date(at), "HH:mm");
    }

    // A timer's name, its own or the vocabulary's for one with none.
    function named(record) {
        return record.name !== "" ? record.name : Words.name("timer");
    }

    // What it says beside its name, now.
    function side(record, now) {
        return record.state === "rung" ? said.done ?? "" : Timing.clock(Timing.left(record, now));
    }

    // What it says below, now: what it counts and when it ends, that it's
    // paused, or when it rang.
    function note(record, now) {
        const span = Timing.said(record.span, units);
        switch (record.state) {
        case "running":
            return (said.running ?? "").replace("{span}", span)
                .replace("{clock}", clockAt(record.ends));
        case "paused":
            return (said.paused ?? "").replace("{span}", span);
        default:
            return ((record.late ? said.late : said.rung) ?? "").replace("{clock}",
                clockAt(record.rang));
        }
    }

    // What Return does to it: pause one running, resume one paused, and
    // set one rung going again.
    function returned(record) {
        if (record.state === "running")
            return { verb: Words.verb("verb:timer:pause"), run: () => provider.ask("pause", record) };
        if (record.state === "paused")
            return { verb: Words.verb("verb:timer:pause", "paused"),
                run: () => provider.ask("resume", record) };
        return { verb: Words.verb("verb:timer:again"), run: () => provider.ask("again", record) };
    }

    // Asks the daemon to change it, saying in the foot why when it won't.
    function ask(act, record) {
        const then = answer => {
            if (answer?.error !== undefined)
                provider.refused(record.key, answer.error);
        };
        switch (act) {
        case "pause":
            HeldTimers.pause(record.key, then);
            break;
        case "resume":
            HeldTimers.resume(record.key, then);
            break;
        case "again":
            HeldTimers.again(record.key, then);
            break;
        case "add":
            HeldTimers.add(record.key, 60 * 1000, then);
            break;
        default:
            HeldTimers.end(record.key, then);
        }
    }

    // A timer's row.
    function made(record) {
        const now = Date.now();
        const back = returned(record);
        const row = member({
            thing: true,
            key: record.key,
            name: named(record),
            glyph: Words.glyph("timer"),
            hue: record.state === "rung" ? "alert" : "machine",
            side: side(record, now),
            note: note(record, now),
            at: record.set,
            record: record,
            verb: back.verb,
            stays: true,
            run: back.run
        });
        row.verbs = () => provider.acts(record);
        row.operands = () => provider.operands(record);
        return row;
    }

    // One of a timer's acts, by its id, as the vocabulary names it.
    function act(id, state, fields) {
        return thing(Object.assign({
            key: id,
            what: "",
            name: Words.name(id, state),
            verb: Words.verb(id, state),
            glyph: Words.glyph(id, state),
            hue: "action",
            stays: true
        }, Words.ending(id), fields));
    }

    // Inside a timer, in the one order acts take: what Return does first,
    // then what changes it, then what ends it, then Copy.
    function acts(record) {
        const rows = [];
        if (record.state !== "rung")
            rows.push(act("verb:timer:pause", record.state === "paused" ? "paused" : "",
                { run: () => provider.ask(record.state === "paused" ? "resume" : "pause", record) }));
        rows.push(act("verb:timer:again", "", { run: () => provider.ask("again", record) }));
        rows.push(act("verb:timer:add", "", { run: () => provider.ask("add", record) }));
        rows.push(act("verb:timer:end", "", {
            stays: false,
            run: () => provider.ask("end", record)
        }));
        rows.push(copyRow({
            key: "verb:timer:copy",
            name: Words.name("verb:timer:copy"),
            glyph: Words.glyph("verb:timer:copy"),
            figure: false,
            hue: "action",
            copy: provider.side(record, Date.now())
        }));
        return rows;
    }

    // What d and y take of a timer, as plain fields: End It, which u takes
    // back by setting it going again with what it had left, and Copy the
    // Time Left.
    function operands(record) {
        const left = Timing.left(record, Date.now());
        const end = Object.assign({ key: "verb:timer:end", verb: Words.verb("verb:timer:end") },
            Words.ending("verb:timer:end"), {
            run: () => provider.ask("end", record),
            undo: record.state === "rung" ? null : () => HeldTimers.start(left, record.name,
                started => {
                    if (record.state === "paused" && started?.key)
                        HeldTimers.pause(started.key);
                })
        });
        return [end, copyOperand("verb:timer:copy", provider.side(record, Date.now()))];
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const found = [];
        for (const row of rows()) {
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        return {
            rows: found.filter(f => f.grade === finest).map(f => f.row),
            grade: finest
        };
    }

    function everything() {
        return rows();
    }

    function counted() {
        const held = HeldTimers.timers.length;
        return held === 0 ? "empty" : String(held);
    }

    // What's typed after "timer", read, or null for anything else.
    function asked(typed) {
        const text = String(typed ?? "");
        if (!text.toLowerCase().startsWith(start))
            return null;
        return Timing.read(text.slice(start.length));
    }

    // The timer started from what's typed, by what was typed, so while it
    // stays typed the answer is that timer as it stands rather than
    // another start.
    property var started: ({ typed: "", key: "" })

    function answer(typed) {
        const asking = asked(typed);
        if (asking === null)
            return null;
        const kept = started.typed === typed ? HeldTimers.timer(started.key) : null;
        if (kept !== null)
            return counting(kept);
        if (asking.unfit)
            return { headline: said.unfit ?? "", copy: "", rows: [], missed: true };
        const span = Timing.said(asking.span, units);
        return {
            headline: (asking.name !== "" ? said.start ?? "" : said.startUnnamed ?? "")
                .replace("{name}", asking.name).replace("{span}", span),
            verb: Words.command("command:timer").verb,
            stays: true,
            copy: "",
            rows: [],
            note: said.from ?? "",
            run: () => HeldTimers.start(asking.span, asking.name, record => {
                if (record === null || record.error !== undefined) {
                    provider.refused("command:timer", record?.error ?? said.away ?? "");
                    return;
                }
                provider.started = { typed: typed, key: record.key };
                provider.answered();
            })
        };
    }

    // The answer once its timer is going: the timer as its row says it,
    // counting, Return doing what the row's Return does.
    function counting(record) {
        const row = made(record);
        return {
            headline: `${row.name} \u00b7 ${row.side}`,
            verb: row.verb,
            stays: true,
            copy: "",
            rows: [],
            note: row.note,
            run: row.run,
            live: provider
        };
    }

    // A change the daemon says is shown wherever its rows are.
    Connections {
        target: HeldTimers

        function onHeard() {
            provider.changedTo(provider.rows());
            provider.answered();
        }
    }

    // While the launcher is open over a timer counting, its rows are made
    // again each second, so what they say counts down; never while it's
    // closed, nor while every timer is paused or rung.
    readonly property bool counts: launcher !== null && launcher.open
        && HeldTimers.timers.some(record => record.state === "running")

    Timer {
        interval: 1000
        repeat: true
        running: provider.counts
        onTriggered: {
            provider.changedTo(provider.rows());
            if (provider.launcher?.answer?.live === provider)
                provider.answered();
        }
    }
}

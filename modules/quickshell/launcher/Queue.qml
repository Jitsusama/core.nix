// The machine's queue, a collection in Agents beside the sessions whose
// work it holds: each heavy job running or waiting its turn, as pi's
// harness writes them (Jobs.qml), running first and then in line. Its row
// at Agents says how many run and how many wait, as the queue's headline
// always has.
//
// A job is named for what it is, in words rather than a check's
// identifier ("Zathura Opens a PDF" for zathura-opens-a-pdf), or for its
// kind when the queue can't tell; it wears the stage it's at, active
// running and queued waiting, since the running glyph is an arrow that
// would read as a step in. Beside it, in a column whose times tick once a
// minute: how long a running one has gone against its last run ("21 of
// 25 min", "4 min over", or "for 6 min" the first time), or what a waiting
// one waits for. Below it, while it's picked: its quest's title, its kind,
// the memory it holds or needs, and whether it's quick, capped or put
// first (Joel, 2026-10-26: "we should have more detail on each one if they
// are going to be useful and not just noise").
//
// Return goes to the window of the session it's for, or copies its name
// when none is open; the right arrow steps into its quest, its session,
// the command it runs and the rest of what the queue knows of it. Nothing
// here is read for a key: the rows are made as the queue changes, and the
// times and memory are read once a minute while the launcher's open.
pragma ComponentBehavior: Bound
import qs
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Numbers.js" as Numbers

Provider {
    id: provider

    what: Words.of("jobs").what

    collection: Words.collection("jobs")

    // Where the slices' memory is read, a stand-in's own in a test.
    property string cgroups: "/sys/fs/cgroup"

    // The minute the times are said at, and the memory each running job's
    // slice held then, by slice, in GiB.
    property real minute: Date.now()
    property var held: ({})

    // Words a name is said with but for its first and last, as the
    // glossary's titles are (docs/glossary.md), and those a name spells in
    // capitals, or as the glossary spells them.
    readonly property var small: ["a", "an", "the", "and", "or", "but", "of", "to", "in", "on",
        "at", "by", "for", "with"]
    readonly property var spelled: ({
        ai: "AI",
        dns: "DNS",
        gpu: "GPU",
        gtk: "GTK",
        ipc: "IPC",
        pdf: "PDF",
        qml: "QML",
        ssh: "SSH",
        usb: "USB",
        vm: "VM",
        vms: "VMs",
        vpn: "VPN",
        wifi: "Wi-Fi"
    })
    readonly property var kindIds: ({
        "vm-test": "jobs:vm-test",
        "vm-check": "jobs:vm-check",
        build: "jobs:build",
        benchmark: "jobs:benchmark"
    })

    // What the queue's rows say, as the vocabulary words it, filled in.
    function said(key, fill) {
        let words = (Words.of("jobs").said ?? {})[key] ?? key;
        for (const name of Object.keys(fill ?? {}))
            words = words.replace(`{${name}}`, fill[name]);
        return words;
    }

    // A name in words: an identifier's dashes are spaces, a VM test's own
    // "vm" is said by its glyph, and a whole system's checks are Checks.
    function named(job) {
        const raw = String(job.name ?? "");
        if (raw === "")
            return Words.name(kindIds[job.kind] ?? "jobs:build");
        if (raw === "x86_64-linux" || raw === "checks")
            return Words.name("jobs:checks");
        const words = raw.replace(/^vm-/, "").split(/[-_\s]+/).filter(word => word !== "");
        return words.map((word, at) => {
            const lower = word.toLowerCase();
            if (spelled[lower])
                return spelled[lower];
            if (at > 0 && at < words.length - 1 && small.includes(lower))
                return lower;
            return lower.charAt(0).toUpperCase() + lower.slice(1);
        }).join(" ");
    }

    // Its name and how many more it runs with, when the queue says it runs
    // a whole list: "a-theme-changes-everything and 57 more".
    function more(job) {
        const whole = /^(.*) and (\d+) more$/.exec(String(job.name ?? ""));
        return whole ? { job: Object.assign({}, job, { name: whole[1] }), more: Number(whole[2]) }
            : { job: job, more: 0 };
    }

    function minutes(seconds) {
        return Math.max(1, Math.round(seconds / 60));
    }

    // How long it's run against the last time, or what it waits for.
    function side(job) {
        if (job.state === "waiting") {
            const why = job.waitingOn ?? "order";
            return why === "order" ? said("inLine", { place: Numbers.ordinal(job.place) })
                : said(why);
        }
        const ran = minutes((minute - Date.parse(job.since)) / 1000);
        if (job.lastRanS === undefined)
            return said("ranFor", { ran: ran });
        const last = minutes(job.lastRanS);
        return ran <= last ? said("ranOf", { ran: ran, last: last })
            : said("over", { over: ran - last });
    }

    function gib(value) {
        return Number(value).toFixed(1).replace(/\.0$/, "");
    }

    // What it's for, its kind, what it holds or needs, and how it's marked.
    function note(job, more, quest) {
        const kind = said(job.kind ?? "build");
        const memory = job.state === "running" && held[job.slice] !== undefined
            ? said("holds", { gib: gib(held[job.slice]) })
            : job.memoryGib !== undefined ? said("needs", { gib: gib(job.memoryGib) }) : "";
        return [quest, more > 0 ? said("more", { more: more }) : "", kind, memory,
            job.first ? said("first") : "", job.quick ? said("quick") : "",
            job.capped ? said("capped") : ""]
            .filter(part => part !== "").join(" \u00b7 ");
    }

    // The session a job is for, while it runs.
    function sessionOf(job) {
        return Sessions.running.find(session => session.presence.session === job.owner?.session)
            ?? null;
    }

    // The quest's own row in Quests, so its Return opens it as there.
    function questRow(id) {
        const quests = provider.launcher?.sources?.find(one => one.settings.name === "quests");
        return quests ? (quests.all ?? []).find(row => row.key === `quest:${id}`) ?? null : null;
    }

    function goes(session) {
        return () => Sessions.goTo(session);
    }

    function made(given) {
        const both = more(given);
        const job = both.job;
        const session = sessionOf(job);
        const quest = job.quest ? questRow(job.quest) : null;
        const fields = {
            thing: true,
            key: `job:${job.id}`,
            name: named(job),
            side: side(job),
            note: note(job, both.more, quest ? quest.name : job.quest ?? ""),
            glyph: Theme.glyph[job.state === "waiting" ? "queued" : "active"],
            hue: job.state === "waiting" ? "muted" : "action",
            verbs: () => provider.parts(job, session, quest)
        };
        // A row that copies still wears how its job stands.
        if (session === null)
            return copyRow(Object.assign(fields, { copy: fields.name, figure: false }));
        return member(Object.assign(fields, {
            verb: Words.verb("verb:job:go"),
            later: true,
            run: provider.goes(session)
        }));
    }

    // What the queue knows of a job, each a step in: its quest and session
    // go there, and the rest copies what it says.
    function parts(job, session, quest) {
        const out = [];
        if (quest)
            out.push(thing({
                key: `job:${job.id}:quest`,
                name: Words.name("jobs:quest"),
                side: quest.name,
                verb: quest.verb,
                later: true,
                run: quest.run
            }));
        else if (job.quest)
            out.push(copyRow({ key: `job:${job.id}:quest`, name: Words.name("jobs:quest"),
                side: job.quest, copy: job.quest }));
        if (session)
            out.push(thing({
                key: `job:${job.id}:session`,
                name: Words.name("jobs:session"),
                side: session.address ?? "",
                verb: Words.verb("verb:job:go"),
                later: true,
                run: provider.goes(session)
            }));
        const facts = [
            ["command", job.what ?? ""],
            ["class", job.class ?? ""],
            ["needs", job.memoryGib !== undefined ? `${gib(job.memoryGib)} GiB` : ""],
            ["held", job.state === "running" && held[job.slice] !== undefined
                ? `${gib(held[job.slice])} GiB` : ""],
            ["since", job.since ? Qt.formatDateTime(new Date(job.since), "HH:mm") : ""],
            ["last", job.lastRanS !== undefined ? `${minutes(job.lastRanS)} min` : ""]
        ];
        for (const fact of facts.filter(one => one[1] !== ""))
            out.push(copyRow({
                key: `job:${job.id}:${fact[0]}`,
                name: Words.name(`jobs:${fact[0]}`),
                side: fact[1],
                copy: fact[1]
            }));
        return out;
    }

    // Running first, then in line.
    readonly property var all: provider.launcher?.open ?? true
        ? Jobs.running.concat(Jobs.waiting.slice().sort((a, b) => (a.place ?? 0) - (b.place ?? 0)))
            .map(job => made(job))
        : []
    onAllChanged: changedTo(all)

    function everything() {
        return all;
    }

    // Its row at Agents: how many run and how many wait.
    function counted() {
        if (Jobs.said === null)
            return said("silent");
        const counts = [Jobs.running.length > 0 ? said("running", { count: Jobs.running.length }) : "",
            Jobs.waiting.length > 0 ? said("waiting", { count: Jobs.waiting.length }) : ""]
            .filter(part => part !== "");
        return counts.length > 0 ? counts.join(", ") : said("none");
    }

    // Below its row: how much room the machine has for more, how held up
    // its processors are, and the minutes work waited today while there
    // was room for it.
    function noted() {
        if (Jobs.said === null)
            return "";
        const machine = Jobs.said.machine ?? {};
        const wasted = Jobs.said.wasted?.today ?? 0;
        return [machine.availableGib !== undefined ? said("spare", { gib: gib(machine.availableGib) }) : "",
            machine.pressure?.cpu !== undefined
                ? said("heldUp", { percent: Math.round(machine.pressure.cpu) }) : "",
            wasted > 0 ? said("wasted", { minutes: wasted }) : ""]
            .filter(part => part !== "").join(" \u00b7 ");
    }

    // Its name, and its quest's title, as a search reads them.
    function readyTexts(row) {
        return [row.prepared, Fuzzy.kept(row.note)];
    }

    // Found by its name, or by its quest's title, best first.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const found = [];
        for (const row of all) {
            const byName = Fuzzy.match(row.prepared, words, false);
            const byNote = Fuzzy.match(Fuzzy.kept(row.note), words, false);
            const best = byName.score >= byNote.score ? byName : byNote;
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row });
        }
        found.sort((a, b) => b.score - a.score);
        return { rows: found.map(f => f.row), grade: 0 };
    }

    // The memory each running job's slice holds now.
    function readHeld() {
        const out = {};
        for (const job of Jobs.running.filter(one => one.slice)) {
            memory.path = `${provider.cgroups}${job.slice}/memory.current`;
            const bytes = Number(read(memory).trim());
            if (bytes > 0)
                out[job.slice] = bytes / 1073741824;
        }
        return out;
    }

    // The minute turns over: the times are said again, and the memory read.
    function tick() {
        held = readHeld();
        minute = Date.now();
    }

    FileView {
        id: memory
        blockLoading: true
        printErrors: false
    }

    // Once a minute while the launcher's open, and as it opens.
    Timer {
        interval: 60000
        repeat: true
        triggeredOnStart: true
        running: provider.launcher?.open ?? false
        onTriggered: provider.tick()
    }
}

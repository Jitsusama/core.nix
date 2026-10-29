// pi's quests, a collection: every quest in the store, as pi-quests reads
// them (quests.rs), asked again each time the launcher opens, since pi
// changes them as it works and reading them all takes milliseconds. The
// open ones are all of it with nothing typed, the one driving first, as
// the quest workflow ranks them; a concluded or retired one comes as
// typing finds it, after the open ones that match as well. Each wears its
// status's glyph, and says its kind unless it's a quest, its priority while
// it's open or when it was last touched once it's not, and the session
// that has it loaded, if one does. The selected one reads its summary's
// first paragraph below the rows, for what it's about.
// Return goes to that session's window, or opens pi on it in a new
// terminal, autoloaded as the harness's spawn does, in the folder the
// harness would pick (quests.rs); the right arrow steps into what's known
// of it, each fact a row Return copies.
import qs
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    // Quests, Agents' one word for them (Words.qml), a part of Agents.
    what: Words.of("quests").what

    collection: Words.collection("quests")

    // As pi-quests wrote them, or none before it first answers, and the
    // text they were read from, so the same text read again changes nothing.
    property var quests: []
    property string written: ""

    // The status glyph each status wears.
    readonly property var marks: ({
        active: "active",
        blocked: "pending",
        paused: "stopped",
        concluded: "done",
        retired: "cancelled"
    })
    readonly property var priorities: ["driving", "active", "queued", "bench", "someday"]

    function isOpen(quest) {
        return quest.status !== "concluded" && quest.status !== "retired";
    }

    // The session that has each quest loaded, the one used last: its
    // address and what it is (Sessions.isOf). Said as text first, which
    // changes only when they do, since the sessions are listed again
    // whenever a window does.
    readonly property string holding: {
        const by = {};
        for (const session of Sessions.running) {
            const owns = session.presence.owns ?? [];
            const quest = owns.find(owned => /^QEST-/.test(owned));
            if (quest && !by[quest])
                by[quest] = { address: session.address, is: Sessions.isOf(session) };
        }
        return JSON.stringify(by);
    }
    readonly property var loaded: JSON.parse(holding)

    // Open first, by priority and then rank; the rest, last touched first.
    readonly property var all: quests.slice().sort((a, b) => {
        const open = Number(isOpen(b)) - Number(isOpen(a));
        if (open !== 0)
            return open;
        if (isOpen(a)) {
            const bucket = q => {
                const at = priorities.indexOf(q.priority);
                return at < 0 ? priorities.length : at;
            };
            return bucket(a) - bucket(b) || (Number(a.rank) || 1e9) - (Number(b.rank) || 1e9);
        }
        return String(b.updated).localeCompare(String(a.updated));
    }).map(quest => made(quest))
    onAllChanged: changedTo(all)

    // A day as a clock's calendar says it, with its year when it isn't this one.
    function day(updated) {
        const when = new Date(`${updated}T12:00:00`);
        if (isNaN(when))
            return "";
        const thisYear = when.getFullYear() === new Date().getFullYear();
        return Qt.formatDate(when, thisYear ? "d MMM" : "d MMM yyyy");
    }

    function go(quest) {
        const session = loaded[quest.id];
        const ongoing = session
            ? Sessions.running.find(one => one.address === session.address) : undefined;
        if (ongoing)
            Sessions.goTo(ongoing);
        else
            Sessions.start(quest.id, quest.folder);
    }

    function made(quest) {
        const session = loaded[quest.id];
        const row = member({
            key: `quest:${quest.id}`,
            // A quest a session has loaded is that session's window, which
            // its Agents row says more of (Launcher.qml's compose).
            is: session ? session.is : `quest:${quest.id}`,
            rich: 1,
            verb: session ? "switch" : "open",
            name: quest.title || quest.id,
            unfinished: isOpen(quest),
            glyph: Theme.glyph[marks[quest.status] ?? "pending"],
            hue: "program",
            side: [quest.kind !== "quest" ? quest.kind : "",
                isOpen(quest) ? quest.priority : day(quest.updated),
                session ? session.address : ""]
                .filter(part => part !== "").join(" \u00b7 "),
            note: quest.id,
            reads: quest.summary ?? "",
            later: true,
            run: () => provider.go(quest),
            verbs: () => provider.verbs(quest)
        });
        // Its id and kind, made ready to match when first searched, as its
        // name is, and kept while they read the same (Fuzzy.kept), since
        // every quest's row is made again whenever one quest is written.
        let also = null;
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => also ?? (also = Fuzzy.kept(`${quest.id} ${quest.kind}`))
        });
        return row;
    }

    function verbs(quest) {
        const session = loaded[quest.id];
        // Each fact a part, named first (quests:id and the rest), saying its
        // value and copying it, or what `copy` gives.
        const fact = (key, side, note, copy) => copyRow({
            key: `fact:quest:${key}`,
            name: Words.name(`quests:${key}`),
            side: side,
            note: note ?? "",
            copy: copy ?? side,
            glyph: Theme.glyph.agent
        });
        // A folder at home reads from ~, as a shell says it; the copy is whole.
        const home = Sessions.home;
        const homely = quest.folder === home || quest.folder.startsWith(`${home}/`)
            ? `~${quest.folder.slice(home.length)}` : quest.folder;
        const parent = quest.parent ? quests.find(other => other.id === quest.parent) : null;
        return [thing({
            key: session ? "verb:quest:switch" : "verb:quest:open",
            verb: Words.verb(session ? "verb:quest:switch" : "verb:quest:open"),
            name: Words.name(session ? "verb:quest:switch" : "verb:quest:open"),
            glyph: Words.glyph(session ? "verb:quest:switch" : "verb:quest:open"),
            hue: "action",
            later: true,
            run: () => provider.go(quest)
        })].concat([
            fact("id", quest.id),
            fact("status", [quest.status, isOpen(quest) ? quest.priority : ""]
                .filter(part => part !== "").join(" \u00b7 ")),
            quest.updated ? fact("updated", day(quest.updated)) : null,
            parent ? fact("parent", parent.title || parent.id, parent.id) : null,
            fact("folder", homely, "where a session starts", quest.folder)
        ].filter(row => row !== null));
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
            .sort((a, b) => Number(b.row.unfinished) - Number(a.row.unfinished) || b.score - a.score);
        return { rows: kept.map(f => f.row), grade: grade };
    }

    function everything() {
        return all.filter(row => row.unfinished);
    }

    function counted() {
        const open = everything();
        if (open.length === 0)
            return "empty";
        const driving = quests.filter(quest => isOpen(quest) && quest.priority === "driving").length;
        return [`${open.length} open`, driving > 0 ? `${driving} driving` : ""]
            .filter(part => part !== "").join(", ");
    }

    Process {
        id: reading
        command: ["pi-quests", Sessions.quests]
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
                        provider.quests = said;
                    }
                } catch (error) {
                    // Half a file, or none: what was read before stands.
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

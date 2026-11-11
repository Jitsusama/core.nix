// pi's quests, a collection: every quest in the store, as the realm's
// mycelium node serves the two lists the harness keeps of them, the open
// ones and the ended ones, each heard as the harness writes it. The
// open ones are all of it with nothing typed, the one driving first, as
// the quest workflow ranks them; a concluded or retired one comes as
// typing finds it, after the open ones that match as well. Each wears its
// status's glyph, and says its kind unless it's a quest, its priority while
// it's open or when it was last touched once it's not, and the session
// that has it loaded, if one does. The selected one reads its summary's
// first paragraph below the rows, for what it's about.
// Return goes to that session's window, or opens pi on it in a new
// terminal, autoloaded as the harness's spawn does, in the folder the
// harness picks and its lists say; the right arrow steps into what's known
// of it, each fact a row Return copies. Start a Session there lists the
// machines the node places on, each saying how it stands as Machines says
// it, from the same records.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../PiSource.js" as Pi
import "../When.js" as When
import "Order.js" as Order
import "RealmMachines.js" as Listed

Provider {
    id: provider

    // Quests, Agents' one word for them (Words.qml), a part of Agents.
    what: Words.of("quests").what

    collection: Words.collection("quests")

    // The open quests and the ended ones, as the node serves each list, or
    // none while it hasn't answered.
    readonly property var openKind: openLink.item
    readonly property var endedKind: endedLink.item
    LazyLoader {
        id: openLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "quest"
            changesDiffer: true
            answersWhole: true
        }
    }
    LazyLoader {
        id: endedLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "ended-quest"
            changesDiffer: true
            answersWhole: true
        }
    }

    // The realm's machines, as Machines reads them, so a machine Start a
    // Session offers says how it stands in the same words.
    readonly property var machineKind: machineLink.item
    LazyLoader {
        id: machineLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "machine"
            changesDiffer: true
            answersWhole: true
        }
    }

    // A machine's record by its name, or this machine's, said as here, from
    // a node too old to list it.
    function machineOf(name) {
        void machineKind?.revision;
        const record = Object.values(machineKind?.held ?? {}).find(one => one.name === name);
        return record ?? (name === Sessions.machine ? { here: true } : null);
    }

    // What RealmMachines.js needs to say a machine as Start a Session lists
    // it: Machines' words, now, and a moment as When says it, as Machines
    // says one out of reach since.
    function startPlace() {
        const said = Words.of("machines").said ?? {};
        return {
            said: said,
            now: Date.now(),
            day: (at, now) => When.day(at, now, {
                yesterday: said.yesterday ?? "",
                date: (date, format) => Qt.formatDate(date, format)
            }),
            clock: at => Qt.formatDateTime(new Date(at), "HH:mm")
        };
    }

    // Every quest, open first, each as its record says it (questOf). A
    // quest in both lists, as it ends or reopens, is the open one until
    // both are written. Said anew only when a quest heard reads otherwise
    // than it did, so a list the harness writes again, which tells every
    // quest anew with only when it was heard moved, changes nothing here.
    property var quests: []

    // What each list's records made, by the record's key: the record and
    // its quest. Each change remakes only the keys it names (Daemon.qml's
    // heard), where every quest of both lists, 1,063 on Joel's shell, was
    // copied field by field at each list written, the launcher shut too
    // (2026-11-10). Plain objects, so keeping one notifies nothing.
    readonly property var openMade: ({})
    readonly property var endedMade: ({})

    // A quest as its record says it, without what the node adds: its key,
    // when it was heard, which moves for every quest each time a list is
    // written, and the node's own kind, which stands over the quest's, so
    // the harness says that again as quest.kind.
    function questOf(record) {
        const quest = {};
        for (const field of Object.keys(record)) {
            if (!["key", "heard", "kind", "quest.kind", "start"].includes(field))
                quest[field] = record[field];
        }
        quest.kind = record["quest.kind"] ?? "quest";
        quest.folder = record.start ?? Sessions.home;
        return quest;
    }

    // Takes what a list said of these keys into what it made, a key whose
    // record reads the same but for when it was heard keeping its quest,
    // and lists the quests again if any changed.
    function took(list, made, changed, gone) {
        let moved = false;
        for (const key of changed) {
            const record = list.held[key];
            const before = made[key];
            if (!record || before && Pi.fieldsAlike(before.record, record, ["heard"]))
                continue;
            made[key] = { record: record, quest: questOf(record) };
            moved = true;
        }
        for (const key of gone) {
            if (key in made) {
                delete made[key];
                moved = true;
            }
        }
        if (moved)
            listed();
    }

    function listed() {
        const seen = new Set();
        const all = [];
        for (const made of [openMade, endedMade]) {
            for (const key in made) {
                const quest = made[key].quest;
                if (seen.has(quest.id))
                    continue;
                seen.add(quest.id);
                all.push(quest);
            }
        }
        quests = all;
    }

    // A list's link made again, as the node's socket moves, starts it
    // afresh from what the new one holds.
    function linked(list, made) {
        for (const key of Object.keys(made))
            delete made[key];
        if (list)
            took(list, made, Object.keys(list.held), []);
        listed();
    }
    onOpenKindChanged: linked(openKind, openMade)
    onEndedKindChanged: linked(endedKind, endedMade)
    Connections {
        target: provider.openKind
        function onHeard(changed, gone) {
            provider.took(provider.openKind, provider.openMade, changed, gone);
        }
    }
    Connections {
        target: provider.endedKind
        function onHeard(changed, gone) {
            provider.took(provider.endedKind, provider.endedMade, changed, gone);
        }
    }

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
    // address and what it is (Sessions.isOf), a running one before one
    // parked. Said as text first, which changes only when they do, since
    // the sessions are listed again whenever a window does.
    readonly property string holding: {
        const by = {};
        for (const session of Sessions.running.concat(Sessions.parked)) {
            const owns = session.presence.owns ?? [];
            const quest = owns.find(owned => /^QEST-/.test(owned));
            if (quest && !by[quest])
                by[quest] = {
                    address: session.address,
                    is: Sessions.isOf(session),
                    parked: Sessions.parked.includes(session)
                };
        }
        return JSON.stringify(by);
    }
    readonly property var loaded: JSON.parse(holding)

    // Each quest's row as last made, by its id, with what it was made from:
    // its record and the session that has it loaded. The quests are all
    // told again each time the harness writes a list of them, and every
    // quest's row was once made again at each change, about 824 on Joel's
    // shell (2026-11-03); a row is now kept while nothing it says changed.
    // A plain object, so keeping one notifies nothing. What it was made
    // from is compared in place rather than as text, since the text of
    // Joel's 1,063 quests, about a million characters, started a garbage
    // collection at every list the harness wrote (2026-11-10).
    readonly property var kept: ({})

    // Open first, by priority and then rank; the rest, last touched first.
    // Made only while they're wanted (Provider.qml's wanted), giving those
    // last made otherwise and reading nothing else, so a list the harness
    // writes with the launcher shut makes none: Joel's shut launcher made
    // 184 in 240 s, 128 in one burst (YQZGPC, 2026-11-10). Asked for while
    // not wanted, as Start a Session steps in, they're laid out then
    // (laidOut), keeping those that read the same.
    readonly property var all: provider.wanted ? laidOut() : kept.rows ?? []

    // Each quest's title broken into the pieces A to Z reads (Order.js's
    // pieces), by its id, with the title they were made from. Sorted by
    // name alone, both titles were broken apart at each comparison, about
    // 10,000 for Joel's 1,063 quests: 20% of the stall after an opening's
    // frame (H6EDLC's profile, 2026-11-10). Now each is broken once, and
    // again only when its title changes. A stopgap: Quests' ready rows, from
    // a quests daemon of their own by Thu 12 (RCTRF8), take the sort out of
    // the launcher. A plain object, so keeping one notifies nothing.
    readonly property var named: ({})
    function piecesKept() {
        const next = {};
        for (const quest of quests) {
            const title = quest.title || quest.id;
            const before = provider.named[quest.id];
            next[quest.id] = before && before.title === title
                ? before : { title: title, pieces: Order.pieces(title) };
        }
        for (const id of Object.keys(provider.named))
            delete provider.named[id];
        Object.assign(provider.named, next);
        return next;
    }

    function laidOut() {
        const bucket = q => {
            const at = priorities.indexOf(q.priority);
            return at < 0 ? priorities.length : at;
        };
        const named = piecesKept();
        const sorted = Order.sorted(quests, Order.returned({
            first: isOpen,
            placed: (a, b) => bucket(a) - bucket(b)
                || (Number(a.rank) || 1e9) - (Number(b.rank) || 1e9),
            last: q => String(q.updated ?? ""),
            pieces: q => named[q.id].pieces
        }));
        const sessions = loaded;
        const next = {};
        const rows = sorted.map(quest => {
            const from = [quest, sessions[quest.id] ?? null];
            const before = provider.kept[quest.id];
            next[quest.id] = before && Pi.alike(before.from, from)
                ? before : { from: from, row: made(quest) };
            return next[quest.id].row;
        });
        for (const id of Object.keys(provider.kept))
            delete provider.kept[id];
        Object.assign(provider.kept, next);
        // Not a quest's id, which begins with QEST-.
        provider.kept.rows = rows;
        return rows;
    }
    function rowsNow() {
        return provider.wanted ? all : laidOut();
    }
    onAllChanged: changedTo(all)

    // A day as a clock's calendar says it, with its year when it isn't this
    // one, from a date or a time, as the harness writes either.
    function day(updated) {
        const when = new Date(`${String(updated).slice(0, 10)}T12:00:00`);
        if (isNaN(when))
            return "";
        const thisYear = when.getFullYear() === new Date().getFullYear();
        return Qt.formatDate(when, thisYear ? "d MMM" : "d MMM yyyy");
    }

    function go(quest) {
        const session = loaded[quest.id];
        const ongoing = session
            ? Sessions.running.find(one => one.address === session.address) : undefined;
        // One parked is woken in its own window, never joined by a second.
        const resting = session && !ongoing
            ? Sessions.parked.find(one => one.address === session.address) : undefined;
        if (ongoing) {
            Sessions.goTo(ongoing);
        } else if (resting) {
            starting(quest, "wake");
            Sessions.wake(resting);
        } else {
            starting(quest, "start");
            Sessions.start(quest.id, quest.folder);
        }
    }

    // A quest woken or started on reads from this key's frame as it will once
    // its session is up, which Return then goes to, until it is: a second
    // Return while pi still loads went on to start a second session (Joel,
    // 2026-11-07: "it should show right away").
    function starting(quest, act) {
        const up = () => Sessions.running.find(session => session.presence?.quest === quest.id);
        provider.becomes(`quest:${quest.id}`, {
            verb: Words.verb("verb:quest:switch"),
            run: () => {
                const session = up();
                if (session)
                    Sessions.goTo(session);
            }
        }, quest.title || quest.id, act, {
            done: () => up() !== undefined,
            within: Sessions.startsWithin
        });
    }

    function made(quest) {
        const session = loaded[quest.id];
        const row = member({
            thing: true,
            key: `quest:${quest.id}`,
            // A quest a session has loaded is that session's window, which
            // its Agents row says more of (Launcher.qml's compose).
            is: session ? session.is : `quest:${quest.id}`,
            rich: 1,
            // Return does the first of its acts (verbs), so it says that
            // act's verb, and a step in lifts no second act before them.
            verb: session ? (session.parked ? Words.verb("verb:quest:wake") : "go to")
                : Words.verb("verb:quest:open"),
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
            verbs: () => provider.verbs(quest),
            // What y takes: its id, the first of its facts, as its verbs
            // list them.
            operands: () => [copyOperand("fact:quest:id", quest.id)]
        });
        // Its id and kind, made ready to match when first searched, as its
        // name is, and kept while they read the same (Fuzzy.kept), for a
        // row made again when its quest is written.
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
        // Start a Session says where its Return starts one, and, where the
        // realm has another machine, the right arrow steps into each.
        const choosing = !session && Sessions.choosesMachine;
        // Return goes to its session, wakes the one parked, or starts one.
        const first = !session ? "verb:quest:open"
            : session.parked ? "verb:quest:wake" : "verb:quest:switch";
        // One with no session, that orchestrates nothing yet, can start one
        // that orchestrates it and the quests under it, as an act beside
        // Start Here rather than a question asked first (VKCF3Z,
        // 2026-11-07 [9a4b1dd0]). One listed as orchestrating reopens on
        // its own row in Agents.
        const orchestrating = (provider.launcher?.sources ?? [])
            .find(one => one.settings.name === "orchestrators")?.listed ?? [];
        const orchestrate = !session && !orchestrating.some(one => one.quest === quest.id)
            ? thing({
                key: "verb:quest:orchestrate",
                verb: Words.verb("verb:quest:orchestrate"),
                name: Words.name("verb:quest:orchestrate"),
                glyph: Words.glyph("verb:quest:orchestrate"),
                hue: "action",
                later: true,
                run: () => {
                    provider.starting(quest, "start");
                    Sessions.start(quest.id, quest.folder, true);
                }
            }) : null;
        return [thing({
            key: first,
            verb: Words.verb(first),
            name: Words.name(first),
            glyph: Words.glyph(first),
            hue: "action",
            note: choosing ? "this machine" : "",
            later: true,
            verbs: choosing ? () => provider.machines(quest) : null,
            run: () => provider.go(quest)
        }), orchestrate,
            fact("id", quest.id),
            fact("status", [quest.status, isOpen(quest) ? quest.priority : ""]
                .filter(part => part !== "").join(" \u00b7 ")),
            quest.updated ? fact("updated", day(quest.updated)) : null,
            parent ? fact("parent", parent.title || parent.id, parent.id) : null,
            fact("folder", homely, "where a session starts", quest.folder)
        ].filter(row => row !== null);
    }

    // Where a session on `quest` may start, as Start a Session's right
    // arrow shows it (VKCF3Z and Y2YBTS, 2026-10-29): this machine first,
    // where its Return starts one, then each other machine the realm
    // links, then Wherever It Fits, which the node places. One that can't
    // take a session now stays in its place, dimmed, its note saying why,
    // and does nothing: a gone row says no key and its Return runs
    // nothing (Launcher.qml's keysFor and run), though it keeps its run.
    function machines(quest) {
        const on = one => thing({
            key: `quest:${quest.id}:start:${one.name}`,
            verb: "start",
            name: one.name,
            glyph: Theme.glyph.agent,
            hue: "action",
            note: Listed.startNote(one, machineOf(one.name), startPlace()),
            gone: one.cannot !== "",
            later: true,
            run: () => {
                provider.starting(quest, "start");
                Sessions.startOn(one.name, quest.id, quest.folder);
            }
        });
        const anywhere = thing({
            key: `quest:${quest.id}:start:anywhere`,
            verb: Words.verb("verb:quest:start-anywhere"),
            name: Words.name("verb:quest:start-anywhere"),
            glyph: Words.glyph("verb:quest:start-anywhere"),
            hue: "action",
            note: Sessions.fits !== "" ? `on ${Sessions.fits}` : "no machine can take one now",
            gone: Sessions.fits === "",
            later: true,
            run: () => {
                provider.starting(quest, "start");
                Sessions.startOn("", quest.id, quest.folder);
            }
        });
        return Sessions.places.map(on).concat([anywhere]);
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        // Only the quests at the finest grade any reaches are kept, so one
        // found finer starts the list again, and a letter typed costs no
        // more than the quests it keeps.
        let grade = 3;
        let kept = [];
        for (const row of all) {
            const byName = (row.prepared.mask & need) === need
                ? Fuzzy.match(row.prepared, words, false) : null;
            const byWord = (row.also.mask & need) === need
                ? Fuzzy.match(row.also, words, false) : null;
            const best = !byWord || (byName && byName.score >= byWord.score) ? byName : byWord;
            if (!best || best.score < 0 || best.grade > grade)
                continue;
            if (best.grade < grade) {
                grade = best.grade;
                kept = [];
            }
            kept.push({ score: best.score, row: row });
        }
        const ranked = Order.sorted(kept, (a, b) =>
            Number(b.row.unfinished) - Number(a.row.unfinished) || b.score - a.score);
        return { rows: ranked.map(f => f.row), grade: grade };
    }

    function everything() {
        return rowsNow().filter(row => row.unfinished);
    }

    // A search looks through every quest, finished ones too.
    function looksThrough() {
        return rowsNow();
    }

    // Counted from the quests themselves, so saying how many makes no row.
    function counted() {
        const open = quests.filter(quest => isOpen(quest)).length;
        if (open === 0)
            return "empty";
        const driving = quests.filter(quest => isOpen(quest) && quest.priority === "driving").length;
        return [`${open} open`, driving > 0 ? `${driving} driving` : ""]
            .filter(part => part !== "").join(", ");
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                Sessions.lookAround();
        }
    }

    Component.onCompleted: Sessions.lookAround()
}

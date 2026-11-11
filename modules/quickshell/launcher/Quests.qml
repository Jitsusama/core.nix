// pi's quests, a collection: every quest in the store, its rows made
// ready by the quests daemon (modules/quests) from the realm's mycelium
// node's two lists of them, the open ones and the ended ones, and its
// sessions. The open ones are all of it with nothing typed, the one
// driving first, as the quest workflow ranks them; a concluded or retired
// one comes as typing finds it, after the open ones that match as well.
// Each wears its status's glyph, and says its kind unless it's a quest,
// its priority while it's open or when it was last touched once it's not,
// and the session that has it loaded, if one does, all as the daemon
// says it. The selected one reads its summary's first paragraph below the
// rows, for what it's about.
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
import "../When.js" as When
import "Order.js" as Order
import "RealmMachines.js" as Listed

Provider {
    id: provider

    // Quests, Agents' one word for them (Words.qml), a part of Agents.
    what: Words.of("quests").what

    collection: Words.collection("quests")

    // Every quest's row, in Quests' order, as the quests daemon makes them
    // ready: the order, open before ended by priority and rank, then the
    // newest, then A to Z, and every word a row says are the daemon's, so
    // nothing here sorts, words or lays out a quest (Joel, 2026-11-10: "the
    // daemons need to be the data layer, and they need to serve the
    // launcher"). Sorting Joel's 1,084 quests here, with their titles in
    // pieces and their rows laid out, was a third of the stall after an
    // opening's frame (H6EDLC's profile, 2026-11-10). Its rows are held by
    // key as the daemon tells each change (rowsKind), each wrapped with its
    // acts only as it changes, and laid out only while they're wanted
    // (Provider.qml's wanted), giving those last laid out otherwise.
    readonly property var all: provider.wanted ? laidOut() : lastLaidOut.rows ?? []
    readonly property var lastLaidOut: ({})

    // The daemon's rows of Quests, each record held by its key, its fields
    // under `row` (Y2YBTS's ready-rows.md), watched whether or not the
    // launcher's open, so it opens on rows already held.
    readonly property var rowsKind: rowsLink.item
    LazyLoader {
        id: rowsLink
        active: !!Quickshell.env("XDG_RUNTIME_DIR")

        Daemon {
            socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/quests.sock`
            kind: ["row", "summary"]
            watching: ({ place: "quests" })
            answersWhole: true
            onHeard: (changed, gone) => provider.placed(changed, gone)
        }
    }
    readonly property string summaryKey: "summary:quests"

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

    // The keys of its rows in the order the daemon gives them, each put in
    // its place by its `order` as it changes, never the list sorted; and
    // each row as last wrapped, by key. Plain, so keeping them notifies
    // nothing; `placings` says they moved. As Agents.qml holds its rows,
    // until ReadyRows.qml (Y2YBTS, due 2026-11-11) holds every place's.
    readonly property var placing: ({ keys: [] })
    readonly property var wrapped: ({})
    property int placings: 0

    // Takes what the daemon said moved: each key gone or changed let go,
    // and each changed row put back where its order places it. Its lists
    // come through the signal converted, so they're read by index.
    function placed(changed, gone) {
        const held = rowsKind?.held ?? {};
        const placedKeys = placing.keys;
        const moved = [];
        for (let at = 0; at < changed.length; at += 1)
            moved.push(changed[at]);
        for (let at = 0; at < gone.length; at += 1)
            moved.push(gone[at]);
        for (const key of moved) {
            delete wrapped[key];
            const at = placedKeys.indexOf(key);
            if (at >= 0)
                placedKeys.splice(at, 1);
        }
        for (let at = 0; at < changed.length; at += 1) {
            const key = changed[at];
            const order = held[key]?.row?.order;
            if (typeof order !== "number")
                continue;
            let low = 0;
            let high = placedKeys.length;
            while (low < high) {
                const middle = (low + high) >> 1;
                if ((held[placedKeys[middle]]?.row?.order ?? 0) < order)
                    low = middle + 1;
                else
                    high = middle;
            }
            placedKeys.splice(low, 0, key);
        }
        placings += 1;
    }

    // Every quest's row, in the daemon's order.
    function laidOut() {
        void placings;
        const held = rowsKind?.held ?? {};
        const rows = placing.keys.map(key => rowOf(key, held)).filter(row => row !== null);
        lastLaidOut.rows = rows;
        return rows;
    }
    function rowsNow() {
        return provider.wanted ? all : laidOut();
    }
    onAllChanged: changedTo(all)

    // A row of the daemon's, wrapped with what doing it needs, as it was
    // last wrapped while the daemon told no change to it.
    function rowOf(key, held) {
        const served = held[key]?.row;
        if (!served)
            return null;
        if (!(key in wrapped))
            wrapped[key] = made(served);
        return wrapped[key];
    }

    // A quest's title by its id, as its row says it, for a place naming
    // the quest a session has loaded.
    function titleOf(id) {
        return rowsKind?.held?.[`row:quests:${id}`]?.row?.name;
    }

    // A side item as the shell says it: its words, or a time in the
    // ready-row form, which no daemon words since the day depends on the
    // clock and the zone it's read by (KP0L6A, 2026-11-10). Here until
    // ReadyRows.qml words every place's times.
    function sideSaid(item) {
        if (typeof item.said === "string")
            return item.said;
        return item.as === "day" ? dayOf(item.at) : "";
    }

    // A moment's day by this calendar, "22 Oct", its year when not this
    // one, "5 Jan 2025"; nothing where there's none.
    function dayOf(at) {
        if (typeof at !== "number")
            return "";
        const when = new Date(at);
        const thisYear = when.getFullYear() === new Date().getFullYear();
        return Qt.formatDate(when, thisYear ? "d MMM" : "d MMM yyyy");
    }

    // What a row's acts need of its quest, from what the daemon holds
    // under its `held`.
    function questOf(served) {
        const held = served.held ?? {};
        return {
            id: held.id,
            title: served.name,
            kind: held.kind,
            status: held.status,
            priority: held.priority,
            updated: held.updated,
            day: dayOf(held.touched),
            folder: held.folder,
            open: held.open,
            parent: held.parent ?? null,
            session: held.session ?? null
        };
    }

    // The session a quest's row names, by its address, running or parked.
    function sessionAt(address) {
        return Sessions.running.find(one => one.address === address)
            ?? Sessions.parked.find(one => one.address === address) ?? null;
    }

    function go(quest) {
        const session = quest.session;
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

    function made(served) {
        const quest = questOf(served);
        const session = quest.session;
        const row = member({
            thing: true,
            key: served.row,
            rich: 1,
            // Return does the first of its acts (verbs), so it says that
            // act's verb, and a step in lifts no second act before them.
            verb: session ? (session.parked ? Words.verb("verb:quest:wake") : "go to")
                : Words.verb("verb:quest:open"),
            name: served.name,
            unfinished: served.unfinished === true,
            glyph: Theme.glyph[served.glyph] ?? Theme.glyph.pending,
            hue: served.hue ?? "program",
            side: (served.side ?? []).map(sideSaid).join(" \u00b7 "),
            note: served.note ?? "",
            reads: served.reads ?? "",
            later: true,
            run: () => provider.go(quest),
            verbs: () => provider.verbs(quest),
            // What y takes: its id, the first of its facts, as its verbs
            // list them.
            operands: () => [copyOperand("fact:quest:id", quest.id)]
        });
        // A quest a session has loaded is that session's window, which its
        // Agents row says more of (Launcher.qml's compose), read as it's
        // asked, since the window can change while the row doesn't.
        Object.defineProperty(row, "is", {
            enumerable: true,
            get: () => {
                const one = session ? provider.sessionAt(session.address) : null;
                return one ? Sessions.isOf(one) : `quest:${quest.id}`;
            }
        });
        // Its id and kind, made ready to match when first searched, as its
        // name is, and kept while they read the same (Fuzzy.kept).
        let also = null;
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => also ?? (also = Fuzzy.kept(served.held?.also ?? quest.id))
        });
        return row;
    }

    function verbs(quest) {
        const session = quest.session;
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
            fact("status", [quest.status, quest.open ? quest.priority : ""]
                .filter(part => part !== "").join(" \u00b7 ")),
            quest.updated ? fact("updated", quest.day) : null,
            quest.parent ? fact("parent", quest.parent.name, quest.parent.id) : null,
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

    // How many are open and driving, as the daemon says it, so saying how
    // many makes no row.
    function counted() {
        void rowsKind?.revision;
        return String(rowsKind?.held?.[summaryKey]?.summary?.said ?? "");
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

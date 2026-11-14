// pi's quests, a collection: every quest in the store, its rows made
// ready by the quests daemon (modules/quests) from the realm's mycelium
// node's two lists of them, the open ones and the ended ones, and its
// sessions. The open ones are all of it with nothing typed, the one
// driving first, as the quest workflow ranks them, and the only ones its
// watch serves; a concluded or retired one comes as typing finds it,
// after the open ones that match as well, the daemon finding it among
// every quest and sending its row with the answer.
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

    // While the daemon can't search, or isn't there to ask, only the open
    // quests it served are matched (`matched`), so the head says so in
    // Quests rather than a finished one going unfound in silence (VKCF3Z,
    // 2026-11-13).
    heading: ready.searchless || !ready.link
        ? String((Words.of("quests").said ?? {}).unsearched ?? "") : ""

    // Every open quest's row, in Quests' order, as the quests daemon makes them
    // ready: the order, open before ended by priority and rank, then the
    // newest, then A to Z, and every word a row says are the daemon's, so
    // nothing here sorts, words or lays out a quest (Joel, 2026-11-10: "the
    // daemons need to be the data layer, and they need to serve the
    // launcher"). Sorting Joel's 1,084 quests here, with their titles in
    // pieces and their rows laid out, was a third of the stall after an
    // opening's frame (H6EDLC's profile, 2026-11-10). Its rows are held and
    // placed as the daemon tells each change (ReadyRows.qml), each wrapped
    // with its acts only as it changes, and laid out only while they're
    // wanted (Provider.qml's wanted), giving those last laid out otherwise.
    readonly property var all: provider.wanted ? laidOut() : lastLaidOut.rows ?? []
    readonly property var lastLaidOut: ({})

    // The daemon's rows of Quests, watched whether or not the launcher's
    // open, so it opens on rows already held. Its acts are made here from
    // what each row holds until the quests daemon serves them (RCTRF8).
    ReadyRows {
        id: ready
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/quests.sock`
        place: "quests"
        beside: ["summary"]
        make: (row, key) => provider.made(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
        // The daemon's answer to what's typed now, the ended quests it
        // found among it, is searched again (Launcher.qml's foundMore).
        onSearched: words => {
            if (words === provider.searchedLast)
                provider.foundMore(false);
        }
    }
    // The words searched last, the only ones an answer is worth searching
    // again for.
    property string searchedLast: ""
    // What the daemon serves, for a reader told of each change it hears.
    readonly property alias served: ready
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

    // Every quest's row, in the daemon's order.
    function laidOut() {
        const rows = ready.rows();
        lastLaidOut.rows = rows;
        return rows;
    }
    function rowsNow() {
        return provider.wanted ? all : laidOut();
    }
    onAllChanged: changedTo(all)

    // A quest's title by its id, for a place naming the quest a session
    // has loaded, or what's filling the disk, as Machine does: an open
    // one's as its row says it, an ended one's, which the watch doesn't
    // serve, as the daemon answers it, asked once for the ids named
    // together (`titling`) and kept; `titlesHeard` tells a place to name
    // them again. Until then, or when it's no quest, undefined.
    function titleOf(id) {
        const served = ready.served(`row:quests:${id}`)?.name;
        if (served !== undefined || !id)
            return served;
        if (id in titled)
            return titled[id] || undefined;
        if (ready.link && !(id in titling)) {
            titling[id] = true;
            Qt.callLater(askTitles);
        }
        return undefined;
    }
    // Ended quests' titles by id, as the daemon answered them; "" for one
    // it has none of, forgotten as the launcher closes, since its ended
    // list may not have landed when it was asked.
    property var titled: ({})
    // The ids named since titles were last asked for.
    property var titling: ({})
    signal titlesHeard

    function askTitles() {
        const ids = Object.keys(titling).filter(id => !(id in titled));
        if (ids.length === 0 || !ready.link)
            return;
        ready.link.ask({ ask: "titles", ids: ids }, answer => {
            for (const id of ids)
                delete titling[id];
            // The daemon gone, or too old to answer, leaves them to be
            // asked again when they're named.
            const titles = answer?.titles;
            if (!titles)
                return;
            for (const id of ids)
                titled[id] = String(titles[id] ?? "");
            if (ids.some(id => titled[id] !== ""))
                titlesHeard();
        });
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
            day: ready.timeSaid({ at: held.touched, as: "day" }),
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

    // A quest's row as the daemon serves it, made as every place's ready
    // rows are (ReadyRows.qml's thing), with what only a quest's session
    // knows over it until the daemon serves its acts (RCTRF8).
    function made(served, key) {
        const quest = questOf(served);
        const session = quest.session;
        const row = ready.thing(served, key, {
            thing: true,
            rich: 1,
            // Return does the first of its acts (verbs), so it says that
            // act's verb, and a step in lifts no second act before them.
            verb: session ? (session.parked ? Words.verb("verb:quest:wake") : "go to")
                : Words.verb("verb:quest:open"),
            glyph: Theme.glyph[served.glyph] ?? Theme.glyph.pending,
            hue: served.hue ?? "program",
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

    // What's typed, as the quests daemon finds it among every quest, the
    // open first (modules/quests' search.rs): the watch serves only the
    // open ones, 230 of Joel's 1,237, since holding every ended one whole
    // cost the shell's collector a walk over them all (BXNMUC, 2026-11-13),
    // so the ended ones are found where they're kept, and come with their
    // rows. While its answer is asked, the rows that still hold every
    // letter typed stand: the last answer's, when what's typed goes on
    // from it, else the open ones; the answer, a moment later, takes their
    // place (`onSearched`). A daemon that can't search, or none to ask,
    // leaves the open ones matched here (`matched`).
    function search(typed) {
        const asked = Fuzzy.words(typed).join(" ");
        searchedLast = asked;
        if (ready.searchless || !ready.link)
            return matched(typed);
        const answer = ready.search(asked);
        if (answer !== null)
            return { rows: ready.foundRows(answer), grade: answer.keys.length > 0 ? 0 : 3 };
        const need = Fuzzy.letters(Fuzzy.words(typed));
        const holds = item => !!item && (item.mask & need) === need;
        const last = ready.found.words !== null && asked.startsWith(ready.found.words)
            ? ready.foundRows(ready.found) : all;
        const rows = last.filter(row => holds(row.prepared) || holds(row.also));
        return { rows: rows, grade: rows.length > 0 ? 0 : 3 };
    }

    // Whether a search of these letters waits on the daemon's answer, asked
    // now if it isn't yet, so the launcher holds the key's list for it a
    // moment (Launcher.qml's answersWaitedFor).
    function awaits(letters) {
        const asked = Fuzzy.words(letters).join(" ");
        if (asked === "" || ready.searchless || !ready.link)
            return false;
        searchedLast = asked;
        return ready.search(asked) === null && ready.asking === asked;
    }

    // The open quests matched here, the best grade first, for a daemon that
    // can't search.
    function matched(typed) {
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

    // The open quests, and after them the ended ones what's typed found,
    // which the daemon doesn't serve: a match found from home is followed,
    // by Tab or Ctrl+], through Quests and found among what it holds
    // (Launcher.qml's enter), so an ended one is held there too.
    function everything() {
        const open = rowsNow().filter(row => row.unfinished);
        if (ready.found.words === null)
            return open;
        const ended = ready.foundRows(ready.found).filter(row => !row.unfinished);
        return ended.length === 0 ? open : open.concat(ended);
    }

    // How many are open and driving, as the daemon says it, so saying how
    // many makes no row.
    function counted() {
        void ready.placings;
        return String(ready.held[summaryKey]?.summary?.said ?? "");
    }

    Connections {
        target: provider.launcher

        // Closing lets go of what was found, so the ended quests' rows are
        // held only while a search shows them.
        function onOpenChanged() {
            if (provider.launcher.open) {
                Sessions.lookAround();
                return;
            }
            ready.ended();
            for (const id of Object.keys(titled)) {
                if (titled[id] === "")
                    delete titled[id];
            }
        }
    }

    Component.onCompleted: Sessions.lookAround()
}

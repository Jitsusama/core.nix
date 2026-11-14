// The session's notifications, a collection: everything that arrived,
// newest first, as the notices daemon serves their rows ready
// (modules/notices, row.rs; ReadyRows.qml), and found by it (search.rs),
// so nothing is sorted, merged, matched or worded here (Joel, 2026-11-10:
// "the daemons need to be the data layer, and they need to serve the
// launcher"). It's no place of its
// own: For You holds the unread under Desktop and the read in History,
// each the daemon's place of that name, and "notices" asked from outside
// leads there (Now.qml's leadsTo); here, a search with a match adds one
// row, whose Return shows all it found (VKCF3Z, 2026-11-09). Return on one
// still open does what its sender does when it's chosen, as clicking it
// does, asked of the daemon with a token while the launcher holds the
// keys, which lets the sender bring its window forward; on one closed, it
// goes to the program that sent it, its window if it has one open, or
// steps into its drawer when nothing sent it that can be gone to. What
// else can be done is a command on the : line, as on its card, each an
// act the daemon serves: its own actions while it's still open, its
// links, marking it read or unread, letting it go and copying what it
// said. Each says whether it's unread beside its age, and acting on one
// reads it.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Notice.js" as Notice

Provider {
    id: provider

    what: "notice"

    // Named Notifications, Joel's word, as the vocabulary says (Words.qml),
    // and never found by it, since it's no place of its own.
    collection: Words.collection("notices", { placed: false })

    readonly property string socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/notices.sock`

    // Every notice, the unread saying so, for search, Notifications and a
    // program's own row (Applications.qml). The daemon serves a page of it, the unread, the live and the newest read
    // (row.rs's paged), with beside it how many it has rows for; the rest
    // are asked for by those that list them (every).
    ReadyRows {
        id: mixed

        socket: provider.socket
        place: "notices"
        beside: ["summary"]
        paced: provider.launcher
        maker: provider
        make: (row, key) => provider.made(mixed, row, key)
        drawn: provider.launcher?.open ?? false
    }
    // For You's Desktop, the unread and those read while Joel looks, each
    // read one saying so. Watched only while the launcher is open, since
    // the daemon keeps one read here only while it's watched, so it goes
    // to History once the launcher closes.
    ReadyRows {
        id: desktop

        socket: provider.socket
        place: "notices-desktop"
        live: provider.launcher?.open ?? false
        paced: provider.launcher
        maker: provider
        make: (row, key) => provider.made(desktop, row, key)
        drawn: provider.launcher?.open ?? false
    }
    // For You's History, the read, each saying how and when it was done
    // with, "read here 14:02", newest done first, a page at a time as the
    // mixed place is, with beside it how many it keeps of each source, so
    // History counts and orders them as the daemon says rather than
    // working it out from every notice (2RCBOJ, 2026-11-13).
    ReadyRows {
        id: past

        socket: provider.socket
        place: "notices-history"
        beside: ["summary"]
        paced: provider.launcher
        maker: provider
        make: (row, key) => provider.made(past, row, key)
        drawn: provider.launcher?.open ?? false
    }
    Connections {
        target: past
        function onHeard() {
            provider.changed();
        }
    }
    Connections {
        target: mixed
        function onHeard() {
            provider.changed();
        }
    }
    Connections {
        target: desktop
        function onHeard() {
            provider.changed();
        }
    }

    // A served row as the launcher holds a notice's, the place's `ready`.
    // Its note is the line of its body that holds what was typed (VKCF3Z,
    // 2026-11-08), as each search says it (lined), and its sender's name
    // says once, in its drawer's eyebrow, while the drawer is out, so the
    // cursor row's side is its age alone (VKCF3Z, 2026-11-09).
    function made(ready, row, key) {
        const held = row.held ?? {};
        const said = String(held.said ?? "");
        const cut = said.indexOf("\n");
        const one = ready.thing(row, key, {
            notice: held.id,
            count: typeof row.note === "string" ? row.note : "",
            // Its note gives way by whole words, never an ellipsis (Choice.qml).
            countWords: true,
            // Its body, which its note is a line of as each search says it.
            wrote: cut < 0 ? "" : said.slice(cut + 1),
            // Stepped into, the head says its title alone, its words being
            // the drawer's (VKCF3Z, 2026-11-08).
            title: row.name ?? "",
            note: held.app ?? "",
            noteInDrawer: true,
            // Who sent it, which ends a paragraph of rows (Launcher.qml's
            // runOf), so dip takes away what one sender sent.
            sender: held.app ?? "",
            at: held.at ?? 0,
            unread: held.read !== true,
            operands: () => provider.operands(ready, row, key, held)
        });
        if (one === null)
            return null;
        // History's says when it was done with and which source it's
        // History's for, as the daemon holds them, and is muted and For
        // You's as the daemon serves it (row.rs), so Now lists it as is.
        if (ready === past)
            Object.assign(one, { doneAt: held.readAt ?? held.at ?? 0,
                pastSource: held.source ?? "", forYou: row.forYou ?? null });
        // With nothing to open or go to, the daemon serves its details
        // first, which a launcher with no drawer to step into can't show,
        // so Return steps in among its acts, as the right arrow does,
        // rather than doing the next act: copying is never first (Joel,
        // 2026-10-28: "having the separate copy option at the top is very
        // confusing"), nor is marking it read. Its command is then no
        // act's, so Return from the find steps in rather than landing, as
        // one that would mark it read does (Launcher.lands).
        if ((row.acts ?? [])[0]?.step === "drawer"
            && typeof provider.launcher?.intoDrawer !== "function") {
            Object.assign(one, { verb: "more", now: false, later: false, stays: true,
                targets: one.verbs, wordsId: undefined,
                run: () => provider.launcher?.enter(one) });
        }
        // Where Ctrl+] goes from it, as Vim follows a tag: its sender's own
        // row, among Programs (Applications.qml), or nowhere for a sender
        // no program is, as the daemon found its entry.
        one.follows = typeof held.entry === "string" && held.entry !== ""
            ? `program:${held.entry}` : "";
        return one;
    }

    // What d and y take of a notice: done with it while it's live, and
    // copying what it said, as plain fields rather than rows made
    // searchable, since d or y over a history of hundreds asks every row
    // (Launcher.qml's actionsOf).
    function operands(ready, row, key, held) {
        const acts = row.acts ?? [];
        const copying = acts.find(one => one.key === "verb:notice:copy") ?? null;
        const copy = copying === null ? [] : [{
            key: "verb:notice:copy",
            verb: Words.verb("verb:notice:copy"),
            copies: copying.copy,
            run: () => ready.act(key, copying)
        }];
        const forgetting = acts.find(one => one.key === "verb:notice:forget") ?? null;
        if (forgetting === null || parting.some(one => one.id === held.id))
            return copy;
        // d reads it, at its sender where it offers Read, as the daemon's
        // Mark It Read does then, and only forgets it where not, so it
        // says which, as the held notices' d does.
        const marking = held.read === true ? null
            : acts.find(one => one.key === "verb:notice:read") ?? null;
        const read = marking?.ask?.ask === "invoke"
            ? { ends: Words.field.operators?.read ?? "" } : {};
        return [Object.assign({ key: "verb:notice:forget" }, Words.ending("verb:notice:forget"),
            read, done(held, forgetting, marking))].concat(copy);
    }

    // d's done, as the held notices' is (HeldNotices.qml's finish): read at
    // once, at its sender too where it offers Read, and let go as the
    // launcher closes, a dismissed notice being gone for good, so u brings
    // it back until then, its read state staying, as the spec's notices do.
    // Each is asked through the place every notice is in, which stays
    // watched as the launcher closes, where Desktop's is let go.
    function done(held, forgetting, marking) {
        const key = `row:${mixed.place}:${held.id}`;
        return {
            run: () => {
                if (marking !== null)
                    mixed.act(key, marking);
                provider.parting = provider.parting.concat([
                    { id: held.id, key: key, act: forgetting }]);
            },
            undo: () => {
                provider.parting = provider.parting.filter(one => one.id !== held.id);
            }
        };
    }

    // The notices d is done with, each with how it's let go, as the
    // launcher closes.
    property var parting: []

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                return;
            // A sender's Read closes a notice it doesn't keep, so only one
            // still open, still offering to be let go, is let go.
            for (const one of provider.parting) {
                if ((provider.servedRow(one.key)?.acts ?? []).some(act => act.key === one.act.key))
                    mixed.act(one.key, one.act);
            }
            provider.parting = [];
            // The rows past the page go, kept only while they're listed.
            for (const state of [provider.whole, provider.pastWhole]) {
                state.kept = null;
                state.of = null;
                state.rows = [];
            }
        }
    }

    // Those a program sent, newest first, as the daemon orders them, so a
    // program's own row (Applications.qml) holds them and Return on one
    // does what it does here. `sent` says whether a sender's name is the
    // program.
    function sentBy(sent) {
        return lined(every().filter(row => sent(row.sender)), []);
    }

    // A notice's row as last served, by the place or past its page by an
    // answer still kept: null for one no longer served.
    function servedRow(key) {
        return mixed.served(key) ?? whole.kept?.rows[key]
            ?? answers.kept.map(one => one.rows[key]).find(row => !!row) ?? null;
    }

    // Every notice's row, those past the page too, in the daemon's order:
    // the page alone while the daemon has nothing past it, and until it's
    // answered the `rows` it's asked for once it's needed, as
    // Notifications and a program's own row list them, and History's own
    // as History lists them (pastRows). It's asked again only once the
    // notices changed past what the answer can list, a notice new to it or
    // one it didn't bring that the page no longer serves, rather than at every change, each answer being about
    // 1.3 MB of Joel's. Those past the page are made once and let go as
    // the launcher closes: held while it's closed, all 1,000 of Joel's
    // cost the shell about 8 MB of its live heap in the lab, of 45
    // (2026-11-12).
    readonly property string summaryKey: "summary:notices"
    readonly property var whole: ({ ready: mixed, summary: "summary:notices",
        ask: { ask: "rows" }, kept: null, asking: false, of: null, at: -1, rows: [] })
    // History's, asked of its own place, in its order.
    readonly property var pastWhole: ({ ready: past, summary: "summary:notices-history",
        ask: { ask: "rows", place: "notices-history" },
        kept: null, asking: false, of: null, at: -1, rows: [] })
    function every() {
        return paged(whole, all());
    }

    // How many notices each sender sent, and how many of those are unread,
    // by its name, for a program's own row (Applications.qml): read from
    // the daemon's records, those past the page as its answer brought
    // them, never from their rows, since counting every row made all
    // 1,000 of Joel's past the page for two fields each, 97 ms of Google
    // Calendar's acts (ZINAWZ, 2026-11-14). One object until the notices
    // change, so a count kept by it holds. A stopgap until the notices
    // daemon serves each sender's count in its summary, which needs no
    // answer past the page at all.
    readonly property var sentKept: ({ of: null, at: -1, by: null })
    function sentCounts() {
        const page = all();
        const count = mixed.held[whole.summary]?.summary?.count ?? page.length;
        const kept = count > page.length ? whole.kept : null;
        if (count > page.length && kept === null)
            wholeAsked(whole);
        if (sentKept.by !== null && sentKept.of === kept && sentKept.at === mixed.placings)
            return sentKept.by;
        const by = Object.create(null);
        const add = (sender, unread) => {
            const one = by[sender] ?? (by[sender] = { count: 0, unread: 0 });
            one.count += 1;
            if (unread)
                one.unread += 1;
        };
        if (kept === null) {
            for (const row of page)
                add(row.sender, row.unread);
        } else {
            let missing = false;
            for (const key of kept.keys) {
                const held = (mixed.served(key) ?? kept.rows[key])?.held;
                if (held)
                    add(String(held.app ?? ""), held.read !== true);
                else
                    missing = true;
            }
            behind(whole, page, missing);
        }
        sentKept.of = kept;
        sentKept.at = mixed.placings;
        sentKept.by = by;
        return by;
    }

    // A paged place's every row, `state` its whole or History's, `page`
    // the rows its watch serves.
    function paged(state, page) {
        const ready = state.ready;
        const count = ready.held[state.summary]?.summary?.count ?? page.length;
        if (count <= page.length)
            return page;
        const kept = state.kept;
        if (kept === null) {
            wholeAsked(state);
            return page;
        }
        if (state.of !== kept || state.at !== ready.placings) {
            const rows = kept.keys.map(key => foundRow(kept, key, ready));
            state.rows = rows.filter(row => row !== null);
            state.of = kept;
            state.at = ready.placings;
            behind(state, page, state.rows.length < rows.length);
        }
        return state.rows;
    }

    // Asks again for every row of a paged place once what's kept falls
    // behind: a row it listed is `missing`, or the page serves a notice it
    // doesn't list.
    function behind(state, page, missing) {
        const kept = state.kept;
        if (!kept.listed)
            kept.listed = new Set(kept.keys);
        const listed = row => kept.listed.has(`row:${state.ready.place}:${row.notice}`);
        if (missing || !page.every(listed))
            wholeAsked(state);
    }

    // Asks the daemon for every row of a paged place, unless it's being
    // asked.
    function wholeAsked(state) {
        const link = state.ready.link;
        if (state.asking || !link || link.wire === null)
            return;
        state.asking = true;
        link.find(state.ask, answer => provider.wholeAnswered(state, answer));
    }

    // Every row answered is kept, and what lists them told, so a list
    // shown from the page alone fills in; one come once the launcher
    // closed is let go at once.
    function wholeAnswered(state, answer) {
        state.asking = false;
        if (!Array.isArray(answer?.keys) || provider.launcher?.open === false)
            return;
        // A row past the page that reads as it did is kept as made, so a
        // notice read, which History asks again for, remakes none of the
        // rest it already held.
        const rows = answer.rows ?? {};
        const was = state.kept;
        const made = {};
        for (const key of Object.keys(rows)) {
            if (was !== null && key in was.made
                && JSON.stringify(was.rows[key]) === JSON.stringify(rows[key]))
                made[key] = was.made[key];
        }
        state.kept = { keys: answer.keys, rows: rows, made: made, listed: null };
        provider.changed();
    }

    // History's rows of `sources` (Lately.js's pastSources), every one, as
    // the daemon serves them ready: newest done with first, muted, each
    // saying whose For You it's in, and its note its body's first line, so
    // each is listed as it's made, once, rather than copied (row.rs). Each
    // list is kept while the page it's taken from is the same, so stepping
    // into History again hands back the rows it held: lining every one
    // anew at each step made all 594 to 679 of Joel's new each time, 81 to
    // 152 ms to the frame (2026-11-13).
    readonly property var pastPage: ({ at: -1, rows: [] })
    readonly property var pastLists: ({ page: null, lists: ({}) })
    function pastRows(sources) {
        if (pastPage.at !== past.placings) {
            pastPage.rows = past.rows();
            pastPage.at = past.placings;
        }
        const page = paged(pastWhole, pastPage.rows);
        const kept = pastLists;
        if (kept.page !== page) {
            kept.page = page;
            kept.lists = {};
        }
        const asked = sources.join(" ");
        if (asked in kept.lists)
            return kept.lists[asked];
        const wanted = new Set(sources);
        const rows = page.filter(row => wanted.has(row.pastSource));
        kept.lists[asked] = rows;
        return rows;
    }

    // How many History keeps, as the daemon counts them, by source; told
    // again each time its rows move.
    readonly property var pastSummary: past.placings >= 0
        ? past.held["summary:notices-history"]?.summary ?? null : null

    // Every notice's row, one list until the daemon's places move, since
    // the daemon's rows() gives a new list each time, for the program's
    // own row, For You and everything a collection holds.
    readonly property var everyRow: ({ at: -1, rows: [] })
    function all() {
        if (everyRow.at !== mixed.placings) {
            everyRow.rows = mixed.rows();
            everyRow.at = mixed.placings;
            // A search's copies of rows the daemon no longer serves go.
            const copies = Object.create(null);
            for (const row of everyRow.rows) {
                if (searched.copies[row.key]?.row === row)
                    copies[row.key] = searched.copies[row.key];
            }
            searched.copies = copies;
        }
        return everyRow.rows;
    }

    // The notices of `ids`, as For You's Desktop (Now.qml) holds the
    // unread: each read one says so, as one read while Joel looks stays
    // where it was (VKCF3Z, 2026-11-09), as the daemon words it.
    // Desktop's are in the order of `ids`, each its own place's row once
    // that's told of it, and until then, while unread, the one every
    // notice has: Desktop is watched again only as the launcher opens, so
    // one come while it was closed would be missing from the step into
    // it, then put at its end once told, rather than where it belongs. A
    // read one is Desktop's only once it's been watched, so it's told.
    // Whether Desktop holds the notice of `id` as its own watch told it,
    // rather than desktopRows taking it from every notice while that watch
    // hasn't told it yet, as it waits on a frame drawn.
    function desktopHolds(id) {
        return `row:${desktop.place}:${id}` in desktop.held;
    }

    function desktopRows(ids) {
        return lined(ids.map(id => desktop.rowAt(`row:${desktop.place}:${id}`)
            ?? [mixed.rowAt(`row:${mixed.place}:${id}`)].find(row => row?.unread === true))
            .filter(row => !!row), []);
    }

    // Whether a text holds a word typed, as the daemon's search finds it,
    // for the line of a notice's body its note is: worked out here only
    // for the rows read, a screen's twenty, a stopgap until the daemon's
    // answer says each found notice's line.
    function holds(text, word) {
        return Fuzzy.match(Fuzzy.prepare(text), [word], false).score >= 0;
    }

    // Each row's note, the line of its body that holds the words typed,
    // or its first line with none. Not `noted`, which is Provider's own,
    // what a collection's row says below it. Each is a copy, its note
    // worked out once it's read (Notice.js's lined).
    function lined(rows, words) {
        return Notice.lined(rows, words, provider.holds);
    }

    // What a search lists: each row's one copy (Notice.js's lining), its
    // note for the words typed last, kept by its key while its row is the
    // same, so a key copies only rows it hasn't seen. Its copies sit in an
    // object with no prototype, never a Map, which Qt's engine looks a key
    // up in by walking them all (RowsKept.js).
    readonly property var searched: ({ words: [], copies: Object.create(null) })
    function listed(rows, words) {
        searched.words = words;
        const copies = searched.copies;
        return rows.map(row => {
            const kept = copies[row.key];
            if (kept?.row === row)
                return kept.copy;
            const copy = Notice.lining(row, searched, provider.holds);
            copies[row.key] = { row: row, copy: copy };
            return copy;
        });
    }

    // The daemon finds the notices what's typed holds, by their titles,
    // which name them and show where, else by anything else they said,
    // closest first and newest first among equals (modules/notices'
    // search.rs), and nothing is matched here: matching them in Qt's
    // engine took 64 of the 89 ms "ca" spent on Joel's (2026-11-11).
    // Its answer comes a moment after the key, which the launcher waits
    // for a little (`awaits`, Launcher.qml's answersWaitedFor); until it
    // comes, what's typed finds no notice, and once it does, the launcher
    // is told it found more. One kept from before the notices changed is
    // asked again and not listed meanwhile: it can list a notice the
    // daemon no longer finds, which stays, with the selection on it, once
    // the new answer lands what it does find under it. A notice found
    // whose row the place doesn't serve comes with its row in the answer
    // (ready-rows.md's search), so the place needn't hold every notice for
    // search: all 1,000 of Joel's rows were about 11 MB kept live in the
    // lab (2026-11-12).
    function search(typed) {
        const words = Fuzzy.words(typed);
        const asked = words.join(" ");
        answers.wanted = asked;
        const kept = answers.kept.find(one => one.words === asked
            && one.at === mixed.placings) ?? null;
        if (kept === null) {
            searchAsked(asked);
            return { rows: [], grade: 3 };
        }
        all();
        const rows = kept.keys.map(key => foundRow(kept, key)).filter(row => row !== null);
        // With how many the daemon found, as it lists at most 60.
        return { rows: listed(rows, words), grade: rows.length > 0 ? 0 : 3, found: kept.found };
    }

    // The row of `key` a kept answer found: the place's where it serves
    // one, else the answer's own, made as the place makes one the first
    // time it's listed and let go with the answer; null for neither.
    function foundRow(kept, key, ready = mixed) {
        if (key in ready.held)
            return ready.rowAt(key);
        if (!(key in kept.made)) {
            const row = kept.rows[key];
            kept.made[key] = row ? ready.make(row, key) : null;
        }
        return kept.made[key];
    }

    // Whether a search of these letters waits on the daemon's answer, asked
    // now if it isn't yet: none is kept for them as the notices are now,
    // and the daemon is there to ask.
    function awaits(letters) {
        const asked = Fuzzy.words(letters).join(" ");
        answers.wanted = asked;
        if (asked === "" || answers.kept.some(one => one.words === asked
            && one.at === mixed.placings))
            return false;
        return searchAsked(asked);
    }

    // The daemon's answers to the last few searches, each with the place's
    // placings when it was asked, so one from before a change is asked
    // again, with the rows it found that the place doesn't serve and those
    // made of them; the words being asked, with theirs; and the words
    // searched last, the only ones an answer is worth searching again for.
    // The last few, so typing back over what was typed finds it at once.
    readonly property var answers: ({ kept: [], asking: Object.create(null), wanted: "" })
    readonly property int answersKept: 8

    // Asks the daemon what `asked` finds, unless it's being asked already,
    // and says whether an answer is coming. Named apart from Provider's
    // `ask` and `answered`, which are a URL's.
    function searchAsked(asked) {
        if (asked in answers.asking)
            return true;
        const link = mixed.link;
        if (!link || link.wire === null)
            return false;
        answers.asking[asked] = mixed.placings;
        link.find({ ask: "search", words: asked }, answer => provider.searchAnswered(asked, answer));
        return true;
    }

    // An answer is kept, and the launcher told it found more when it's for
    // what was searched last. One the daemon couldn't give is let go, to be
    // asked again by the next search.
    function searchAnswered(asked, answer) {
        const at = answers.asking[asked];
        delete answers.asking[asked];
        if (!Array.isArray(answer?.keys))
            return;
        answers.kept = answers.kept.filter(one => one.words !== asked)
            .concat([{ words: asked, keys: answer.keys, rows: answer.rows ?? {}, made: {},
                found: answer.found ?? answer.keys.length, at: at }]).slice(-answersKept);
        if (answers.wanted === asked)
            provider.foundMore(false);
    }

    function everything() {
        return listed(every(), []);
    }

    function looksThrough() {
        return all();
    }

    function counted() {
        const held = mixed.held[summaryKey]?.summary?.count ?? all().length;
        return Notices.waiting.length > 0 ? `${Notices.waiting.length} missed`
            : held === 0 ? "none" : countedAs(held);
    }

    function opened() {
        Notices.seen();
    }

    // The notices on screen step aside while the launcher holds a place
    // listing them, so each is in one place: where the keys reach it. That's
    // a row that says it gathers them, as For You's Desktop does, where
    // every way to the notices goes (Now.qml's leadsTo), or one of these.
    Binding {
        target: Notices
        property: "gathered"
        value: provider.launcher !== null && provider.launcher.open
            && provider.launcher.held.some(step => step.row.from === provider
                || step.row.gathers === true)
    }
}

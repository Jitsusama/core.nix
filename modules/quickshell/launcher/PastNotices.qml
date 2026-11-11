// The session's notifications, a collection: everything that arrived,
// newest first, as the notices daemon serves their rows ready
// (modules/notices, row.rs; ReadyRows.qml), so nothing is sorted, merged
// or worded here (Joel, 2026-11-10: "the daemons need to be the data
// layer, and they need to serve the launcher"). It's no place of its
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
import "Order.js" as Order

Provider {
    id: provider

    what: "notice"

    // Named Notifications, Joel's word, as the vocabulary says (Words.qml),
    // and never found by it, since it's no place of its own.
    collection: Words.collection("notices", { placed: false })

    readonly property string socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/notices.sock`

    // Every notice, the unread saying so, for search, Notifications, a
    // program's own row (Applications.qml) and For You's History, which
    // holds a notice its sender withdrew unread beside those read, so the
    // daemon's own history place, the read alone, isn't watched.
    ReadyRows {
        id: mixed

        socket: provider.socket
        place: "notices"
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
        let whole = null;
        let made = null;
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
                run: () => provider.launcher?.enter(made) });
        }
        // Where Ctrl+] goes from it, as Vim follows a tag: its sender's own
        // row, among Programs (Applications.qml), or nowhere for a sender
        // no program is, as the daemon found its entry.
        one.follows = typeof held.entry === "string" && held.entry !== ""
            ? `program:${held.entry}` : "";
        // What it said, made ready to match only once a search asks, as a
        // row made afresh with its getter (Provider.gotten).
        made = provider.gotten(one, {
            also: () => whole ?? (whole = Fuzzy.kept(said.replace(/\s+/g, " ")))
        });
        return made;
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
                if ((mixed.served(one.key)?.acts ?? []).some(act => act.key === one.act.key))
                    mixed.act(one.key, one.act);
            }
            provider.parting = [];
        }
    }

    // Those a program sent, newest first, as the daemon orders them, so a
    // program's own row (Applications.qml) holds them and Return on one
    // does what it does here. `sent` says whether a sender's name is the
    // program.
    function sentBy(sent) {
        return lined(all().filter(row => sent(row.sender)), []);
    }

    // Every notice's row, one list until the daemon's places move. A
    // search goes on from the last one's matches only while it looks
    // through the very list that one did (Provider's poolFor), and the
    // daemon's rows() gives a new list each time: so each key looked
    // through every notice, as "ba" spent 45 of its 63 ms on Joel's
    // (2026-11-11).
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

    // The notices of `ids`, in the daemon's order, as For You's Desktop
    // (Now.qml) holds the unread and its History those done with,
    // `where` saying which: each read one in Desktop says so, as one read
    // while Joel looks stays where it was (VKCF3Z, 2026-11-09), as the
    // daemon words it.
    // Desktop's are in the order of `ids`, each its own place's row once
    // that's told of it, and until then, while unread, the one every
    // notice has: Desktop is watched again only as the launcher opens, so
    // one come while it was closed would be missing from the step into
    // it, then put at its end once told, rather than where it belongs. A
    // read one is Desktop's only once it's been watched, so it's told.
    function rowsOf(ids, where) {
        if (where !== "desktop")
            return lined(all().filter(row => ids.includes(row.notice)), []);
        return lined(ids.map(id => desktop.rowAt(`row:${desktop.place}:${id}`)
            ?? [mixed.rowAt(`row:${mixed.place}:${id}`)].find(row => row?.unread === true))
            .filter(row => !!row), []);
    }

    // Whether a text holds a word typed, as search finds it.
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

    // Found by its title, which names it and shows where, else by
    // anything else it said, which its note shows the line of.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        const every = all();
        for (const row of poolFor(every, typed)) {
            if ((row.also.mask & need) !== need)
                continue;
            const byName = Fuzzy.match(row.prepared, words, false);
            const m = byName.score >= 0 ? byName : Fuzzy.match(row.also, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        matchedIn(every, typed, found.map(f => f.row));
        // The closest first, and newest first among equals, since a notice
        // is wanted by when it came.
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score || b.row.at - a.row.at);
        return { rows: listed(kept.map(f => f.row), words), grade: finest };
    }

    function everything() {
        return listed(all(), []);
    }

    function looksThrough() {
        return all();
    }

    function counted() {
        const held = all().length;
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

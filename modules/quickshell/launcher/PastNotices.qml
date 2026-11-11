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
        // confusing"), nor is marking it read.
        if ((row.acts ?? [])[0]?.step === "drawer"
            && typeof provider.launcher?.intoDrawer !== "function") {
            Object.assign(one, { verb: "more", now: false, later: false, stays: true,
                targets: one.verbs, run: () => provider.launcher?.enter(one) });
        }
        Object.defineProperty(one, "also", {
            enumerable: true,
            get: () => whole ?? (whole = Fuzzy.kept(said.replace(/\s+/g, " ")))
        });
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
        return [Object.assign({ key: "verb:notice:forget" }, Words.ending("verb:notice:forget"),
            done(ready, key, held, forgetting))].concat(copy);
    }

    // d's done, as the held notices' is (HeldNotices.qml's finish): read at
    // once, and let go as the launcher closes, a dismissed notice being
    // gone for good, so u brings it back until then, its read state
    // staying, as the spec's notices do.
    function done(ready, key, held, forgetting) {
        return {
            run: () => {
                if (held.read !== true)
                    ready.act(key, { key: "verb:notice:read",
                        ask: { ask: "mark", notices: [held.id], read: true } });
                provider.parting = provider.parting.concat([
                    { id: held.id, ready: ready, key: key, act: forgetting }]);
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
            for (const one of provider.parting)
                one.ready.act(one.key, one.act);
            provider.parting = [];
        }
    }

    // Those a program sent, newest first, as the daemon orders them, so a
    // program's own row (Applications.qml) holds them and Return on one
    // does what it does here. `sent` says whether a sender's name is the
    // program.
    function sentBy(sent) {
        return lined(mixed.rows().filter(row => sent(row.sender)), []);
    }

    // The notices of `ids`, in the daemon's order, as For You's Desktop
    // (Now.qml) holds the unread and its History those done with,
    // `where` saying which: each read one in Desktop says so, as one read
    // while Joel looks stays where it was (VKCF3Z, 2026-11-09), as the
    // daemon words it.
    function rowsOf(ids, where) {
        const ready = where === "desktop" ? desktop : mixed;
        return lined(ready.rows().filter(row => ids.includes(row.notice)), []);
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

    // Found by its title, which names it and shows where, else by
    // anything else it said, which its note shows the line of.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        const all = mixed.rows();
        for (const row of poolFor(all, typed)) {
            if ((row.also.mask & need) !== need)
                continue;
            const byName = Fuzzy.match(row.prepared, words, false);
            const m = byName.score >= 0 ? byName : Fuzzy.match(row.also, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        matchedIn(all, typed, found.map(f => f.row));
        // The closest first, and newest first among equals, since a notice
        // is wanted by when it came.
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score || b.row.at - a.row.at);
        return { rows: lined(kept.map(f => f.row), words), grade: finest };
    }

    function everything() {
        return lined(mixed.rows(), []);
    }

    function looksThrough() {
        return mixed.rows();
    }

    function counted() {
        const all = mixed.rows().length;
        return Notices.waiting.length > 0 ? `${Notices.waiting.length} missed`
            : all === 0 ? "none" : countedAs(all);
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

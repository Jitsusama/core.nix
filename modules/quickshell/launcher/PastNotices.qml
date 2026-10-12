// The session's notifications, a collection: everything that arrived,
// newest first, from the shell's own history (Notices.qml), so nothing is
// asked of another program. A search with a match adds one row; the right
// arrow, or its row at home, shows them all, and its row at home says how
// many were missed until it's opened. Return goes to the program that sent
// one, its window if it has one open, or copies what it said when nothing
// sent it that can be gone to. The right arrow steps into what else can be
// done: copying what it said, marking it read or unread, and while it's
// still on screen, its own actions (Reply, Mark as read) and letting it go,
// so a notice is driven by the keys as well as by the pointer. Each says
// whether it's unread beside its age; acting on one reads it.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    collection: ({
        noun: "notices",
        title: "Notices",
        glyph: String.fromCodePoint(0xf009c),
        ask: "which notice?"
    })

    function ago(at) {
        const seconds = Math.max(0, (Date.now() - at) / 1000);
        if (seconds < 60)
            return "just now";
        if (seconds < 3600)
            return `${Math.floor(seconds / 60)} min ago`;
        if (seconds < 86400)
            return `${Math.floor(seconds / 3600)} h ago`;
        return `${Math.floor(seconds / 86400)} d ago`;
    }

    // The window of the program that sent it, or the program to open.
    function source(app) {
        const entry = DesktopEntries.heuristicLookup(app);
        const ids = [app.toLowerCase()].concat(entry ? [entry.id.toLowerCase()] : []);
        const window = Object.values(Niri.windows)
            .find(w => w.app_id && ids.includes(w.app_id.toLowerCase()));
        return { entry: entry, window: window ?? null };
    }

    // Goes to the program that sent it: its window as things are now, since
    // windows come and go, or the program, opened.
    function go(app) {
        const now = source(app);
        if (now.window)
            Niri.act({ FocusWindow: { id: now.window.id } });
        else if (now.entry)
            Niri.spawn(command(now.entry));
    }

    function made(notice) {
        const said = notice.body === "" ? notice.summary : `${notice.summary}\n${notice.body}`;
        const from = source(notice.app);
        return member({
            key: `notice:${notice.app}:${notice.summary}`,
            verb: from.window ? "switch" : from.entry ? "open" : "copy",
            name: notice.body === "" ? notice.summary
                : `${notice.summary} · ${notice.body.replace(/\s+/g, " ")}`,
            glyph: String.fromCodePoint(notice.glyph),
            hue: notice.hue,
            note: notice.app,
            at: notice.at,
            unread: !notice.read,
            // The clipboard takes text only while the launcher holds the
            // keys; going to a window waits for the launcher to be gone.
            now: !from.window && !from.entry,
            later: true,
            verbs: () => provider.verbs(notice, said),
            run: () => {
                Notices.mark(notice.id, true);
                if (!from.window && !from.entry)
                    Quickshell.clipboardText = said;
                else
                    provider.go(notice.app);
            }
        });
    }

    // What can be done to a notice: what its program offers while it's on
    // screen, then copying what it said, then letting it go. Quickshell
    // destroys a notification's actions when it closes or its program
    // replaces it, so each is found again by its identifier when it's run;
    // one whose notice has gone by then goes to its program instead.
    function verbs(notice, said) {
        const live = Notices.live[notice.id] ?? null;
        const offered = live ? live.actions.map(action => ({
            identifier: action.identifier,
            text: action.text
        })).map(action => thing({
            key: `verb:notice:${action.identifier}`,
            name: action.text,
            glyph: String.fromCodePoint(/reply/i.test(action.text) ? 0xf045a : 0xf0ce0),
            hue: "action",
            later: true,
            run: () => {
                Notices.mark(notice.id, true);
                const still = Notices.live[notice.id] ?? null;
                const now = still
                    ? still.actions.find(a => a.identifier === action.identifier) ?? null : null;
                if (now)
                    now.invoke();
                else
                    provider.go(notice.app);
            }
        })) : [];
        return offered.concat([thing({
            key: "verb:notice:copy",
            name: "Copy what it said",
            glyph: String.fromCodePoint(0xf018f),
            hue: "action",
            now: true,
            run: () => {
                Notices.mark(notice.id, true);
                Quickshell.clipboardText = said;
            }
        }), thing({
            key: "verb:notice:mark",
            name: notice.read ? "Mark it unread" : "Mark it read",
            glyph: String.fromCodePoint(notice.read ? 0xf0b92 : 0xf01ef),
            hue: "action",
            run: () => Notices.mark(notice.id, !notice.read)
        })], live ? [thing({
            key: "verb:notice:dismiss",
            name: "Let it go",
            glyph: String.fromCodePoint(0xf0a91),
            hue: "action",
            run: () => {
                const still = Notices.live[notice.id] ?? null;
                if (still)
                    still.dismiss();
            }
        })] : []);
    }

    // Made again only when the history changes, each with its age as of
    // when it's shown.
    readonly property var all: Notices.history.map(notice => made(notice))

    function aged(rows) {
        for (const row of rows)
            row.side = row.unread ? `unread · ${ago(row.at)}` : ago(row.at);
        return rows;
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ item: row.prepared, score: m.score, grade: m.grade, row: row });
        }
        // Newest first among equals, since a notice is wanted by when it came.
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = found.filter(f => f.grade === finest)
            .sort((a, b) => b.score - a.score || b.row.at - a.row.at);
        return { rows: aged(kept.map(f => f.row)), grade: finest };
    }

    function everything() {
        return aged(all.slice());
    }

    function counted() {
        return Notices.waiting.length > 0 ? `${Notices.waiting.length} missed`
            : all.length === 0 ? "none" : String(all.length);
    }

    function opened() {
        Notices.seen();
    }

    // The newest notice still on screen that offers something to do, whose
    // actions the collection's chord opens on (Launcher.qml's into), as its
    // card says.
    function leading() {
        const index = Notices.history.findIndex(notice => notice.id in Notices.live
            && Notices.live[notice.id].actions.length > 0);
        return index < 0 ? null : all[index];
    }

    // The notices on screen step aside while this is open, so each is in
    // one place: here, where the keys reach it.
    Binding {
        target: Notices
        property: "gathered"
        value: provider.launcher !== null && provider.launcher.open
            && provider.launcher.held.length > 0 && provider.launcher.held[0].row.from === provider
    }
}

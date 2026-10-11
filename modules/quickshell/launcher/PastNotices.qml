// The session's notifications, a collection: everything that arrived,
// newest first, from the shell's own history (Notices.qml), so nothing is
// asked of another program. A search with a match adds one row; the right
// arrow, or its row at home, shows them all, and its row at home says how
// many were missed until it's opened. Return goes to the program that sent
// one, its window if it has one open, or copies what it said when nothing
// sent it that can be gone to.
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
            // The clipboard takes text only while the launcher holds the
            // keys; going to a window waits for the launcher to be gone.
            now: !from.window && !from.entry,
            later: true,
            run: () => {
                if (!from.window && !from.entry) {
                    Quickshell.clipboardText = said;
                    return;
                }
                // Its window as things are now, since windows come and go.
                const now = provider.source(notice.app);
                if (now.window)
                    Niri.act({ FocusWindow: { id: now.window.id } });
                else if (now.entry)
                    Niri.spawn(provider.command(now.entry));
            }
        });
    }

    // Made again only when the history changes, each with its age as of
    // when it's shown.
    readonly property var all: Notices.history.map(notice => made(notice))

    function aged(rows) {
        for (const row of rows)
            row.side = ago(row.at);
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
        return Notices.missed > 0 ? `${Notices.missed} missed`
            : all.length === 0 ? "none" : String(all.length);
    }

    function opened() {
        Notices.seen();
    }
}

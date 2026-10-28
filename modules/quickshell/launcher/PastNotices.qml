// The session's notifications, a collection: everything that arrived,
// newest first, from the shell's own history (Notices.qml), so nothing is
// asked of another program. A search with a match adds one row; the right
// arrow, or its row at home, shows them all, and its row at home says how
// many were missed until it's opened. Return on one still open, on screen
// or resting off it once its time there was up (Notices.qml), does what its
// sender does when it's chosen, as clicking it does, which takes Joel to
// the very conversation or tab it's about; on one closed, it goes to
// the program that sent it, its window if it has one open, or copies what
// it said when nothing sent it that can be gone to. Whatever a sender is
// asked to do runs while the launcher holds the keys, since that's what
// lets it bring its window forward. The right arrow steps into what else
// can be done: copying what it said, marking it read or unread, and while
// it's still open, its own actions (Reply, Mark as read) and letting it go,
// so a notice is driven by the keys as well as by the pointer. Each says
// whether it's unread beside its age; acting on one reads it, and the
// selected one's whole message reads below the rows, wrapped, since one
// row cuts it short.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "notice"

    // Named Notifications, Joel's word, as the vocabulary says (Words.qml).
    collection: Words.collection("notices")

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

    // What its sender does when a notice still open is chosen, or null.
    function opens(notice) {
        const live = Notices.live[notice.id] ?? null;
        return live ? live.actions.find(action => action.identifier === "default") ?? null : null;
    }

    function made(notice) {
        const said = notice.body === "" ? notice.summary : `${notice.summary}\n${notice.body}`;
        const from = source(notice.app);
        const opened = opens(notice) !== null;
        return member({
            key: `notice:${notice.app}:${notice.summary}`,
            verb: opened || (!from.window && from.entry) ? "open" : from.window ? "switch" : "copy",
            name: notice.body === "" ? notice.summary
                : `${notice.summary} · ${notice.body.replace(/\s+/g, " ")}`,
            // The whole of what it said, summary over body.
            reads: said,
            // Its sender's, which the history keeps as a code point
            // (Notices.qml).
            glyph: String.fromCodePoint(notice.glyph),
            hue: notice.hue,
            note: notice.app,
            at: notice.at,
            unread: !notice.read,
            // The clipboard takes text only while the launcher holds the
            // keys, and a sender can raise its window only from a token
            // asked for while it does; going to a window niri focuses
            // waits for the launcher to be gone.
            now: opened || (!from.window && !from.entry),
            later: !opened,
            verbs: () => provider.verbs(notice, said),
            run: () => {
                Notices.mark(notice.id, true);
                const opening = provider.opens(notice);
                if (opening)
                    Notices.open(opening, notice.app);
                else if (!from.window && !from.entry)
                    Quickshell.clipboardText = said;
                else
                    provider.go(notice.app);
            }
        });
    }

    // What can be done to a notice: what its program offers while it's
    // open, then going to the program, copying what it said, and letting
    // it go. Quickshell
    // destroys a notification's actions when it closes or its program
    // replaces it, so each is found again by its identifier when it's run;
    // one whose notice has gone by then goes to its program instead.
    function verbs(notice, said) {
        const live = Notices.live[notice.id] ?? null;
        const offered = live ? live.actions.filter(action => action.identifier !== "default")
            .map(action => ({
            identifier: action.identifier,
            text: action.text
        })).map(action => thing({
            key: `verb:notice:${action.identifier}`,
            name: action.text,
            glyph: /reply/i.test(action.text) ? Theme.glyph.reply : Theme.glyph.offer,
            hue: "action",
            now: true,
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
        return offered.concat(provider.program(notice), [thing({
            key: "verb:notice:copy",
            name: Words.name("verb:notice:copy"),
            glyph: Words.glyph("verb:notice:copy"),
            hue: "action",
            now: true,
            run: () => {
                Notices.mark(notice.id, true);
                Quickshell.clipboardText = said;
            }
        }), thing({
            key: "verb:notice:mark",
            name: Words.name("verb:notice:mark", notice.read ? "read" : ""),
            glyph: Words.glyph("verb:notice:mark", notice.read ? "read" : ""),
            hue: "action",
            // It changes the notice rather than taking Joel anywhere, so
            // the launcher stays and shows it changed.
            stays: true,
            run: () => Notices.mark(notice.id, !notice.read)
        })], live ? [thing({
            key: "verb:notice:forget",
            name: Words.name("verb:notice:forget"),
            glyph: Words.glyph("verb:notice:forget"),
            hue: "action",
            stays: true,
            run: () => {
                const still = Notices.live[notice.id] ?? null;
                if (still)
                    still.dismiss();
            }
        })] : []);
    }

    // Going to its program while a window of it is open, whatever the
    // notice offers. The launcher puts Return's own act first
    // (Catalogue.js), and once the notice's actions are gone that act is
    // this switch. Opening a program that isn't open would be named Open,
    // which the launcher would take for opening the notice.
    function program(notice) {
        const from = source(notice.app);
        return !from.window ? [] : [thing({
            key: "verb:notice:goTo",
            name: Words.name("verb:notice:goTo", "",
                { program: from.entry ? from.entry.name : notice.app }),
            verb: Words.verb("verb:notice:goTo"),
            glyph: Words.glyph("verb:notice:goTo"),
            hue: "action",
            later: true,
            run: () => provider.go(notice.app)
        })];
    }

    // Made again only when the history changes, each with its age as of
    // when it's shown.
    readonly property var all: Notices.history.map(notice => made(notice))

    function aged(rows) {
        for (const row of rows)
            row.side = row.unread ? `unread · ${ago(row.at)}` : ago(row.at);
        return rows;
    }

    // Those a program sent, newest first, each as it is here, so a program's
    // own row (Applications.qml) holds them and Return on one does what it
    // does here. `sent` says whether a sender's name is the program.
    function sentBy(sent) {
        return aged(all.filter((row, index) => sent(Notices.history[index]?.app ?? "")));
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of poolFor(all, typed)) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ item: row.prepared, score: m.score, grade: m.grade, row: row });
        }
        matchedIn(all, typed, found.map(f => f.row));
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
            && !(notice.id in Notices.resting)
            && Notices.live[notice.id].actions.some(action => action.identifier !== "default"));
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

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
import "Notice.js" as Notice

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
        return { entry: DesktopEntries.heuristicLookup(app), window: Notices.senderWindow(app) };
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
        // With nothing to open or go to, Return steps in, as the right
        // arrow does, rather than copying: copying is never first (Joel,
        // 2026-10-28: "having the separate copy option at the top is very
        // confusing"). Its verbs and targets both make it a row holding
        // rows, which Return's own act never leads (Catalogue.js's leading).
        const nowhere = !opened && !from.window && !from.entry;
        const verbs = () => provider.verbs(notice, said);
        const row = member({
            key: `notice:${notice.app}:${notice.summary}`,
            verb: nowhere ? "more" : opened || !from.window ? "open" : "switch",
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
            now: opened,
            later: !opened && !nowhere,
            stays: nowhere,
            verbs: verbs,
            targets: nowhere ? verbs : null,
            run: () => {
                if (nowhere) {
                    provider.launcher?.enter(row);
                    return;
                }
                Notices.mark(notice.id, true);
                const opening = provider.opens(notice);
                if (opening)
                    Notices.open(opening, notice.app);
                else
                    provider.go(notice.app);
            }
        });
        return row;
    }

    // What can be done to a notice, the acts its card lists too, in the
    // same order (Notice.js). Quickshell destroys a notification's actions
    // when it closes or its program replaces it, so each is found again by
    // its identifier when it's run; one whose notice has gone by then goes
    // to its program instead. The launcher puts Return's own act first
    // (Catalogue.js): opening what it's about while it's open, else going
    // to the program.
    function verbs(notice, said) {
        const live = Notices.live[notice.id] ?? null;
        const from = source(notice.app);
        const acts = Notice.acts({
            live: live !== null,
            opens: live ? Notice.opens(live.actions) : false,
            offers: live ? Notice.offers(live.actions) : []
        }, { window: from.window !== null, stack: false });
        return acts.map(act => provider.act(notice, said, from, act));
    }

    // One of a notice's acts as a row of its own.
    function act(notice, said, from, act) {
        const action = identifier => {
            const still = Notices.live[notice.id] ?? null;
            return still ? still.actions.find(a => a.identifier === identifier) ?? null : null;
        };
        // As the vocabulary names it.
        const named = id => ({
            key: id,
            name: Words.name(id),
            glyph: Words.glyph(id),
            hue: "action"
        });
        switch (act.act) {
        case "open":
            return thing(Object.assign(named("verb:notice:open"), {
                verb: Words.verb("verb:notice:open"),
                now: true,
                run: () => {
                    Notices.mark(notice.id, true);
                    const opening = action("default");
                    if (opening)
                        Notices.open(opening, notice.app);
                    else
                        provider.go(notice.app);
                }
            }));
        case "offer":
            return thing({
                key: `verb:notice:${act.identifier}`,
                name: act.text,
                glyph: /reply/i.test(act.text) ? Theme.glyph.reply : Theme.glyph.offer,
                hue: "action",
                now: true,
                run: () => {
                    Notices.mark(notice.id, true);
                    const now = action(act.identifier);
                    if (now)
                        now.invoke();
                    else
                        provider.go(notice.app);
                }
            });
        case "goTo":
            // Going to its program while a window of it is open. Opening a
            // program that isn't open would be named Open, which the
            // launcher would take for opening the notice. Going there reads
            // it, as opening it does, and as Go To on its card does.
            return thing(Object.assign(named("verb:notice:goTo"), {
                verb: Words.verb("verb:notice:goTo"),
                name: Words.name("verb:notice:goTo", "",
                    { program: from.entry ? from.entry.name : notice.app }),
                later: true,
                run: () => {
                    Notices.mark(notice.id, true);
                    provider.go(notice.app);
                }
            }));
        case "read":
            // One switch, its name the same either way and what it is at
            // its side, as every setting's is. It changes the notice rather
            // than taking Joel anywhere, so the launcher stays and shows it
            // changed.
            return thing(Object.assign(named("verb:notice:read"), {
                glyph: Words.glyph("verb:notice:read", notice.read ? "read" : ""),
                side: notice.read ? "yes" : "no",
                stays: true,
                run: () => Notices.mark(notice.id, !notice.read)
            }));
        case "forget":
            return thing(Object.assign(named("verb:notice:forget"), {
                stays: true,
                run: () => {
                    const still = Notices.live[notice.id] ?? null;
                    if (still)
                        still.dismiss();
                }
            }));
        default:
            // Copying what it said, always last.
            return thing(Object.assign(named("verb:notice:copy"), {
                now: true,
                run: () => {
                    Notices.mark(notice.id, true);
                    Quickshell.clipboardText = said;
                }
            }));
        }
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

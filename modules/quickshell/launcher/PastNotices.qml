// The session's notifications, a collection: everything that arrived,
// newest first, from the shell's own history (Notices.qml), so nothing is
// asked of another program. It's no place of its own: For You holds the
// unread under Desktop and the read in History, its rows these, and
// "notices" asked from outside leads there (Now.qml's leadsTo); here,
// a search with a match adds one row, whose Return shows all it found
// (VKCF3Z, 2026-11-09). Return on one still open, on screen
// or resting off it once its time there was up (Notices.qml), does what its
// sender does when it's chosen, as clicking it does, which takes Joel to
// the very conversation or tab it's about; on one closed, it goes to
// the program that sent it, its window if it has one open, or copies what
// it said when nothing sent it that can be gone to. Whatever a sender is
// asked to do runs while the launcher holds the keys, since that's what
// lets it bring its window forward. What else can be done is a command
// on the : line, as on its card: copying what it said, marking it read or unread, and while
// it's still open, its own actions (Reply, Mark as read) and letting it go,
// so a notice is driven by the keys as well as by the pointer. Each says
// whether it's unread beside its age; acting on one reads it, and the
// selected one's whole message reads below the rows, wrapped, since one
// row cuts it short.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../NoticeBody.js" as NoticeBody
import "Notice.js" as Notice
import "Ago.js" as Ago
import "Order.js" as Order

Provider {
    id: provider

    what: "notice"

    // Named Notifications, Joel's word, as the vocabulary says (Words.qml),
    // and never found by it, since it's no place of its own.
    collection: Words.collection("notices", { placed: false })

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
        // With nothing to open or go to, Return steps in rather than
        // copying: copying is never first (Joel,
        // 2026-10-28: "having the separate copy option at the top is very
        // confusing"). Its verbs and targets both make it a row holding
        // rows, which Return's own act never leads (Catalogue.js's leading).
        const nowhere = !opened && !from.window && !from.entry;
        const verbs = () => provider.verbs(notice, said);
        // Named by its title, the same in every list, its note the line of
        // its body that holds what was typed (VKCF3Z, 2026-11-08), as
        // search says it each time; the whole of what it said is what a
        // search looks through.
        const named = Notice.named(notice.summary, notice.body, [], provider.holds);
        let whole = null;
        const row = member({
            thing: true,
            key: `notice:${notice.app}:${notice.summary}`,
            verb: nowhere ? "more" : opened || !from.window ? "open" : "go to",
            name: named.name,
            count: named.note,
            // Its note gives way by whole words, never an ellipsis (Choice.qml).
            countWords: true,
            // Its body, which its note is a line of as each search says it
            // (lined).
            wrote: notice.body,
            // Stepped into, the head says its title alone, its words being
            // the drawer's (VKCF3Z, 2026-11-08).
            title: notice.summary,
            // The whole of what it said, each part once: the summary as
            // the drawer's title, then the body, of which the row says only
            // a line (VKCF3Z, 2026-11-02). With no body it would only repeat
            // the name, so there's no drawer (Showing.js's adds). Over them,
            // who sent it and what kind it is. Its words read in the face its card
            // reads them in, so they don't change face between the two
            // (VKCF3Z, 2026-11-03). Its sender is a name from the world,
            // so the eyebrow is set as it's written (VKCF3Z, 2026-11-07).
            // Its words styled as its card draws them where they carry
            // markup or a link, and plain otherwise, so a long log keeps its
            // own lines to scroll by; its own picture while it has one; and
            // how it was sent and how it left, as facts. Made as it's shown,
            // since only the selected row's ever is: made with every row, its
            // eyebrow and facts were most of the 150 to 300 ms the history's
            // 862 rows took to make on the first key that searched them
            // (2026-11-10).
            drawer: () => ({ eyebrow: provider.fromAndKind(notice), eyebrowNames: true,
                title: notice.summary,
                body: provider.marked(notice) ? notice.styled : notice.body,
                styled: provider.marked(notice),
                // Its picture as its card draws it, a file's URL or one held
                // in memory while it lives, since the drawer draws either
                // (Pictures.js's source; VKCF3Z, 210afa2f).
                prose: true, picture: Notices.pictureOf(notice),
                facts: provider.facts(notice) }),
            // Its sender's, which the history keeps as a code point
            // (Notices.qml), muted for one sent at low urgency, as its card
            // draws it.
            glyph: String.fromCodePoint(notice.glyph),
            hue: notice.low ? "muted" : notice.hue,
            note: notice.app,
            // Said once, in its drawer's eyebrow, while the drawer is out,
            // so the cursor row's side is its age alone, as a Slack
            // message's is (VKCF3Z, 2026-11-09).
            noteInDrawer: true,
            // Who sent it, which ends a paragraph of rows (Launcher.qml's
            // runOf), so dip takes away what one sender sent.
            sender: notice.app,
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
            operands: () => provider.operands(notice, said),
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
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => whole ?? (whole = Fuzzy.kept(said.replace(/\s+/g, " ")))
        });
        // Where Ctrl+] goes from it, as Vim follows a tag: its sender's own
        // row, among Programs (Applications.qml), or nowhere for a sender
        // no program is. Looked up as it's asked, since the history's rows
        // are made before the programs' entries are read.
        Object.defineProperty(row, "follows", {
            enumerable: true,
            get: () => {
                const entry = source(notice.app).entry;
                return entry ? `program:${entry.id}` : "";
            }
        });
        return row;
    }

    // Whether what a notice said carries markup or a link of its own, which
    // its drawer draws styled.
    function marked(notice) {
        return typeof notice.styled === "string" && /<(?:a|b|i|u)[ >]/.test(notice.styled);
    }

    // What its drawer says of a notice beyond its words: when it arrived,
    // how it left the screen and when, or that it's still open; and the
    // urgency its sender asked (Notice.js). What the spec gives besides is
    // left out: a sound, which the desktop never plays for a sender; the
    // icons of its actions, since its acts wear the desktop's marks; and
    // how long it asked to stay, which its leaving already tells.
    function facts(notice) {
        const how = notice.left ?? (notice.id in Notices.resting ? "resting"
            : notice.id in Notices.live ? "shown" : "");
        return Notice.facts(
            { label: Words.name("notices:arrived"), at: clock(notice.at) },
            { label: Words.name("notices:left"),
                said: how === "" ? "" : Words.of("notices:left", how).said ?? "",
                at: notice.leftAt ? clock(notice.leftAt) : "" },
            // Urgency only where it isn't normal, the urgency nearly
            // every notice is sent at (VKCF3Z, 2026-11-09).
            { label: Words.name("notices:urgency"),
                said: notice.urgency && notice.urgency !== "normal"
                    ? Words.of("notices:urgency", notice.urgency).said ?? "" : "" });
    }

    // A time as its drawer writes it: alone today, with its day before.
    function clock(at) {
        if (!at)
            return "";
        const when = new Date(at);
        const today = new Date().toDateString() === when.toDateString();
        return Qt.formatDateTime(when, today ? "HH:mm" : "ddd d MMM, HH:mm");
    }

    // Who sent a notice and what kind it is, as its drawer's eyebrow says
    // it: the sender by the name its card signs it with (Notices.qml), so
    // the two never differ (VKCF3Z, 2026-11-03), and the kind its category
    // names, never a time, which Arrived says (VKCF3Z, 2026-11-08). One the
    // card signs as the desktop's, which For You's path already names, is
    // from the program it came through, "from notify-send", and one whose
    // category names no kind is a notice (VKCF3Z, 2026-11-09).
    function fromAndKind(notice) {
        const said = Words.of("notices").said;
        const kinds = {};
        for (const [type, state] of Object.entries(Words.of("notices:kind").states ?? {}))
            kinds[type] = state.said;
        const signed = notice.app === "" ? "" : Notices.sender(notice.app).name;
        const who = signed === Words.name("for-you:source:desktop") ? notice.app : signed;
        return Notice.eyebrow(who, Notice.kind(notice.category,
            { kinds: kinds, failed: said.failed, bounced: said.bounced }) || said.notice,
            said.from);
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
            offers: live ? Notice.offers(live.actions) : [],
            links: notice.links ?? []
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
        const named = id => Object.assign({
            key: id,
            name: Words.name(id),
            glyph: Words.glyph(id),
            hue: "action"
        }, Words.ending(id));
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
        case "link":
            // A link in what it said, opened in the program the desktop
            // opens it with, its host at its side so two read apart.
            return thing(Object.assign(named("verb:notice:link"), {
                key: `verb:notice:link:${act.href}`,
                verb: Words.verb("verb:notice:link"),
                side: NoticeBody.host(act.href),
                later: true,
                run: () => {
                    Notices.mark(notice.id, true);
                    Niri.spawn(["xdg-open", act.href]);
                }
            }));
        case "goTo":
            // Going to its program while a window of it is open. Opening a
            // program that isn't open would be named Open, which the
            // launcher would take for opening the notice. Going there reads
            // it, as opening it does, and as Go To on its card does.
            return thing(Object.assign(named("verb:notice:goTo"), {
                verb: Words.verb("verb:notice:goTo"),
                side: from.entry ? from.entry.name : notice.app,
                later: true,
                run: () => {
                    Notices.mark(notice.id, true);
                    provider.go(notice.app);
                }
            }));
        case "read":
            // Named for what it will do, Mark It Read or Mark It Unread,
            // with nothing at its side, since a state there would read as
            // the act's (VKCF3Z, 2026-11-08). It changes the notice rather
            // than taking Joel anywhere, so the launcher stays and shows it
            // changed.
            return thing(Object.assign(named("verb:notice:read"), {
                name: Words.name("verb:notice:read", notice.read ? "read" : ""),
                glyph: Words.glyph("verb:notice:read", notice.read ? "read" : ""),
                stays: true,
                run: () => Notices.mark(notice.id, !notice.read)
            }));
        case "forget":
            return thing(Object.assign(named("verb:notice:forget"), { stays: true },
                forgetting(notice)));
        default:
            // Copying what it said, always last.
            return thing(Object.assign(named("verb:notice:copy"), { now: true },
                copying(notice, said)));
        }
    }

    // What d and y take of a notice: done with it while it's live, and
    // copying what it said, as plain fields rather than rows made
    // searchable, since d or y over a history of hundreds asks every row
    // (Launcher.qml's actionsOf).
    function operands(notice, said) {
        const copy = Object.assign({ key: "verb:notice:copy",
            verb: Words.verb("verb:notice:copy") }, copying(notice, said));
        const still = Notices.live[notice.id] ?? null;
        if (!still || parting.includes(notice.id))
            return [copy];
        // d reads it where its sender offers Read, and only forgets it
        // where not, so it says which, as the held notices' d does.
        const read = Notice.readOffer(still.actions) !== null
            ? { ends: Words.field.operators?.read ?? "" } : {};
        return [Object.assign({ key: "verb:notice:forget" }, Words.ending("verb:notice:forget"),
            read, done(notice)), copy];
    }

    // d's done, as the held notices' is (HeldNotices.qml's finish): read at
    // once, at its sender too where it offers Read, and taken away as the
    // launcher closes, a dismissed notice being gone for good, so u brings
    // it back until then, its read state staying, as the spec's notices do.
    function done(notice) {
        return {
            run: () => {
                const still = Notices.live[notice.id] ?? null;
                if (!still)
                    return;
                Notices.mark(notice.id, true);
                Notice.readOffer(still.actions)?.invoke();
                provider.parting = provider.parting.concat([notice.id]);
            },
            undo: () => {
                provider.parting = provider.parting.filter(id => id !== notice.id);
            }
        };
    }

    // The notices d is done with, by id, taken away as the launcher closes.
    property var parting: []

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                return;
            // A sender's Read closes a notice it doesn't keep, so only one
            // still open is taken away.
            for (const id of provider.parting)
                Notices.live[id]?.dismiss();
            provider.parting = [];
        }
    }

    function forgetting(notice) {
        return {
            run: () => {
                const still = Notices.live[notice.id] ?? null;
                if (still)
                    still.dismiss();
            }
        };
    }

    function copying(notice, said) {
        return {
            copies: said,
            run: () => {
                Notices.mark(notice.id, true);
                Quickshell.clipboardText = said;
            }
        };
    }

    // Each notice's row, each with its age as of when it's shown, made
    // again only once something reads them after the history changed, so
    // the thousand kept (Notices.qml, VKCF3Z 2026-11-09) cost nothing while
    // notices come and go with the launcher closed. Even then only a
    // notice that changed is made again, the history keeping every other
    // as it was (Notices.qml): every one was made anew at any change, 488
    // as Joel stepped into For You's Desktop (2026-11-10). A row is kept
    // by what Return on it does too, which follows whether its sender's
    // notice is still open and whether a window of it is.
    property var rowsMade: []
    property bool rowsStale: true
    readonly property var rowsKept: keeper()
    function all() {
        if (rowsStale) {
            keepingBegun(rowsKept);
            rowsMade = Notices.history.map(notice => {
                const from = source(notice.app);
                const goes = `${notice.id}:${opens(notice) !== null}`
                    + `:${!!from.window}:${!!from.entry}`;
                return keptOr(rowsKept, goes, notice, () => made(notice));
            });
            keptOnly(rowsKept);
            rowsStale = false;
        }
        return rowsMade;
    }
    // Said, so a notice read, forgotten or arriving shows while the launcher
    // holds the list, rather than once something else makes it again.
    Connections {
        target: Notices
        function onHistoryChanged() {
            provider.rowsStale = true;
            provider.changed();
        }
    }

    // The same rows, without saying how long ago each came.
    function looksThrough() {
        return all();
    }

    // Each row's age at its side, after how it stands where `standing`
    // says it, as items, so the side gives way by whole items, its age
    // first, never an ellipsis (VKCF3Z, 2026-11-09).
    function aged(rows, standing) {
        const now = Date.now();
        for (const row of rows) {
            row.sideItems = [{ said: standing(row), keep: 2 },
                { said: Ago.ago(row.at, now), keep: 1 }].filter(item => item.said !== "");
            row.side = row.sideItems.map(item => item.said).join(" \u00b7 ");
        }
        return rows;
    }

    // How a notice stands, in its words: "unread" or "read".
    function stood(read) {
        return (Words.of("notices").said ?? {})[read ? "read" : "unread"] ?? "";
    }

    // Where read and unread are listed together, the unread say so.
    function unreadOnes(row) {
        return row.unread ? stood(false) : "";
    }

    // Those a program sent, newest first, each as it is here, so a program's
    // own row (Applications.qml) holds them and Return on one does what it
    // does here. `sent` says whether a sender's name is the program.
    function sentBy(sent) {
        return lined(aged(all().filter((row, index) => sent(Notices.history[index]?.app ?? "")),
            unreadOnes), []);
    }

    // The notices of `ids`, newest first, each as it is here, so For You's
    // Desktop row (Now.qml) holds the unread ones, and its History the
    // read, as they're listed here. Every one Desktop holds is unread, which
    // its head says once, so a row says only that it's read, as one read
    // while Joel looks stays where it was (VKCF3Z, 2026-11-09).
    function rowsOf(ids) {
        return lined(aged(all().filter((row, index) => ids.includes(Notices.history[index]?.id)),
            row => row.unread ? "" : stood(true)), []);
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
        for (const row of poolFor(all(), typed)) {
            if ((row.also.mask & need) !== need)
                continue;
            const byName = Fuzzy.match(row.prepared, words, false);
            const m = byName.score >= 0 ? byName : Fuzzy.match(row.also, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        matchedIn(all(), typed, found.map(f => f.row));
        // Newest first among equals, since a notice is wanted by when it came.
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score || b.row.at - a.row.at);
        return { rows: lined(aged(kept.map(f => f.row), unreadOnes), words), grade: finest };
    }

    function everything() {
        return lined(aged(all().slice(), unreadOnes), []);
    }

    function counted() {
        return Notices.waiting.length > 0 ? `${Notices.waiting.length} missed`
            : Notices.history.length === 0 ? "none" : countedAs(Notices.history.length);
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

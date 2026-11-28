// What each program had open lately and has open now for Programs
// (Applications.qml): the sources of its Recent, read through Recents,
// which tells the provider to draw a program held from the top again,
// and its Recent and Tabs rows as its step lists them. Applications.qml
// keeps every name another file reads of this, Screenshots' recents
// among them, and makes this once.
pragma ComponentBehavior: Bound

import Quickshell
import qs
import QtQuick
import "Program.js" as Program
import "Recent.js" as Recent

Scope {
    id: recentsPart

    // Applications, whose programs' Recent this is, which makes their rows.
    required property var provider

    // The sources of each program's Recent: a page's own list, one with a
    // `name` as Chrome's downloads, is its page's to read, not Recent's.
    readonly property var recentSources: (provider.settings.recents ?? [])
        .filter(each => !each.name)

    // What each program had open lately, from the files they write.
    readonly property Recents recents: Recents {
        sources: recentsPart.recentSources
        showing: recentsPart.provider.launcher?.open ?? false
    }

    // A list read after a program was stepped into, or changing while it's
    // shown, draws its Recent again, as Media does with what's playing.
    Connections {
        target: recentsPart.recents

        function onLatelyChanged() {
            recentsPart.provider.redrawHeld();
        }

        function onSharedChanged() {
            recentsPart.provider.redrawHeld();
        }

        function onOpenNowChanged() {
            recentsPart.provider.redrawHeld();
        }

        function onClosedChanged() {
            recentsPart.provider.redrawHeld();
        }
    }

    // What another provider gives a program (recentOf, programActs)
    // changing while it's shown draws it again too: the launcher remakes a
    // step only for the provider its row came from, and a program's row
    // is this one's.
    Variants {
        model: (recentsPart.provider.launcher?.sources ?? [])
            .filter(other => other !== recentsPart.provider
            && (typeof other.recentOf === "function" || typeof other.programActs === "function"))

        Connections {
            required property var modelData

            target: modelData
            // A giver may have no changed to say (holdsSafely holds nothing
            // then), and that's no fault to warn of.
            ignoreUnknownSignals: true

            function onChanged() {
                recentsPart.provider.redrawHeld();
            }
        }
    }

    // The pages a closed browser's saved session holds, each as a line of
    // the sites it visited (`line`, its address, a tab and its title) and
    // when it was last looked at (`at`, in milliseconds), newest first,
    // each once.
    function savedSites(entry) {
        const seen = new Set();
        return recentsPart.recents.openIn(entry.id).map(line => line.split("\t"))
            .filter(tab => (tab[4] ?? "") !== "")
            .sort((a, b) => Number(b[3]) - Number(a[3]))
            .filter(tab => !seen.has(tab[4]) && seen.add(tab[4]))
            .map(tab => ({ line: `${tab[4]}\t${tab[5] ?? ""}`, at: Number(tab[3]) || undefined }));
    }

    // Recent: what it had open lately and has open no more (Recents.qml),
    // laid out as every Recent is (Recent.js): newest first, each once,
    // never what it has open now, which its step lists above it
    // (`openRows`), nine at most. Each is a folder or a file by its name,
    // beside the folder it's in.
    // Return opens it through the program as its own list's settings say:
    // `open` while a window of it is open, through the process behind the
    // window used last ({pid}), and `start` while none is. A file from the
    // desktop's list programs share opens in the program, by its own
    // command given the file, when its entry takes files; one given to a
    // program that opens none, such as a chat's attachment, opens as the
    // desktop opens it, which is what the list itself says of those.
    // A site Chrome visited, its source's `site` kind, is the page's title
    // beside its host, opened by the program's own command given the page
    // when its source names no command of its own. A closed browser's
    // saved session is the sites it visited last, before its history. A
    // file source may say what's open now, as mpv's says the file each mpv
    // plays, and one saying no more than that keeps the list programs
    // share for the rest. Any other provider may give a program's Recent
    // its own things after these, each its one row, by
    // `recentOf(program)`, newest first, as Docs' documents are given
    // (Documents.qml).
    //
    // It stands wherever the program has a source of what it had, its own
    // list's settings or a provider that says it keeps some
    // (`keepsRecentOf(program)`), saying "none" while it holds nothing, and
    // an ellipsis until a source read only as it's stepped into first
    // answers, so it never comes late at the bottom of a step already
    // drawn. Its side says how many, and its drawer their names, as a
    // place's does (VKCF3Z, dafc2199).
    function recent(entry) {
        if (!recentsPart.keepsRecent(entry))
            return [];
        recentsPart.recents.ask(entry.id);
        const words = Words.of("program:recent");
        const kept = recentsPart.kept(entry);
        // Named from the rows themselves, kept while each reads the same,
        // and only those it holds now are kept on.
        const keeping = recentsPart.recentKeeper(entry);
        provider.keepingBegun(keeping);
        const names = kept.map(one => recentsPart.made(entry, one).name);
        provider.keptOnly(keeping);
        const source = recentsPart.sourceOf(entry);
        const waiting = source?.asked === true && !(entry.id in recentsPart.recents.lately);
        const stands = Recent.said(names, waiting, words.said);
        return [provider.said("program:recent", "", {
            hue: "program",
            side: stands.side,
            heldNames: stands.held,
            ask: "which?",
            nested: true,
            empty: waiting ? "\u2026" : words.empty,
            // Its verbs are the same rows as its targets, as People's
            // Recent's are, so it steps in inside a program whose step
            // lists a thing's verbs, as Tuple's does, holding only acts.
            verbs: () => recentsPart.kept(entry).map(one => recentsPart.made(entry, one)),
            targets: () => recentsPart.kept(entry).map(one => recentsPart.made(entry, one))
        })];
    }

    // Its list's settings, or null where it has none.
    function sourceOf(entry) {
        return recentSources.find(each => each.app === entry.id) ?? null;
    }

    // Whether it has a source of what it had: its own list's settings, the
    // list programs share naming it, or another provider keeping some or
    // giving some now.
    // `given`, what other providers give it, is asked here when not passed,
    // as it is by a note that reads it once for all it counts.
    function keepsRecent(entry, given) {
        return recentsPart.sourceOf(entry) !== null || recentsPart.recents.of(entry.id).length > 0
            || provider.hooking.keepsRecentOf.some(other => other.keepsRecentOf(entry.id))
            || (given ?? provider.givenRecent(entry.id)).length > 0;
    }

    // What its Recent holds, as Recent.js lays them out, each { key, at,
    // row } with `row` what its row is made from (`made`), so counting
    // them makes no rows: a closed browser's saved pages, what its list
    // keeps and what other providers give, merged by when each was last
    // used where its source says (a dated source, recents.nix), none of
    // what it has open now.
    // `asked` and `opens`, what other providers give it and list as open in
    // it, are asked here when not passed, as keepsRecent's `given` is.
    function kept(entry, asked, opens) {
        const source = recentsPart.sourceOf(entry);
        const site = source?.kind === "site";
        const kind = site ? "site" : source?.list && source.kind === "folder" ? "folder" : "file";
        const keyed = line => {
            const tab = line.indexOf("\t");
            return provider.keyOf(kind, site && tab >= 0 ? line.slice(0, tab) : line);
        };
        const saved = site && !provider.isRunning(entry) ? recentsPart.savedSites(entry)
            .map(one => ({ key: keyed(one.line), at: one.at, row: { line: one.line } })) : [];
        const listed = recentsPart.recents.of(entry.id).map(line => ({
            key: keyed(line),
            at: recentsPart.recents.atOf(entry.id, line),
            row: { line: line }
        }));
        const given = (asked ?? provider.givenRecent(entry.id))
            .map(row => ({ key: row.key, at: row.at, row: { made: row } }));
        return Recent.listed([saved, listed, given], recentsPart.openKeys(entry, opens),
            recentsPart.recents.most);
    }

    // One of what its Recent holds as its row, kept while its line is, so
    // a step held, drawn again at every change of Programs (redrawHeld),
    // makes none of its Recent again: Chrome's step on Joel's shell was
    // made again whole as its downloads were read (2026-11-10, 136 ms).
    function made(entry, one) {
        if (one.row.made)
            return one.row.made;
        const source = recentsPart.sourceOf(entry);
        const line = one.row.line;
        // Kept while it says the same when, too, so a file used again, or
        // an hour on, never keeps saying "just now".
        const when = Number.isFinite(one.at) ? provider.usedWhen(one.at) : "";
        return provider.keptOr(recentsPart.recentKeeper(entry), line, `${line}\t${when}`,
            () => source?.kind === "site" ? provider.visited(entry, line)
                : provider.lately(entry, source?.list ? source : null, line, one.at));
    }

    // Each program's keeper of its Recent's rows, by its desktop entry's
    // id, made anew when its entry or its list's settings are, as when
    // the desktop entries are read again, so no kept row holds one gone.
    // A plain object, so keeping notifies nothing.
    readonly property var recentKept: ({})
    function recentKeeper(entry) {
        const source = recentsPart.sourceOf(entry);
        const had = recentKept[entry.id];
        if (had && had.entry === entry && had.source === source)
            return had.keeping;
        const keeping = provider.keeper();
        recentKept[entry.id] = { entry: entry, source: source, keeping: keeping };
        return keeping;
    }

    // What it has open now, where its source says (`opened`), each the
    // thing it is, listed in its step before its Recent: every page a
    // browser has open while it runs (openSites), each file mpv plays
    // (openFiles). A browser's tabs are open only while it runs: one its
    // saved session still holds once it's closed is a site it visited,
    // in its Recent, never "open" beside a Return that starts it (VKCF3Z,
    // 2026-11-07). Not named opened, the hook the launcher calls as
    // Programs is stepped into (Provider.qml).
    function openRows(entry) {
        const source = recentsPart.sourceOf(entry);
        if (!opensNow(entry, source))
            return [];
        return source.kind === "site" ? provider.openSites(entry, source)
            : provider.openFiles(entry, source);
    }

    // Whether its source says what it has open now, asked of it once it
    // does, which a browser's does only while it runs.
    function opensNow(entry, source) {
        if (!recentsPart.recents.tellsOpen(source) || !provider.isRunning(entry))
            return false;
        recentsPart.recents.ask(entry.id);
        return true;
    }

    // What a browser has open, as one row of its own, Tabs, holding its
    // tabs by their pages, so they read apart from its windows, acts and settings
    // (Joel, 2026-11-11: "it also needs to be more obvious what part of
    // the program menu is tabs versus other stuff"). Typing in its step
    // still finds a page, as Recent's are found. What another program has
    // open, as mpv's files, stays as it was, among its acts. Its pages are
    // counted without making them, and made only once Tabs is stepped into
    // or searched, once for each making of its acts.
    function openGroup(entry) {
        const source = recentsPart.sourceOf(entry);
        if (source?.kind !== "site")
            return recentsPart.openRows(entry);
        const count = opensNow(entry, source)
            ? Program.pagesOpen(recentsPart.recents.openIn(entry.id)) : 0;
        if (count === 0)
            return [];
        let made = null;
        const open = () => {
            made = made ?? provider.openSites(entry, source);
            return made;
        };
        const words = Words.of("program:open-tabs").said ?? {};
        return [provider.said("program:open-tabs", "", {
            hue: "program",
            side: ((count === 1 ? words.one : words.many) ?? "")
                .replace("{count}", String(count)),
            ask: "which?",
            nested: true,
            verbs: open,
            targets: open
        })];
    }

    // The keys of what it has open now, read from what its source said
    // without making a row, and of what other providers list as open in
    // it (givenOpen), so its Recent never lists one again.
    function openKeys(entry, given) {
        return ownOpenKeys(entry).concat(given ?? provider.givenOpen(entry.id));
    }

    function ownOpenKeys(entry) {
        const source = recentsPart.sourceOf(entry);
        if (!recentsPart.recents.tellsOpen(source) || !provider.isRunning(entry))
            return [];
        const lines = recentsPart.recents.openIn(entry.id).map(line => line.split("\t"));
        return source.kind === "site"
            ? lines.filter(tab => (tab[4] ?? "") !== "").map(tab => provider.keyOf("site", tab[4]))
            : lines.filter(([who, path]) => (path ?? "") !== ""
                && provider.fileWindow(entry, who) !== null)
                .map(([, path]) => provider.keyOf("file", path));
    }
}

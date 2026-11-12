// What each program had open lately, for its Recent inside the launcher
// (Applications.qml): the paths it keeps, in the order it keeps them, as
// the programs daemon holds them (modules/programs), which runs each
// list's command as its file changes, and only while the launcher is
// open (`showing`), so no list's command runs while nothing shows it,
// however often a program writes (the shell's design, RSCH-20261029-0E8050).
// The shell runs none of them.
// Its `sources`, each { app, watch, list }, are recents.nix's, which the
// daemon is given too: `app` the program's desktop entry, `watch` the
// file the program writes as it goes, and `list` the command that prints
// its paths. A source with no `app` is a list many programs share, the
// desktop's files opened lately (open-with's recent-files): a program and
// a path a line, newest first, and each program is matched to its
// desktop entry by its id or its window's class, as heuristicLookup does,
// or else by its name, which is how GTK names a program there, or else by
// its command, which is how recent-files names one (zathura's entry is
// org.pwmt.zathura), taking the entry the launcher shows rather than a
// hidden one running the same command (zathura's plugins have one each).
// At most nine are kept a program, so 1 to 9 pick one. A source that's
// `asked` is read only as its program is stepped into, once each time the
// launcher opens (`ask`), for a program such as Chrome whose history
// changes with every page. A source that also says what its program has
// open now (`opened`, Chrome's tabs from its session, or the file each
// mpv plays) is asked that then too, and never otherwise. One saying no
// more than that, as mpv's does, has no list of its own, so its program's
// paths stay the shared list's. A source that's `dated` says when each
// was last used, kept apart from its line (`atOf`), so Recent places it
// among the program's other sources by when. Each is kept as the line its
// command printed (Recents.js), as Applications.qml reads them.
pragma ComponentBehavior: Bound

import Quickshell
import QtQuick
import qs
import "Recents.js" as Lines

Scope {
    id: recents

    property var sources: []
    // Whether the launcher is open, which the lists are watched while.
    property bool showing: false

    // Each program's paths by its desktop entry's id: from its own source,
    // and from the list programs share, matched again as the desktop
    // entries are read, since they may come after the list.
    property var lately: ({})
    property var sharedLines: []
    readonly property var shared: Lines.sharedFrom(sharedLines,
        program => entryOf(program, DesktopEntries.applications.values), most)

    // When each line a dated source printed was last used, in milliseconds,
    // by program and then by the line as `of` gives it, and the shared
    // list's by its line as it's kept without its time.
    property var latelyAt: ({})
    property var sharedTimes: ({})
    readonly property var sharedAt: Lines.sharedAtFrom(sharedLines,
        program => entryOf(program, DesktopEntries.applications.values), sharedTimes)

    readonly property int most: 9

    // What each program has open now, by its desktop entry's id, as its
    // source's `opened` command printed it as it was last stepped into.
    property var openNow: ({})

    // The pages closed from the launcher since it opened, by program, each
    // staying where it was, said closed, until the next opening.
    property var closed: ({})

    // The programs whose asked sources were read since the launcher opened.
    property var asked: ({})

    // The keys of the program stepped into last, watched by name, so the
    // daemon reads its asked list and what it has open.
    property var stepped: []

    // Each list read as the shell starts is watched until it's first
    // heard, so the launcher's first opening finds it held, as it did
    // when the shell read each list once at start; then only while the
    // launcher is open.
    readonly property var started: sources.filter(source => !source.asked
        && (source.list ?? []).length > 0).map(source => `recent:${source.app ?? "shared"}`)
    property var heardOnce: ({})
    readonly property bool primed: started.every(key => key in heardOnce)

    // Reads the asked sources of a program stepped into, and what it has
    // open, once an opening.
    function ask(app) {
        if (recents.asked[app])
            return;
        recents.asked = Object.assign({}, recents.asked, { [app]: true });
        const mine = recents.sources.filter(source => source.app === app);
        const keys = [];
        if (mine.some(source => source.asked && (source.list ?? []).length > 0))
            keys.push(`recent:${app}`);
        if (mine.some(tellsOpen))
            keys.push(`opened:${app}`);
        if (keys.length > 0)
            recents.stepped = keys;
    }

    // Whether a source says what its program has open now: by a command
    // (`opened`), or, as Chrome's does, by the session the programs
    // daemon reads itself (`session`).
    function tellsOpen(source) {
        return (source?.opened ?? []).length > 0 || typeof source?.session?.profile === "string";
    }

    // What a program has open now, a line each, or none.
    function openIn(app) {
        return openNow[app] ?? [];
    }

    // The launcher closed, so the next opening reads them again, and no
    // program is stepped into until one is.
    function forget() {
        recents.asked = {};
        recents.closed = {};
        recents.stepped = [];
    }

    // A page a program had open, closed from the launcher.
    function closing(app, address) {
        recents.closed = Object.assign({}, recents.closed,
            { [app]: (recents.closed[app] ?? []).concat([address]) });
    }

    function of(app) {
        return lately[app] ?? shared[app] ?? [];
    }

    // When a program last used what one of its lines (`of`) names, in
    // milliseconds since 1970, or undefined where its source didn't say.
    function atOf(app, line) {
        return (app in lately ? latelyAt[app] : sharedAt[app])?.[line];
    }

    // The desktop entry a program on the shared list is, by its id, its
    // name, a keyword, as an entry given a short name keeps the long one
    // the list knows it by among its keywords (GIMP's, modules/gimp), or
    // its command.
    function entryOf(program, entries) {
        const entry = DesktopEntries.heuristicLookup(program)
            ?? entries.find(each => each.name === program)
            ?? entries.find(each => Array.from(each.keywords ?? []).includes(program))
            ?? entries.find(each => !each.noDisplay
                && (each.command[0] ?? "").split("/").pop() === program);
        return entry?.id ?? "";
    }

    // The lines and times of the records a change touched, each set only
    // where it differs, so a change to one program's list draws no other's.
    function took(keys) {
        let lists = recents.lately;
        let times = recents.latelyAt;
        let opened = recents.openNow;
        const once = Object.assign({}, recents.heardOnce);
        for (const key of keys)
            once[key] = true;
        if (Object.keys(once).length !== Object.keys(recents.heardOnce).length)
            recents.heardOnce = once;
        for (const key of keys) {
            const record = daemon.held[key];
            const colon = key.indexOf(":");
            const app = key.slice(colon + 1);
            if (key.slice(0, colon) === "opened") {
                const now = record ? Lines.openedLines(record) : [];
                opened = Object.assign({}, opened, { [app]: now });
                continue;
            }
            const lines = record ? Lines.recentLines(record) : [];
            const when = {};
            for (const one of lines) {
                if (one.at !== undefined && !(one.line in when))
                    when[one.line] = one.at;
            }
            if (app === "shared") {
                // Heard again as it was, as the daemon says every list
                // again as the launcher opens, it's left be: set anew,
                // everything reading it made its rows again, every
                // program's acts and every gallery twice an opening on
                // Joel's shell (2026-11-09).
                const said = lines.map(one => one.line);
                if (JSON.stringify(when) !== JSON.stringify(recents.sharedTimes))
                    recents.sharedTimes = when;
                if (JSON.stringify(said) !== JSON.stringify(recents.sharedLines))
                    recents.sharedLines = said;
                continue;
            }
            lists = Object.assign({}, lists,
                { [app]: lines.map(one => one.line).slice(0, recents.most) });
            times = Object.assign({}, times, { [app]: when });
        }
        if (JSON.stringify(times) !== JSON.stringify(recents.latelyAt))
            recents.latelyAt = times;
        if (JSON.stringify(lists) !== JSON.stringify(recents.lately))
            recents.lately = lists;
        if (JSON.stringify(opened) !== JSON.stringify(recents.openNow))
            recents.openNow = opened;
    }

    // Every program's list while the launcher is open, or until each is
    // first heard, and the program stepped into last by name.
    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/programs.sock`
        kind: "recent"
        live: recents.showing || !recents.primed
        ahead: recents.stepped

        onHeard: (changed, gone) => recents.took(changed.concat(gone))
    }
}

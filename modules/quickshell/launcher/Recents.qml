// What each program had open lately, for its Recent inside the launcher
// (Applications.qml): the paths it keeps, in the order it keeps them, read
// once at start and again each time the file it keeps them in changes,
// and never on a timer. A change while the launcher is closed (`showing`)
// is only noted, and read as it next opens, so no list's command runs
// while nothing shows it, however often a program writes (the shell's
// design, RSCH-20261029-0E8050).
// Its `sources`, each { app, watch, list }: `app` the program's desktop
// entry, `watch` the file the program writes as it goes, and `list` the
// command that prints its paths, one a line. A source with no `app` is a
// list many programs share, the desktop's files opened lately
// (open-with's recent-files): its command prints a program and a path a
// line, a tab between, newest first, and each program is matched to its
// desktop entry by its id or its window's class, as heuristicLookup does,
// or else by its name, which is how GTK names a program there, or else by
// its command, which is how recent-files names one (zathura's entry is
// org.pwmt.zathura), taking the entry the launcher shows rather than a
// hidden one running the same command (zathura's plugins have one each).
// At most nine are kept a program, so 1 to 9 pick one. A source that's
// `asked` watches nothing and is read only as its program is stepped into,
// once each time the launcher opens (`ask`), for a program such as Chrome
// whose history changes with every page. A source that also says what its
// program has open now (`opened`, Chrome's tabs through chrome-open-tabs)
// is asked that then too, and never otherwise, a line each as its command
// prints them.
pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: recents

    property var sources: []
    // Whether the launcher is open, which a change is read while.
    property bool showing: false

    // Each program's paths by its desktop entry's id: from its own source,
    // and from the list programs share, matched again as the desktop
    // entries are read, since they may come after the list.
    property var lately: ({})
    property var sharedLines: []
    readonly property var shared: sharedFrom(sharedLines, DesktopEntries.applications.values)

    readonly property int most: 9

    // What each program has open now, by its desktop entry's id, as its
    // source's `opened` command printed it as it was last stepped into.
    property var openNow: ({})

    // The pages closed from the launcher since it opened, by program, each
    // staying where it was, said closed, until the next opening.
    property var closed: ({})

    // The programs whose asked sources were read since the launcher opened.
    property var asked: ({})

    // Reads the asked sources of a program stepped into, and what it has
    // open, once an opening.
    function ask(app) {
        if (recents.asked[app])
            return;
        recents.asked = Object.assign({}, recents.asked, { [app]: true });
        for (const source of each.instances) {
            if (source.modelData.app !== app)
                continue;
            if (source.modelData.asked)
                source.read();
            if (source.modelData.opened)
                source.look();
        }
    }

    // What a program has open now, a line each, or none.
    function openIn(app) {
        return openNow[app] ?? [];
    }

    // The launcher closed, so the next opening reads them again.
    function forget() {
        recents.asked = {};
        recents.closed = {};
    }

    // A page a program had open, closed from the launcher.
    function closing(app, address) {
        recents.closed = Object.assign({}, recents.closed,
            { [app]: (recents.closed[app] ?? []).concat([address]) });
    }

    function of(app) {
        return lately[app] ?? shared[app] ?? [];
    }

    // The desktop entry a program on the shared list is, by its id.
    function entryOf(program, entries) {
        const entry = DesktopEntries.heuristicLookup(program)
            ?? entries.find(each => each.name === program)
            ?? entries.find(each => !each.noDisplay
                && (each.command[0] ?? "").split("/").pop() === program);
        return entry?.id ?? "";
    }

    // The shared list's lines, each program's paths newest first, each once.
    function sharedFrom(lines, entries) {
        const each = {};
        for (const line of lines) {
            const tab = line.indexOf("\t");
            const app = tab > 0 ? recents.entryOf(line.slice(0, tab), entries) : "";
            if (app === "")
                continue;
            const paths = each[app] ?? [];
            const path = line.slice(tab + 1);
            if (paths.length < recents.most && !paths.includes(path))
                each[app] = paths.concat([path]);
        }
        return each;
    }

    Variants {
        id: each

        model: recents.sources

        Scope {
            id: source

            required property var modelData

            // A change while the list is being read is read again once it's
            // done, so the last change always counts.
            property bool again: false
            // A change while the launcher was closed, read as it opens.
            property bool stale: false

            readonly property bool showing: recents.showing

            onShowingChanged: {
                if (source.showing && source.stale) {
                    source.stale = false;
                    source.read();
                }
            }

            function changedNow() {
                if (source.showing)
                    source.read();
                else
                    source.stale = true;
            }

            function read() {
                if (listing.running)
                    source.again = true;
                else
                    listing.running = true;
            }

            // What its program has open, read only when asked.
            function look() {
                if (!opening.running)
                    opening.running = true;
            }

            Process {
                id: opening

                command: source.modelData.opened ?? []
                running: false
                stdout: StdioCollector {
                    id: opens

                    onStreamFinished: recents.openNow = Object.assign({}, recents.openNow, {
                        [source.modelData.app]: opens.text.split("\n").filter(line => line !== "")
                    })
                }
            }

            FileView {
                path: source.modelData.asked ? "" : source.modelData.watch ?? ""
                watchChanges: true
                printErrors: false
                onFileChanged: source.changedNow()
            }

            Process {
                id: listing

                command: source.modelData.list
                running: !source.modelData.asked
                onExited: {
                    if (source.again) {
                        source.again = false;
                        listing.running = true;
                    }
                }
                stdout: StdioCollector {
                    id: listed

                    onStreamFinished: {
                        const lines = listed.text.split("\n").filter(line => line !== "");
                        if (source.modelData.app === undefined)
                            recents.sharedLines = lines;
                        else
                            recents.lately = Object.assign({}, recents.lately,
                                { [source.modelData.app]: lines.slice(0, recents.most) });
                    }
                }
            }
        }
    }
}

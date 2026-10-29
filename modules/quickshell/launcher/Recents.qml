// What each program had open lately, for its Recent inside the launcher
// (Applications.qml): the paths it keeps, in the order it keeps them, read
// again each time the file it keeps them in changes, and never on a timer.
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
// whose history changes with every page.
pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: recents

    property var sources: []

    // Each program's paths by its desktop entry's id: from its own source,
    // and from the list programs share, matched again as the desktop
    // entries are read, since they may come after the list.
    property var lately: ({})
    property var sharedLines: []
    readonly property var shared: sharedFrom(sharedLines, DesktopEntries.applications.values)

    readonly property int most: 9

    // The programs whose asked sources were read since the launcher opened.
    property var asked: ({})

    // Reads the asked sources of a program stepped into, once an opening.
    function ask(app) {
        if (recents.asked[app])
            return;
        recents.asked = Object.assign({}, recents.asked, { [app]: true });
        for (const source of each.instances)
            if (source.modelData.asked && source.modelData.app === app)
                source.read();
    }

    // The launcher closed, so the next opening reads them again.
    function forget() {
        recents.asked = {};
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

            function read() {
                if (listing.running)
                    source.again = true;
                else
                    listing.running = true;
            }

            FileView {
                path: source.modelData.asked ? "" : source.modelData.watch ?? ""
                watchChanges: true
                printErrors: false
                onFileChanged: source.read()
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

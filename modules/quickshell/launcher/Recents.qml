// What each program had open lately, for its Recent inside the launcher
// (Applications.qml): the paths it keeps, in the order it keeps them, read
// again each time the file it keeps them in changes, and never on a timer.
// Its `sources`, each { app, watch, list }: `app` the program's desktop
// entry, `watch` the file the program writes as it goes, and `list` the
// command that prints its paths, one a line. At most nine are kept, so 1
// to 9 pick one.
pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: recents

    property var sources: []

    // Each program's paths by its desktop entry's id.
    property var lately: ({})

    readonly property int most: 9

    function of(app) {
        return lately[app] ?? [];
    }

    Variants {
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
                path: source.modelData.watch
                watchChanges: true
                printErrors: false
                onFileChanged: source.read()
            }

            Process {
                id: listing

                command: source.modelData.list
                running: true
                onExited: {
                    if (source.again) {
                        source.again = false;
                        listing.running = true;
                    }
                }
                stdout: StdioCollector {
                    id: listed

                    onStreamFinished: {
                        const paths = listed.text.split("\n").filter(line => line !== "")
                            .slice(0, recents.most);
                        recents.lately = Object.assign({}, recents.lately,
                            { [source.modelData.app]: paths });
                    }
                }
            }
        }
    }
}

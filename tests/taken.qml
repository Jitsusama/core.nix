// Drives what was taken lately (modules/quickshell/Taken.qml) finding
// screenshots by their words, against a stand-in for the screen-text
// daemon (tests/desktop/stand-in-screen-text.py), through Quickshell
// itself. Its folders read, it holds no connection until a search is
// asked; a search gives none at first and says once its answer comes,
// whose screenshots, with the line that matched, it then gives at once; a
// text asked again, in other case and spacing, asks nothing; once four
// texts have been asked since, the first is asked again; and every one of
// them goes on one connection. Each step waits on what Taken says, at most
// ten seconds. It exits 0 once every case holds, and otherwise 1, saying
// which didn't.
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string shots: Quickshell.env("SHOTS")
    readonly property string picture: `${root.shots}/2026-10-31 09-00-00.png`
    // What a search gave last, and the texts Taken said it found.
    property var given: undefined
    property var heard: []

    FileView {
        id: asked

        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/screen-text-asked`
        blockLoading: true
        printErrors: false
    }

    // What the stand-in was asked, a line each, as it wrote them down.
    function askedSoFar() {
        asked.reload();
        return asked.text().split("\n").filter(line => line !== "");
    }

    Connections {
        target: Taken

        function onSearched(words: string): void {
            root.heard = root.heard.concat([words]);
        }
    }

    function fail(what) {
        console.warn(`FAIL ${what}: given ${JSON.stringify(root.given)}, heard`
            + ` ${JSON.stringify(root.heard)}, asked ${JSON.stringify(root.askedSoFar())}`);
        Qt.exit(1);
    }

    // Asks for a text, and waits for Taken to say it was found.
    function searched(words) {
        return {
            what: `"${words}" is asked and said found`,
            run: () => {
                root.given = Taken.search(words);
            },
            holds: () => root.given === null && root.heard.includes(words)
        };
    }

    property int step: 0
    property int left: 0

    readonly property var steps: [
        {
            what: "the folder is read with no connection made",
            run: () => Taken.want(root.shots),
            holds: () => Taken.files.some(file => file.path === root.picture)
                && root.askedSoFar().length === 0
        },
        {
            what: "a search gives none at first, and says once it's found",
            run: () => {
                root.given = Taken.search("Thursday  REVIEW");
            },
            holds: () => root.given === null && root.heard.includes("thursday review")
        },
        {
            what: "what was found is given at once, with the line that matched",
            run: () => {
                root.given = Taken.search("thursday review");
            },
            holds: () => JSON.stringify(root.given) === JSON.stringify([{
                    path: root.picture,
                    line: "Move the review to Thursday morning at ten"
                }])
        },
        {
            what: "a text asked again asks nothing",
            holds: () => JSON.stringify(root.askedSoFar())
                === JSON.stringify(["connected", "search thursday review"])
        },
        root.searched("weather"),
        root.searched("quarterly"),
        root.searched("planning"),
        root.searched("morning"),
        {
            what: "the first text, four texts on, is asked again",
            run: () => {
                root.heard = [];
                root.given = Taken.search("thursday review");
            },
            holds: () => root.given === null && root.heard.includes("thursday review")
        },
        {
            what: "every text went on one connection, each asked once a time",
            holds: () => JSON.stringify(root.askedSoFar()) === JSON.stringify(["connected",
                    "search thursday review", "search weather", "search quarterly",
                    "search planning", "search morning", "search thursday review"])
        }
    ]

    function begin() {
        const now = steps[step];
        left = 200;
        if (now.run)
            now.run();
        waiting.start();
    }

    Timer {
        id: waiting

        interval: 50
        repeat: true
        onTriggered: {
            const now = root.steps[root.step];
            if (now.holds()) {
                stop();
                root.step += 1;
                if (root.step === root.steps.length)
                    Qt.callLater(() => Qt.exit(0));
                else
                    root.begin();
            } else {
                root.left -= 1;
                if (root.left === 0) {
                    stop();
                    root.fail(now.what);
                }
            }
        }
    }

    Component.onCompleted: begin()
}

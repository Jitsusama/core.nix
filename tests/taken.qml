// Drives what was taken lately (modules/quickshell/Taken.qml) finding
// screenshots by their words, against a stand-in for the screen-text
// daemon (tests/desktop/stand-in-screen-text.py), through Quickshell
// itself. Its folders read, it holds no connection until a search is
// asked; a search gives none at first and says once its answer comes,
// whose screenshots, with the line that matched, it then gives at once; a
// text asked again, in other case and spacing, asks nothing, and so do
// fewer than three letters, finding none; once four texts have been
// asked since, the first is asked again; and every one of them goes on
// one connection. Once it rests the connection goes, an answer kept is
// still given without one, and the next text asked opens it again. A
// ragged line is given as one line, its spaces single, cut at a word
// within eighty, or begun a word or two before the first word matched
// when that lies past them. A few thousand files are found by name, newest first,
// as Recent finds them should the programs daemon not search, whose search is
// what a key does (modules/programs/src/taken.rs). Each step waits on what Taken says, at most
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
            + ` ${JSON.stringify(root.heard)}, asked ${JSON.stringify(root.askedSoFar())}`
            + `, wombats ${JSON.stringify(root.wombats)}`);
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
            what: "the folder is read through the programs daemon, with no search asked",
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
        {
            what: "fewer than three letters ask nothing, and find none",
            run: () => {
                root.given = Taken.search(" Th ");
            },
            holds: () => JSON.stringify(root.given) === "[]"
                && JSON.stringify(root.askedSoFar())
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
        },
        {
            what: "resting lets the connection go",
            run: () => Taken.rest(),
            holds: () => root.askedSoFar().slice(-1)[0] === "gone"
        },
        {
            what: "an answer kept is given at rest, with nothing asked",
            run: () => {
                root.given = Taken.search("morning");
            },
            holds: () => (root.given ?? []).length === 1
                && root.askedSoFar().slice(-1)[0] === "gone"
        },
        {
            what: "the next text asked opens the connection again",
            run: () => {
                root.heard = [];
                root.given = Taken.search("ten");
            },
            holds: () => root.heard.includes("ten")
                && JSON.stringify(root.askedSoFar().slice(-3))
                === JSON.stringify(["gone", "connected", "search ten"])
        },
        root.searched("wombat"),
        {
            what: "a ragged line is given as one line, its spaces single, cut at a word",
            run: () => {
                root.given = Taken.search("wombat");
            },
            holds: () => JSON.stringify(root.given) === JSON.stringify([{
                    path: `${root.shots}/2026-10-30 09-00-00.png`,
                    line: "Notes: the wombat review moved to Friday afternoon, with the planning"
                        + " board and"
                }])
        },
        root.searched("koala"),
        {
            what: "a line matched past its first eighty begins a word or two before the match",
            run: () => {
                root.given = Taken.search("koala");
            },
            holds: () => JSON.stringify(root.given) === JSON.stringify([{
                    path: `${root.shots}/2026-10-29 09-00-00.png`,
                    line: "lastly the koala exhibit plans"
                }])
        },
        {
            what: "a few thousand files are found by name, newest first",
            run: () => root.findNames(),
            holds: () => JSON.stringify(root.wombats) === JSON.stringify(root.wombatsMeant)
        }
    ]

    // What "wombat plan" found.
    property var wombats: []
    readonly property var wombatsMeant: [`${root.shots}/wombat plan.png`,
        `${root.shots}/wombat plan 2.png`]

    // Three thousand files as the daemon says them, a time, a size and a path
    // each, two named for the wombat plan, the rest as screenshots and
    // recordings are named, searched for the wombat plan.
    function findNames() {
        const lines = [];
        for (let i = 0; i < 3000; i++) {
            const day = String(1 + i % 28).padStart(2, "0");
            const kind = i % 3 === 0 ? "Recording" : "Screenshot";
            const path = `${root.shots}/${kind} from 2026-10-${day} ${i}.png`;
            lines.push(`${1760000000 + i}\t3610000\t${path}`);
        }
        lines.push(`1700000000\t3610000\t${root.shots}/wombat plan.png`);
        lines.push(`1690000000\t3610000\t${root.shots}/wombat plan 2.png`);
        Taken.heard(lines);
        root.wombats = Taken.named("wombat plan", [root.shots], 50);
    }

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

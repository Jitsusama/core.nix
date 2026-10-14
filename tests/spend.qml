// Drives the AI spend piece (modules/quickshell/pieces/Spend.qml) and the
// launcher's collection of it (launcher/Spend.qml) through Quickshell
// itself, over the files spend-watch wrote from a small history beside
// this, and over states written by hand, with niri, do not disturb and the
// shortcuts stood in for. With nothing written there's no piece and no
// collection; what the watch wrote is drawn as it says, with a way to see
// where it went; a cap near its end wants Joel, in the alert colour, with
// its badge and the finding's action first; while the screen is shared the
// amounts go; a watch that stopped writing is a warning with a way to start
// it; and a file of another version is no piece. The collection answers
// "spend", "cap" and a report by its words, leaves alone what isn't one,
// and a finding goes to its session and steps into marking it expected. It
// exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property real now: Date.now() / 1000
    readonly property string written: real.text()
    readonly property string collected: realCollection.text()

    function said(value, badge, hint, context) {
        return { value: value, badge: badge, hint: hint, context: context };
    }

    function status(fields) {
        return JSON.stringify(Object.assign({
            v: 1,
            heartbeat: root.now,
            active: false,
            role: "text",
            wants: false,
            title: "AI spend",
            open: said("$148 today · usual", "", "$148 today · usual", "at list prices"),
            quiet: said("today · usual", "", "today · usual", "at list prices"),
            actions: []
        }, fields));
    }

    readonly property string alert: status({
        active: true,
        role: "alert",
        wants: true,
        open: said("$3,642 of $4,000 this week", "91%", "The weekly cap is 91% spent",
            "a 60% chance of the cap by Thu 14:00"),
        quiet: said("the weekly cap is close", "91%", "The weekly cap is close", ""),
        actions: [{
            kind: "go",
            name: "Go to the session",
            run: ["spend-watch", "go", "s1"]
        }, {
            kind: "mark",
            name: "It's expected",
            run: ["spend-watch", "findings", "expect", "k"]
        }]
    })

    // Each case: what the watch wrote (null: nothing), whether the screen
    // is shared, and what the piece makes of it.
    readonly property var cases: [{
        what: "a watch that never wrote",
        status: null,
        holds: piece => !piece.present && !piece.active
    }, {
        what: "what the watch wrote",
        status: root.written,
        holds: piece => piece.present && !piece.silent && /today/.test(piece.value)
            && piece.title === "AI spend" && piece.context !== ""
            && piece.actions.some(action => action.name === "Show where it went"
                && action.chord.map(key => key.text).join("+") === "Super+U")
            && piece.actions.some(action => action.name === "Read this week's report")
    }, {
        what: "a cap close to its end",
        status: root.alert,
        holds: piece => piece.active && piece.role === "alert" && piece.wants
            && piece.value === "$3,642 of $4,000 this week" && piece.badge === "91%"
            && piece.hint === "The weekly cap is 91% spent"
            && piece.actions[0].name === "Go to the session"
            && piece.actions[0].glyph === String.fromCodePoint(0xf03cb)
            && piece.actions[1].glyph === String.fromCodePoint(0xf012c)
    }, {
        what: "the actions run what the watch says",
        status: root.alert,
        act: piece => piece.actions[0].run(),
        holds: piece => Niri.spawned.join(" ") === "spend-watch go s1"
    }, {
        what: "a cap close to its end, while sharing",
        status: root.alert,
        sharing: true,
        holds: piece => piece.wants && piece.value === "the weekly cap is close"
            && !piece.value.includes("$") && !piece.hint.includes("$") && piece.context === ""
    }, {
        what: "a watch that stopped",
        status: status({ heartbeat: root.now - 600 }),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants
            && piece.value === "not watching" && piece.badge === ""
            && piece.actions.some(action => action.name === "Start the watch")
    }, {
        what: "a file of another version",
        status: JSON.stringify({ v: 2, open: {}, quiet: {} }),
        holds: piece => !piece.present
    }]

    property int at: 0
    property int left: 0
    property bool acted: false

    Spend {
        id: piece
        settings: ({ status: `${root.runtime}/status.json`, every: 50 })
    }

    Launcher.Spend {
        id: spend
        settings: ({ name: "spend", collection: `${root.runtime}/collection.json` })
    }

    FileView {
        id: real
        path: `${root.runtime}/written/status.json`
    }

    FileView {
        id: realCollection
        path: `${root.runtime}/written/collection.json`
    }

    FileView {
        id: file
        path: `${root.runtime}/status.json`
        printErrors: false
    }

    FileView {
        id: collectionFile
        path: `${root.runtime}/collection.json`
        printErrors: false
    }

    function begin() {
        const step = cases[at];
        Quiet.sharing = step.sharing ?? false;
        file.setText(step.status ?? "");
        left = 100;
        acted = false;
        waiting.start();
    }

    // Quickshell reads the files in the background, so each case waits for
    // the piece to hold, for five seconds at most.
    Timer {
        id: waiting
        interval: 50
        repeat: true
        onTriggered: {
            const step = root.cases[root.at];
            if (step.act && !root.acted && piece.present) {
                root.acted = true;
                step.act(piece);
            }
            if (step.holds(piece)) {
                stop();
                if (++root.at === root.cases.length)
                    root.collect();
                else
                    root.begin();
            } else if (--root.left === 0) {
                stop();
                console.warn(`FAIL ${step.what}: present ${piece.present}, active ${piece.active},`
                    + ` role ${piece.role}, wants ${piece.wants}, value "${piece.value}",`
                    + ` badge "${piece.badge}", hint "${piece.hint}", context "${piece.context}",`
                    + ` actions ${piece.actions.map(action => action.name).join(", ")}`);
                Qt.exit(1);
            }
        }
    }

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    // The collection, over nothing and then over what the watch wrote.
    property int collecting: 0

    function collect() {
        if (spend.collection !== null || spend.answer("spend") !== null) {
            console.warn(`FAIL a collection with nothing written: ${JSON.stringify(spend.collection)}`);
            Qt.exit(1);
            return;
        }
        collectionFile.setText(root.collected);
        collecting = 100;
        reading.start();
    }

    Timer {
        id: reading
        interval: 50
        repeat: true
        onTriggered: {
            if (spend.said !== null) {
                stop();
                root.answers();
            } else if (--root.collecting === 0) {
                stop();
                console.warn("FAIL the collection never read what the watch wrote");
                Qt.exit(1);
            }
        }
    }

    function answers() {
        try {
            expect("a collection", spend.collection?.noun === "spend", "none");
            const today = spend.answer("spend");
            expect("spend", today !== null && /^Today: /.test(today.headline)
                && today.copy.startsWith("## AI spend"), JSON.stringify(today));
            const week = spend.answer("spend last week by quest");
            expect("a report by its words", week !== null
                && week.headline.startsWith("Spend last week by quest: ")
                && week.copy.startsWith("## Spend last week by quest"), JSON.stringify(week));
            const model = spend.answer("ai today by model");
            expect("another report", model !== null
                && model.headline.startsWith("Spend today by model"), JSON.stringify(model));
            const cap = spend.answer("cap");
            expect("the caps", cap !== null && cap.copy.includes("cap"), JSON.stringify(cap));
            for (const typed of ["spend nonsense", "hello", "spend last week by nothing"])
                expect(`not a report: ${typed}`, spend.answer(typed) === null,
                    JSON.stringify(spend.answer(typed)));
            const all = spend.everything();
            expect("everything", all.length > 10 && all[0].name === "Today"
                && all.some(row => row.name === "Spend this week by model"),
                all.map(row => row.name).join(", "));
            const found = spend.search("waste").rows;
            expect("a search", found.length > 0, "nothing for waste");

            // A finding with an action goes to its session, and steps into
            // marking it expected.
            const shown = JSON.parse(root.collected);
            shown.findings = [{
                key: "finding:burn:s1", name: "A burn in Aquifer", side: "alert",
                copy: "A burn in Aquifer.", role: "alert",
                run: ["spend-watch", "go", "s1"],
                expect: ["spend-watch", "findings", "expect", "burn:s1"]
            }];
            spend.said = shown;
            const burn = spend.everything().find(row => row.name === "A burn in Aquifer");
            expect("a finding", burn && burn.verb === "go" && burn.hue === "alert", "missing");
            burn.run();
            expect("going to its session", Niri.spawned.join(" ") === "spend-watch go s1",
                Niri.spawned.join(" "));
            const expected = burn.verbs().find(row => row.name === "It's expected");
            expected.run();
            expect("marking it expected",
                Niri.spawned.join(" ") === "spend-watch findings expect burn:s1",
                Niri.spawned.join(" "));
            expect("its count", spend.counted() === "1 finding", spend.counted());
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }

    Component.onCompleted: begin()
}

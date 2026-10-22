// Drives the AI spend piece (modules/quickshell/pieces/Spend.qml) and the
// launcher's collection of it (launcher/Spend.qml) through Quickshell
// itself, over the files spend-watch wrote from a small history beside
// this, and over states written by hand, with niri, do not disturb and the
// shortcuts stood in for. With nothing written there's no piece and no
// collection; what the watch wrote is drawn as it says, with a way to see
// where it went; a cap near its end wants Joel, in the alert colour, with
// its badge and the finding's action first; while the screen is shared the
// amounts go; a watch that stopped writing is a warning with a way to start
// it; a file of another version is no piece, and nor is one taken away
// again. The collection answers "spend", "cap" and a report by its words,
// leaves alone what isn't one, steps from a period into its cuts and from a
// cut into its rows, each with its footer, shows no amount while the screen
// is shared, draws nothing from a file of another version, and puts a
// finding that wants a look first, going to its session and stepping into
// marking it expected. It exits 0 once every case holds, and otherwise 1,
// saying which didn't.
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
        // A day of logs and no feed: list prices are all there is, and a
        // day has no usual or forecast yet, so there's nothing to add.
        what: "what the watch wrote",
        status: root.written,
        holds: piece => piece.present && !piece.silent && /today/.test(piece.value)
            && piece.title === "AI spend" && piece.context === ""
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
    }, {
        what: "what the watch wrote, again",
        status: root.written,
        holds: piece => piece.present && /today/.test(piece.value)
    }, {
        what: "the file taken away",
        status: null,
        holds: piece => !piece.present && !piece.active
    }]

    property int at: 0
    property int left: 0
    property bool acted: false

    Spend {
        id: piece
        settings: ({ status: `${root.runtime}/status.json` })
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

    // Takes the status file away, as nothing having written one leaves it.
    Process {
        id: removing
        command: ["rm", "-f", `${root.runtime}/status.json`]
    }

    function begin() {
        const step = cases[at];
        Quiet.sharing = step.sharing ?? false;
        if (step.status === null)
            removing.running = true;
        else
            file.setText(step.status);
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
            const shown = JSON.stringify(spend.collection);
            console.warn(`FAIL a collection with nothing written: ${shown}`);
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
            // A day of history has no forecast yet and the history no cap,
            // so today and Earlier.
            expect("everything", all.length >= 2 && all[0].name === "Today"
                && all.every(row => row.side.length <= 32),
                all.map(row => `${row.name}: ${row.side}`).join(", "));
            expect("how today stands, at home", spend.counted() === all[0].side,
                spend.counted());

            // A period steps into its cuts, and a cut into its rows, each
            // saying more in the footer and copying its report. This week
            // is among the periods behind Earlier.
            const earlier = all.find(row => row.name === "Earlier");
            expect("no week of its own on the first screen",
                earlier && !all.some(row => row.name === "This week"),
                all.map(row => row.name).join(", "));
            const thisWeek = earlier.verbs().find(row => row.name === "This week");
            expect("a period with its cuts",
                thisWeek && thisWeek.nested && thisWeek.ask === "which?",
                JSON.stringify(thisWeek));
            const cuts = thisWeek.verbs();
            const byModel = cuts.find(row => row.name === "By model");
            expect("a cut", byModel && byModel.note !== "" && byModel.nested,
                cuts.map(row => row.name).join(", "));
            const models = byModel.verbs();
            expect("its rows", models.length === 2 && models.every(row => !row.verbs),
                models.map(row => row.name).join(", "));
            expect("copying its report", byModel.verb === "copy", byModel.verb);
            // Typing finds a period however deep it's kept.
            const found = spend.search("this week").rows;
            expect("a search", found.length > 0 && found[0].name === "This week",
                found.map(row => row.name).join(", "));
            // In the main search the collection adds its best match as a
            // row, unless it answered what's typed: its answer already
            // lists them, so "spend" says each thing once.
            const gathered = spend.gather("this week").rows;
            expect("a row in the main search",
                gathered.length > 0 && gathered[0].name === "This week",
                gathered.map(row => row.name).join(", "));
            expect("no row beside its answer", spend.gather("spend").rows.length === 0,
                spend.gather("spend").rows.map(row => row.name).join(", "));
            expect("an older file draws nothing", spend.parsed('{"v":1}') === null, "drawn");

            // While sharing, no row and no answer holds an amount.
            Quiet.sharing = true;
            const quiet = spend.everything();
            expect("quiet rows", quiet.length > 0
                && quiet.every(row => !`${row.name} ${row.side}`.includes("$")),
                quiet.map(row => `${row.name}: ${row.side}`).join(", "));
            const hidden = spend.answer("spend last week by quest");
            expect("a quiet answer", hidden !== null && !hidden.headline.includes("$")
                && hidden.rows.every(row => !`${row.name} ${row.side}`.includes("$")),
                JSON.stringify(hidden));
            Quiet.sharing = false;

            // A finding with an action goes to its session, and steps into
            // marking it expected.
            const shown = JSON.parse(root.collected);
            shown.findings = [{
                key: "finding:burn:s1", name: "A burn in Aquifer", side: "alert",
                copy: "A burn in Aquifer.", role: "alert",
                run: ["spend-watch", "go", "s1"], verb: "go",
                expect: ["spend-watch", "findings", "expect", "burn:s1"]
            }];
            spend.said = shown;
            const burn = spend.everything()[0];
            expect("a finding first", burn.name === "A burn in Aquifer" && burn.verb === "go"
                && burn.hue === "alert", burn.name);
            // The answer for "spend" leads with it too, as the collection
            // does, before the rows of how things stand.
            const answered = spend.answer("spend").rows;
            expect("the answer's finding first", answered[0].name === "A burn in Aquifer",
                answered.map(row => row.name).join(", "));
            burn.run();
            expect("going to its session", Niri.spawned.join(" ") === "spend-watch go s1",
                Niri.spawned.join(" "));
            const expected = burn.verbs().find(row => row.name === "It's expected");
            expected.run();
            expect("marking it expected",
                Niri.spawned.join(" ") === "spend-watch findings expect burn:s1",
                Niri.spawned.join(" "));
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }

    Component.onCompleted: begin()
}

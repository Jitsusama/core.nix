// Drives the AI spend piece (modules/quickshell/pieces/Spend.qml) and the
// launcher's collection of it (launcher/Spend.qml) through Quickshell
// itself, over the files spend-watch wrote from a small history beside
// this, and over states written by hand, with niri, do not disturb and the
// shortcuts stood in for. With nothing written there's no piece and no
// collection; what the watch wrote is drawn as it says, with a way to see
// where it went; a cap near its end wants Joel, in the alert colour, with
// the finding's action first; while the screen is shared the amounts go; a
// watch that stopped writing is a warning with a way to start it; a file of another version is no piece, and nor is one taken away
// again. The collection answers "spend", "cap" and a report by its words,
// leaves alone what isn't one, steps from a period into its cuts and from a
// cut into its rows, each with its footer, shows no amount while the screen
// is shared, draws nothing from a file of another version, opens a drawer
// of every fact budgets.json holds of a budget's row and a window's, and
// puts a
// finding that wants a look first, going to its session and stepping into
// marking it expected; read from the file, its figures are said to be the
// last ones, since the file is read only while spend-watch's socket isn't
// answering (tests/spend-told.qml drives the socket). The tokens piece (pieces/Tokens.qml), over the same
// file, is there only while the watch's `tokens` warns: as the watch words
// it, wanting Joel, said as it arrives and again as its level steps down,
// with See What Spends and Pause for a session that's running and nothing
// else; urgent in the alert colour once out; quiet while shared; saying
// what can't be read; and gone while the watch has stopped. It exits 0
// once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "launcher/SpendDrawer.js" as SpendDrawer

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property real now: Date.now() / 1000
    readonly property string written: real.text()
    readonly property string collected: realCollection.text()
    readonly property string budgeted: realBudgets.text()

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

    readonly property var running: ({
        level: "hour",
        now: false,
        title: "Tokens Run Out in About 40 Min",
        value: "resets 14:00",
        context: "at this hour's rate \u00b7 62% used",
        hint: "Tokens run out in about 40 min, at 14:00 they reset",
        quiet: {
            title: "Tokens Run Out Soon",
            value: "resets 14:00",
            context: "at this hour's rate",
            hint: "Tokens run out soon"
        },
        actions: [
            { kind: "spends", name: "See What Spends" },
            { kind: "pause", name: "Pause 15J7N3", session: "s-running", quest: "Q" },
            { kind: "pause", name: "Pause Gone", session: "s-gone", quest: "Q" },
            { kind: "stop", name: "Stop" }
        ]
    })

    // Each case: what the watch wrote (null: nothing), whether the screen
    // is shared, and what the piece makes of it, and the tokens piece where
    // it says.
    readonly property var cases: [{
        what: "a watch that never wrote",
        status: null,
        holds: piece => !piece.present && !piece.active,
        tokens: piece => !piece.present && !piece.active
    }, {
        what: "tokens running out",
        status: status({ tokens: root.running }),
        holds: piece => piece.present,
        tokens: piece => piece.present && piece.active && piece.wants && !piece.now
            && piece.role === "warning" && piece.arrives && piece.withdraws
            && piece.telling === "hour" && piece.title === "Tokens Run Out in About 40 Min"
            && piece.value === "resets 14:00" && piece.context === "at this hour's rate \u00b7 62% used"
            && piece.hint === piece.title
            && piece.message === "resets 14:00 \u00b7 at this hour's rate \u00b7 62% used"
            && piece.actions.map(action => action.name).join(", ")
                === "See What Spends, Pause The Launcher's Shape"
    }, {
        what: "pausing the session that spends most",
        status: status({ tokens: root.running }),
        act: (piece, tokens) => tokens.actions[1].run(),
        holds: piece => Sessions.steered === "ADDR1 pause"
    }, {
        what: "tokens running out, while sharing",
        status: status({ tokens: root.running }),
        sharing: true,
        holds: piece => piece.present,
        tokens: piece => piece.wants && piece.title === "Tokens Run Out Soon"
            && piece.hint === "Tokens Run Out Soon" && !piece.message.includes("%")
    }, {
        what: "tokens that can't be read",
        status: status({ tokens: Object.assign({}, root.running, { unread: "can't be read since 10:12" }) }),
        holds: piece => piece.present,
        tokens: piece => piece.wants && piece.context === "can't be read since 10:12"
    }, {
        what: "tokens out",
        status: status({
            tokens: Object.assign({}, root.running, { level: "out", now: true, title: "Tokens Ran Out" })
        }),
        holds: piece => piece.present,
        tokens: piece => piece.now && piece.role === "alert" && piece.telling === "out"
            && piece.title === "Tokens Ran Out" && piece.message.startsWith("resets 14:00")
    }, {
        what: "tokens while the watch has stopped",
        status: status({ heartbeat: root.now - 600, tokens: root.running }),
        holds: piece => piece.value === "not watching",
        tokens: piece => !piece.present && !piece.active
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
            && piece.value === "$3,642 of $4,000 this week"
            && piece.hint === "The weekly cap is 91% spent"
            && piece.actions[0].name === "Go to the session"
            && piece.actions[0].glyph === Theme.glyph.windows
            && piece.actions[1].glyph === Theme.glyph.fine
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
            && piece.value === "not watching"
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

    Tokens {
        id: tokensPiece
        settings: ({ status: `${root.runtime}/status.json` })
    }

    Launcher.Spend {
        id: spend
        settings: ({ name: "spend" })
    }

    // The launcher as a source asks it whether its rows are wanted, shut
    // or open elsewhere (Provider.qml's wanted).
    QtObject {
        id: shutLauncher

        property bool wanting: true
        property bool open: true

        function wants(source) {
            return wanting;
        }

        function afterFrame(then) {
            then();
        }
    }

    Launcher.Spend {
        id: shutSpend
        settings: ({ name: "spend" })
        launcher: shutLauncher
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
        id: realBudgets
        path: `${root.runtime}/written/budgets.json`
    }

    FileView {
        id: budgetsFile
        path: `${root.runtime}/spend-watch/budgets.json`
        printErrors: false
    }

    FileView {
        id: file
        path: `${root.runtime}/status.json`
        printErrors: false
    }

    FileView {
        id: collectionFile
        // Where spend-watch writes it, as SpendWatch reads it.
        path: `${root.runtime}/spend-watch/collection.json`
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
                step.act(piece, tokensPiece);
            }
            if (step.holds(piece) && (step.tokens === undefined || step.tokens(tokensPiece))) {
                stop();
                if (++root.at === root.cases.length)
                    root.collect();
                else
                    root.begin();
            } else if (--root.left === 0) {
                stop();
                console.warn(`FAIL ${step.what}: present ${piece.present}, active ${piece.active},`
                    + ` role ${piece.role}, wants ${piece.wants}, value "${piece.value}",`
                    + ` hint "${piece.hint}", context "${piece.context}",`
                    + ` actions ${piece.actions.map(action => action.name).join(", ")};`
                    + ` tokens present ${tokensPiece.present}, now ${tokensPiece.now},`
                    + ` title "${tokensPiece.title}", context "${tokensPiece.context}",`
                    + ` actions ${tokensPiece.actions.map(action => action.name).join(", ")}`);
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
        // With no socket and nothing written, it says the watch isn't
        // answering, with a way to start it, rather than going silently.
        const down = spend.everything();
        const asked = spend.answer("spend");
        if (spend.collection?.noun !== "spend" || down.length !== 1
            || down[0].name !== "Start the Watch" || down[0].note !== "the watch isn't answering"
            || asked?.headline !== "the watch isn't answering" || asked.rows.length !== 1
            || spend.answer("hello") !== null) {
            const shown = JSON.stringify([down.map(row => [row.name, row.note]), asked]);
            console.warn(`FAIL a collection with nothing written says the watch is down: ${shown}`);
            Qt.exit(1);
            return;
        }
        collectionFile.setText(root.collected);
        budgetsFile.setText(root.budgeted);
        collecting = 100;
        reading.start();
    }

    Timer {
        id: reading
        interval: 50
        repeat: true
        onTriggered: {
            if (spend.known && SpendWatch.standing !== null) {
                stop();
                root.answers();
            } else if (--root.collecting === 0) {
                stop();
                console.warn("FAIL the collection never read what the watch wrote, "
                    + `budgets ${SpendWatch.standing !== null}`);
                Qt.exit(1);
            }
        }
    }

    // How every budget stands, as budgets.json says it: a plan of two
    // share windows, one past where it warns and one not read yet; the
    // employer's money of two dollar windows, one past where it warns,
    // raised and overridden for a while, and one its feed refuses; and a
    // team's token of one window.
    readonly property var standing: ({
        v: 1, default: "proxy", sessions: { "s-running": "proxy" },
        budgets: [{
            name: "claude", label: "Claude Pro", words: "your own plan",
            providers: ["claude-code"], standing: "85% of 5 h",
            allowances: [{
                name: "five-hours", label: "Five Hours", unit: "share",
                standing: "85% of 5 h", nearest: 0.85, warn: 0.8,
                said: { brief: "85% \u00b7 resets 16:20", window: "5 hours",
                    warns: "at 80%", resets: "16:20",
                    read: "Claude at 14:20", state: "" }
            }, {
                name: "week", label: "Week", unit: "share", warn: 0.8,
                said: { window: "a week", warns: "at 80%", resets: "", read: "", state: "" }
            }],
            said: { what: "your own \u00b7 plan", spent: "", worth: "$84 today",
                reading: "reading fine", providers: "Claude Code" }
        }, {
            name: "proxy", label: "Your AI proxy token", words: "the employer's money",
            providers: [], standing: "$37,000 of $40,000 this week",
            allowances: [{
                name: "weekly", label: "Week", unit: "dollars",
                standing: "$37,000 of $40,000", nearest: 0.925, warn: 0.9,
                said: { window: "a week", warns: "at $36,000", resets: "",
                    ends: "Wed 2 Dec \u00b7 a guess",
                    changed: "raised $30,000 to $40,000 \u00b7 Thursday 11:31",
                    read: "the AI proxy at 21:35", state: "" }
            }, {
                name: "premium", label: "Premium Day", unit: "dollars",
                standing: "$0 of $300", nearest: 0, warn: 0.5,
                said: { window: "a day", warns: "at $150", resets: "05:00 tomorrow",
                    read: "the AI proxy at 21:35", state: "refusing requests" }
            }],
            said: { what: "the employer's \u00b7 money", spent: "$3,075 today", worth: "",
                reading: "signed out" }
        }, {
            name: "delta", label: "Delta team token", words: "a team's token",
            providers: [], standing: "$40 of $50 today",
            allowances: [{
                name: "day", label: "Day", unit: "dollars", standing: "$40 of $50",
                nearest: 0.8, warn: 0.9,
                said: { window: "a day", warns: "at $45", resets: "05:00 tomorrow",
                    read: "the AI proxy at 21:35", state: "" }
            }],
            said: { what: "a team's \u00b7 token", spent: "$40 today", worth: "", reading: "" }
        }]
    })

    // A drawer's facts as one line: each name and value, its figure's
    // shape after, a figure past where it warns marked so, and words set
    // in the words face marked with a tilde.
    function factsOf(drawer) {
        return drawer.facts.map(fact => `${fact.label}=${fact.value}`
            + (fact.figure ? `:${fact.figure.shape}${fact.figure.warn ? "!" : ""}` : "")
            + (fact.words ? "~" : ""))
            .join("; ");
    }

    // A cap's row opens its window's drawer, whichever row reaches it, and
    // a period's opens its days and the facts its row carries, named by
    // the vocabulary, words in the words face.
    function capsAndPeriods(rows) {
        SpendWatch.standing = root.standing;
        const opened = key => {
            const row = rows.find(one => one.key === `spend:${key}`);
            return row?.thing === true && row.drawer ? row.drawer() : null;
        };
        const cap = opened("cap:weekly");
        const window = rows.find(one => one.key === "spend:budgets").verbs()
            .find(one => one.key === "spend:budget:proxy:weekly").drawer();
        expect("a cap's drawer is its window's", cap !== null
            && JSON.stringify(cap) === JSON.stringify(window), cap ? factsOf(cap) : "none");
        const week = opened("period:this-week");
        expect("a period's drawer", week?.eyebrow === "Mon 2 Nov to Sat 7 Nov \u00b7 week"
            && week.title === "This week" && factsOf(week) === "Read From=the bill~; "
            + "Rebuilding Context=$310 \u00b7 6%; "
            + "Most by Quest=Build the shell \u00b7 31%~; "
            + "Most by Model=claude-opus-5-5 \u00b7 all", week ? factsOf(week) : "none");
    }

    // A budget's row and a window's each open a drawer of every fact
    // budgets.json holds of it, in the vocabulary's names, and the
    // Budgets row, which only gathers them, has none of its own.
    function drawers(budgets) {
        SpendWatch.standing = root.standing;
        const rows = budgets.verbs();
        // The launcher draws a drawer only for a thing (Launcher.qml's
        // detailed), so a budget's row with no `thing` shows none.
        const drawn = key => {
            const row = rows.find(one => one.key === `spend:${key}`);
            return row?.thing === true && row.drawer ? row.drawer() : null;
        };
        const said = key => {
            const drawer = drawn(key);
            return drawer ? factsOf(drawer) : "none";
        };
        // A share window's row says how much, so its drawer has neither a
        // Standing nor a meter that would say it again.
        const five = drawn("budget:claude:five-hours");
        expect("a share window's drawer", five?.eyebrow === "in Claude Pro \u00b7 window"
            && five.eyebrowNames && five.title === "Five Hours"
            && factsOf(five) === "Resets=16:20; Read By=Claude at 14:20; Warns=at 80%",
            said("budget:claude:five-hours"));
        expect("a window not read yet", said("budget:claude:week") === "Warns=at 80%",
            said("budget:claude:week"));
        // A dollar window says its standing with its meter, and what its
        // feed says comes first, while it says anything.
        expect("a dollar window past where it warns", said("budget:proxy:weekly")
            === "Standing=$37,000 of $40,000:stroke!; Override Ends=Wed 2 Dec \u00b7 a guess; "
            + "Last Changed=raised $30,000 to $40,000 \u00b7 Thursday 11:31; "
            + "Read By=the AI proxy at 21:35; Warns=at $36,000", said("budget:proxy:weekly"));
        expect("a window its feed refuses", said("budget:proxy:premium")
            === "Feed Says=refusing requests; Standing=$0 of $300:stroke; "
            + "Resets=05:00 tomorrow; Read By=the AI proxy at 21:35; Warns=at $150",
            said("budget:proxy:premium"));
        const plan = drawn("budget:claude");
        // Whose and what kind, our own words, so in Label's capitals; each
        // window its figure and when it resets, since the row's side is its
        // standing in full; its providers as each names itself.
        expect("a plan's drawer", plan?.eyebrow === "your own \u00b7 plan" && !plan.eyebrowNames
            && plan.title === "Claude Pro"
            && factsOf(plan) === "Five Hours=85% \u00b7 resets 16:20:stroke!; "
            + "Week=not read yet; Worth at List=$84 today; Feed=reading fine; "
            + "Providers=Claude Code", plan ? factsOf(plan) : "none");
        expect("the employer's money's drawer", said("budget:proxy")
            === "Week=$37,000 of $40,000:stroke!; Premium Day=$0 of $300:stroke; "
            + "Spent=$3,075 today; Feed=signed out", said("budget:proxy"));
        // A window in dollars has no brief, so its one window's reset is
        // among its facts.
        expect("a budget of one window says its facts", said("budget:delta")
            === "Day=$40 of $50:stroke; Resets=05:00 tomorrow; Read By=the AI proxy at 21:35; "
            + "Warns=at $45; Spent=$40 today", said("budget:delta"));
        // While sharing, no drawer holds a dollar figure, as no row does,
        // and a dollar window's meter goes with its Standing.
        Quiet.sharing = true;
        const quiet = ["budget:claude:five-hours", "budget:proxy:weekly", "budget:claude",
            "budget:proxy", "budget:delta"].map(said);
        Quiet.sharing = false;
        expect("quiet drawers", quiet.join(" | ") === "Resets=16:20; "
            + "Read By=Claude at 14:20; Warns=at 80% | Override Ends=Wed 2 Dec \u00b7 a guess; "
            + "Read By=the AI proxy at 21:35 | "
            + "Five Hours=85% \u00b7 resets 16:20:stroke!; Week=not read yet; "
            + "Feed=reading fine; Providers=Claude Code | Feed=signed out | "
            + "Resets=05:00 tomorrow; Read By=the AI proxy at 21:35", quiet.join(" | "));
        expect("the Budgets row's own drawer", !budgets.drawer, "drawn");
        expect("a session's budget", SpendDrawer.sessionBudget(SpendWatch.budgets,
            SpendWatch.budgetOf, "s-running")
            === "Your AI proxy token \u00b7 $37,000 of $40,000 this week"
            && SpendDrawer.sessionBudget(SpendWatch.budgets, SpendWatch.budgetOf, "s-none") === "",
            SpendDrawer.sessionBudget(SpendWatch.budgets, SpendWatch.budgetOf, "s-running"));
    }

    function answers() {
        try {
            expect("a collection", spend.collection?.noun === "spend", "none");
            // The session that spent it, by its whole id, for Agents.
            expect("each session's day", /^\$\d/.test(SpendWatch.sessions.s1 ?? ""),
                JSON.stringify(SpendWatch.sessions));
            const today = spend.answer("spend");
            expect("spend", today !== null && /^Today: /.test(today.headline)
                && today.copy.startsWith("## AI spend"), JSON.stringify(today));
            // Read from the file, with no socket answering, its figures
            // are said to be the last ones rather than passing as current.
            expect("the file's figures said as its last",
                today.note === "the watch isn't answering; these are its last figures",
                today.note);
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
            // so today and the periods before.
            expect("everything", all.length >= 2 && all[0].name === "Today"
                && all.every(row => row.side.length <= 32),
                all.map(row => `${row.name}: ${row.side}`).join(", "));
            // At home and in Agents, today's dollars with their period,
            // as All AI Spend says them.
            expect("how today stands, at home", spend.counted() === SpendWatch.today
                && spend.counted().startsWith(`${all[0].side.split(" \u00b7 ")[0]} today`),
                spend.counted());

            // A period steps into its cuts, and a cut into its rows, each
            // saying more in the footer and copying its report. The
            // periods sit in AI spend itself, behind no row of their own.
            expect("the periods on the first screen", !all.some(row => row.name === "Earlier")
                && all.some(row => row.name === "This week"),
                all.map(row => row.name).join(", "));
            const thisWeek = all.find(row => row.name === "This week");
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
            // Typing finds a period.
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
            expect("an older file draws nothing", SpendWatch.parsed('{"v":1}', 2) === null,
                "drawn");
            // The budgets as the watch wrote them: the session that spent
            // today drew on the default, which says its spend in words.
            const drewOn = SpendWatch.budgetOf.s1 ?? "";
            expect("which budget a session drew on", drewOn !== ""
                && /today/.test(SpendWatch.budgets[drewOn]?.said?.spent ?? ""),
                JSON.stringify(SpendWatch.standing));
            expect("a session's budget, as its drawer says it",
                SpendDrawer.sessionBudget(SpendWatch.budgets, SpendWatch.budgetOf, "s1")
                    .startsWith(SpendWatch.budgets[drewOn]?.label ?? "-"),
                SpendDrawer.sessionBudget(SpendWatch.budgets, SpendWatch.budgetOf, "s1"));

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

            // A budget wears AI spend's mark and a window a clock, muted at
            // rest and the warning's colour past it, drawn on a figure row
            // too; a window is named and copied as its feed names it.
            const budgeted = JSON.parse(root.collected);
            budgeted.now = [{
                key: "budgets", name: "Budgets", side: "$100 this month",
                copy: "Budgets: $100 this month", role: "warning", mark: "agent",
                rows: [{
                    key: "budget:claude:five-hours", name: "Five Hours", side: "85%",
                    copy: "Five Hours: 85%", role: "warning", mark: "clock"
                }, {
                    key: "budget:claude:week", name: "Week", side: "40%",
                    copy: "Week: 40%", role: "text", mark: "clock"
                }, {
                    key: "budget:proxy:premium", name: "Premium Day", side: "$0 of $300",
                    copy: "Premium Day: $0 of $300", role: "text", mark: "clock"
                }, {
                    key: "budget:claude", name: "Claude Pro", side: "85% of 5 h",
                    copy: "Claude Pro: 85% of 5 h", role: "warning", mark: "agent"
                }, {
                    key: "budget:proxy", name: "Your AI proxy token", side: "$37,000 of $40,000",
                    copy: "Your AI proxy token: $37,000 of $40,000", role: "warning", mark: "agent"
                }, {
                    key: "budget:proxy:weekly", name: "Week", side: "$37,000 of $40,000",
                    copy: "Week: $37,000 of $40,000", role: "warning", mark: "clock"
                }, {
                    key: "budget:delta", name: "Delta team token", side: "$40 of $50",
                    copy: "Delta team token: $40 of $50", role: "text", mark: "agent"
                }].map(row => Object.assign({ budget: row.key }, row))
            }, {
                key: "cap:weekly", name: "Week", side: "93%",
                copy: "Week: 93% of $40,000", mark: "clock", role: "warning", budget: "budget:proxy:weekly"
            }, {
                key: "period:this-week", name: "This week", side: "$4,100",
                copy: "This week: $4,100", role: "text",
                drawn: { eyebrow: "Mon 2 Nov to Sat 7 Nov \u00b7 week", facts: [
                    { id: "from", value: "the bill", words: true },
                    { id: "rebuilding", value: "$310 \u00b7 6%" },
                    { id: "most-quest", value: "Build the shell \u00b7 31%", words: true },
                    { id: "most-model", value: "claude-opus-5-5 \u00b7 all" }
                ] }
            }];
            SpendWatch.readCollection(budgeted);
            const budgets = spend.everything()[0];
            expect("a budget's mark in the warning's colour",
                budgets.glyph === Theme.glyph.agent && budgets.hue === "warning"
                && budgets.marked, JSON.stringify(budgets));
            const windows = budgets.verbs().slice(0, 3);
            expect("each window as its feed names it, its copy too",
                windows.map(row => row.name).join(", ") === "Five Hours, Week, Premium Day"
                && windows[2].copies() === "Premium Day: $0 of $300",
                windows.map(row => `${row.name}: ${row.copies()}`).join(", "));
            expect("a clock, muted at rest", windows.every(row => row.glyph === Theme.glyph.clock
                && row.marked) && windows[0].hue === "warning" && windows[1].hue === "muted",
                windows.map(row => row.hue).join(", "));
            root.drawers(budgets);
            root.capsAndPeriods(spend.everything());

            // A finding with an action goes to its session, and steps into
            // marking it expected.
            const shown = JSON.parse(root.collected);
            shown.findings = [{
                key: "finding:burn:s1", name: "A burn in Aquifer", side: "alert",
                copy: "A burn in Aquifer.", role: "alert",
                run: ["spend-watch", "go", "s1"], verb: "go to",
                expect: ["spend-watch", "findings", "expect", "burn:s1"]
            }];
            SpendWatch.readCollection(shown);
            const burn = spend.everything()[0];
            expect("a finding first", burn.name === "A burn in Aquifer" && burn.verb === "go to"
                && burn.hue === "alert", burn.name);
            // The answer for "spend" leads with it too, as the collection
            // does, before the rows of how things stand.
            const answered = spend.answer("spend").rows;
            expect("the answer's finding first", answered[0].name === "A burn in Aquifer",
                answered.map(row => row.name).join(", "));
            burn.run();
            expect("going to its session", Niri.spawned.join(" ") === "spend-watch go s1",
                Niri.spawned.join(" "));
            const expected = burn.verbs().find(row => row.name === "It's Expected");
            expected.run();
            expect("marking it expected",
                Niri.spawned.join(" ") === "spend-watch findings expect burn:s1",
                Niri.spawned.join(" "));
            root.shut();
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }

    // With the launcher shut, spend-watch writing its file again, as it
    // does every few seconds, makes none of the collection's rows, nor does
    // a pick asking every collection what it holds (Launcher.qml's
    // knowing): every row was made again at each write, 90 in four minutes
    // on Joel's shell (YQZGPC, 2026-11-10). Home's figure reads what was
    // written, and a row shown elsewhere is made as it reads now. Wanted
    // again, they're as last written, none made twice.
    function shut() {
        const finding = rows => rows.some(row => row.name === "A burn in Aquifer");
        expect("wanted, it lists what was last written", finding(shutSpend.everything()),
            shutSpend.everything().map(row => row.name).join(", "));
        shutLauncher.wanting = false;
        const before = shutSpend.tally.rows;
        SpendWatch.readCollection(JSON.parse(root.collected));
        SpendWatch.readCollection(JSON.parse(root.collected));
        const held = shutSpend.everything();
        expect("shut, spend-watch writing again makes none of its rows",
            shutSpend.tally.rows === before && finding(held), shutSpend.tally.rows - before);
        expect("and home's figure says what was written", shutSpend.counted() === spend.counted(),
            `${shutSpend.counted()} against ${spend.counted()}`);
        const today = spend.everything()[0];
        const now = shutSpend.rowNow(today.key);
        expect("a row shown elsewhere is made as it reads now", now?.side === today.side
            && shutSpend.tally.rows === before + 1, JSON.stringify([now?.side, today.side]));
        const made = shutSpend.tally.rows;
        shutLauncher.wanting = true;
        const wanted = shutSpend.everything();
        const again = shutSpend.tally.rows;
        shutSpend.everything();
        expect("wanted again, they're as last written, none made twice", !finding(wanted)
            && wanted.length === spend.everything().length && again - made <= wanted.length
            && shutSpend.tally.rows === again,
            JSON.stringify([again - made, shutSpend.tally.rows - again,
                wanted.map(row => row.name)]));
    }

    Component.onCompleted: begin()
}

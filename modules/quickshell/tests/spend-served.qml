// Drives the launcher's collection of AI spend (launcher/Spend.qml) over
// the rows `spend-watch serve` serves ready, through Quickshell itself,
// from the files spend-watch wrote from a small history beside this, put
// where it writes them and written again as its pass does, with niri, do
// not disturb and the machine daemon stood in for. With no socket it says
// the watch isn't answering and starts it by asking the machine daemon,
// spawning nothing. Served, it answers "spend", "cap" and a report by its
// words, leaves alone what isn't one, steps from a period into its cuts
// and from a cut into its rows, each with its footer, copies a cut's
// report whole at Return, asked of the socket then, holding the launcher
// till it's copied, and saying so when serve doesn't answer, the launcher
// closes first or it's walked shut, flashing y's copy once copied, shows no amount
// while the screen is shared, opens the drawer each row brings as it's
// served, lists the rows in the order served, and puts a finding that
// wants a look first, whose going to its session and marking it expected
// are asked of the socket and say why serve refuses them. Shut, the rows
// written again make none of its rows. A row shown elsewhere reads as
// served, and as none while down but for Start, while shared, or once
// served with no rows. It exits 0 once every case holds,
// and otherwise 1, saying which didn't. Long because its cases run as one
// chain over one real serve, each from where the one before left the
// rows, the launcher and the clipboard, which a split would start again.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "launcher/ActForms.js" as ActForms
import "launcher/Catalogue.js" as Catalogue
import "launcher/SpendDrawer.js" as SpendDrawer
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string collected: realCollection.text()
    readonly property string budgeted: realBudgets.text()


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
        // Each act refused, as its row's key and why.
        property var refusals: []
        // Each act told not done, as its row's name, what and why.
        property var told: []
        // How many times a copy shut it, and what the clipboard held as
        // it did, so a copy is seen landing before the keys go.
        property int shuts: 0
        property string atShut: ""
        // When a copy last shut it, so a failure says whose ask copied: not
        // root's copiedAt, which is when the clipboard last changed.
        property real shutAt: 0
        // What y has done once a copy it ran has copied (Launcher.qml's).
        property var yankedAlone: null
        // What a walk from outside was told, while one runs (Launcher.qml's).
        property var walkTold: null
        // What's typed, the steps it's in and the word it parted with (Launcher.qml's).
        property string typed: ""
        property var held: []
        property string parting: ""
        // The rows listed, the one he's on, the act that kept it, and whether
        // it's letting go or visual holds the rows (Launcher.qml's).
        property var matches: []
        property int current: 0
        property var acted: null
        property bool lettingGo: false
        property bool holdingRows: false

        function wants(source) {
            return wanting;
        }

        // An act's row drawn as it becomes, until it's answered.
        function becomesAll(source, holds, how) {
        }

        function becameNot(key, why) {
            refusals.push(`${key} ${why}`);
        }

        function toldNot(name, act, why) {
            told.push(`${name} ${act} ${why}`);
        }

        function shut() {
            open = false;
        }

        function shutCopied() {
            shuts += 1;
            shutAt = Date.now();
            atShut = Quickshell.clipboardText ?? "";
            open = false;
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
        id: collectionFile
        // Where spend-watch writes it, as serve reads it.
        path: `${root.runtime}/spend-watch/collection.json`
        printErrors: false
    }

    // spend-watch serving what this writes where it writes it, as a VM
    // test's does, so Spend's rows come over the daemon's own socket.
    Process {
        id: serving
        command: [Quickshell.env("SPEND_WATCH"), "--config", Quickshell.env("SERVE_CONFIG"),
            "serve"]
        // What serve says of itself goes on the check's log, so a serve that
        // stops, or can't hear its files, says why rather than a wait
        // running out.
        stderr: SplitParser {
            onRead: line => console.warn(`spend-watch serve: ${line}`)
        }
    }

    // Each case, as the test library says it.
    readonly property var expect: Expect.expect

    // Each wait for the daemon, as the test library waits.
    Until {
        id: waits
    }

    // spend-watch's collection written again, as its pass writes it, which
    // serve hears and tells; once `holds`, on to `then`.
    function rewrite(collection, what, holds, then) {
        collectionFile.setText(JSON.stringify(collection));
        waits.until(what, holds, then);
    }

    // What the machine daemon, stood in for, was asked to do.
    property var restarts: []

    SocketServer {
        active: true
        path: `${root.runtime}/daemons/machine.sock`
        handler: Socket {
            id: machineAsked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    if (ask.ask === "act")
                        root.restarts.push(`${ask.act} ${ask.unit}`);
                    machineAsked.write(`${JSON.stringify({ id: ask.id, done: true })}\n`);
                    machineAsked.flush();
                }
            }
        }
    }

    function collect() {
        // With no socket, it says the watch isn't answering, with a way to
        // start it, rather than going silently.
        const down = spend.everything();
        const asked = spend.answer("spend");
        if (spend.collection?.noun !== "spend" || down.length !== 1
            || down[0].name !== "Start the Watch" || down[0].note !== "not answering"
            || asked?.headline !== "not answering" || asked.rows.length !== 1
            || spend.answer("hello") !== null) {
            const shown = JSON.stringify([down.map(row => [row.name, row.note]), asked]);
            console.warn(`FAIL a collection with no socket says the watch is down: ${shown}`);
            Qt.exit(1);
            return;
        }
        // Every row has a drawer giving a little more than its line (Joel, 2026-11-11
        // 22:00), an act's saying what it does.
        const startDrawer = typeof down[0].drawer === "function" ? down[0].drawer()
            : down[0].drawer;
        // It's a thing, so the launcher draws that drawer (Launcher.qml's
        // drawable), rather than holding a drawer it never shows.
        if (!String(startDrawer?.body ?? "").startsWith("Starts the watch on AI spend")
            || down[0].thing !== true) {
            console.warn(`FAIL Start the Watch says what it does: ${JSON.stringify(startDrawer)}`);
            Qt.exit(1);
            return;
        }
        // Asked while its rows aren't wanted, as Agents' Budgets asks, a down
        // Spend makes no row but its down one, which reads as it is, so a
        // hold on Start isn't let go as though it went (core.nix#6073).
        const start = unwanted("spend:start");
        const budgetsDown = unwanted("spend:budgets");
        if (start?.name !== "Start the Watch" || budgetsDown !== null) {
            console.warn(`FAIL a down Spend makes only its down row: ${
                JSON.stringify([start?.name, budgetsDown])}`);
            Qt.exit(1);
            return;
        }
        down[0].run();
        collectionFile.setText(root.collected);
        budgetsFile.setText(root.budgeted);
        serving.running = true;
        // Serve may first serve what the folder held before these were
        // written, so the wait is for the rows they list, in their order.
        const listed = JSON.parse(root.collected).shown.map(key => `spend:${key}`).join(", ");
        waits.until("the collection served", () => spend.known
            && Object.keys(SpendWatch.budgets).length > 0
            && spend.everything().map(row => row.key).join(", ") === listed, answers);
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

    // The drawer of the row of `key` among `rows`, as the launcher draws
    // one only for a thing (Launcher.qml's detailed), or null.
    function drawnOf(rows, key) {
        const row = rows.find(one => one.key === `spend:${key}`);
        return row?.thing === true && row.drawer ? row.drawer() : null;
    }

    // A cap's row opens its window's drawer, whichever row reaches it, and
    // a period's opens its days and the facts its row carries, named by
    // the vocabulary, words in the words face.
    function capsAndPeriods(rows) {
        const cap = drawnOf(rows, "cap:weekly");
        const window = drawnOf(rows.find(one => one.key === "spend:budgets").verbs(),
            "budget:proxy:weekly");
        expect("a cap's drawer is its window's", cap !== null
            && JSON.stringify(cap) === JSON.stringify(window), cap ? factsOf(cap) : "none");
        const week = drawnOf(rows, "period:this-week");
        expect("a period's drawer", week?.eyebrow === "Mon 2 Nov to Sat 7 Nov \u00b7 week"
            && week.title === "This week" && factsOf(week) === "Read From=the bill~; "
            + "Rebuilding Context=$310 \u00b7 6%; "
            + "Most by Quest=Build the shell \u00b7 31%~; "
            + "Most by Model=claude-opus-5-5 \u00b7 all", week ? factsOf(week) : "none");
    }

    // A budget's row and a window's each open the drawer the daemon serves
    // with it, drawn as served: a window's fact by its own name, the rest by
    // the vocabulary's, each meter beside its figure. A row that brings
    // none opens none, and the Budgets row, which only gathers them, has
    // none of its own.
    function drawers(budgets) {
        SpendWatch.standing = root.standing;
        const rows = budgets.verbs();
        const said = key => {
            const drawer = drawnOf(rows, key);
            return drawer ? factsOf(drawer) : "none";
        };
        const five = drawnOf(rows, "budget:claude:five-hours");
        expect("a window's drawer, as the daemon serves it",
            five?.eyebrow === "in Claude Pro \u00b7 window" && five.eyebrowNames
            && five.title === "Five Hours"
            && factsOf(five) === "Resets=16:20; Read By=Claude at 14:20",
            said("budget:claude:five-hours"));
        const plan = drawnOf(rows, "budget:claude");
        expect("a budget's drawer, its windows by their own names",
            plan?.eyebrow === "your own \u00b7 plan" && !plan.eyebrowNames
            && plan.title === "Claude Pro"
            && factsOf(plan) === "Five Hours=85% \u00b7 resets 16:20:stroke!; "
            + "Worth at List=$84 today", said("budget:claude"));
        expect("a row that brings no drawer opens none", said("budget:delta") === "none",
            said("budget:delta"));
        expect("the Budgets row's own drawer", budgets.thing !== true, "drawn");
        expect("a session's budget", SpendDrawer.sessionBudget(SpendWatch.budgets,
            SpendWatch.budgetOf, "s-running")
            === "Your AI proxy token \u00b7 $37,000 of $40,000 this week"
            && SpendDrawer.sessionBudget(SpendWatch.budgets, SpendWatch.budgetOf, "s-none") === "",
            SpendDrawer.sessionBudget(SpendWatch.budgets, SpendWatch.budgetOf, "s-running"));
    }

    // Each step after the last, failing out loud.
    function step(then) {
        try {
            then();
        } catch (error) {
            console.warn(`FAIL ${error.message}\n${error.stack ?? ""}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }

    function answers() {
        expect("a collection", spend.collection?.noun === "spend", "none");
        expect("starting the watch asks the machine daemon",
            root.restarts.join(", ") === "restart-unit spend-watch.service"
            && Niri.spawned[0] !== "systemctl", root.restarts.join(", "));
        // The session that spent it, by its whole id, for Agents.
        expect("each session's day", /^\$\d/.test(SpendWatch.sessions.s1 ?? ""),
            JSON.stringify(SpendWatch.sessions));
        const today = spend.answer("spend");
        expect("spend", today !== null && /^Today: /.test(today.headline)
            && today.copy.startsWith("## AI spend") && today.rows.length > 0,
            JSON.stringify(today));
        // serve answers with what was written, so its figures pass as
        // current.
        expect("its figures as current", today.note === "Return copies the summary",
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
        // saying more in the footer and copying its report. The periods
        // sit in AI spend itself, behind no row of their own.
        expect("the periods on the first screen", !all.some(row => row.name === "Earlier")
            && all.some(row => row.name === "This week"), all.map(row => row.name).join(", "));
        const thisWeek = all.find(row => row.name === "This week");
        const cuts = thisWeek.verbs();
        expect("a period with its cuts", cuts.length > 0, JSON.stringify(thisWeek));
        const byModel = cuts.find(row => row.name === "By model");
        expect("a cut", byModel && byModel.note !== "" && byModel.verbs().length > 0,
            cuts.map(row => row.name).join(", "));
        const models = byModel.verbs();
        expect("its rows, each copied", models.length === 2
            && models.every(row => typeof row.copies === "function"),
            models.map(row => row.name).join(", "));
        // Its verb, which its footer says, names what Return copies before
        // anything is asked.
        expect("copying its report", byModel.verb === "copy", byModel.verb);
        // y over the cut itself takes this row, so it copies alone and
        // doesn't shut the launcher at once, as its listed Copy does.
        expect("copying only on its own, the launcher not shut at once",
            byModel.alone === true && byModel.now === false,
            JSON.stringify([byModel.alone, byModel.now]));
        // Typing finds a period.
        const found = spend.search("this week").rows;
        expect("a search", found.length > 0 && found[0].name === "This week",
            found.map(row => row.name).join(", "));
        // In the main search the collection adds its best match as a row,
        // unless it answered what's typed: its answer already lists them,
        // so "spend" says each thing once.
        const gathered = spend.gather("this week").rows;
        expect("a row in the main search", gathered.length > 0 && gathered[0].name === "This week",
            gathered.map(row => row.name).join(", "));
        expect("no row beside its answer", spend.gather("spend").rows.length === 0,
            spend.gather("spend").rows.map(row => row.name).join(", "));
        expect("an older file draws nothing", SpendWatch.parsed('{"v":1}', 2) === null, "drawn");
        // The budgets as the watch wrote them: the session that spent today
        // drew on the default, which says its spend in words.
        const drewOn = SpendWatch.budgetOf.s1 ?? "";
        expect("which budget a session drew on", drewOn !== ""
            && /today/.test(SpendWatch.budgets[drewOn]?.said?.spent ?? ""),
            JSON.stringify(SpendWatch.budgets));
        expect("a session's budget, as its drawer says it",
            SpendDrawer.sessionBudget(SpendWatch.budgets, SpendWatch.budgetOf, "s1")
                .startsWith(SpendWatch.budgets[drewOn]?.label ?? "-"),
            SpendDrawer.sessionBudget(SpendWatch.budgets, SpendWatch.budgetOf, "s1"));

        copiedWhole(byModel);
    }

    // When the clipboard last changed, so Return's copy is timed to the
    // moment its words land rather than to the wait's next look.
    property real copiedAt: 0
    Connections {
        target: Quickshell
        function onClipboardTextChanged() {
            root.copiedAt = Date.now();
        }
    }

    // Return on a cut copies its report whole, its words asked of the
    // socket only then, never carried in its row. How long it took goes
    // on the log, to read against the 50 ms Return has to show what it
    // did; the case holds it only to the 2 s the launcher waits, since a
    // busy builder's timing isn't Joel's.
    function copiedWhole(byModel) {
        const held = spend.served.held;
        const rows = Object.keys(held).filter(key => key.startsWith("row:"));
        expect("no row carries a report's words", rows.length > 0
            && !rows.some(key => JSON.stringify(held[key]).includes("## ")),
            rows.filter(key => JSON.stringify(held[key]).includes("## ")).join(", "));
        Quickshell.clipboardText = "";
        const began = Date.now();
        byModel.run();
        const whole = text => text.startsWith("## ") && text.includes("by model");
        waits.until("the report copied", () => whole(Quickshell.clipboardText ?? ""),
            () => step(() => {
                const took = root.copiedAt - began;
                expect("Return copies the report whole", took >= 0 && took < 2000,
                    `${took} ms: ${(Quickshell.clipboardText ?? "").split("\n")[0]}`);
                console.info(`spend: a report copied in ${took} ms`);
                launcherCopies();
            }));
    }

    // The clipboard's words, their first line, as a case says them.
    function clipped() {
        return (Quickshell.clipboardText ?? "").split("\n")[0];
    }

    // The copy a step into a cut lists, as Return reaches it: the cut only
    // copies, so Return steps in, and its copy is listed inside
    // (Catalogue.js's leading), its run the cut's own.
    function cutsCopy() {
        const cut = shutSpend.everything().find(row => row.name === "This week")
            .verbs().find(row => row.name === "By model");
        return Catalogue.leading(cut, cut.verbs(), "").find(row => row.key === "return:copy");
    }

    // SIGCONT in a while whatever a case does, so serve is never left
    // stopped by a case failing while it is.
    Timer {
        id: resume
        interval: 5000
        onTriggered: serving.signal(18)
    }

    // Serve's state as the kernel says it, the letter after its name in
    // /proc's stat. That's the leader thread's, which reads T only once the
    // stop is set on every thread, so none runs on to answer an ask sent
    // after it. Every read blocks, since a reload otherwise reads in the
    // background and text() says the last.
    FileView {
        id: serveStat
        blockAllReads: true
        printErrors: false
    }
    function serveState() {
        serveStat.path = `/proc/${serving.processId}/stat`;
        serveStat.reload();
        const stat = serveStat.text();
        return stat.slice(stat.lastIndexOf(")") + 2).split(" ")[0];
    }

    // In the launcher, Return on a cut keeps it, holding the keys a Wayland
    // program needs to set the clipboard, until its report is copied, then
    // shuts it saying so.
    function launcherCopies() {
        const copy = cutsCopy();
        expect("a step into a cut lists its copy, which keeps the launcher till it's copied",
            copy?.stays === true && copy?.now === false,
            JSON.stringify([copy?.stays, copy?.now]));
        Quickshell.clipboardText = "";
        copy.run();
        expect("and it's open while the report is asked for", shutLauncher.open, "shut at once");
        waits.until("the report copied, then the launcher shut", () => !shutLauncher.open,
            () => step(() => {
                expect("copied whole before it shut, and shut saying so",
                    shutLauncher.atShut.startsWith("## ") && shutLauncher.shuts === 1,
                    `${shutLauncher.shuts}: ${shutLauncher.atShut.split("\n")[0]}`);
                closedFirst();
            }));
    }

    // Closed before the answer, the wait ends, said as closed first, and
    // its answer copies nothing, though the launcher's opened again by
    // the time it lands.
    function closedFirst() {
        shutLauncher.open = true;
        Quickshell.clipboardText = "";
        cutsCopy().run();
        shutLauncher.open = false;
        expect("closed before it's answered, it says so",
            shutLauncher.told.join(", ") === "By model copy the launcher closed first",
            shutLauncher.told.join(", "));
        shutLauncher.told = [];
        shutLauncher.open = true;
        unanswered();
    }

    // With serve not answering, the launcher's own wait ends it, said where
    // Joel is looking, the launcher left open, copying nothing.
    function unanswered() {
        // SIGSTOP: serve holds the ask unanswered, as a stuck daemon would.
        // Asked only once the kernel says serve has stopped, since kill
        // returns before the stop takes: serve answers on a thread of each
        // connection's, and an ask sent sooner was answered 51 ms on, under
        // a loaded builder (Y2YBTS's gate, 2026-11-19, df6d69bf).
        // Cleared first, so a copy said here landed in this case, before
        // or after its ask.
        shutLauncher.shutAt = 0;
        shutLauncher.shuts = 0;
        serving.signal(19);
        // Armed now too, so serve isn't left stopped if the stop's never seen.
        resume.restart();
        const signalled = Date.now();
        waits.until("serve stopped", () => serveState() === "T", () => step(() => {
            // Timed from the ask, so the stop's wait takes none of its hold.
            resume.restart();
            const began = Date.now();
            cutsCopy().run();
            // Ended by a shut too, so a copy that landed says how it ended.
            waits.until("the wait for a report ended",
                () => shutLauncher.told.length > 0 || !shutLauncher.open, () => step(() => {
                    serving.signal(18);
                    resume.stop();
                    const waited = Date.now() - began;
                    // Copied before the ask, it was the case before's late answer.
                    const shut = shutLauncher.shutAt - began;
                    const shutWhen = shut < 0 ? `${-shut} ms before` : `${shut} ms after`;
                    expect("not answered, it's said at once, the launcher left open",
                        shutLauncher.told.join(", ") === "By model copy no answer"
                        && shutLauncher.open && waited >= 1900 && waited < 4000,
                        `${waited} ms, open ${shutLauncher.open}, shut ${shutLauncher.shuts}`
                        + ` times, told ${shutLauncher.told.join(", ")}, copied ${clipped()}`
                        + `, shut by a copy ${shutLauncher.shutAt ? `${shutWhen} asking` : "never"}`
                        + `, asked ${began - signalled} ms after the SIGSTOP was sent`);
                    expect("and copies nothing as it gives up", clipped() === "", clipped());
                    shutLauncher.told = [];
                    overtaken();
                }));
        }));
    }

    // Two Returns in a row: the first's wait ends quietly, and only the
    // second copies. Serve answers in order, so by the second's answer the
    // late answers before it have landed, and they copied and shut nothing.
    function overtaken() {
        shutLauncher.shuts = 0;
        cutsCopy().run();
        cutsCopy().run();
        waits.until("the newer copy copied", () => !shutLauncher.open, () => step(() => {
            expect("a late or overtaken answer copies nothing, and only the newest shuts it",
                shutLauncher.shuts === 1 && shutLauncher.told.length === 0
                && shutLauncher.atShut.startsWith("## "),
                `${shutLauncher.shuts}: ${shutLauncher.told.join(", ")}`);
            movedOn();
        }));
    }

    // A Return still waiting, a walk from outside refused beside it leaves it
    // waiting; typed on before its answer, it copies nothing and says so, the
    // launcher left open under his keys.
    function movedOn() {
        shutLauncher.open = true;
        shutLauncher.shuts = 0;
        Quickshell.clipboardText = "";
        serving.signal(19);
        resume.restart();
        cutsCopy().run();
        shutLauncher.walkTold = [];
        cutsCopy().run();
        shutLauncher.walkTold = null;
        shutLauncher.typed = "x";
        serving.signal(18);
        resume.stop();
        waits.until("the moved-on copy said", () => shutLauncher.told.length > 1,
            () => step(() => {
                expect("typed on first, it says so, copying nothing and leaving it open",
                    shutLauncher.told[1] === "By model copy you moved on before it came"
                    && clipped() === "" && shutLauncher.open && shutLauncher.shuts === 0,
                    `${shutLauncher.told.join(", ")}: ${clipped()}`);
                shutLauncher.told = [];
                shutLauncher.typed = "";
                copiedSince();
            }));
    }

    // Something else copied before its answer, as a y or a copy that keeps
    // the launcher, it leaves that copy be and says so, the launcher open.
    function copiedSince() {
        serving.signal(19);
        resume.restart();
        cutsCopy().run();
        Quickshell.clipboardText = "his own";
        serving.signal(18);
        resume.stop();
        waits.until("the copy after his own said", () => shutLauncher.told.length > 0,
            () => step(() => {
                expect("copied over first, it leaves his copy and says so, the launcher open",
                    shutLauncher.told[0] === "By model copy you moved on before it came"
                    && clipped() === "his own" && shutLauncher.open,
                    `${shutLauncher.told.join(", ")}: ${clipped()}`);
                shutLauncher.told = [];
                stillThere();
            }));
    }

    // Still on the row, though rows came above it, its act kept as Return
    // keeps one before running it, and visual ending as its y copies, the
    // answer copies and shuts it.
    function stillThere() {
        shutLauncher.matches = [{ key: "model" }, { key: "return:copy" }];
        shutLauncher.current = 1;
        shutLauncher.acted = { key: "return:copy" };
        shutLauncher.holdingRows = true;
        shutLauncher.shuts = 0;
        serving.signal(19);
        resume.restart();
        cutsCopy().run();
        shutLauncher.holdingRows = false;
        shutLauncher.matches = [{ key: "day" }].concat(shutLauncher.matches);
        shutLauncher.current = 2;
        serving.signal(18);
        resume.stop();
        waits.until("the copy still there shut it", () => !shutLauncher.open,
            () => step(() => {
                expect("still on its row, it copies and shuts, saying nothing else",
                    shutLauncher.told.length === 0 && shutLauncher.shuts === 1
                    && shutLauncher.atShut.startsWith("## "),
                    `${shutLauncher.shuts}: ${shutLauncher.told.join(", ")}`);
                shutLauncher.matches = [];
                shutLauncher.current = 0;
                shutLauncher.acted = null;
                shutLauncher.shuts = 0;
                shutLauncher.open = true;
                chosenInstead();
            }));
    }

    // Shut by another copy Joel chose instead, a waiting one ends quietly, as
    // an overtaken one does: shut past its late answer and its wait, nothing
    // is said, and the next opening's copy copies alone.
    function chosenInstead() {
        serving.signal(19);
        resume.restart();
        cutsCopy().run();
        shutLauncher.parting = "copied";
        shutLauncher.open = false;
        serving.signal(18);
        resume.stop();
        const shut = Date.now();
        waits.until("past the wait it ended", () => Date.now() - shut > ActForms.COPY_WAIT + 500,
            () => step(() => {
                expect("ended by another copy, its late answer and its wait say nothing",
                    shutLauncher.told.length === 0 && shutLauncher.shuts === 0,
                    `${shutLauncher.shuts}: ${shutLauncher.told.join(", ")}`);
                copiesAfter();
            }));
    }

    // Opened again after a copy chosen instead, Return's copy copies and
    // shuts it, nothing said of the one before.
    function copiesAfter() {
        shutLauncher.parting = "";
        shutLauncher.open = true;
        cutsCopy().run();
        waits.until("the copy after another copied", () => !shutLauncher.open,
            () => step(() => {
                expect("shut by another copy, nothing's said of the one it ended",
                    shutLauncher.told.length === 0 && shutLauncher.shuts === 1,
                    `${shutLauncher.shuts}: ${shutLauncher.told.join(", ")}`);
                yanked();
            }));
    }

    // y over the copy listed inside a cut runs it itself, as Launcher.qml's
    // yanked does, the launcher kept, its row flashed only once the words
    // are on the clipboard; and walked from outside, shut or open, it's told
    // it copies only from the launcher.
    function yanked() {
        shutLauncher.open = true;
        shutLauncher.shuts = 0;
        Quickshell.clipboardText = "";
        const listed = cutsCopy();
        const done = ActForms.yanking([{ at: 3, action: listed }], "", {});
        expect("y over a cut's listed copy runs it, flashing it only once it's copied",
            done.run === listed && done.flashed.length === 0 && done.later.join() === "3",
            JSON.stringify([done.run === listed, done.flashed, done.later]));
        let flashed = null;
        shutLauncher.yankedAlone = () => {
            flashed = clipped();
        };
        done.run.run();
        shutLauncher.yankedAlone = null;
        expect("y's flash waits for the answer", flashed === null, flashed);
        waits.until("y's copy flashed", () => flashed !== null, () => step(() => {
            expect("y flashes once it's copied, the launcher kept",
                flashed.startsWith("## ") && shutLauncher.open && shutLauncher.shuts === 0,
                `${shutLauncher.open} ${shutLauncher.shuts}: ${flashed}`);
            shutLauncher.open = false;
            Quickshell.clipboardText = "";
            cutsCopy().run();
            expect("walked with the launcher shut, it's told at once it copies only from it",
                shutLauncher.told.join(", ")
                === "By model copy it copies only from the launcher, with Return"
                && clipped() === "", shutLauncher.told.join(", "));
            shutLauncher.told = [];
            shutLauncher.open = true;
            shutLauncher.walkTold = [];
            cutsCopy().run();
            shutLauncher.walkTold = null;
            expect("walked with it open, it's told so too, and the launcher's left open",
                shutLauncher.told.join(", ")
                === "By model copy it copies only from the launcher, with Return"
                && shutLauncher.open, shutLauncher.told.join(", "));
            shutLauncher.told = [];
            // While sharing, no row and no answer holds an amount. The quiet
            // place is a place of its own, served apart, so its rows may
            // come after the ones the collection was waited for by.
            Quiet.sharing = true;
            waits.until("the quiet rows served", () => spend.everything().length > 0,
                quietServed);
        }));
    }

    function quietServed() {
        const quiet = spend.everything();
        expect("quiet rows", quiet.length > 0
            && quiet.every(row => !`${row.name} ${row.side}`.includes("$")),
            quiet.map(row => `${row.name}: ${row.side}`).join(", "));
        const hidden = spend.answer("spend last week by quest");
        expect("a quiet answer", hidden !== null && !hidden.headline.includes("$")
            && hidden.rows.every(row => !`${row.name} ${row.side}`.includes("$")),
            JSON.stringify(hidden));
        Quiet.sharing = false;
        rewrite(budgetsCollection(), "the budgets served",
            () => spend.everything()[0]?.name === "Budgets", () => step(budgetsServed));
    }

    // The collection with a budgets row first, its windows and budgets
    // behind it, each with the drawer the daemon writes, a cap and a
    // period.
    function budgetsCollection() {
        const budgeted = JSON.parse(root.collected);
        // A dollar window's drawer, on its row and on its cap's.
        const weekly = { eyebrow: "in Your AI proxy token \u00b7 window", names: true,
            title: "Week", facts: [{ id: "standing", value: "$37,000 of $40,000",
                figure: { shape: "stroke", values: [0.925], warn: true } },
            { id: "read", value: "the AI proxy at 21:35" }] };
        const drawers = {
            "budget:claude:five-hours": { eyebrow: "in Claude Pro \u00b7 window",
                names: true, title: "Five Hours", facts: [
                    { id: "resets", value: "16:20" },
                    { id: "read", value: "Claude at 14:20" }] },
            "budget:claude": { eyebrow: "your own \u00b7 plan", title: "Claude Pro", facts: [
                { id: "", label: "Five Hours", value: "85% \u00b7 resets 16:20",
                    figure: { shape: "stroke", values: [0.85], warn: true } },
                { id: "worth", value: "$84 today" }] },
            "budget:proxy:weekly": weekly
        };
        const window = (key, name, side, role) => ({ key: key, name: name, side: side,
            copy: `${name}: ${side}`, role: role, mark: "clock" });
        const budget = (key, name, side, role) => ({ key: key, name: name, side: side,
            copy: `${name}: ${side}`, role: role, mark: "agent" });
        budgeted.now = [{
            key: "budgets", name: "Budgets", side: "$100 this month",
            copy: "Budgets: $100 this month", role: "warning", mark: "agent",
            rows: [
                window("budget:claude:five-hours", "Five Hours", "85%", "warning"),
                window("budget:claude:week", "Week", "40%", "text"),
                window("budget:proxy:premium", "Premium Day", "$0 of $300", "text"),
                budget("budget:claude", "Claude Pro", "85% of 5 h", "warning"),
                budget("budget:proxy", "Your AI proxy token", "$37,000 of $40,000", "warning"),
                window("budget:proxy:weekly", "Week", "$37,000 of $40,000", "warning"),
                budget("budget:delta", "Delta team token", "$40 of $50", "text")
            ].map(row => Object.assign({ budget: row.key }, row,
                drawers[row.key] ? { drawn: drawers[row.key] } : {}))
        }, {
            key: "cap:weekly", name: "Week", side: "93%",
            copy: "Week: 93% of $40,000", mark: "clock", role: "warning",
            budget: "budget:proxy:weekly", drawn: weekly
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
        // Listed as the daemon would list them.
        budgeted.shown = budgeted.now.map(line => line.key);
        return budgeted;
    }

    // A budget wears AI spend's mark and a window a clock, muted at rest
    // and the warning's colour past it, drawn on a figure row too; a window
    // is named and copied as its feed names it.
    function budgetsServed() {
        const budgets = spend.everything()[0];
        expect("a budget's mark in the warning's colour",
            budgets.glyph === Theme.glyph.agent && budgets.hue === "warning" && budgets.marked,
            JSON.stringify(budgets));
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
        // Shown elsewhere, it reads as served, and as none while the screen
        // is shared, whose quiet place holds no Budgets, rather than with
        // its amounts (core.nix#6073).
        const elsewhere = unwanted("spend:budgets");
        Quiet.sharing = true;
        const shared = unwanted("spend:budgets");
        Quiet.sharing = false;
        expect("shown elsewhere it reads as served, and as none while sharing",
            elsewhere?.side === "$100 this month" && shared === null,
            JSON.stringify([elsewhere?.side, shared]));

        // The rows come as the daemon placed them, the shell ranking none
        // (2026-11-10).
        const placed = JSON.parse(root.collected);
        placed.shown = placed.shown.slice().reverse();
        const order = placed.shown.map(key => `spend:${key}`).join(", ");
        rewrite(placed, "the rows placed again",
            () => spend.everything().map(row => row.key).join(", ") === order,
            () => step(findingServed));
    }

    // A finding with an action goes to its session, and steps into marking
    // it expected. The daemon lists it first, and first in the summary's
    // rows.
    function findingServed() {
        const shown = JSON.parse(root.collected);
        shown.findings = [{
            key: "finding:burn:s1", name: "A burn in Cistern", side: "alert",
            copy: "A burn in Cistern.", role: "alert",
            run: ["spend-watch", "sessions", "s1", "go"], verb: "go to",
            expect: ["spend-watch", "findings", "burn:s1", "expect"]
        }];
        shown.shown = ["finding:burn:s1"].concat(shown.shown);
        shown.summaryRows = ["finding:burn:s1"].concat(shown.summaryRows);
        rewrite(shown, "a finding served",
            () => spend.everything()[0]?.name === "A burn in Cistern", () => step(() => {
                const burn = spend.everything()[0];
                expect("a finding first", burn.verb === "go to" && burn.hue === "alert",
                    `${burn.verb} ${burn.hue}`);
                // The answer for "spend" leads with it too, as the
                // collection does, before the rows of how things stand.
                const answered = spend.answer("spend").rows;
                expect("the answer's finding first", answered[0]?.name === "A burn in Cistern",
                    answered.map(row => row.name).join(", "));
                // Asked of serve, which acts on nothing, each act says
                // why, spawning nothing.
                Niri.spawned = [];
                const told = shutSpend.everything()[0];
                told.run();
                told.verbs().find(row => row.name === "It's Expected").run();
                waits.until("its acts answered", () => shutLauncher.refusals.length === 2,
                    () => step(refusedActs));
            }));
    }

    function refusedActs() {
        // In words.json's words for serve's refusal, not its message.
        const why = "still reading";
        expect("asking serve to act runs nothing and says why", Niri.spawned.length === 0
            && shutLauncher.refusals.join(", ") === `spend:finding:burn:s1 ${why}, `
            + `spend:finding:burn:s1:expect ${why}`,
            [Niri.spawned.join(" ")].concat(shutLauncher.refusals).join(", "));
        root.shut();
    }

    // With the launcher shut, spend-watch writing its file again, as it
    // does every few seconds, makes none of the collection's rows, nor does
    // a pick asking every collection what it holds (Launcher.qml's
    // knowing): every row was made again at each write, 90 in four minutes
    // on Joel's shell (YQZGPC, 2026-11-10). Home's figure reads what was
    // served, and a row shown elsewhere is made as it reads now. Wanted
    // again, they're as last served, none made twice.
    function shut() {
        const finding = rows => rows.some(row => row.name === "A burn in Cistern");
        expect("wanted, it lists what was last served", finding(shutSpend.everything()),
            shutSpend.everything().map(row => row.name).join(", "));
        shutLauncher.wanting = false;
        const before = shutSpend.tally.rows;
        rewrite(JSON.parse(root.collected), "the finding gone, while shut",
            () => !shutSpend.served.keys.includes("row:spend:finding:burn:s1"), () => step(() => {
                const held = shutSpend.everything();
                expect("shut, spend-watch writing again makes none of its rows",
                    shutSpend.tally.rows === before && finding(held),
                    shutSpend.tally.rows - before);
                expect("and home's figure says what was served",
                    shutSpend.counted() === spend.counted(),
                    `${shutSpend.counted()} against ${spend.counted()}`);
                const today = spend.everything()[0];
                const now = shutSpend.rowNow(today.key);
                expect("a row shown elsewhere is made as it reads now", now?.side === today.side
                    && shutSpend.tally.rows <= before + 1,
                    JSON.stringify([now?.side, today.side, shutSpend.tally.rows - before]));
                const made = shutSpend.tally.rows;
                shutLauncher.wanting = true;
                const wanted = shutSpend.everything();
                const again = shutSpend.tally.rows;
                shutSpend.everything();
                expect("wanted again, they're as last served, none made twice",
                    !finding(wanted) && wanted.length === spend.everything().length
                    && again - made <= wanted.length && shutSpend.tally.rows === again,
                    JSON.stringify([again - made, shutSpend.tally.rows - again,
                        wanted.map(row => row.name)]));
                emptied();
            }));
    }

    // Answered with no rows, as after a restart with no collection, a row
    // shown elsewhere is none, not what was last listed while wanted
    // (core.nix#6073).
    function emptied() {
        const none = JSON.parse(root.collected);
        // A report's cuts are rows of the place too, so they go with it.
        none.shown = [];
        none.summaryRows = [];
        none.reports = Array.isArray(none.reports) ? [] : {};
        rewrite(none, "no rows served", () => shutSpend.served.keys.length === 0,
            () => step(() => {
                const budgets = unwanted("spend:budgets");
                expect("answered with no rows, a row shown elsewhere is none",
                    budgets === null, JSON.stringify(budgets));
                Qt.callLater(() => Qt.exit(0));
            }));
    }

    // What the shut Spend says a row reads now while its rows aren't
    // wanted, as Agents' Budgets and Now's drawers ask it, wanted again after.
    function unwanted(key) {
        shutLauncher.wanting = false;
        const now = shutSpend.rowNow(key);
        shutLauncher.wanting = true;
        return now;
    }


    // Spend over the files spend-watch wrote, read once they're there.
    Component.onCompleted: waits.until("the written files read, and the socket heard down",
        () => root.collected !== "" && root.budgeted !== "" && shutSpend.down, collect)
}

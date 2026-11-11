// Drives the launcher's collection of AI spend (launcher/Spend.qml) over
// the rows `spend-watch serve` serves ready, through Quickshell itself,
// from the files spend-watch wrote from a small history beside this, put
// where it writes them and written again as its pass does, with niri, do
// not disturb and the machine daemon stood in for. With no socket it says
// the watch isn't answering and starts it by asking the machine daemon,
// spawning nothing. Served, it answers "spend", "cap" and a report by its
// words, leaves alone what isn't one, steps from a period into its cuts
// and from a cut into its rows, each with its footer, shows no amount
// while the screen is shared, opens the drawer each row brings as it's
// served, lists the rows in the order served, and puts a finding that
// wants a look first, whose going to its session and marking it expected
// are asked of the socket and say why serve refuses them. Shut, the rows
// written again make none of its rows. It exits 0 once every case holds,
// and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "launcher/SpendDrawer.js" as SpendDrawer

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

        function wants(source) {
            return wanting;
        }

        // An act's row drawn as it becomes, until it's answered.
        function becomesAll(source, holds, how) {
        }

        function becameNot(key, why) {
            refusals.push(`${key} ${why}`);
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
    }

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    // Waits for `holds`, ten seconds at most, then goes on to `then`, or
    // fails saying `what`; `then` failing fails too.
    function until(what, holds, then) {
        waits.what = what;
        waits.holds = holds;
        waits.then = then;
        waits.left = 200;
        waits.start();
    }

    Timer {
        id: waits

        property string what: ""
        property var holds: null
        property var then: null
        property int left: 0

        interval: 50
        repeat: true
        onTriggered: {
            try {
                if (holds()) {
                    stop();
                    then();
                } else if (--left === 0) {
                    stop();
                    throw new Error(`${what} never held`);
                }
            } catch (error) {
                stop();
                console.warn(`FAIL ${error.message}`);
                Qt.callLater(() => Qt.exit(1));
            }
        }
    }

    // spend-watch's collection written again, as its pass writes it, which
    // serve hears and tells; once `holds`, on to `then`.
    function rewrite(collection, what, holds, then) {
        collectionFile.setText(JSON.stringify(collection));
        until(what, holds, then);
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
        down[0].run();
        collectionFile.setText(root.collected);
        budgetsFile.setText(root.budgeted);
        serving.running = true;
        until("the collection served", () => spend.known
            && Object.keys(SpendWatch.budgets).length > 0, answers);
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
            console.warn(`FAIL ${error.message}`);
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
        expect("copying its report", byModel.verb === "copy", byModel.verb);
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
            key: "finding:burn:s1", name: "A burn in Aquifer", side: "alert",
            copy: "A burn in Aquifer.", role: "alert",
            run: ["spend-watch", "go", "s1"], verb: "go to",
            expect: ["spend-watch", "findings", "expect", "burn:s1"]
        }];
        shown.shown = ["finding:burn:s1"].concat(shown.shown);
        shown.summaryRows = ["finding:burn:s1"].concat(shown.summaryRows);
        rewrite(shown, "a finding served",
            () => spend.everything()[0]?.name === "A burn in Aquifer", () => step(() => {
                const burn = spend.everything()[0];
                expect("a finding first", burn.verb === "go to" && burn.hue === "alert",
                    `${burn.verb} ${burn.hue}`);
                // The answer for "spend" leads with it too, as the
                // collection does, before the rows of how things stand.
                const answered = spend.answer("spend").rows;
                expect("the answer's finding first", answered[0]?.name === "A burn in Aquifer",
                    answered.map(row => row.name).join(", "));
                // Asked of serve, which acts on nothing, each act says
                // why, spawning nothing.
                Niri.spawned = [];
                const told = shutSpend.everything()[0];
                told.run();
                told.verbs().find(row => row.name === "It's Expected").run();
                until("its acts answered", () => shutLauncher.refusals.length === 2,
                    () => step(refusedActs));
            }));
    }

    function refusedActs() {
        // In words.json's words for serve's refusal, not its message.
        const why = "the watch can't act yet";
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
        const finding = rows => rows.some(row => row.name === "A burn in Aquifer");
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
                Qt.callLater(() => Qt.exit(0));
            }));
    }


    // Spend over the files spend-watch wrote, read once they're there.
    Component.onCompleted: until("the written files read",
        () => root.collected !== "" && root.budgeted !== "", collect)
}

// What AI use cost, a collection, from what spend-watch tells (SpendWatch.qml)
// (modules/spend-watch). It's laid out as the other collections are: each
// row a short name with a few muted words beside it, what else is known of
// it in the footer while it's picked, and the rest a right arrow away,
// behind the row it belongs to. First what was found that wants a look;
// then how things stand: today, with a day's cap behind it, each longer
// cap, with the week ahead behind the first, and the periods before; then
// the rest of what was found, and the reports that belong behind nothing
// else. A period steps into its cuts and a cut into its rows; a cap steps
// into its changes and the sign-ins its feed offers; a budget's row, a
// window's and a cap's have a drawer of every fact budgets.json holds of
// it, and a period's a drawer of its days and what its cuts went to most
// (SpendDrawer.js). Return copies a row, or a report whole as markdown,
// to paste where it's asked about; a finding with something to do does
// it, and steps into marking it expected.
//
// It answers in the field too: "spend" or "ai" for how today stands, "cap"
// for the caps, and a report by its words, "spend last week by quest" or
// "spend today by model", whose words the daemon gives with the reports. The
// daemon works out every figure and every word, so nothing here counts or
// formats; with nothing told there's no collection, and nothing to answer. While
// the screen is shared it lists the daemon's quiet lines, which hold no
// amounts, and no report, and answers only that the amounts are hidden.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "SpendDrawer.js" as SpendDrawer

Provider {
    id: provider

    what: "AI spend"

    readonly property bool known: SpendWatch.known
    readonly property bool sharing: Quiet.sharing
    readonly property var reports: SpendWatch.reports

    // Named, and found, as the vocabulary says (Words.qml): a part of
    // Agents, wearing their glyph, and no longer found by pi's names.
    // While there's nothing to tell, as when the watch never ran, it's
    // there still, saying so, with a way to start it.
    collection: known || SpendWatch.down ? Words.collection("spend") : null
    examples: SpendWatch.examples

    // Only what's drawn is watched: the rows while the launcher is open,
    // and the reports while these rows are wanted. A check's, with no
    // launcher, watches everything.
    Binding {
        target: SpendWatch
        property: "listing"
        value: provider.launcher?.open ?? true
        when: provider.launcher !== null
    }
    Binding {
        target: SpendWatch
        property: "reporting"
        value: provider.wanted
        when: provider.launcher !== null
    }
    Binding {
        target: SpendWatch
        property: "paced"
        value: provider.launcher
        when: provider.launcher !== null
    }

    // While the rows or the reports are read for this opening, or the
    // daemon's first pass is, what was held is drawn, and a report never
    // held is said as "\u2026".
    stillReading: SpendWatch.rowsReading || SpendWatch.reportsReading || SpendWatch.lastRun

    // The one row while there's nothing to tell: the watch isn't
    // answering, and starting it.
    readonly property var downLine: ({
        key: "start",
        name: Words.name("spend:start"),
        side: "",
        note: Words.of("spend").said?.down ?? "",
        role: "warning",
        verb: Words.verb("spend:start"),
        run: ["systemctl", "--user", "restart", "spend-watch.service"]
    })

    Connections {
        target: SpendWatch
        // A session's or a quest's figure is Agents' alone, so it changes
        // nothing here.
        function onHeard(changed, gone) {
            if (changed.concat(gone).some(key => !key.startsWith("session:")
                && !key.startsWith("quest:")))
                provider.changed();
        }
    }

    // Each row of the lists kept by its record's key while its line, and
    // the report it stands for, are the ones held before (keptAlike): a
    // record SpendWatch didn't hear change is the same object, so a pass
    // remakes only the rows whose records it changed.
    readonly property var keeping: keeper()

    function made(one) {
        const given = one.line;
        return keptAlike(keeping, one.key,
            [given, given.report ? reports[given.report] ?? null : null, sharing],
            () => build(given));
    }

    // Each row's mark and colour by its role: what AI use has cost for how
    // things stand, a trend for what was found, the warning or the urgent
    // mark for a finding that wants a look, all's fine for one marked
    // expected, the report's for a row standing for one, and signing in's.
    // The warning is the theme's caution, as every other warning wears.
    readonly property var roleMarks: ({
        "alert": "urgent",
        "warning": "caution",
        "found": "trend",
        "expected": "fine",
        "report": "report",
        "sign-in": "signIn"
    })

    function glyph(role) {
        return Theme.glyph[roleMarks[role] ?? "cost"];
    }

    function hue(role) {
        return role === "alert" || role === "warning" ? role
            : role === "sign-in" ? "action"
            : role === "expected" ? "muted" : "voice";
    }

    // A row whose role colours a mark of its own, a budget's, a window's or
    // a day's: muted at rest, the warning or the alert past them, and drawn
    // on a figure row too (VKCF3Z, 2026-11-07).
    function markedHue(role) {
        return role === "alert" || role === "warning" ? role : "muted";
    }

    // The vocabulary's words for spend's drawers (SpendDrawer.js), and
    // whether the screen is shared, when its dollars go as the rows' do.
    readonly property var budgetWords: ({
        name: id => Words.name(`spend:${id}`),
        unread: Words.of("spend").said?.unread ?? "",
        window: Words.of("spend").said?.window ?? "",
        sharing: provider.sharing
    })

    // A row as the daemon gave it, with what's behind it made only when
    // it's stepped into, since a month's cuts are hundreds of rows.
    function build(given) {
        const report = given.report ? reports[given.report] ?? null : null;
        const marked = (given.mark ?? "") !== "";
        const fields = {
            key: `spend:${given.key}`,
            name: given.name,
            side: given.side,
            note: given.note ?? "",
            glyph: marked ? Theme.glyph[given.mark] ?? glyph(given.role) : glyph(given.role),
            hue: marked ? markedHue(given.role) : hue(given.role),
            marked: marked
        };
        // A row naming a budget or a window opens a drawer of every fact
        // budgets.json holds of it, read as it's shown, so the daemon's next
        // pass reaches an open drawer; a period's opens its own facts. Each
        // is a thing, as the launcher's drawer rule asks of a row with
        // details (Launcher.qml's detailed).
        if (given.budget || given.drawn) {
            fields.thing = true;
            fields.drawer = () => SpendDrawer.drawer(given, SpendWatch.budgets, budgetWords);
        }
        const row = given.run
            ? thing(Object.assign(fields, {
                verb: given.verb || "run",
                later: true,
                run: () => Niri.spawn(given.run)
            }))
            : copyRow(Object.assign(fields, { copy: report ? report.markdown : given.copy }));
        const nested = given.rows ?? [];
        const inside = report ? report.rows : [];
        // A finding Return acts on can still be copied, and one not yet
        // marked can be.
        const doing = [];
        if (given.run && given.key.startsWith("finding:"))
            doing.push(copyRow({
                key: `spend:${given.key}:copy`,
                name: Words.name("spend:copy"),
                copy: given.copy,
                figure: false,
                glyph: glyph(given.role),
                hue: hue(given.role)
            }));
        if (given.expect)
            doing.push(thing({
                key: `spend:${given.key}:expect`,
                name: Words.name("spend:expect"),
                verb: Words.verb("spend:expect"),
                glyph: Theme.glyph.fine,
                hue: "action",
                run: () => Niri.spawn(given.expect)
            }));
        if (doing.length + nested.length + inside.length > 0) {
            row.verbs = () => doing.concat(nested.map(build), inside.map(build));
            row.nested = true;
            // Behind most rows are more of them to choose from, not
            // things to do.
            if (doing.length === 0)
                row.ask = "which?";
        }
        // Found by the words the daemon gives its report too: "waste" for
        // the context rebuilt, "model" for a period cut by model.
        if (report)
            row.found = Fuzzy.prepare(`${report.title} ${report.words ?? ""}`);
        return row;
    }

    function matched(row, words) {
        const name = Fuzzy.match(row.prepared, words, false);
        if (name.score >= 0 || !row.found)
            return name;
        const byWord = Fuzzy.match(row.found, words, false);
        // A report's words count for less than its name.
        return byWord.score < 0 ? byWord : { score: byWord.score / 2, grade: byWord.grade };
    }

    // A finding that wants a look comes first, under the cursor as the
    // collection opens; the rest come after how things stand.
    function asking(given) {
        return given.role === "alert" || given.role === "warning";
    }

    function ordered(rows, found) {
        return found.filter(asking).concat(rows, found.filter(given => !asking(given)));
    }

    // Made only while they're wanted (Provider.qml's whileWanted), since
    // every row was made again at each of the daemon's passes: 90 in four
    // minutes with the launcher shut on Joel's shell (YQZGPC, 2026-11-10).
    // Home's figure is SpendWatch's `today`.
    readonly property var all: whileWanted("all", () => {
        keepingBegun(keeping);
        const rows = listedRows().map(made);
        keptOnly(keeping);
        return rows;
    })

    // The rows the daemon gives to be listed, each { key, line }, in order.
    function listedRows() {
        const rows = SpendWatch.rows;
        return SpendWatch.down ? [{ key: "start", line: downLine }]
            : !known ? []
            : sharing ? orderedRows([], rows.quiet)
            : orderedRows(rows.now, rows.findings).concat(rows.more);
    }

    function orderedRows(rows, found) {
        const asks = one => asking(one.line);
        return found.filter(asks).concat(rows, found.filter(one => !asks(one)));
    }

    // What the daemon gives to be listed, as it gives it.
    function listed() {
        return listedRows().map(one => one.line);
    }

    // A row of the list as it reads now, for those who show it while these
    // rows aren't wanted: Now's Spend and Tokens borrowing its drawer, and
    // Agents' Budgets. Made then, of the rows the list holds; a row inside
    // another, and every row while they're wanted, when `all` is as it
    // reads now, is looked for where it's shown.
    function rowNow(key) {
        if (provider.wanted)
            return undefined;
        const given = listed().find(one => `spend:${one.key}` === key);
        return given === undefined ? undefined : build(given);
    }

    onSharingChanged: changed()

    function everything() {
        return all;
    }

    // After its title, at home and in Agents, how today stands with its
    // period said, "$412 today", as All AI Spend says it (VKCF3Z,
    // 2026-11-08); while the screen is shared, the first quiet line's,
    // which holds no amount. Until the daemon writes `today`, as just
    // after a switch, today's row's side.
    function counted() {
        if (!sharing && SpendWatch.today !== "")
            return SpendWatch.today;
        const today = (sharing ? SpendWatch.rows.quiet : SpendWatch.rows.now)[0];
        return today ? today.line.side : "";
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const found = [];
        for (const row of all) {
            const m = matched(row, words);
            if (m.score >= 0)
                found.push({ item: row.prepared, score: m.score, grade: m.grade, row: row });
        }
        const kept = Fuzzy.keep(found);
        return { rows: kept.map(f => f.row), grade: kept.length > 0 ? kept[0].grade : 0 };
    }

    // The id of the period or cut whose words begin what's typed, longest
    // first, and what's left after them.
    function named(text, choices) {
        // Quickshell's JavaScript has no flatMap.
        const words = [];
        for (const choice of choices)
            for (const word of choice.words)
                words.push({ id: choice.id, word: word });
        words.sort((a, b) => b.word.length - a.word.length);
        const hit = words.find(w => text === w.word || text.startsWith(`${w.word} `));
        return hit ? { id: hit.id, rest: text.slice(hit.word.length).trim() } : null;
    }

    // Which report what's typed asks for, by id, or null. Named apart from
    // Provider's `asked`, the answers each URL gave, which it would hide.
    function reportFor(typed) {
        let text = typed.trim().toLowerCase().replace(/\s+/g, " ");
        if (/^(caps?|ai caps?|spend caps?)$/.test(text))
            return "caps";
        const prefix = /^(ai spend|spend|ai)( |$)/.exec(text);
        if (prefix === null)
            return null;
        text = text.slice(prefix[0].length).trim();
        if (text === "")
            return "summary";
        if (["waste", "caps", "cap", "forecast", "quests", "outliers"].includes(text))
            return text === "cap" ? "caps" : text;
        const period = named(text, SpendWatch.periods);
        const rest = period ? period.rest : text;
        const by = /^by (.+)$/.exec(rest);
        const cut = by ? named(by[1], SpendWatch.cuts) : null;
        if ((rest !== "" && (cut === null || cut.rest !== "")) || (period === null && cut === null))
            return null;
        return `${period ? period.id : "this-week"}/${cut ? cut.id : "quest"}`;
    }

    // While the reports are first read, held in its place as Firmware's
    // answer is, rather than said as none.
    function waitingAnswer() {
        return {
            headline: "\u2026",
            rows: [],
            asking: true,
            note: "",
            live: provider
        };
    }

    // Beside each answer, unless the daemon answers with this run's
    // figures: its last figures, or its last run's, are said as such
    // rather than passing as current.
    function noted(note) {
        const said = Words.of("spend").said ?? {};
        return SpendWatch.fromFile ? said.lastFigures ?? note
            : SpendWatch.lastRun ? said.lastRun ?? note : note;
    }

    function answer(typed) {
        const id = reportFor(typed);
        if (SpendWatch.down)
            return id === null ? null : {
                headline: downLine.note,
                copy: "",
                rows: [build(downLine)],
                note: ""
            };
        if (!known)
            return null;
        const shown = id === null ? null : reports[id] ?? null;
        if (shown === null)
            return id !== null && SpendWatch.reportsReading ? waitingAnswer() : null;
        const quiet = SpendWatch.rows.quiet.map(one => one.line);
        const now = SpendWatch.rows.now.map(one => one.line);
        const found = SpendWatch.rows.findings.map(one => one.line);
        if (sharing)
            return {
                // In lower case, as every other answer's headline is.
                headline: "amounts are hidden while the screen is shared",
                copy: "",
                rows: ordered([], quiet).map(build),
                note: "ask again once sharing ends"
            };
        if (id === "summary") {
            const lead = now[0];
            return {
                headline: lead ? lead.copy : shown.headline,
                copy: shown.markdown,
                rows: ordered(now.slice(1), found).map(build),
                note: noted("Return copies the summary")
            };
        }
        return {
            headline: `${shown.title}: ${shown.headline}`,
            copy: shown.markdown,
            rows: shown.rows.map(build),
            // Not "the report", which in Agents is a quest's document
            // (docs/glossary.md).
            note: noted("Return copies it whole")
        };
    }
}

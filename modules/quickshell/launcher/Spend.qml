// What AI use cost, a collection, its rows served ready by spend-watch
// (modules/spend-watch, its src/ready.rs) and held as every place's are
// (ReadyRows.qml). It's laid out as the other collections are: each row a
// short name with a few muted words beside it, what else is known of it
// in the footer while it's picked, and the rest a right arrow away,
// behind the row it belongs to. First what was found that wants a look;
// then how things stand: today, with a day's cap behind it, each longer
// cap, with the week ahead behind the first, and the periods before; then
// the rest of what was found, and the reports that belong behind nothing
// else. A period steps into its cuts and a cut into its rows; a cap steps
// into its changes and the sign-ins its feed offers; a budget's row, a
// window's and a cap's have a drawer of how it stands, and a period's a
// drawer of its days and what its cuts went to most. Return copies a row,
// or a report whole as markdown, to paste where it's asked about; a
// finding with something to do does it, and steps into marking it
// expected, each asked of spend-watch on the rows' own socket.
//
// spend-watch orders, words and lays out every row, its drawer and its
// acts, so nothing here sorts, counts, words or builds one (Joel,
// 2026-11-10: "the daemons need to be the data layer, and they need to
// serve the launcher"). Spend made its rows here from the collection's
// records before, and read the collection's file whole while the socket
// wasn't there; a check or a VM test standing in for the daemon now runs
// `spend-watch serve` over the files it writes. The rows are watched
// whether or not the launcher's open, so it opens on rows already held,
// and made into the launcher's only while they're wanted.
//
// It answers in the field too: "spend" or "ai" for how today stands, "cap"
// for the caps, and a report by its words, "spend last week by quest" or
// "spend today by model", whose words the daemon gives with the reports
// (SpendWatch.qml), each answer's rows the ones it serves inside the row
// standing for that report, or inside the answer where none does. While
// the screen is shared it lists the daemon's quiet place, which holds no
// amounts, and no report, and answers only that the amounts are hidden.
// With no socket answering, there's one row saying so, with a way to
// start the watch.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "AI spend"

    readonly property bool sharing: Quiet.sharing
    readonly property var reports: SpendWatch.reports

    // The place read now: the quiet one while the screen is shared.
    readonly property var place: sharing ? quiet : ready

    // Whether spend-watch's socket isn't answering, so there's nothing to
    // tell but that, and whether any row is held to tell.
    readonly property bool down: ready.state === "not answering"
    readonly property bool known: {
        void ready.placings;
        return ready.keys.length > 0;
    }
    // Whether the daemon is still on its first pass, telling what its last
    // run wrote, so those figures are said as still being read rather
    // than passing as current.
    readonly property bool lastRun: known && ready.state === "starting"

    // Named, and found, as the vocabulary says (Words.qml): a part of
    // Agents, wearing their glyph, and no longer found by pi's names.
    collection: known || down ? Words.collection("spend") : null
    examples: SpendWatch.examples

    // Spend's rows as spend-watch serves them, and the ones it lists while
    // the screen is shared, each watched always so neither waits on a
    // read as the launcher opens.
    ReadyRows {
        id: ready
        socket: SpendWatch.socket
        place: "spend"
        beside: ["summary"]
        make: (row, key) => provider.made(ready, row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
    }
    ReadyRows {
        id: quiet
        socket: SpendWatch.socket
        place: "spend-quiet"
        beside: ["summary"]
        make: (row, key) => provider.made(quiet, row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
    }
    // What the daemon serves, for a reader told of each change it hears.
    readonly property alias served: ready

    // The budgets its drawers and Agents' say are watched while the
    // launcher is open, and the reports while these rows are wanted. A
    // check's, with no launcher, watches everything.
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

    // While the reports are read for this opening, or the daemon's first
    // pass is, what was held is drawn, and a report never held is said as
    // "\u2026".
    stillReading: SpendWatch.reportsReading || lastRun

    Connections {
        target: ready
        function onHeard(changed, gone) {
            if (!provider.sharing)
                provider.changed();
        }
    }
    Connections {
        target: quiet
        function onHeard(changed, gone) {
            if (provider.sharing)
                provider.changed();
        }
    }
    onSharingChanged: changed()
    onDownChanged: changed()

    // A row as spend-watch serves it, made as every place's ready rows are
    // (ReadyRows.qml's thing), with what its `held` says of it: whether its
    // mark is its own, the verb its Return says, whether a copy is a
    // figure, and the report words it's found by too, "waste" for the
    // context rebuilt, "model" for a period cut by model.
    function made(from, served, key) {
        const held = served.held ?? {};
        const first = (served.acts ?? [])[0] ?? {};
        const fields = { marked: held.marked === true };
        if (typeof held.verb === "string")
            fields.verb = held.verb;
        // Going to a session or signing in shows once it's done, so the
        // launcher waits on it before it closes.
        if (first.key === "verb:spend:run")
            fields.later = true;
        if (held.figure === false)
            fields.figure = false;
        const row = from.thing(served, key, fields);
        if (row && typeof held.also === "string" && held.also !== "") {
            let found = null;
            Object.defineProperty(row, "found", {
                enumerable: true,
                get: () => found ?? (found = Fuzzy.kept(held.also))
            });
        }
        return row;
    }

    // The one row while the socket isn't answering, and starting the
    // watch, which isn't there to ask, so the machine daemon does.
    readonly property string downNote: Words.of("spend").said?.down ?? ""
    function downRow() {
        return thing({
            key: "spend:start",
            name: Words.name("spend:start"),
            note: downNote,
            glyph: Theme.glyph.caution,
            hue: "warning",
            verb: Words.verb("spend:start"),
            later: true,
            run: () => machine.ask({ ask: "act", act: "restart-unit",
                unit: "spend-watch.service" }, answer => {
                const unanswered = answer === null || answer.error;
                provider.refused("spend:start", unanswered ? downNote : "");
            })
        });
    }

    // The machine daemon, asked only to start the watch again; nothing of
    // it is watched.
    Daemon {
        id: machine

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "unit"
        live: false
    }

    function matched(row, words) {
        const name = Fuzzy.match(row.prepared, words, false);
        if (name.score >= 0 || !row.found)
            return name;
        const byWord = Fuzzy.match(row.found, words, false);
        // A report's words count for less than its name.
        return byWord.score < 0 ? byWord : { score: byWord.score / 2, grade: byWord.grade };
    }

    // Made only while they're wanted (Provider.qml's whileWanted), and
    // each only once its record changes (ReadyRows.qml's rowAt).
    readonly property var all: whileWanted("all", () => down ? [downRow()] : place.rows(""))

    // A row as it reads now, for those who show it while these rows aren't
    // wanted: Now's Spend and Tokens borrowing its drawer, and Agents'
    // Budgets, made then as it was served. While they're wanted, when
    // `all` is as it reads now, it's looked for where it's shown.
    function rowNow(key) {
        if (provider.wanted || down || !key.startsWith("spend:"))
            return undefined;
        const record = `row:${place.place}:${key.slice("spend:".length)}`;
        return place.served(record) ? place.rowAt(record) : undefined;
    }

    function everything() {
        return all;
    }

    // After its title, at home and in Agents, how today stands with its
    // period said, "$412 today", as All AI Spend says it (VKCF3Z,
    // 2026-11-08), or while the screen is shared what holds no amount, as
    // the daemon's summary of the place says it.
    function counted() {
        return String(summaryOf(place)?.said ?? "");
    }

    function summaryOf(from) {
        void from.placings;
        return from.held[`summary:${from.place}`]?.summary ?? null;
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

    // The rows a report's answer lists: those inside the row standing for
    // it, or where no row does, inside its answer, as spend-watch serves
    // them.
    function reportRows(id) {
        void ready.placings;
        const standing = ready.keys.find(key => ready.served(key)?.held?.report === id);
        return ready.rows(standing ?? `row:spend:answer:${id}`);
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
    // figures: its last run's are said as such rather than passing as
    // current.
    function noted(note) {
        return lastRun ? Words.of("spend").said?.lastRun ?? note : note;
    }

    function answer(typed) {
        const id = reportFor(typed);
        if (down)
            return id === null ? null : {
                headline: downNote,
                copy: "",
                rows: [downRow()],
                note: ""
            };
        if (!known)
            return null;
        const shown = id === null ? null : reports[id] ?? null;
        if (shown === null)
            return id !== null && SpendWatch.reportsReading ? waitingAnswer() : null;
        if (sharing)
            return {
                // In lower case, as every other answer's headline is.
                headline: "amounts are hidden while the screen is shared",
                copy: "",
                rows: quiet.rows(""),
                note: "ask again once sharing ends"
            };
        if (id === "summary") {
            const summary = summaryOf(ready) ?? {};
            return {
                headline: summary.headline || shown.headline,
                copy: shown.markdown,
                rows: (summary.rows ?? []).map(key => ready.rowAt(key)).filter(row => row !== null),
                note: noted("Return copies the summary")
            };
        }
        return {
            headline: `${shown.title}: ${shown.headline}`,
            copy: shown.markdown,
            rows: reportRows(id),
            // Not "the report", which in Agents is a quest's document
            // (docs/glossary.md).
            note: noted("Return copies it whole")
        };
    }
}

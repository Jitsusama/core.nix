// What AI use cost, a collection, from the file spend-watch writes
// (SpendWatch.qml)
// (modules/spend-watch). It's laid out as the other collections are: each
// row a short name with a few muted words beside it, what else is known of
// it in the footer while it's picked, and the rest a right arrow away,
// behind the row it belongs to. First what was found that wants a look;
// then how things stand: today, with a day's cap behind it, each longer
// cap, with the week ahead behind the first, and the periods before; then
// the rest of what was found, and the reports that belong behind nothing
// else. A period steps into its cuts and a cut into its rows, and is found
// by typing however deep it's kept; a cap steps into its changes and the
// sign-ins its feed offers. Return copies a row, or a report whole as markdown, to paste
// where it's asked about; a finding with something to do does it, and
// steps into marking it expected.
//
// It answers in the field too: "spend" or "ai" for how today stands, "cap"
// for the caps, and a report by its words, "spend last week by quest" or
// "spend today by model", whose words the daemon gives with the reports. The
// daemon works out every figure and every word, so nothing here counts or
// formats; with no file there's no collection, and nothing to answer. While
// the screen is shared it lists the daemon's quiet lines, which hold no
// amounts, and no report, and answers only that the amounts are hidden.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "AI spend"

    readonly property var said: SpendWatch.said
    onSaidChanged: changed()
    readonly property bool sharing: Quiet.sharing
    readonly property var reports: said?.reports ?? {}

    collection: said === null ? null : {
        noun: "spend",
        title: "AI spend",
        words: ["cost", "money", "usage", "tokens", "bill", "budget", "claude", "pi"],
        glyph: String.fromCodePoint(0xf01c1),
        ask: "which?",
        empty: "nothing spent yet"
    }
    examples: said?.examples ?? []

    // Each row's glyph and colour by its role: money for how things stand,
    // a trend for what was found, the warning or the alert for a finding
    // that wants a look, a tick for one marked expected, the chart for a
    // row standing for a report, and the door for a sign-in.
    function glyph(role) {
        return String.fromCodePoint(role === "alert" ? 0xf0028
            : role === "warning" ? 0xf0026
            : role === "found" ? 0xf012a
            : role === "expected" ? 0xf012c
            : role === "report" ? 0xf0215
            : role === "sign-in" ? 0xf0342 : 0xf01c1);
    }

    function hue(role) {
        return role === "alert" || role === "warning" ? role
            : role === "sign-in" ? "action"
            : role === "expected" ? "muted" : "voice";
    }

    // A row as the daemon gave it, with what's behind it made only when
    // it's stepped into, since a month's cuts are hundreds of rows.
    function build(given) {
        const report = given.report ? reports[given.report] ?? null : null;
        const fields = {
            key: `spend:${given.key}`,
            name: given.name,
            side: given.side,
            note: given.note ?? "",
            glyph: glyph(given.role),
            hue: hue(given.role)
        };
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
                name: "Copy what it found",
                copy: given.copy,
                figure: false,
                glyph: glyph(given.role),
                hue: hue(given.role)
            }));
        if (given.expect)
            doing.push(thing({
                key: `spend:${given.key}:expect`,
                name: "It's expected",
                glyph: String.fromCodePoint(0xf012c),
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

    readonly property var all: said === null ? []
        : sharing ? ordered([], said.quiet ?? []).map(build)
        : ordered(said.now ?? [], said.findings ?? []).concat(said.more ?? []).map(build)

    onSharingChanged: changed()

    function everything() {
        return all;
    }

    // At home, after its title, how today stands, as the margin says it.
    function counted() {
        const today = ((sharing ? said?.quiet : said?.now) ?? [])[0];
        return today ? today.side : "";
    }

    // What typing finds: every row, and the periods kept behind one, so
    // "this week" finds it behind Earlier.
    readonly property var findable: {
        const rows = all.slice();
        if (said !== null && !sharing)
            for (const given of said.now ?? [])
                for (const inner of given.rows ?? [])
                    if (inner.key.startsWith("period:"))
                        rows.push(build(inner));
        return rows;
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const found = [];
        for (const row of findable) {
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
        const period = named(text, said.periods ?? []);
        const rest = period ? period.rest : text;
        const by = /^by (.+)$/.exec(rest);
        const cut = by ? named(by[1], said.cuts ?? []) : null;
        if ((rest !== "" && (cut === null || cut.rest !== "")) || (period === null && cut === null))
            return null;
        return `${period ? period.id : "this-week"}/${cut ? cut.id : "quest"}`;
    }

    function answer(typed) {
        if (said === null)
            return null;
        const id = reportFor(typed);
        const shown = id === null ? null : reports[id] ?? null;
        if (shown === null)
            return null;
        if (sharing)
            return {
                headline: "Amounts are hidden while the screen is shared",
                copy: "",
                rows: ordered([], said.quiet ?? []).map(build),
                note: "Ask again once sharing ends"
            };
        if (id === "summary") {
            const lead = (said.now ?? [])[0];
            return {
                headline: lead ? lead.copy : shown.headline,
                copy: shown.markdown,
                rows: ordered((said.now ?? []).slice(1), said.findings ?? []).map(build),
                note: "Return copies the summary"
            };
        }
        return {
            headline: `${shown.title}: ${shown.headline}`,
            copy: shown.markdown,
            rows: shown.rows.map(build),
            note: "Return copies the report"
        };
    }
}

// What AI use cost, a collection, from the file spend-watch writes
// (modules/spend-watch): how today stands and what's ahead, then what it
// found, then its reports, this week's and last week's cut every way and
// the waste, the caps, the forecast, the quests and the costly sessions.
// Each report's row says its total and steps into its rows on the right
// arrow; Return copies it as markdown, to paste where it's asked about. A
// finding with something to do does it on Return, going to the session for
// a burn, and steps into marking it expected.
//
// It answers in the field too: "spend" or "ai" for how today stands, "cap"
// for the caps, and a report by its words, "spend last week by quest" or
// "spend today by model", whose words the daemon gives with the reports. The
// daemon works out every figure, so nothing here counts or formats; with no
// file there's no collection, and nothing to answer. While the screen is
// shared it lists the daemon's quiet lines, which hold no amounts, and no
// report, and answers only that the amounts are hidden.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    property var said: null
    readonly property bool sharing: Quiet.sharing
    readonly property var reports: said?.reports ?? {}

    collection: said === null ? null : {
        noun: "spend",
        title: "AI spend",
        glyph: String.fromCodePoint(0xf01c1),
        ask: "which report?",
        empty: "nothing spent yet"
    }
    examples: said?.examples ?? []

    // Each line's glyph and colour by its role, so a finding that wants a
    // look wears the warning or the alert.
    function glyph(role) {
        return String.fromCodePoint(role === "alert" ? 0xf0028
            : role === "warning" ? 0xf0026 : 0xf01c1);
    }

    function hue(role) {
        return role === "alert" || role === "warning" ? role : "voice";
    }

    function line(given) {
        return copyRow({
            key: `spend:${given.key}`,
            name: given.name,
            side: given.side,
            copy: given.copy,
            glyph: glyph(given.role),
            hue: hue(given.role)
        });
    }

    function finding(line) {
        const row = line.run ? thing({
            key: `spend:${line.key}`,
            verb: "go",
            name: line.name,
            side: line.side,
            glyph: glyph(line.role),
            hue: hue(line.role),
            later: true,
            run: () => Niri.spawn(line.run)
        }) : provider.line(line);
        row.verbs = () => [provider.line(Object.assign({}, line, {
            key: `${line.key}:copy`,
            name: "Copy what it found",
            side: ""
        }))].concat(line.expect ? [thing({
            key: `spend:${line.key}:expect`,
            name: "It's expected",
            glyph: String.fromCodePoint(0xf012c),
            hue: "action",
            run: () => Niri.spawn(line.expect)
        })] : []);
        return row;
    }

    // A report as one row: its title and total, Return copying all of it
    // and the right arrow stepping into its rows.
    function report(shown) {
        const row = copyRow({
            key: `spend:report:${shown.id}`,
            name: shown.title,
            side: shown.headline,
            copy: shown.markdown,
            glyph: String.fromCodePoint(0xf0215),
            hue: "voice"
        });
        row.verbs = () => shown.rows.map(given => provider.line(given));
        // Found by the words the daemon gives it too: "waste" for the
        // context rebuilt, "last week by model" for that report.
        row.found = Fuzzy.prepare(`${shown.title} ${shown.words ?? ""}`);
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

    // The reports the collection lists, in the order they're likeliest
    // wanted: this week's, last week's, then the rest.
    readonly property var listed: {
        const ids = [];
        for (const period of ["this-week", "last-week"])
            for (const cut of said?.cuts ?? [])
                ids.push(`${period}/${cut.id}`);
        return ids.concat(["waste", "caps", "forecast", "quests", "outliers"])
            .filter(id => id in reports);
    }

    readonly property var all: said === null ? []
        : sharing ? (said.quiet ?? []).map(given => given.run ? finding(given) : line(given))
        : (said.now ?? []).map(given => line(given))
            .concat((said.findings ?? []).map(given => finding(given)))
            .concat(listed.map(id => report(reports[id])))

    onSharingChanged: changed()

    function everything() {
        return all;
    }

    function counted() {
        const found = (said?.findings ?? []).length;
        return found === 0 ? "all well" : found === 1 ? "1 finding" : `${found} findings`;
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

    // Which report what's typed asks for, by id, or null.
    function asked(typed) {
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
        const id = asked(typed);
        const shown = id === null ? null : reports[id] ?? null;
        if (shown === null)
            return null;
        if (sharing)
            return {
                headline: "Amounts are hidden while the screen is shared",
                copy: "",
                rows: (said.quiet ?? []).map(given => line(given)),
                note: "Ask again once sharing ends"
            };
        if (id === "summary") {
            const lead = (said.now ?? [])[0];
            return {
                headline: lead ? lead.copy : shown.headline,
                copy: shown.markdown,
                rows: (said.now ?? []).slice(1).map(given => line(given))
                    .concat((said.findings ?? []).map(given => finding(given))),
                note: "Return copies the summary"
            };
        }
        return {
            headline: `${shown.title}: ${shown.headline}`,
            copy: shown.markdown,
            rows: shown.rows.map(given => line(given)),
            note: "Return copies the report"
        };
    }

    function parsed(text) {
        try {
            const value = JSON.parse(text);
            return typeof value === "object" && value !== null && value.v === 1 ? value : null;
        } catch (error) {
            return null;
        }
    }

    // The daemon writes the file anew only when something in it changed,
    // which a watch can miss, and may start after the shell, so it's read
    // again every little while too.
    Timer {
        interval: provider.settings.every ?? 30000
        running: true
        repeat: true
        onTriggered: file.reload()
    }

    FileView {
        id: file
        path: provider.settings.collection ?? `${provider.runtime}/spend-watch/collection.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        // What was read last, so a reading that changed nothing says nothing.
        property string read: ""
        onLoaded: {
            if (text() === read)
                return;
            read = text();
            provider.said = provider.parsed(read);
            provider.changed();
        }
        onLoadFailed: {
            read = "";
            provider.said = null;
        }
    }
}

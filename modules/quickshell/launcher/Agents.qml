// pi's sessions, a collection: every pi running in a window, the one used
// last first, each named by the quest it has loaded and beside it its
// address, whether it's working or waiting, and the folder it works in.
// Return goes to its window; the right arrow offers copying its address or
// its quest's id. What runs, and how each is doing, is Sessions.qml's.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "pi session"

    collection: ({
        noun: "sessions",
        title: "Agents",
        words: ["pi", "sessions", "agents", "quests", "claude", "ai"],
        glyph: Theme.glyph.agent,
        ask: "which session?",
        empty: "No pi running."
    })

    readonly property var all: Sessions.running.map(session => made(session))
    onAllChanged: changed()

    function made(session) {
        const presence = session.presence;
        const doing = presence === null ? "" : presence.busy ? "working" : "waiting";
        const quest = presence && presence.owns
            ? presence.owns.find(owned => /^QEST-/.test(owned)) ?? "" : "";
        return member({
            key: `agent:${session.address}`,
            verb: "switch",
            name: session.title,
            keywords: `${session.address} ${session.folder} ${quest}`,
            glyph: Theme.glyph.agent,
            hue: "program",
            side: [session.address, doing, session.folder].filter(part => part !== "").join(" \u00b7 "),
            note: presence && presence.worksOn ? presence.worksOn : "",
            later: true,
            run: () => Niri.act({ FocusWindow: { id: session.window.id } }),
            verbs: () => provider.verbs(session, quest)
        });
    }

    function verbs(session, quest) {
        const copy = (key, name, text) => thing({
            key: key,
            name: name,
            glyph: String.fromCodePoint(0xf018f),
            hue: "action",
            now: true,
            run: () => Quickshell.clipboardText = text
        });
        return [
            thing({
                key: "verb:agent:switch",
                name: "Go to its window",
                glyph: String.fromCodePoint(0xf05b2),
                hue: "action",
                later: true,
                run: () => Niri.act({ FocusWindow: { id: session.window.id } })
            }),
            copy("verb:agent:address", `Copy its address, ${session.address}`, session.address)
        ].concat(quest !== "" ? [copy("verb:agent:quest", `Copy its quest, ${quest}`, quest)] : []);
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            const byName = (row.prepared.mask & need) === need
                ? Fuzzy.match(row.prepared, words, false) : { score: -1 };
            const byWord = row.keywords && (row.keywords.mask & need) === need
                ? Fuzzy.match(row.keywords, words, false) : { score: -1 };
            const best = byName.score >= byWord.score ? byName : byWord;
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row });
        }
        const grade = Math.min(3, ...found.map(f => f.grade));
        const kept = found.filter(f => f.grade === grade).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: grade };
    }

    function everything() {
        return all.slice();
    }

    function counted() {
        const running = Sessions.running.length;
        return Sessions.working > 0 ? `${running} running, ${Sessions.working} working` : `${running} running`;
    }
}

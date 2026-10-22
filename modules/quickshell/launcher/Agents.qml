// pi's sessions, a collection: every pi running in a window, the one used
// last first, each named by the quest it has loaded and beside it its
// address, whether it's working or waiting, and the folder it works in.
// Return goes to its window; the right arrow offers copying its address or
// its quest's id.
//
// What runs comes from niri's windows, whose titles pi sets to "π -", its
// address, its quest's title cut short, and its folder, so a session is
// known the moment its window is and gone the moment it closes. Its quest's
// whole title is the first line of the quest's README, read once a session
// is seen. Whether it's working comes from the file each session keeps in
// the mesh's presence folder while it runs, watched for changes, and a
// session that keeps none says neither. Nothing is spawned and nothing is
// asked of pi.
import qs
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "pi session"

    readonly property string state: `${Quickshell.env("HOME")}/.local/state/pi/agentic-harness.pi`
    readonly property string quests: `${Quickshell.env("HOME")}/.local/share/pi/agentic-harness.pi/quest-workflow/quests`

    collection: ({
        noun: "sessions",
        title: "Agents",
        words: ["pi", "sessions", "agents", "quests", "claude", "ai"],
        glyph: Theme.glyph.agent,
        ask: "which session?",
        empty: "No pi running."
    })

    // Each session's presence, by its address, as its file says it.
    property var present: ({})

    // Each window pi runs in: its address, its quest's title as the window
    // gives it, and its folder.
    readonly property var running: {
        Niri.revision;
        const when = window => {
            const stamp = window.focus_timestamp;
            return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
        };
        return Object.values(Niri.windows)
            .sort((a, b) => when(b) - when(a))
            .map(window => {
                const said = /^\u03c0 - (\S+)\s+(.*?)\s+-\s+([^-]*)$/.exec(window.title ?? "");
                return said ? { window: window, address: said[1], cut: said[2], folder: said[3] } : null;
            })
            .filter(session => session !== null);
    }

    // A quest's whole title, by the address its id ends in, read once the
    // quests are listed; until then, the window's cut-short one.
    property var titles: ({})
    function titled(address, cut) {
        if (titles[address] !== undefined)
            return titles[address] || cut;
        if (folders.status !== FolderListModel.Ready)
            return cut;
        let title = "";
        for (let i = 0; i < folders.count; i++) {
            const name = folders.get(i, "fileName");
            if (name.endsWith(`-${address}`)) {
                probe.path = `${quests}/${name}/README.md`;
                const first = read(probe).split("\n").find(line => line.startsWith("# "));
                title = first ? first.slice(2).trim() : "";
                break;
            }
        }
        titles[address] = title;
        return title || cut;
    }

    // Made again once the quests are listed, so each takes its whole title.
    readonly property bool listed: folders.status === FolderListModel.Ready && folders.count >= 0
    readonly property var all: {
        listed;
        return running.map(session => provider.made(session));
    }
    onAllChanged: changed()

    function made(session) {
        const presence = present[session.address] ?? null;
        const title = titled(session.address, session.cut);
        const doing = presence === null ? "" : presence.busy ? "working" : "waiting";
        const quest = presence && presence.owns
            ? presence.owns.find(owned => /^QEST-/.test(owned)) ?? "" : "";
        return member({
            key: `agent:${session.address}`,
            verb: "switch",
            name: title,
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
        const working = running.filter(session => (present[session.address] ?? {}).busy).length;
        return working > 0 ? `${running.length} running, ${working} working` : `${running.length} running`;
    }

    // The quests, listed once, to find a session's README by its address.
    FolderListModel {
        id: folders
        folder: `file://${provider.quests}`
        showFiles: false
        showDotAndDotDot: false
    }

    // Each session's presence file, read as it changes.
    FolderListModel {
        id: presences
        folder: `file://${provider.state}/mesh-workflow/presence`
        showDirs: false
        nameFilters: ["*.json"]
    }

    Instantiator {
        model: presences

        FileView {
            required property string filePath
            path: filePath
            watchChanges: true
            printErrors: false
            onFileChanged: reload()
            onLoaded: provider.heard(filePath, text())
        }
    }

    // Who a presence file belongs to, kept by each address it answers to.
    property var files: ({})
    function heard(file, text) {
        let said;
        try {
            said = JSON.parse(text);
        } catch (e) {
            return;
        }
        const next = Object.assign({}, present);
        for (const address of files[file] ?? [])
            delete next[address];
        files[file] = said.addresses ?? [];
        for (const address of files[file])
            next[address] = said;
        present = next;
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }
}

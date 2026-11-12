// Chrome's open tabs in the launcher, a collection found by title or site
// from home, as People and Agents are (Joel, 2026-11-11: "i need to be
// able to freeform search for chrome tabs outside of the program like i
// can for people, and agent sessions"): home shows the best tab and how
// many more, and the right arrow steps into them all. Each is the row the
// programs daemon serves ready (modules/programs/src/tab_rows.rs), one a
// page, the one looked at last first, made again as Chrome writes its
// session, and the daemon ranks them for what's typed (search.rs), so
// the shell only draws them. Return goes to the Chrome window the page is
// in front of, found by its title as Chrome titles its windows, else
// Chrome's newest window; inside are Go To and Copy the Link. Kitty's tabs
// are served among them (src/kitty_tabs.rs), one Tabs for both (VKCF3Z,
// 2026-11-12), and a kitty tab's Return asks the daemon to go to it;
// inside are Go to the Tab, Close the Tab and Copy the Title.
//
// Its bookmarks are found the same way (Joel, 2026-11-11: "searchable
// chrome bookmarks would be nice"), served ready too (modules/programs/
// src/bookmark_rows.rs), but for a page open now, whose tab is its row.
// Return opens one in Chrome; inside are Open and Copy the Link.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "site"
    collection: Words.collection("tabs")

    // Watched whether or not the launcher's open, so it opens on rows
    // already held, and a change while shut only keeps its record.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/programs.sock`
        place: "tabs"
        maker: provider
        drawn: provider.launcher?.open ?? false
        // A page whose Return goes to its program is gone to here, to the
        // window it's in front of; a kitty tab's Return asks the daemon,
        // which goes to the tab itself (modules/programs/src/kitty_tabs.rs),
        // where the program's newest window could be another tab's.
        make: (row, key) => ready.thing(row, key, Object.assign({
            what: "site",
            component: "site",
            hue: row.hue || "program",
            later: true
        }, row.acts?.[0]?.go ? { run: () => provider.goTo(row) } : {}))
    }
    // Named, so a test can tell it records as the daemon would.
    readonly property alias served: ready

    // Its bookmarks, watched as its tabs are; a row's own Return opens it.
    ReadyRows {
        id: marked

        socket: ready.socket
        place: "bookmarks"
        maker: provider
        drawn: ready.drawn
        make: (row, key) => marked.thing(row, key, {
            what: "site",
            component: "site",
            hue: row.hue || "program"
        })
    }
    readonly property alias bookmarks: marked

    // Its bookmarks, made only while the launcher is open or its rows are
    // wanted (Provider.qml's wanted), as Documents.qml's are, and searched
    // from home among everything else; its tabs are its collection's.
    readonly property bool rowsWanted: provider.launcher?.open === true || provider.wanted
    readonly property var madeRows: ({ rows: [] })
    things: {
        if (!provider.rowsWanted)
            return provider.madeRows.rows;
        const rows = marked.rows();
        provider.madeRows.rows = rows;
        return rows;
    }
    onThingsChanged: changedTo(things)

    function everything() {
        return ready.rows();
    }

    // Its tabs, as the programs daemon ranks them for what's typed. While
    // its answer is asked, the tabs that hold every letter typed stand:
    // the last answer's, in its order, when what's typed goes on from it,
    // else every tab. The answer, a moment later, is searched again
    // (`foundMore`), ranked as the key's own search where it came within
    // a frame of it. A daemon that can't search leaves them matched here,
    // as they were before, saying so once (ReadyRows' searchless).
    property real askedAt: 0
    readonly property int freshFor: 50
    function search(typed) {
        const answer = ready.search(typed);
        if (answer !== null)
            return { rows: ready.foundRows(answer), grade: answer.keys.length > 0 ? 0 : 3 };
        if (ready.searchless)
            return matched(typed);
        if (ready.asking === typed)
            provider.askedAt = Date.now();
        const need = Fuzzy.letters(Fuzzy.words(typed));
        const holds = item => !!item && (item.mask & need) === need;
        const said = Fuzzy.words(typed).join(" ");
        const last = ready.found.words !== null
            && said.startsWith(Fuzzy.words(ready.found.words).join(" "))
            ? ready.foundRows(ready.found) : ready.rows();
        const rows = last.filter(row => holds(row.prepared) || holds(row.keywords));
        return { rows: rows, grade: rows.length > 0 ? 0 : 3 };
    }
    Connections {
        target: ready
        function onSearched(words) {
            provider.foundMore(Date.now() - provider.askedAt < provider.freshFor);
        }
    }

    // Its tabs by title or site, the best grade first, only where the
    // daemon can't search.
    function matched(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const missed = { score: -1, grade: 3 };
        const found = [];
        for (const row of ready.rows()) {
            const byName = (row.prepared.mask & need) === need
                ? Fuzzy.match(row.prepared, words, false) : missed;
            const bySite = row.keywords && (row.keywords.mask & need) === need
                ? Fuzzy.match(row.keywords, words, false) : missed;
            const best = byName.score >= bySite.score ? byName : bySite;
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row, at: found.length });
        }
        const grade = Math.min(3, ...found.map(each => each.grade));
        const kept = found.filter(each => each.grade === grade)
            .sort((a, b) => b.score - a.score || a.at - b.at);
        return { rows: kept.map(each => each.row), grade: grade };
    }

    // The window a page is in front of, by the title Chrome gives it, as
    // "core.nix - Google Chrome", looked up only at Return; else the
    // program's newest window, as Go To finds it.
    function goTo(row) {
        const app = String(row.held?.app ?? "").toLowerCase();
        const front = String(row.held?.front ?? "");
        const titled = front === "" ? null : Object.values(Niri.windows).find(window =>
            String(window.app_id ?? "").toLowerCase() === app
            && String(window.title ?? "").startsWith(front));
        const window = titled ?? Niri.byApp[app] ?? null;
        if (window)
            Niri.act({ FocusWindow: { id: window.id } });
    }
}

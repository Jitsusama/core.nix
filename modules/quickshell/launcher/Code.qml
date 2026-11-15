// Every repository on the machine in the launcher, once each however many
// checkouts and trees it has, a collection found by its name or owner from
// home, beside Agents (Joel, 2026-11-14: a Code place). Each is the row the
// programs daemon serves ready (modules/programs/src/code_rows.rs), read
// with the repos library under the folders repos reads (modules/repos),
// and the daemon ranks them for what's typed (search.rs), so the shell only
// draws them. Return opens a repository's page where its remote is on
// GitHub or GitLab, else copies its path; inside are Open Its Page and Copy
// Its Path, and its drawer says its trunk, checkout and branch.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: Words.of("code:repository").said?.kind ?? ""
    collection: Words.collection("code")

    // Watched while the launcher's open or its rows are wanted, as
    // Applications' are: each watch begun has the daemon read the disk again
    // behind it, so a repository cloned since comes a moment after the
    // opening, which draws the rows it held without waiting for the read.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/programs.sock`
        place: "code"
        live: provider.wanted || provider.launcher?.open === true
        maker: provider
        drawn: provider.launcher?.open ?? false
        make: (row, key) => ready.thing(row, key, {
            what: provider.what,
            hue: row.hue || "program"
        })
    }
    // Named, so a test can tell it records as the daemon would.
    readonly property alias served: ready

    function everything() {
        return ready.rows();
    }

    // Its repositories, as the programs daemon ranks them for what's typed.
    // While its answer is asked, those that hold every letter typed stand:
    // the last answer's, in its order, when what's typed goes on from it,
    // else every repository. The answer, a moment later, is searched again
    // (`foundMore`), ranked as the key's own search where it came within a
    // frame of it, as Tabs' are.
    property real askedAt: 0
    readonly property int freshFor: 50
    function search(typed) {
        const answer = ready.search(typed);
        if (answer !== null)
            return { rows: ready.foundRows(answer), grade: answer.keys.length > 0 ? 0 : 3 };
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
}

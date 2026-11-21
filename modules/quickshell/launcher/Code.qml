// Every repository on the machine in the launcher, once each however many
// checkouts and trees it has, a collection found by its name or owner from
// home, beside Agents (Joel, 2026-11-14: a Code place). Each is the row the
// programs daemon serves ready (modules/programs/src/code_rows.rs), read
// with the repos library under the folders repos reads (modules/repos),
// and the daemon ranks them for what's typed (search.rs), so the shell only
// draws them. Return opens a repository's page where its remote is on
// GitHub or GitLab, else copies its path; inside are its acts, then Open a
// File by Name, which steps into its checkout's files, the one changed last
// first, each opened by Return in its default program. The daemon lists
// them only while that row is ahead (`filesAhead`), and finds among them
// what's typed there (`finds`). Its drawer says its
// trunk, checkout and branch.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "CodeLinks.js" as Links

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
        // A repository's own acts lead inside it, and Open a File by Name
        // follows them.
        insideAfterActs: true
        ahead: provider.filesAhead !== "" ? [provider.filesAhead] : []
        make: (row, key) => ready.thing(row, key, Object.assign({
            what: provider.what,
            hue: row.hue || "program",
            // The key it's served by, so a step into it says which.
            servedKey: key
        }, key.endsWith(provider.files) ? { finds: provider.findsIn(key) } : {}))
    }

    // What's typed in Open a File by Name, found among the files inside it
    // by the daemon, by name or folder, as Recent's is. Made here, holding
    // only the key, since a closure keeps all the function it's made in
    // held (ReadyRows' thing).
    function findsIn(key) {
        return letters => ready.typedIn(key, letters) ?? [];
    }

    // How the key of an Open a File by Name row ends (code_rows.rs's FILES).
    readonly property string files: ":files"

    // The Open a File by Name row whose checkout's files the daemon lists:
    // the one stepped into, else the one inside the repository stepped
    // into, so its files are there by the time it's stepped into; none
    // with the launcher shut.
    readonly property string filesAhead: {
        const launcher = provider.launcher;
        if (!(launcher?.open ?? false))
            return "";
        const served = (launcher.held ?? []).map(step => step.row?.servedKey ?? "")
            .filter(key => key.startsWith("row:code:"));
        const files = served.find(key => key.endsWith(provider.files));
        return files ?? (served.length > 0 ? `${served[0]}${provider.files}` : "");
    }
    // Named, so a test can tell it records as the daemon would.
    readonly property alias served: ready

    function everything() {
        return ready.rows();
    }

    // The row a mycelium:repository/ link lands on, by the key its
    // repository's row is served by, or none for a key no row has once
    // they're read (CodeLinks.js's linked).
    function linked(said) {
        return Links.linked(ready.rows().map(row => row?.servedKey ?? ""), said);
    }

    // Whether a row is the one that link lands on (CodeLinks.js's lands).
    function lands(row) {
        return Links.lands(row);
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

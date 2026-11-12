// Chrome's open tabs in the launcher, each found by its title or its site
// from home, as a document or a person is (Joel, 2026-11-11: "i need to
// be able to freeform search for chrome tabs outside of the program").
// Each is the row the programs daemon serves ready (modules/programs/src/
// tab_rows.rs), one a page, the one looked at last first, made again as
// Chrome writes its session, so the shell only draws it. Return goes to
// the Chrome window the page is in front of, found by its title as Chrome
// titles its windows, else Chrome's newest window; inside are Go To and
// Copy the Link.
import qs
import Quickshell
import QtQuick

Provider {
    id: provider

    what: "site"

    // Watched whether or not the launcher's open, so it opens on rows
    // already held, and a change while shut only keeps its record.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/programs.sock`
        place: "tabs"
        maker: provider
        drawn: provider.launcher?.open ?? false
        make: (row, key) => ready.thing(row, key, {
            what: "site",
            component: "site",
            hue: row.hue || "program",
            later: true,
            run: () => provider.goTo(row)
        })
    }
    // Named, so a test can tell it records as the daemon would.
    readonly property alias served: ready

    // Its rows, made only while the launcher is open or its rows are
    // wanted (Provider.qml's wanted), as Documents.qml's are.
    readonly property bool rowsWanted: provider.launcher?.open === true || provider.wanted
    readonly property var madeRows: ({ rows: [] })
    things: {
        if (!provider.rowsWanted)
            return provider.madeRows.rows;
        const rows = ready.rows();
        provider.madeRows.rows = rows;
        return rows;
    }
    onThingsChanged: changedTo(things)

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

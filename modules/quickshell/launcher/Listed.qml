pragma ComponentBehavior: Bound
// A program page's own list, as the programs daemon holds it (its
// `listed` records, recents.nix's sources with a `name`): the lines its
// command printed, read by the daemon as the page watches its key, so
// the page starts no process of its own for it (Joel, 2026-11-10: the
// daemons are the data layer, reached over a socket held open). A page
// watches it from its first step in an opening (`wanted`), which has the
// daemon read it afresh, and hears what it holds at once, then what it
// reads. The page still makes its rows from the lines, a stopgap until
// the daemon serves them as ready rows.
import qs
import Quickshell
import QtQuick

Scope {
    id: listed

    // The daemon's key for it, as `listed:mpv:stopped`, or "" where no
    // source lists it.
    required property string key
    // Whether the page wants it now, from its first step in an opening
    // until the launcher closes; each time it turns true, it's read again.
    property bool wanted: false
    // The lines it printed, as the daemon last said them.
    property var lines: []
    // How many panes the daemon left out as agents' (agents.rs), where it
    // serves it without them, as kitty's open list.
    property int agents: 0
    // Whether the daemon has said it once since the shell started.
    property bool read: false
    // Said each time the daemon says it.
    signal heard

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/programs.sock`
        kind: "listed"
        live: false
        ahead: listed.wanted && listed.key !== "" ? [listed.key] : []

        onHeard: changed => {
            if (!changed.includes(listed.key))
                return;
            listed.lines = daemon.held[listed.key]?.lines ?? [];
            listed.agents = daemon.held[listed.key]?.agents ?? 0;
            listed.read = true;
            listed.heard();
        }
    }
}

// Windows' agents' windows, given way to one Agents row where the one used
// last would be (VKCF3Z's ruling 2, 2026-11-11 [23daa469]): the agents
// daemon says which windows are agents', the row is made here, and Tab
// steps into them by orchestration. Windows.qml lists its rows through
// `folded` and its keys through `agents`; AgentWindows.js works it out,
// tested in Qt's engine (tests/agent-windows.qml).
import Quickshell
import qs
import QtQuick
import "AgentWindows.js" as AgentWindows

Scope {
    id: folding

    // Windows, whose rows these are, which makes them and lists them.
    required property var provider

    // The agents' windows as the agents daemon pairs them with the windows
    // daemon's, each session's record by its window's key. Watched while
    // the shell runs, as a session starting or ending is all it tells, so
    // the first frame of any opening folds them. Down, none folds.
    property var agents: ({})
    Daemon {
        id: agentsDaemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/agents.sock`
        kind: "agent-window"
        watching: ({ place: "agents" })
        answersWhole: true
        onHeard: folding.heard()
        onStateChanged: folding.heard()
    }
    function heard() {
        folding.agents = agentsDaemon.state === "not answering" ? {}
            : AgentWindows.agentsOf(Object.values(agentsDaemon.held));
        if (folding.provider.wanted)
            folding.provider.changed();
    }

    // The rows with every agents' window given way to the one Agents row,
    // where the agents' window used last would be: made again only as
    // what it says moves, so a list read again finds the same row.
    property var fold: ({ said: "", row: null, inside: [] })
    function folded(rows) {
        const made = AgentWindows.foldOf(rows, folding.agents,
            Words.of("windows:agents").said.more);
        if (made === null)
            return rows;
        if (made.said !== folding.fold.said)
            folding.fold = { said: made.said, inside: made.inside,
                row: folding.foldRow(made.lead, made.side, made.needing) };
        const fold = AgentWindows.foldKey();
        return made.listed.map(key => key === fold ? folding.fold.row : made.byKey[key]);
    }

    // The one Agents row: Return goes to the agents' window used last, as
    // Super+Tab then Return always went to the window used last, and its
    // side names that window's session so he sees where.
    function foldRow(lead, side, needing) {
        return folding.provider.thing({
            thing: true,
            key: "windows:agents",
            is: "windows:agents",
            verb: Words.verb("windows:agents"),
            name: Words.name("windows:agents"),
            glyph: Words.glyph("windows:agents"),
            hue: "program",
            side: side,
            waiting: needing > 0,
            count: AgentWindows.needed(needing, Words.of("windows:agents").said),
            run: () => Niri.act({ FocusWindow: { id: lead } }),
            verbs: () => folding.inside()
        });
    }

    // Inside it, the agents' windows by orchestration, each its own row as
    // Windows makes it, a step in under its orchestrator's own window.
    function inside() {
        const placed = AgentWindows.depths(
            AgentWindows.byOrchestration(folding.fold.inside, folding.agents));
        return placed.map(one => ({ row: folding.provider.kept[one.key], depth: one.depth }))
            .filter(one => one.row !== undefined)
            .map(one => folding.provider.gotten(one.row, { depth: () => one.depth }));
    }
}

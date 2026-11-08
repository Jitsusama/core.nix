pragma Singleton
import Quickshell
import QtQuick

// Every agent source on this machine (KP0L6A's agent-source-contract.md):
// each kind of agent it runs, behind one shape, and what they tell of
// together. Agents is one place whatever runs its sessions (Joel,
// 2026-11-07: "password management should be a universal concern with
// 1password being a particular implementation. same with agents."), so
// the places read the sources here, and a source is added by naming it in
// `sources` alone. pi, through the realm's mycelium node, is the first.
// Each source holds `name`, `state`, `sessions`, `runs`, `talks` and
// `acts`, and offers an act only where it does it.
Singleton {
    id: root

    readonly property var sources: [PiSource]

    // Every session, run and talk any source tells of, each with the name
    // of the source it came from, as `gen_ai.agent.name` says it.
    readonly property var sessions: gathered("sessions")
    readonly property var runs: gathered("runs")
    readonly property var talks: gathered("talks")
    function gathered(member) {
        const all = [];
        for (const source of sources) {
            for (const record of source[member] ?? [])
                all.push(Object.assign({ "gen_ai.agent.name": source.name }, record));
        }
        return all;
    }

    // Whether a row says its source: only while two or more sources tell
    // of sessions (VKCF3Z, 2026-11-07 [cb4cc728]).
    readonly property bool several: sources.filter(source => (source.sessions ?? []).length > 0)
        .length > 1

    // The source of a session record, by the name it carries, or null.
    function sourceOf(record) {
        return sources.find(source => source.name === record?.["gen_ai.agent.name"]) ?? null;
    }
}

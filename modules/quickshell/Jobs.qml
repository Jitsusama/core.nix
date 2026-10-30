pragma Singleton
// The machine's queue of heavy work, as the harness writes it for every pi
// session (its placement, in agentic-harness): what runs and what waits its
// turn, each job's owner, and the room the machine has. One file,
// $XDG_RUNTIME_DIR/jobs/queue.json, replaced whole on each change and at
// least every thirty seconds, so it's heard rather than read on a clock,
// and a harness gone quiet shows as silent. The folder is made at login
// (this module's home.nix), so the watch hears the first file.
import qs
import Quickshell
import QtQuick
import "pieces"

Singleton {
    id: root

    // The file as written, or null with none of a version this reads.
    readonly property var said: status.said
    readonly property bool silent: status.silent
    // Running first, then waiting in turn order, as the harness keeps them.
    readonly property var jobs: said?.jobs ?? []
    readonly property var running: jobs.filter(job => job.state === "running")
    readonly property var waiting: jobs.filter(job => job.state === "waiting")

    // What the test waits on before opening the launcher, so a write heard
    // late isn't counted as one the closed launcher made something of.
    onSaidChanged: Report.say(`heard jobs ${said?.machine?.availableGib ?? "nothing"}`)

    // A pi session's jobs, by its whole id.
    function of(session) {
        return jobs.filter(job => job.owner?.session === session);
    }

    StatusFile {
        id: status
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/jobs/queue.json`
        // Three heartbeats missed.
        silence: 90
        parse: text => {
            try {
                const said = JSON.parse(text);
                return typeof said === "object" && said !== null && said.v === 1 ? said : null;
            } catch (error) {
                return null;
            }
        }
    }
}

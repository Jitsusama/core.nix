pragma Singleton
pragma ComponentBehavior: Bound
// The machine's queue of heavy work, as the harness writes it for every pi
// session (its placement, in agentic-harness): what runs and what waits its
// turn, each job's owner, and the room the machine has. The harness
// replaces $XDG_RUNTIME_DIR/jobs/queue.json whole on each change and at
// least every thirty seconds, and the machine's mycelium node serves it as
// one `queue` record (modules/mycelium), so the shell reads no file of the
// queue's: it's told each change as the node hears it, at most four a
// second, and a harness gone quiet shows as silent. Jobs.js decides what
// of each record is the harness's and what's new in it.
//
// The node serves each job as a `job` record too, but the interface's own
// `kind` takes the place of a job's, which Queue draws ("build"), so the
// jobs are read from the queue's record, where they're whole and in the
// harness's order.
import qs
import Quickshell
import QtQuick
import "Jobs.js" as Read

Singleton {
    id: root

    // The file as written, or null with none of a version this reads.
    property var said: null
    property bool silent: false
    // Running first, then waiting in turn order, as the harness keeps them.
    readonly property var jobs: said?.jobs ?? []
    readonly property var running: jobs.filter(job => job.state === "running")
    readonly property var waiting: jobs.filter(job => job.state === "waiting")

    // Whether the launcher is open (shell.qml), the only place anything of
    // the queue is drawn (Agents and Queue): closed, the queue isn't
    // watched at all, so a write costs the shell nothing, and it's told the
    // queue whole as it opens. Under load the queue's pressure changed in
    // nearly every write, 12 a second, each about 20 wakeups and 14 ms of
    // the shell's with the launcher closed (15J7N3, 2026-10-30).
    property bool showing: false

    // What it heard, for the test that a closed launcher hears nothing.
    onSaidChanged: Report.say(`heard jobs ${said?.machine?.availableGib ?? "nothing"}`)

    // A pi session's jobs, by its whole id.
    function of(session) {
        return jobs.filter(job => job.owner?.session === session);
    }

    // Three heartbeats missed, in seconds.
    readonly property real silence: 90
    // When the harness last said it ran, in seconds since the epoch.
    property real beat: 0
    // What it said last beyond its beats, so a write that says only when
    // changes nothing said, and what reads `said` runs again only for news.
    property string saying: ""

    readonly property var queueKind: queueLink.item
    LazyLoader {
        id: queueLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "queue"
            live: root.showing
            changesDiffer: true
            answersWhole: true
        }
    }
    // The node's one record of the queue, keyed by the machine's name, or
    // none while there's no file.
    readonly property var record: {
        void queueKind?.revision;
        return Object.values(queueKind?.held ?? {})[0] ?? null;
    }
    onRecordChanged: hear(Read.told(record))

    function hear(file) {
        beat = file?.heartbeat ?? 0;
        const told = Read.saying(file);
        if (file === null || told !== saying) {
            saying = told;
            said = file;
        }
        quiet.stop();
        const left = Read.quietIn(beat, Date.now(), silence);
        silent = said !== null && left === 0;
        // Its silence isn't kept while the launcher is closed, as nothing
        // shows it: the whole record told as it opens says it.
        if (said !== null && !silent && showing) {
            quiet.interval = Math.max(1, left);
            quiet.start();
        }
    }
    onShowingChanged: {
        if (!showing)
            quiet.stop();
    }

    // The deadline a heartbeat sets, counting the machine's running time,
    // which a suspend stops as it stops the harness.
    Timer {
        id: quiet
        onTriggered: root.silent = true
    }
}

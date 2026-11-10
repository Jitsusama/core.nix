pragma ComponentBehavior: Bound
pragma Singleton
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick
import "PiSource.js" as Pi

// pi, the first agent source (KP0L6A's agent-source-contract.md): every
// pi session, run and talk the realm's mycelium node holds, over the
// daemon interface it speaks (mycelium's PROTOCOL.md, The Daemon
// Interface), and the acts pi alone does on them. Agents is one place with
// sources, pi one implementation of them (Joel, 2026-11-07: "password
// management should be a universal concern with 1password being a
// particular implementation. same with agents."), so another agent's
// source is a file beside this one, and AgentSources lists them both.
// Sessions keeps what the shell does with pi's sessions here: their
// windows, their reopening and their notices. While the node isn't
// answering, as while a switch restarts it, what it last said stays held,
// and nothing is guessed in its place: no folder read, no process looked
// at (Joel, 2026-11-06: "treated way more like our hardware
// integrations").
Singleton {
    id: root

    // The source's own name, as `gen_ai.agent.name` says it.
    readonly property string name: "pi"
    // The acts pi does (the contract's Acts), each offered only on its
    // rows; start, wake and carry are pi's own.
    readonly property var acts: ["goTo", "talk", "pause", "resume", "stop", "close", "reopen",
        "start", "wake", "carry"]
    // Whether it answers: answering, quiet or lost.
    readonly property string state: Pi.state(node)

    readonly property string home: Quickshell.env("HOME")
    // Where the harness keeps its state, by the XDG base directories.
    readonly property string harness: `${Quickshell.env("XDG_STATE_HOME") || `${home}/.local/state`}/pi/agentic-harness.pi`

    FileView {
        id: realmFile
        path: `${Quickshell.env("XDG_STATE_HOME") || `${root.home}/.local/state`}/mycelium/realm`
        blockLoading: true
        printErrors: false
        // A node's first run writes it after the shell has started.
        watchChanges: true
        onFileChanged: reload()
    }
    // The realm this machine's home node runs, and this machine's name.
    readonly property string realm: realmFile.text().trim()
    FileView {
        id: hostname
        path: "/proc/sys/kernel/hostname"
        blockLoading: true
        printErrors: false
    }
    readonly property string machine: hostname.text().trim()

    // This boot, which a record's owner names the boot it began in.
    FileView {
        id: boot
        path: "/proc/sys/kernel/random/boot_id"
        blockLoading: true
        printErrors: false
    }
    readonly property string bootId: boot.text().trim()
    // When this boot began, in milliseconds since the epoch, read once.
    FileView {
        id: uptime
        path: "/proc/uptime"
        blockLoading: true
        printErrors: false
    }
    readonly property real bootedAt: Date.now()
        - Number(uptime.text().split(" ")[0] || 0) * 1000

    // The home realm's node, by its socket, and its links for each kind of
    // record pi's sessions leave: watched whole, then told each change as
    // only what differs.
    readonly property string nodePath: realm !== "" && Quickshell.env("XDG_RUNTIME_DIR")
        ? `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/${realm}.sock` : ""
    readonly property var node: nodeLink.item
    LazyLoader {
        id: nodeLink
        active: root.nodePath !== ""

        Daemon {
            socket: root.nodePath
            kind: "session"
            changesDiffer: true
            onRevisionChanged: root.resort()
        }
    }
    readonly property var runKind: runLink.item
    LazyLoader {
        id: runLink
        active: root.nodePath !== ""

        Daemon {
            socket: root.nodePath
            kind: "run"
            changesDiffer: true
            answersWhole: true
        }
    }
    readonly property var talkKind: talkLink.item
    LazyLoader {
        id: talkLink
        active: root.nodePath !== ""

        Daemon {
            socket: root.nodePath
            kind: "talk"
            changesDiffer: true
            answersWhole: true
        }
    }

    // The runs its sessions started (fleets, councils, a judge's rounds)
    // and what each session answered Joel, each record as the node serves
    // it, by its key.
    readonly property var runs: {
        void runKind?.revision;
        return Object.values(runKind?.held ?? {});
    }
    readonly property var talks: {
        void talkKind?.revision;
        return Object.values(talkKind?.held ?? {});
    }

    // The node's sessions sorted (PiSource.js): `live`, `resting` and
    // `ending`, each `{ file, presence, window, held }` and more, made
    // again when the node tells of a change or a park comes or goes, and
    // kept as they were when nothing in them did, so what reads them isn't
    // told for nothing. `sorted` is said once they change. All three are
    // set at once, so what reads more than one is told once.
    property var sorting: ({ live: [], resting: [], ending: [] })
    readonly property var live: sorting.live
    readonly property var resting: sorting.resting
    readonly property var ending: sorting.ending
    signal sorted()
    // Whether the launcher shows them (shell.qml). While it doesn't, a
    // change only to what it alone shows (Pi.telling) is held back, and taken
    // once it shows them again: just after, so opening isn't slowed by it,
    // since what it first shows, how each session stands, was never held.
    property bool showing: false
    onShowingChanged: {
        if (showing && heldBack)
            Qt.callLater(resort);
    }
    property bool heldBack: false
    // What was last sorted, as Pi.telling said it, and whether it was shown.
    property string sortedSaid: ""
    property bool saidShown: false
    function resort() {
        const next = Pi.sorted(node?.held ?? {}, {
            machine: machine,
            bootId: bootId,
            bootedAt: bootedAt,
            markers: parkedMarkers()
        });
        const said = Pi.telling(next, showing);
        sorts.heard += 1;
        if (said === sortedSaid && saidShown === showing) {
            heldBack = !showing;
            return;
        }
        sortedSaid = said;
        saidShown = showing;
        heldBack = false;
        sorts.taken += 1;
        sorting = next;
        sorted();
    }
    // How many sortings it has made and how many it took, in a plain
    // object, so counting notifies nothing.
    readonly property var sorts: ({ heard: 0, taken: 0 })

    // For the check that a shut launcher takes no change only it shows
    // (vm-desktop-launcher-keeps-up).
    IpcHandler {
        target: "pi"

        function sorts(): string {
            return JSON.stringify(root.sorts);
        }
    }
    // Every session it tells of, as the contract's records.
    readonly property var sessions: Pi.sessions(sorting, machine)

    // The sessions parked now, by the marker each keeps while it is (the
    // harness's park-workflow/parked/<session>.json), deleted as it starts
    // again. A record can go on saying "parked" after its session has
    // stopped being, so it's parked only while its marker stands. One
    // listing of one folder, which changes only as a session parks or
    // wakes, rather than a file watched for each.
    readonly property string parkedFolder: `${harness}/park-workflow/parked`
    // A listing of a folder that isn't there yet says it's empty, and never
    // hears of it after, and the harness makes its park folder only as a
    // session first parks. So the folders above it are listed too, each
    // of nothing but folders, which change only as one is made, and the
    // listing below one listed again as it does: never the one above,
    // which would list itself again for ever.
    function relist(listing, folder) {
        listing.folder = "";
        listing.folder = `file://${folder}`;
    }
    FolderListModel {
        folder: `file://${root.harness}`
        showFiles: false
        onCountChanged: root.relist(parkFolder, `${root.harness}/park-workflow`)
    }
    FolderListModel {
        id: parkFolder
        folder: `file://${root.harness}/park-workflow`
        showFiles: false
        onCountChanged: root.relist(markers, root.parkedFolder)
    }
    FolderListModel {
        id: markers
        folder: `file://${root.parkedFolder}`
        showDirs: false
        nameFilters: ["*.json"]
        onCountChanged: root.resort()
        onStatusChanged: root.resort()
    }
    function parkedMarkers() {
        const ids = {};
        if (markers.status !== FolderListModel.Ready)
            return ids;
        for (let at = 0; at < markers.count; at += 1)
            ids[String(markers.get(at, "fileName")).replace(/\.json$/, "")] = true;
        return ids;
    }
    function parks(presence, markers) {
        return Pi.parks(presence, markers, bootId);
    }

    // A parked session's marker, read when asked.
    FileView {
        id: marker
        blockLoading: true
        printErrors: false
    }
    function read(path) {
        marker.path = path;
        marker.reload();
        marker.waitForJob();
        return marker.text();
    }

    // The file that wakes a parked session, by its id, or "" for one not
    // parked: anything written to the file its marker names has its holder
    // start it again in its window (the harness's askWake).
    function wakes(id) {
        if (!resting.some(one => one.presence.session === id))
            return "";
        let parked = null;
        try {
            parked = JSON.parse(read(`${parkedFolder}/${id}.json`));
        } catch (error) {
            parked = null;
        }
        const file = typeof parked?.wake === "string" ? parked.wake : "";
        return file.startsWith("/") ? file : "";
    }

    // Puts a steer in a session's inbox by its address: `{ text, id }`
    // talks to it, `{ act }` pauses, resumes, stops or closes it. The
    // realm's node is asked on the connection kept to it, here or
    // elsewhere alike: it writes the inbox of the machine running the
    // session, one parked included, carrying it over the link when that's
    // another machine's. `told` is given the node's answer, or null where
    // there's no node to ask.
    function steer(address, fields, told) {
        const said = Object.assign({ at: new Date().toISOString(), from: "joel" }, fields);
        if (!node) {
            told(null);
            return;
        }
        // Its answer is for this steer alone, so nothing of it is held.
        node.find({ ask: "steer", address: String(address).toUpperCase(), steer: said }, told);
    }
}

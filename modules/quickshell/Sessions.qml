pragma Singleton
// pi's sessions, for the launcher's Agents and the pi piece: every pi
// running in a window, the one used last first, with its quest's whole
// title, and whether it's working, as each says.
//
// What runs comes from niri's windows, whose titles pi sets to "π -", its
// address, its quest's title cut short, and its folder, so a session is
// known the moment its window is and gone the moment it closes. Its quest's
// whole title is the first line of the quest's README, read once the quests
// are listed. Whether it's working comes from the file each session keeps
// in the mesh's presence folder while it runs, watched for changes; a
// session that keeps none says neither. Nothing is spawned and nothing is
// asked of pi.
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    readonly property string state: `${Quickshell.env("HOME")}/.local/state/pi/agentic-harness.pi`
    readonly property string quests: `${Quickshell.env("HOME")}/.local/share/pi/agentic-harness.pi/quest-workflow/quests`

    // Each window pi runs in: its window, its address, its quest's title as
    // the window gives it and as its README does, its folder, and what its
    // presence file says, or null.
    readonly property var running: {
        Niri.revision;
        listed;
        const when = window => {
            const stamp = window.focus_timestamp;
            return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
        };
        return Object.values(Niri.windows)
            .sort((a, b) => when(b) - when(a))
            .map(window => {
                const said = /^\u03c0 - (\S+)\s+(.*?)\s+-\s+([^-]*)$/.exec(window.title ?? "");
                return said ? {
                    window: window,
                    address: said[1],
                    title: titled(said[1], said[2]),
                    folder: said[3],
                    presence: present[said[1]] ?? null
                } : null;
            })
            .filter(session => session !== null);
    }
    readonly property int working: running.filter(session => session.presence && session.presence.busy).length

    // Each session's presence, by every address it answers to.
    property var present: ({})

    // A quest's whole title, by the address its id ends in, read once the
    // quests are listed; until then, the window's cut-short one.
    property var titles: ({})
    readonly property bool listed: folders.status === FolderListModel.Ready && folders.count >= 0
    function titled(address, cut) {
        if (titles[address] !== undefined)
            return titles[address] || cut;
        if (folders.status !== FolderListModel.Ready)
            return cut;
        let title = "";
        for (let i = 0; i < folders.count; i++) {
            const name = folders.get(i, "fileName");
            if (name.endsWith(`-${address}`)) {
                readme.path = `${quests}/${name}/README.md`;
                readme.reload();
                readme.waitForJob();
                const first = readme.text().split("\n").find(line => line.startsWith("# "));
                title = first ? first.slice(2).trim() : "";
                break;
            }
        }
        titles[address] = title;
        return title || cut;
    }

    FolderListModel {
        id: folders
        folder: `file://${root.quests}`
        showFiles: false
        showDotAndDotDot: false
    }

    FileView {
        id: readme
        blockLoading: true
        printErrors: false
    }

    FolderListModel {
        id: presences
        folder: `file://${root.state}/mesh-workflow/presence`
        showDirs: false
        nameFilters: ["*.json"]
    }

    Instantiator {
        model: presences

        FileView {
            required property string filePath
            path: filePath
            watchChanges: true
            printErrors: false
            onFileChanged: reload()
            onLoaded: root.heard(filePath, text())
        }
    }

    // Who a presence file belongs to, kept by each address it answers to.
    property var files: ({})
    function heard(file, text) {
        let said;
        try {
            said = JSON.parse(text);
        } catch (e) {
            return;
        }
        const next = Object.assign({}, present);
        for (const address of files[file] ?? [])
            delete next[address];
        files[file] = said.addresses ?? [];
        for (const address of files[file])
            next[address] = said;
        present = next;
    }
}

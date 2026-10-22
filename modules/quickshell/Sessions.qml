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

    // Where the harness keeps its state and its quests, by the XDG base
    // directories, as it finds them.
    readonly property string home: Quickshell.env("HOME")
    readonly property string state: `${Quickshell.env("XDG_STATE_HOME") || `${home}/.local/state`}/pi/agentic-harness.pi`
    readonly property string quests: `${Quickshell.env("XDG_DATA_HOME") || `${home}/.local/share`}/pi/agentic-harness.pi/quest-workflow/quests`

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
                const said = parsed(window.title ?? "");
                return said ? {
                    window: window,
                    address: said.address,
                    title: titled(said.address, said.cut),
                    folder: said.folder,
                    presence: present[said.address] ?? null
                } : null;
            })
            .filter(session => session !== null);
    }
    readonly property int working: running.filter(session => session.presence && session.presence.busy).length
    // Those waiting on Joel, as each says while it does: { kind, since,
    // said }, kind a question, an approval or blocked. The oldest first.
    readonly property var wanting: running
        .filter(session => session.presence && session.presence.wants)
        .sort((a, b) => String(a.presence.wants.since ?? "").localeCompare(String(b.presence.wants.since ?? "")))

    // A want new to this shell is said once as a notice from pi, whose
    // Return goes to the session's window (pi-wants-you, home.nix), unless
    // Joel is in that window already. Each is known by its session and
    // when it began, and kept known: a presence file replaced by rename
    // leaves its folder for a moment, which would otherwise make a want
    // that never ended look new.
    property var told: ({})
    onWantingChanged: {
        for (const session of wanting) {
            const key = `${session.address} ${session.presence.wants.since ?? ""}`;
            if (told[key])
                continue;
            told[key] = true;
            if (session.window.id === Niri.focused)
                continue;
            Niri.spawn(["pi-wants-you", String(session.window.id),
                `${session.title} ${wanted(session.presence.wants)}`,
                session.presence.wants.said ?? ""]);
        }
    }

    // What a want is, in words after the session's name.
    function wanted(wants) {
        return wants.kind === "approval" ? "wants your yes"
            : wants.kind === "blocked" ? "is stuck without you" : "asks you";
    }

    // A pi window's title: "π - ", its address, its quest's title cut
    // short, and " - " its folder, split at the last " - ", since a quest's
    // title may hold one and a folder's name hyphens; or null for any
    // other window.
    function parsed(title) {
        if (!title.startsWith("\u03c0 - "))
            return null;
        const rest = title.slice(4);
        const last = rest.lastIndexOf(" - ");
        const front = last < 0 ? rest : rest.slice(0, last);
        const space = front.indexOf(" ");
        const address = space < 0 ? front : front.slice(0, space);
        if (!/^[A-Z0-9]{4,}$/.test(address))
            return null;
        return {
            address: address,
            cut: space < 0 ? "" : front.slice(space + 1).trim(),
            folder: last < 0 ? "" : rest.slice(last + 3).trim()
        };
    }

    // Each session's presence, by every address it answers to.
    property var present: ({})

    // A quest's whole title, by the address its id ends in, read once it's
    // found; until then, and while its quest isn't listed, the window's
    // cut-short one, looked for again as the quests change.
    property var titles: ({})
    readonly property bool listed: folders.status === FolderListModel.Ready && folders.count >= 0
    function titled(address, cut) {
        if (titles[address])
            return titles[address];
        if (folders.status !== FolderListModel.Ready)
            return cut || address;
        for (let i = 0; i < folders.count; i++) {
            const name = folders.get(i, "fileName");
            // A folder Qt fell back to when the quests' own isn't there.
            if (!folders.get(i, "filePath").startsWith(quests) || !name.endsWith(`-${address}`))
                continue;
            readme.path = `${quests}/${name}/README.md`;
            readme.reload();
            readme.waitForJob();
            const title = heading(readme.text());
            if (title !== "")
                titles[address] = title;
            return title || cut || address;
        }
        return cut || address;
    }

    // The first heading of a README, past any front matter.
    function heading(text) {
        let lines = text.split("\n");
        if (lines[0].trim() === "---") {
            const end = lines.findIndex((line, i) => i > 0 && line.trim() === "---");
            lines = end < 0 ? [] : lines.slice(end + 1);
        }
        const first = lines.find(line => line.startsWith("# "));
        return first ? first.slice(2).trim() : "";
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
        // A session that ends takes its file with it, and its state goes too.
        onObjectRemoved: (index, object) => root.forget(object.path)

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
    function forget(file) {
        const next = Object.assign({}, present);
        for (const address of files[file] ?? [])
            delete next[address];
        delete files[file];
        present = next;
    }
    function heard(file, text) {
        // A folder Qt fell back to when the presence folder isn't there.
        if (!file.startsWith(state))
            return;
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

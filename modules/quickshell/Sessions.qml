pragma Singleton
// pi's sessions, for the launcher's Agents and the pi piece: every pi
// running in a window, the one used last first, with its quest's whole
// title, and whether it's working or wants Joel, as each says; and a
// notice once for each want, withdrawn when the want ends.
//
// What runs comes from niri's windows, whose titles pi sets to "π -", its
// address, its quest's title cut short, and its folder, so a session is
// known the moment its window is and gone the moment it closes. Its quest's
// whole title is the first line of the quest's README, read once the quests
// are listed. Whether it's working comes from the file each session keeps
// in the mesh's presence folder while it runs, watched for changes; a
// session that keeps none says neither. Nothing is asked of pi; a want is
// said by starting pi-wants-you (home.nix), which posts its notice.
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
                    title: titles[said.address] || said.cut || said.address,
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
    // Joel is in that window already, and one that stood before the shell
    // began isn't, since the margin's mark says it. Each is known by its
    // session, when it began and what it said, and kept known: a presence
    // file replaced by rename leaves its folder for a moment, which would
    // otherwise make a want that never ended look new. Its notice carries
    // that key, and is withdrawn once the want ends or its session goes.
    readonly property real began: Date.now()
    property var told: ({})
    function key(session) {
        const wants = session.presence.wants;
        return `${session.address} ${wants.since ?? ""} ${wants.said ?? ""}`;
    }
    onWantingChanged: {
        withdrawing.restart();
        for (const session of wanting) {
            const known = key(session);
            if (told[known])
                continue;
            told[known] = true;
            const since = Date.parse(session.presence.wants.since ?? "");
            if (session.window.id === Niri.current || (since < began))
                continue;
            Niri.spawn(["pi-wants-you", String(session.window.id), known,
                `${session.title} ${wanted(session.presence.wants)}`,
                plain(session.presence.wants.said ?? "")]);
            // Softly, and never while notices are held.
            if (Options.sounds.wants !== "" && !Quiet.on)
                Niri.spawn([Options.sounds.player, "--volume=0.5", Options.sounds.wants]);
        }
    }

    // A want's notice goes a moment after the want does, so a presence
    // file's rename, which takes it away for an instant, doesn't.
    Timer {
        id: withdrawing
        interval: 1000
        onTriggered: {
            const standing = {};
            for (const session of root.wanting)
                standing[root.key(session)] = true;
            for (const id in Notices.live) {
                const notice = Notices.live[id];
                const want = notice.hints["x-pi-want"];
                if (notice.appName === "pi" && want !== undefined && !standing[want]) {
                    Notices.mark(notice.id, true);
                    notice.dismiss();
                }
            }
        }
    }

    // Words as a notice's body draws them, which reads markup, so pi's own
    // say what they say. A summary is plain text, as the spec has it.
    function plain(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
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
    // cut-short one, looked for again as the windows and the quests change.
    // It's learned outside the list's binding, since reading each README
    // through one file view moves what the binding has just read.
    property var titles: ({})
    function learn() {
        if (folders.status !== FolderListModel.Ready)
            return;
        const next = Object.assign({}, titles);
        let changed = false;
        for (const window of Object.values(Niri.windows)) {
            const said = parsed(window.title ?? "");
            if (!said || next[said.address])
                continue;
            const title = read(said.address);
            if (title !== "") {
                next[said.address] = title;
                changed = true;
            }
        }
        if (changed)
            titles = next;
    }
    function read(address) {
        for (let i = 0; i < folders.count; i++) {
            const name = folders.get(i, "fileName");
            // A folder Qt fell back to when the quests' own isn't there.
            if (!folders.get(i, "filePath").startsWith(quests) || !name.endsWith(`-${address}`))
                continue;
            readme.path = `${quests}/${name}/README.md`;
            readme.reload();
            readme.waitForJob();
            return heading(readme.text());
        }
        return "";
    }

    Connections {
        target: Niri
        function onRevisionChanged() {
            root.learn();
        }
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
        onStatusChanged: root.learn()
        onCountChanged: root.learn()
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

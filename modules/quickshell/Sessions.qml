pragma Singleton
// pi's sessions, for the launcher's Agents and the pi piece: every pi
// running, the one whose window was used last first, with its quest's whole
// title, and whether it's working or wants Joel, as each says; and a
// notice once for each want, withdrawn when the want ends.
//
// A session is what its own record says: the file each keeps in the mesh's
// presence folder while it runs. The machine's mycelium node passes each
// record on, and says the moment a session starts, changes or ends, from
// its process rather than a clock (Jitsusama/mycelium), so while the node
// answers, the sessions are this machine's of those it holds. While it
// doesn't, as on a machine with none or while it restarts, the records
// are read from the folder, watched for changes, and a session is running
// while the process its record names is, on this boot, started when the
// record says; a record left by a session that died, or by an earlier
// boot, is passed over. Nothing is read from a window's title, which a
// program can set, a terminal can cut short and two sessions can share
// (Joel, 2026-10-27: "we should have a better system for agent session
// awareness that isn't so fragile"). Each reader here goes through
// `running` and the functions beside it, so neither way shows past them.
//
// A session's window is found through its terminal, as its record names it
// (`terminal`, or the kitty its process was started in): kitty says which
// of its windows the session is in, and niri lists each of kitty's windows
// by kitty's process. Its quest's whole title is the first line of the
// quest's README, read once the quests are listed. Nothing is asked of pi;
// a want is said by starting pi-wants-you (home.nix), which posts its notice.
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick
import "Waiting.js" as Waiting

Singleton {
    id: root

    // Where the harness keeps its state and its quests, by the XDG base
    // directories, as it finds them.
    readonly property string home: Quickshell.env("HOME")
    readonly property string state: `${Quickshell.env("XDG_STATE_HOME") || `${home}/.local/state`}/pi/agentic-harness.pi`
    readonly property string quests: `${Quickshell.env("XDG_DATA_HOME") || `${home}/.local/share`}/pi/agentic-harness.pi/quest-workflow/quests`

    // Each session running: its window or null, its address, its quest's
    // title as its README gives it or its record's label does, its folder,
    // its record, and the kitty window it runs in or null. The one whose
    // window was used last first, then those with none.
    readonly property var running: {
        Niri.revision;
        const when = window => {
            const stamp = window ? window.focus_timestamp : null;
            return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
        };
        const kitties = kittyWindows();
        return live.map(record => {
            const presence = record.presence;
            const address = (presence.addresses ?? [])[0];
            const label = labelled(typeof presence.title === "string" ? presence.title : "");
            return {
                window: windowOf(record.kitty, kitties),
                address: address,
                title: titles[address] || label.cut || address,
                folder: label.folder,
                presence: presence,
                kitty: record.kitty
            };
        }).sort((a, b) => when(b.window) - when(a.window));
    }
    readonly property int working: running.filter(session => session.presence.busy).length
    // Those waiting on Joel, as each says while it does: { kind, since,
    // said }, kind a question, an approval, blocked or an action, and only
    // while it's truly his (Waiting.js): an action while its quest has
    // something he can do now, and a question only from a session no
    // other orchestrates. The oldest first.
    readonly property var wanting: running
        .filter(session => asks(session))
        .sort((a, b) => String(a.presence.wants.since ?? "").localeCompare(String(b.presence.wants.since ?? "")))
    // The windows of those waiting on Joel, as one string, so a session's
    // other changes (working or not, its step) leave it the same and don't
    // remake the windows that read it.
    readonly property string waitingWindows: wanting
        .filter(session => session.window)
        .map(session => session.window.id).join(" ")

    // Starts a session on a quest, in `folder`, as the harness's own spawn
    // does: in the kitty that holds the sessions, through a login shell,
    // so the session has the account's whole environment, with the quest
    // loaded as it starts. Quests, Orchestrators and Agents' Start a
    // Session all start one this way, so a session started anywhere is the
    // same.
    // pi run without a login shell can't reach its model, and exits at
    // once, so a window started that way blinked and went (Joel,
    // 2026-10-28).
    function start(quest, folder) {
        Niri.spawn(["kitty", "-1", "--instance-group", "pi-sessions", "--directory", folder,
            "env", `QUEST_WORKFLOW_AUTOLOAD_ID=${quest}`, "zsh", "-lic", "exec pi"]);
        Report.say(`started a session on ${quest}`);
    }

    // Starts a session told `words` as it opens, as Google's sign-in is
    // "/google-auth" and Ask's carried question its own, in `folder`,
    // through a login shell for the same reason. The words go as the
    // shell's $0, so nothing in them is read as the shell's own.
    function tell(words, folder, program) {
        Niri.spawn(["kitty", "-1", "--instance-group", "pi-sessions", "--directory", folder,
            "zsh", "-lic", `exec ${program ?? "pi"} -- "$0"`, words]);
    }

    // Reopens a session that closed, where it left off: pi told the
    // session, in the folder it ran in, which its file's first line says,
    // as the harness restores one. With no file to say it, a session is
    // started afresh on the quest instead, in `folder`.
    readonly property string sessionFiles: `${home}/.pi/agent/sessions`

    function resume(id, quest, folder) {
        resumer.asked = { id: id, quest: quest, folder: folder };
        resumer.running = true;
    }

    Process {
        id: resumer
        property var asked: null
        command: ["sh", "-c", 'for f in "$1"/*/*_"$2".jsonl; do head -n 1 "$f"; break; done',
            "resume", root.sessionFiles, asked?.id ?? ""]
        stdout: StdioCollector {
            onStreamFinished: root.resumed(resumer.asked, text)
        }
    }

    function resumed(asked, text) {
        let cwd = "";
        try {
            cwd = JSON.parse(text.trim()).cwd ?? "";
        } catch (error) {
            cwd = "";
        }
        if (cwd === "") {
            start(asked.quest, asked.folder);
            return;
        }
        Niri.spawn(["kitty", "-1", "--instance-group", "pi-sessions", "--directory", cwd,
            "zsh", "-lic", `exec pi --session ${asked.id}`]);
        Report.say(`resumed session ${asked.id}`);
    }

    // Go to a session: its window, or, when niri's isn't known, kitty's,
    // which kitty raises if niri lets it.
    function goTo(session) {
        if (session.window)
            Niri.act({ FocusWindow: { id: session.window.id } });
        else if (session.kitty)
            Niri.spawn(["kitten", "@", "--to", session.kitty.socket, "focus-window",
                "--match", `id:${session.kitty.window}`]);
    }

    // What a session is, for the launcher to keep one row of it: its window,
    // which the windows and its quest also find, or else itself.
    function isOf(session) {
        return session.window ? `window:${session.window.id}` : `agent:${session.address}`;
    }

    // What a pi window is about beyond its title: its quest's whole title
    // and what its session says it works on, so Joel finds the window by
    // any word he remembers of its work (Joel, 2026-10-25: "i want to make
    // sure i can easily find the windows i care about even with my shit
    // memory"). Asked as a search runs, never bound, for the same reason.
    function aboutOf(id) {
        const session = running.find(one => one.window && one.window.id === id);
        if (!session)
            return "";
        return [session.title, session.presence.worksOn ?? ""]
            .filter(part => part !== "").join(" ");
    }

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
            const window = session.window ? String(session.window.id) : "";
            if (window === String(Niri.current) || (since < began))
                continue;
            Niri.spawn(["pi-wants-you", window, known,
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

    // Whether a session's want is truly on Joel, rather than one its
    // presence mirrors from a file of things he can't do yet, or a
    // question put to its orchestrator.
    function asks(session) {
        return session.presence !== null
            && Waiting.asksJoel(session.presence, WaitingOnYou.standing);
    }

    // What a want is, in words after the session's name.
    function wanted(wants) {
        return wants.kind === "approval" ? "wants your yes"
            : wants.kind === "blocked" ? "is stuck without you" : "asks you";
    }

    // The label a session's record gives its window, "π - ", its quest's
    // name and " - " its folder, split at the last " - ", since a quest's
    // name may hold one and a folder's name hyphens. Only ever shown.
    function labelled(title) {
        const rest = title.startsWith("\u03c0 - ") ? title.slice(4) : title;
        const last = rest.lastIndexOf(" - ");
        return {
            cut: (last < 0 ? rest : rest.slice(0, last)).trim(),
            folder: last < 0 ? "" : rest.slice(last + 3).trim()
        };
    }

    // This boot, which a record's owner names the boot it began in.
    FileView {
        id: boot
        path: "/proc/sys/kernel/random/boot_id"
        blockLoading: true
        printErrors: false
    }
    readonly property string bootId: boot.text().trim()

    // A process's own files, read when asked.
    FileView {
        id: process
        blockLoading: true
        printErrors: false
    }
    function read(path) {
        process.path = path;
        process.reload();
        process.waitForJob();
        return process.text();
    }

    // Whether the process a record names still runs: on this boot, and
    // started when the record says, since a pid is used again once its
    // process ends. Its start is the stat's 22nd field, in clock ticks,
    // counted after the name's closing bracket, as the harness counts it;
    // one ended but not yet reaped says Z just after it.
    function alive(owner) {
        if (!owner || !owner.pid || owner.bootId !== bootId)
            return false;
        const stat = read(`/proc/${owner.pid}/stat`);
        const name = stat.lastIndexOf(")");
        if (name < 0)
            return false;
        const fields = stat.slice(name + 2).split(" ");
        if (fields[0] === "Z")
            return false;
        return owner.startTicks === undefined || Number(fields[19]) === owner.startTicks;
    }

    // The kitty window a session runs in, { socket, window }, or null: as
    // its record's terminal says, or else as its process was started.
    readonly property var kittySocket: /^unix:@kitty-(\d+)$/
    function kittyOf(presence) {
        const terminal = presence.terminal;
        if (terminal && typeof terminal === "object")
            return terminal.driverId === "kitty" && terminal.scope && Number(terminal.value) > 0
                ? { socket: terminal.scope, window: Number(terminal.value) } : null;
        const environment = read(`/proc/${presence.owner.pid}/environ`).split("\u0000");
        const said = name => (environment.find(pair => pair.startsWith(`${name}=`)) ?? "")
            .slice(name.length + 1);
        const socket = said("KITTY_LISTEN_ON");
        const window = Number(said("KITTY_WINDOW_ID"));
        return socket !== "" && window > 0 ? { socket: socket, window: window } : null;
    }

    // Each running session's record, { file, presence, kitty }: as the node
    // has it while it answers, else each from the folder whose process
    // runs. Made again when a record changes, the node tells of a change or
    // a window comes or goes, and kept as it was when nothing in it did, so
    // what reads it isn't told for nothing.
    property var live: []
    property string liveSaid: ""
    function relive() {
        const next = [];
        // A node from before it passed records on says too little to show.
        if (node !== null && Object.values(node).every(session => session.presence)) {
            for (const id of Object.keys(node).sort()) {
                const session = node[id];
                const presence = session.presence;
                if (session.machine === machine && (session.state === "busy" || session.state === "idle")
                        && presence && (presence.addresses ?? []).length > 0)
                    next.push({ file: id, presence: presence, kitty: kittyOf(presence) });
            }
        } else {
            for (const file of Object.keys(records).sort()) {
                const presence = records[file];
                if ((presence.addresses ?? []).length > 0 && alive(presence.owner))
                    next.push({ file: file, presence: presence, kitty: kittyOf(presence) });
            }
        }
        const said = JSON.stringify(next);
        if (said === liveSaid)
            return;
        liveSaid = said;
        const by = {};
        for (const record of next) {
            for (const address of record.presence.addresses)
                by[address] = record.presence;
        }
        present = by;
        live = next;
        learn();
        ask();
    }

    // niri's windows by the process drawing them, each kitty's oldest first.
    function kittyWindows() {
        const by = {};
        for (const window of Object.values(Niri.windows)) {
            if (window.pid)
                (by[window.pid] = by[window.pid] ?? []).push(window);
        }
        for (const pid in by)
            by[pid].sort((a, b) => a.id - b.id);
        return by;
    }

    // A kitty window's niri window. A kitty with one window has it. One
    // with several numbers them as niri does, each newer than the last, so
    // the nth of kitty's is niri's nth of that kitty's, once kitty has said
    // which of its windows holds the session and has as many as niri shows.
    function windowOf(kitty, kitties) {
        const pid = kitty ? (kittySocket.exec(kitty.socket) ?? [])[1] : undefined;
        const windows = pid ? kitties[pid] ?? [] : [];
        if (windows.length === 1)
            return windows[0];
        const listed = ranks[kitty?.socket ?? ""];
        if (!listed || listed.windows !== windows.length || listed.of[kitty.window] === undefined)
            return null;
        return windows[listed.of[kitty.window]];
    }

    // What each kitty with several windows says of them: how many, and in
    // which of them each of its windows sits. Asked again only when niri's
    // windows of that kitty change, one kitty at a time.
    property var ranks: ({})
    property var asked: ({})
    property var asking: []
    function ask() {
        const kitties = kittyWindows();
        const sockets = {};
        for (const record of live) {
            if (record.kitty)
                sockets[record.kitty.socket] = true;
        }
        for (const socket in sockets) {
            const pid = (kittySocket.exec(socket) ?? [])[1];
            const windows = pid ? kitties[pid] ?? [] : [];
            const seen = windows.map(window => window.id).join(" ");
            if (windows.length > 1 && asked[socket] !== seen && !asking.includes(socket)) {
                asked[socket] = seen;
                asking = asking.concat([socket]);
            }
        }
        if (!lister.running && asking.length > 0) {
            lister.socket = asking[0];
            lister.running = true;
        }
    }
    Process {
        id: lister
        property string socket
        command: ["kitten", "@", "--to", socket, "ls"]
        stdout: StdioCollector {
            id: listing
            onStreamFinished: root.ranked(lister.socket, listing.text)
        }
        onExited: {
            root.asking = root.asking.filter(socket => socket !== lister.socket);
            Qt.callLater(root.ask);
        }
    }
    function ranked(socket, text) {
        let windows;
        try {
            windows = JSON.parse(text);
        } catch (e) {
            return;
        }
        if (!Array.isArray(windows))
            return;
        const of = {};
        windows.slice().sort((a, b) => a.id - b.id).forEach((window, rank) => {
            for (const tab of window.tabs ?? []) {
                for (const inside of tab.windows ?? [])
                    of[inside.id] = rank;
            }
        });
        const next = Object.assign({}, ranks);
        next[socket] = { windows: windows.length, of: of };
        ranks = next;
    }

    // A window that comes or goes can be a session's ending, or a kitty's
    // windows changing.
    property string windowsSeen: ""
    Connections {
        target: Niri
        function onRevisionChanged() {
            const seen = Object.keys(Niri.windows).join(" ");
            if (seen === root.windowsSeen)
                return;
            root.windowsSeen = seen;
            root.relive();
            root.ask();
        }
    }

    // A quest's whole title, by the address its id ends in, read once it's
    // found; until then, and while its quest isn't listed, the record's
    // label, looked for again as the sessions and the quests change. It's
    // learned outside the list's binding, since reading each README
    // through one file view moves what the binding has just read.
    property var titles: ({})
    function learn() {
        if (folders.status !== FolderListModel.Ready)
            return;
        const next = Object.assign({}, titles);
        let changed = false;
        for (const record of live) {
            const address = record.presence.addresses[0];
            if (next[address])
                continue;
            const title = titleOf(address);
            if (title !== "") {
                next[address] = title;
                changed = true;
            }
        }
        if (changed)
            titles = next;
    }
    function titleOf(address) {
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

    // The machine's mycelium node, for the realm the machine is in: the
    // realm's name, which the node keeps in its state, and this machine's,
    // which the node names each session's machine by.
    FileView {
        id: realmFile
        path: `${Quickshell.env("XDG_STATE_HOME") || `${root.home}/.local/state`}/mycelium/realm`
        blockLoading: true
        printErrors: false
    }
    readonly property string realm: realmFile.text().trim()
    FileView {
        id: hostname
        path: "/proc/sys/kernel/hostname"
        blockLoading: true
        printErrors: false
    }
    readonly property string machine: hostname.text().trim()

    // The node's sessions by id, everyone it holds, while it answers a
    // watch; null while it doesn't, so the folder is read instead.
    property var node: null
    Socket {
        id: mycelium
        path: root.realm !== "" && Quickshell.env("XDG_RUNTIME_DIR")
            ? `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/${root.realm}.sock` : ""
        connected: path !== ""
        onConnectionStateChanged: {
            if (connected) {
                write('{"Watch":{"groups":[],"recent":0}}\n');
                flush();
            } else if (root.node !== null) {
                root.node = null;
                root.relive();
            }
        }
        parser: SplitParser {
            onRead: line => root.nodeSaid(line)
        }
    }
    // A node not yet up, or restarting, is asked again every few seconds.
    Timer {
        interval: 5000
        repeat: true
        running: mycelium.path !== "" && !mycelium.connected
        onTriggered: mycelium.connected = true
    }
    // What the node tells a watch: every session first, then each one new,
    // changed or forgotten.
    function nodeSaid(line) {
        let event;
        try {
            event = JSON.parse(line);
        } catch (e) {
            return;
        }
        let next;
        if (event.SessionsChanged) {
            next = {};
            for (const session of event.SessionsChanged.sessions ?? [])
                next[session.id] = session;
        } else if (event.SessionChanged) {
            next = Object.assign({}, node ?? {});
            next[event.SessionChanged.session.id] = event.SessionChanged.session;
        } else if (event.SessionForgotten) {
            next = Object.assign({}, node ?? {});
            delete next[event.SessionForgotten.id];
        } else {
            return;
        }
        node = next;
        relive();
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

    // Each running session's record, by every address it answers to; and
    // every record, by its file.
    property var present: ({})
    property var records: ({})
    function forget(file) {
        const next = Object.assign({}, records);
        delete next[file];
        records = next;
        relive();
    }
    function heard(file, text) {
        // A folder Qt fell back to when the presence folder isn't there.
        if (!file.startsWith(state))
            return;
        let record;
        try {
            record = JSON.parse(text);
        } catch (e) {
            return;
        }
        if (!record || typeof record !== "object")
            return;
        const next = Object.assign({}, records);
        next[file] = record;
        records = next;
        relive();
    }
}

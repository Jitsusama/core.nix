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
import "Places.js" as Places
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
    // its record, the kitty window it runs in or null, the id its holder
    // keeps it by or null, the machine it runs on and whether that's this
    // one, the words that attach it from here or null, whether its
    // machine can be reached, and when it last changed. This machine's
    // first, the one whose window was used last first, then those with
    // none; then those elsewhere, the one that changed last first.
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
                kitty: record.kitty,
                held: record.held,
                machine: record.machine ?? machine,
                here: record.here ?? true,
                attach: record.attach ?? null,
                reachable: record.reachable ?? true,
                changed: record.changed ?? 0
            };
        }).sort((a, b) => Number(b.here) - Number(a.here)
            || (a.here ? when(b.window) - when(a.window) : b.changed - a.changed));
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
            "env", `QUEST_WORKFLOW_AUTOLOAD_ID=${quest}`, "zsh", "-lic", held("pi")]);
        Report.say(`started a session on ${quest}`);
    }

    // Starts a session told `words` as it opens, as Google's sign-in is
    // "/google-auth" and Ask's carried question its own, in `folder`,
    // through a login shell for the same reason. The words go as the
    // shell's $0, so nothing in them is read as the shell's own.
    function tell(words, folder, program) {
        Niri.spawn(["kitty", "-1", "--instance-group", "pi-sessions", "--directory", folder,
            "zsh", "-lic", held(`${program ?? "pi"} -- "$0"`), words]);
    }

    // What a launcher start's login shell runs: `command`, kept by the
    // realm's mycelium holder, so closing its window leaves the session
    // running and Go To attaches it again; or run as it is on a machine in
    // no realm. Every start goes through here, so each is held the same.
    readonly property bool holds: /^[A-Za-z0-9._-]+$/.test(realm)
    // A realm file naming no realm, as a hand edit could leave it, starts
    // every session unheld, which is said as it's read rather than silently.
    onRealmChanged: {
        if (realm !== "" && !holds)
            console.log(`sessions: the realm file says "${realm}", which names no realm, `
                + "so sessions start unheld");
    }
    function held(command) {
        return holds ? `exec mycelium --realm ${realm} hold -- ${command}` : `exec ${command}`;
    }

    // Reopens a session that closed, where it left off: pi told the
    // session, in the folder it ran in, which its file's first line says,
    // as the harness restores one. With no file to say it, a session is
    // started afresh on the quest instead, in `folder`. A session's folder
    // can be gone by now, a tree since given back, and kitty given a
    // missing folder opens in the home folder without a word, where pi
    // starts a new session apart from the old one (Joel's Aquifer
    // orchestrator, 2026-10-29). So the folder is looked for first; when
    // it's gone, the quest starts afresh in `folder` if that's there, and
    // `gone(cwd, startedIn)` is told either way, `startedIn` empty when
    // nothing could be opened, so the caller says so.
    readonly property string sessionFiles: `${home}/.pi/agent/sessions`

    // One at a time, in the order asked, so a second asked before the
    // first's file is read waits rather than taking its place.
    property var resuming: []
    function resume(id, quest, folder, gone) {
        resuming = resuming.concat([{ id: id, quest: quest, folder: folder, gone: gone }]);
        resumeNext();
    }
    function resumeNext() {
        if (resumer.running || resumer.asked !== null || resuming.length === 0)
            return;
        resumer.asked = resuming[0];
        resuming = resuming.slice(1);
        resumer.running = true;
    }

    Process {
        id: resumer
        property var asked: null
        command: ["sh", "-c", 'for f in "$1"/*/*_"$2".jsonl; do head -n 1 "$f"; break; done',
            "resume", root.sessionFiles, asked?.id ?? ""]
        onRunningChanged: Qt.callLater(root.resumeNext)
        stdout: StdioCollector {
            onStreamFinished: {
                const one = resumer.asked;
                resumer.asked = null;
                root.resumed(one, text);
                Qt.callLater(root.resumeNext);
            }
        }
    }

    function resumed(asked, text) {
        let cwd = "";
        try {
            cwd = JSON.parse(text.trim()).cwd ?? "";
        } catch (error) {
            cwd = "";
        }
        lookFor([cwd, asked.folder], there => {
            if (there[0]) {
                reopened({ id: asked.id }, cwd);
                return;
            }
            if (there[1])
                start(asked.quest, asked.folder);
            if (cwd !== "" || !there[1])
                asked.gone?.(cwd, there[1] ? asked.folder : "");
        });
    }

    // Whether each folder is there, told to `then` as a list of yes or no
    // in the same order. Asks made while one look runs wait for the next,
    // so Reopen Them All looks for every folder in one go.
    property var looking: []
    function lookFor(folders, then) {
        looking = looking.concat([{ folders: folders, then: then }]);
        look();
    }
    // Only once the last look has ended and been told, whichever of the
    // two Quickshell says first.
    function look() {
        if (looker.running || looker.asks.length > 0 || looking.length === 0)
            return;
        looker.asks = looking;
        looking = [];
        looker.command = ["sh", "-c",
            'for d; do if [ -n "$d" ] && [ -d "$d" ]; then echo y; else echo n; fi; done',
            "look"].concat(looker.asks.reduce((all, ask) => all.concat(ask.folders), []));
        looker.running = true;
    }
    function looked(asks, text) {
        const said = text.split("\n");
        let at = 0;
        for (const ask of asks) {
            const there = ask.folders.map((_, nth) => said[at + nth] === "y");
            at += ask.folders.length;
            ask.then(there);
        }
        looker.asks = [];
        Qt.callLater(look);
    }
    Process {
        id: looker
        property var asks: []
        onRunningChanged: Qt.callLater(root.look)
        stdout: StdioCollector {
            onStreamFinished: root.looked(looker.asks, text)
        }
    }

    // Says, as a notice of the desktop's own, why a session didn't open as
    // asked, in the words the caller gives.
    function sayUnopened(title, body) {
        Niri.spawn(["notify-send", "--app-name=Sessions", "--", title, body]);
        Report.say(`unopened ${title}: ${body}`);
    }

    // Opens a closed session again where it left off, in a window of its
    // own in `cwd`, held as every start is. One pi never saved, as it
    // saves none before its first answer, is `fresh` in restore.json
    // (85PKB2, 2026-11-01 [e51c350d]): `pi --session` finds no record of
    // it and exits at once, so it starts under its own id, with its quest
    // loaded as start() loads one, since only a record brings one back.
    // `told`, when given, is pi's first prompt, sent as it opens, as the
    // shell's $0 so nothing in it is read as the shell's own.
    function reopened(one, cwd, told) {
        const fresh = one.fresh === true;
        const quest = fresh && typeof one.quest === "string" && one.quest !== ""
            ? ["env", `QUEST_WORKFLOW_AUTOLOAD_ID=${one.quest}`] : [];
        const pi = fresh ? `pi --session-id ${one.id}` : `pi --session ${one.id}`;
        const telling = typeof told === "string" && told !== "";
        // One another session started opens without the focus, by niri's
        // rule for its class (modules/niri/config.kdl), and without the
        // activation token niri hands what it spawns, which kitty would
        // spend on taking the focus once its window shows.
        const quiet = one.quiet === true;
        const untold = quiet ? ["env", "-u", "XDG_ACTIVATION_TOKEN", "-u", "DESKTOP_STARTUP_ID"]
            : [];
        const apart = quiet ? ["--class", "pi-spawned"] : [];
        Niri.spawn(untold.concat(["kitty", "-1", "--instance-group", "pi-sessions"], apart,
            ["--directory", cwd], quest, ["zsh", "-lic", held(telling ? `${pi} -- "$0"` : pi)],
                telling ? [told] : []));
        Report.say(`${fresh ? "started unsaved" : "resumed"} session ${one.id}`);
    }

    // The sessions the last boot's end took away, to reopen after a cold
    // boot (Joel, 2026-10-26: one act brings back every session live at
    // shutdown, each in its own window and folder, never one from before
    // that boot). The harness keeps them in restore.json (its #866),
    // rewritten whole as each pi starts and ends, with the boot they ran
    // in, so a reopened one leaves the list within seconds. Only another
    // boot's are offered, and only until Joel puts that boot off, but for
    // those another machine carried here (`carried`). This boot is
    // `bootId`, as the records are read by.
    readonly property var restoring: restoreOf(restoreFile.text())
    FileView {
        id: restoreFile
        path: `${Quickshell.env("XDG_STATE_HOME") || `${root.home}/.local/state`}/pi/restore.json`
        blockLoading: true
        printErrors: false
        // Written by rename and never removed, so it's seen each time.
        watchChanges: true
        onFileChanged: reload()
    }

    // What the file says, keeping only the sessions it names whole: an id
    // safe to hand pi's command line and a folder to open in.
    function restoreOf(text) {
        let read = null;
        try {
            read = JSON.parse(text);
        } catch (error) {
            read = null;
        }
        if (read === null || read.v !== 1 || !Array.isArray(read.sessions))
            return { boot: "", sessions: [] };
        // One held on another machine is joined again by its words, given
        // as an argv (the harness's #872); its folder is only where the
        // window opens here, and may be unknown.
        const elsewhere = one => typeof one.machine === "string" && one.machine !== ""
            && typeof one.address === "string" && Array.isArray(one.attach)
            && one.attach.length > 0 && one.attach.every(word => typeof word === "string")
            && typeof one.cwd === "string" && (one.cwd === "" || one.cwd.startsWith("/"));
        // One another machine carried here as it went away (the harness's
        // carried.ts) says which and when; one saying it badly is no last
        // boot's either, so it's left out.
        const carried = one => one.carried === undefined
            || one.carried !== null && typeof one.carried === "object"
            && typeof one.carried.from === "string" && one.carried.from !== ""
            && typeof one.carried.at === "string";
        return {
            boot: typeof read.boot === "string" ? read.boot : "",
            sessions: read.sessions.filter(one => one !== null && typeof one === "object"
                && /^[A-Za-z0-9._-]+$/.test(String(one.id ?? "")) && carried(one)
                && (one.machine === undefined
                    ? typeof one.cwd === "string" && one.cwd.startsWith("/") : elsewhere(one)))
        };
    }

    // The boot Joel said Not Now to, kept by the shell, so its sessions
    // stay put off however often the shell starts on this boot. Putting
    // one off says so at once, as its file is written behind.
    property string putOff: {
        try {
            return String(JSON.parse(putOffFile.text()).boot ?? "");
        } catch (error) {
            return "";
        }
    }
    FileView {
        id: putOffFile
        path: Quickshell.statePath("last-boot-put-off.json")
        blockLoading: true
        printErrors: false
    }

    readonly property var lastBoot: restoring.boot !== "" && bootId !== ""
        && restoring.boot !== bootId && restoring.boot !== putOff
        ? restoring.sessions.filter(one => one.carried === undefined) : []

    // The sessions other machines carried here as they went away, offered
    // whatever the boot until each runs here, when the harness drops it,
    // and never put off with the last boot's (VKCF3Z, [17db838c]): the
    // newest carried first, by id when two came at once.
    readonly property var carried: restoring.sessions.filter(one => one.carried !== undefined)
        .sort((one, other) => other.carried.at.localeCompare(one.carried.at)
            || one.id.localeCompare(other.id))

    // Every session there is to reopen: the last boot's, then the carried.
    readonly property var toReopen: lastBoot.concat(carried)

    // One of them, in its own folder only: when that's gone, nothing is
    // opened, and `gone(cwd)` is told so the caller says so, since there's
    // no quest here to start afresh.
    function reopen(lost, gone) {
        if (lost.machine !== undefined) {
            rejoin(lost);
            return;
        }
        lookFor([lost.cwd], there => {
            if (there[0])
                reopened(lost, lost.cwd);
            else
                gone?.(lost.cwd, "");
        });
    }

    // A window the last boot had joined to a session another machine
    // holds is joined again by its words, in its folder here when that's
    // still there, or gone to if a window here has joined it since, never
    // opened twice.
    function rejoin(lost) {
        const joined = running.find(one => !one.here && one.address === lost.address
            && (one.window || one.kitty));
        if (joined) {
            goTo(joined);
            return;
        }
        const joining = `${lost.machine}:${lost.address}`;
        if (Date.now() - (attaching[joining] ?? 0) < attachPatience)
            return;
        attaching = Object.assign({}, attaching, { [joining]: Date.now() });
        const open = there => {
            Niri.spawn(["kitty", "-1", "--instance-group", "pi-sessions"]
                .concat(there ? ["--directory", lost.cwd] : [], lost.attach));
            Report.say(`joined session ${lost.address} on ${lost.machine} again`);
        };
        if (lost.cwd === "")
            open(false);
        else
            lookFor([lost.cwd], there => open(there[0]));
    }

    function putOffLastBoot() {
        putOff = restoring.boot;
        putOffFile.setText(JSON.stringify({ boot: putOff }));
    }

    // Where each session's window sits, kept so Reopen Them All puts every
    // one back on its workspace in its order after a cold boot (Joel,
    // 2026-10-29, after a restore: "every session is all over the place on
    // one workspace, out of order"). Each is kept by the session's own id,
    // which pi keeps as a session is reopened: the screen, by what it is
    // and by its connector, and the workspace as niri numbers it there,
    // then its column and its tile in that column. One's kept after it ends, since a boot's end ends them
    // one by one, and only this boot's and the one before it are kept
    // (arrangedBefore). Every other window's place is kept beside them, by
    // its program and title (`others`), so Reopen Them All can put back
    // those that reopen themselves. It's written once the windows have
    // been still a moment, never for each change.
    FileView {
        id: arrangementFile
        path: Quickshell.statePath("session-arrangement.json")
        blockLoading: true
        printErrors: false
        // Read again whenever it's written, by this shell or another hand.
        watchChanges: true
        onFileChanged: reload()
    }
    readonly property var arrangementRead: {
        try {
            const read = JSON.parse(arrangementFile.text());
            return read?.v === 1 && read.boots && typeof read.boots === "object" ? read : {};
        } catch (error) {
            return {};
        }
    }
    readonly property var arrangement: arrangementRead.boots ?? {}
    readonly property var arrangedOthers: arrangementRead.others
        && typeof arrangementRead.others === "object" ? arrangementRead.others : {}
    // Every tiled window but kitty's, whose windows are sessions or a
    // terminal of Joel's own, oldest first: Chrome's, Slack's and the like.
    function othersNow() {
        return Object.values(Niri.windows)
            .filter(window => window.app_id !== "kitty" && window.layout?.pos_in_scrolling_layout
                && Niri.workspace(window.workspace_id))
            .sort((a, b) => a.id - b.id)
            .map(window => {
                const workspace = Niri.workspace(window.workspace_id);
                const at = window.layout.pos_in_scrolling_layout;
                return { app: window.app_id ?? "", title: window.title ?? "",
                    screen: screens[workspace.output] ?? "", output: workspace.output ?? "",
                    workspace: workspace.idx, column: at[0], tile: at[1] };
            });
    }
    // The boot kept beside this one: the boot being restored when its places
    // are kept, else the one before this. Not only while restore.json names
    // it, since that moves on once its sessions are back: on 2026-11-02 it
    // named another boot at 18:41, and the places Reopen Them All had just
    // used were dropped at 18:43, so nobody could see which it misplaced.
    // This boot's is written first, so the first other is the one before.
    function arrangedBefore() {
        if (restoring.boot !== "" && restoring.boot !== bootId && arrangement[restoring.boot])
            return restoring.boot;
        return Object.keys(arrangement).find(boot => boot !== bootId) ?? "";
    }
    function arrangedNow() {
        const now = {};
        running.forEach(session => {
            const id = session.presence.session;
            const at = session.window?.layout?.pos_in_scrolling_layout;
            const workspace = session.window ? Niri.workspace(session.window.workspace_id) : null;
            if (session.here && typeof id === "string" && at && workspace)
                now[id] = { screen: screens[workspace.output] ?? "",
                    output: workspace.output ?? "", workspace: workspace.idx,
                    column: at[0], tile: at[1] };
        });
        return now;
    }
    // Each connector's screen as niri describes it (Places.js), asked for
    // as places are kept and as they're reopened, since a dock or a boot
    // can move a screen to another connector. Kept as last said when niri
    // doesn't answer. niri never answers a request it refused, nor one
    // sent on a connection that closed, so Reopen Them All waits no more
    // than screensPatience on it, rather than never reopening at all.
    property var screens: ({})
    property var screensWaiting: []
    function lookAtScreens(then) {
        screensWaiting = screensWaiting.concat([then]);
        if (screensWaiting.length > 1)
            return;
        screensPatience.restart();
        Niri.request("Outputs", answer => {
            if (answer?.Outputs)
                screens = Places.screensOf(answer.Outputs);
            screensLooked();
        });
    }
    function screensLooked() {
        screensPatience.stop();
        const waiting = screensWaiting;
        screensWaiting = [];
        waiting.forEach(then => then());
    }
    Timer {
        id: screensPatience
        interval: 2000
        onTriggered: root.screensLooked()
    }
    function keepArrangement() {
        if (bootId === "")
            return;
        const boots = { [bootId]: Object.assign({}, arrangement[bootId] ?? {}, arrangedNow()) };
        const others = { [bootId]: root.othersNow() };
        const before = root.arrangedBefore();
        if (before !== "") {
            boots[before] = arrangement[before];
            if (Array.isArray(arrangedOthers[before]))
                others[before] = arrangedOthers[before];
        }
        const text = JSON.stringify({ v: 1, boots, others });
        if (text !== arrangementFile.text())
            arrangementFile.setText(text);
    }
    Timer {
        id: arranging
        interval: 2000
        onTriggered: root.lookAtScreens(() => {
            root.comeHome();
            root.putBackOpen();
            root.keepArrangement();
        })
    }
    onRunningChanged: arranging.restart()
    Connections {
        target: Niri
        function onLayoutsChanged() {
            arranging.restart();
            root.placeOpened();
            root.settleReopenedSoon();
        }
        // A screen coming or going changes the workspaces alone, and one
        // that comes may be where workspaces wait to go (comeHome), or
        // where a session reopened meanwhile goes.
        function onWorkspacesChanged() {
            if (Object.keys(root.awayFrom).length > 0)
                arranging.restart();
            root.placeOpened();
            root.settleReopenedSoon();
        }
        // The window a session being reopened opens in, placed once niri
        // has laid it out, and the next begun.
        function onWindowOpenedOrChanged(window) {
            if (root.reopening && window.app_id === "kitty")
                root.reopenedWindow(window.id);
            root.placeOpened();
            root.settleReopenedSoon();
            if (root.puttingBack !== null && !root.placingNow())
                root.putBack(window);
        }
    }
    // Whose a new window is. kitty opens them in the order they were
    // asked for, so one that comes after its session was given up on is
    // that session's, the oldest given up on first, and goes to its own
    // workspace; only a window none of those is waiting for is the
    // session being reopened now's. A slow first kitty after a cold boot
    // would otherwise be taken for the next one's, and both misplaced.
    function reopenedWindow(id) {
        const going = reopening;
        const key = String(id);
        if (going.taken.includes(key))
            return;
        const late = going.late[0];
        if (late && !late.known.includes(key)) {
            going.taken.push(key);
            going.late = going.late.slice(1);
            root.besideKept(id, late.kept);
            if (late.quiet && late.at === null) {
                root.placeQuietly(id, late);
                if (going.late.length === 0 && going.queue.length === 0 && !going.waiting
                    && !going.looking && !going.settling)
                    root.reopenDone();
                return;
            }
            const entry = { id, at: late.at, away: late.away, quiet: late.quiet === true };
            const running = going.queue.length > 0 || going.waiting !== null || going.looking;
            // Placed now while the rest are still being reopened, and by
            // the check after them; once that's done, by its own check.
            if (running && entry.at !== null)
                root.placeSession(entry, root.besideOf(going.placed));
            if (entry.at !== null)
                going.placed.push(entry);
            if (going.settling)
                root.settleReopenedSoon();
            else if (!running)
                root.settleReopened();
            return;
        }
        const waiting = going.waiting;
        if (waiting && waiting.opened === null && !waiting.known.includes(key)) {
            going.taken.push(key);
            waiting.opened = id;
        }
    }
    // The session being reopened, once its window is laid out: put where
    // it was kept, beside the sessions back before it, and the next begun.
    // It's placed by those, as niri has them now, never by what's focused
    // or by a workspace's id found before it opened: niri opens a window
    // beside whatever column is focused, which Joel moves as he clicks,
    // types or switches workspaces meanwhile, and a screen that drops and
    // comes back makes its workspaces again. At the 21:55 boot on
    // 2026-11-02 the monitor did so mid-reopen, and the first session of
    // its second workspace stayed at the end of its first. One with no
    // place kept stays beside Joel, where it opened.
    function placeOpened() {
        const waiting = reopening?.waiting;
        if (!waiting || waiting.opened === null || !Niri.windows[waiting.opened])
            return;
        const entry = { id: waiting.opened, at: waiting.at, away: waiting.away,
            quiet: waiting.quiet === true };
        if (entry.at !== null) {
            if (root.placeSession(entry, root.besideOf(reopening.placed)) === "waiting")
                return;
            reopening.placed.push(entry);
        } else if (entry.quiet && root.placeQuietly(entry.id, waiting) === "waiting") {
            return;
        }
        root.besideKept(entry.id, waiting.kept);
        reopening.waiting = null;
        reopenPatience.stop();
        reopenSoon();
    }
    // Puts a session another session started, with no place kept, with its
    // quest's sessions, or beside the one that started it, else leaves it
    // where it opened, beside Joel's column, all without the focus: Joel types on in his
    // window, and his workspace stays (Joel, 2026-11-02: "when quests load
    // up new sessions, i often end up typing into them by accident"). niri
    // puts a window moved so just right of the column last active there.
    // "waiting" while niri hasn't laid it out.
    function placeQuietly(id, waiting) {
        const window = Niri.windows[id];
        if (!window || !Niri.workspace(window.workspace_id))
            return "waiting";
        const others = root.running.filter(session => session.window
            && session.window.id !== id);
        const kin = others.find(session => waiting.quest !== ""
            && session.presence.quest === waiting.quest)
            ?? others.find(session => waiting.from !== ""
                && session.presence.session === waiting.from);
        const into = kin?.window.workspace_id ?? window.workspace_id;
        if (into === window.workspace_id)
            return "held";
        Niri.act({ MoveWindowToWorkspace: { window_id: id, reference: { Id: into },
            focus: false } });
        return "moved";
    }
    // Puts a session reopened where whereBack says, beside the sessions
    // back before it (Places.js), when it isn't there already: "held" when
    // it was, "moved", or "waiting" while niri hasn't laid it out or its
    // screen has no workspace yet, as while a screen comes back. One
    // another session handed over (`quiet`), as `quest restore` does, goes
    // to its workspace without the focus, and so wherever niri puts it
    // there, since only a column that's focused can be moved.
    function placeSession(entry, beside) {
        const window = Niri.windows[entry.id];
        const workspace = window ? Niri.workspace(window.workspace_id) : null;
        const column = window?.layout?.pos_in_scrolling_layout?.[0];
        if (!workspace || column === undefined)
            return "waiting";
        const found = Places.whereBack(entry.at, beside);
        const where = found.make ? Object.assign({}, found,
            { output: found.output ?? entry.at.output }) : found;
        const holds = entry.quiet === true && !where.make ? workspace.id === where.join
            : Places.holds(where, { workspace, column }, beside);
        if (holds) {
            root.noteAway(entry.away, workspace.id);
            return "held";
        }
        const into = entry.quiet === true ? root.moveQuietly(window, where, workspace)
            : root.moveTo(window, where, workspace, column, beside);
        if (into === null)
            return "waiting";
        root.noteAway(entry.away, into);
        return "moved";
    }
    // Moves a window where whereBack says, by its id and its neighbours'
    // rather than by what's focused, and answers the workspace it goes to,
    // or null when its screen has none to take it. niri puts a window moved
    // to a workspace just right of the column active there, so the one it
    // goes beside is focused first; and one on that workspace already is
    // moved to its column by number, since a move to the workspace it's on
    // does nothing. These focus, so Joel's window is focused again once
    // every window is placed.
    function moveTo(window, where, workspace, column, beside) {
        const id = window.id;
        if (where.make) {
            const into = root.madeFor(id, where, workspace);
            if (into === null)
                return null;
            if (into !== workspace.id)
                Niri.act({ MoveWindowToWorkspace: { window_id: id, reference: { Id: into },
                    focus: true } });
            Niri.act({ MoveWorkspaceToIndex: { index: where.index, reference: { Id: into } } });
            return into;
        }
        if (workspace.id === where.join) {
            Niri.act({ FocusWindow: { id } });
            Niri.act({ MoveColumnToIndex: { index: Places.columnTo(where, column, beside) } });
        } else {
            Niri.act({ FocusWindow: { id: where.after ?? where.before } });
            Niri.act({ MoveWindowToWorkspace: { window_id: id, reference: { Id: where.join },
                focus: true } });
            if (where.before !== undefined) {
                Niri.act({ FocusWindow: { id } });
                Niri.act({ MoveColumnLeft: {} });
            }
        }
        if (where.under)
            Niri.act({ ConsumeOrExpelWindowLeft: { id } });
        return where.join;
    }
    // Moves a window to the workspace whereBack says without the focus,
    // answering it as moveTo does: Joel's window and workspace stay his.
    function moveQuietly(window, where, workspace) {
        const into = where.make ? root.madeFor(window.id, where, workspace) : where.join;
        if (into === null)
            return null;
        if (into !== workspace.id)
            Niri.act({ MoveWindowToWorkspace: { window_id: window.id,
                reference: { Id: into }, focus: false } });
        if (where.make)
            Niri.act({ MoveWorkspaceToIndex: { index: where.index, reference: { Id: into } } });
        return into;
    }
    // The workspace a window given one of its own goes to: the one it's on
    // when it's alone there on its screen, else the empty one niri keeps
    // last, or null when its screen has none.
    function madeFor(id, where, workspace) {
        const alone = Object.values(Niri.windows).every(other => other.id === id
            || other.workspace_id !== workspace.id);
        return alone && workspace.output === where.output ? workspace.id
            : root.lastOn(where.output);
    }

    // Once the last is placed, each session reopened is checked again, the
    // lowest kept first, beside those kept before it, and the first not
    // where it goes is moved; then the next round, once niri has said
    // what moved. A window that opened while Joel moved the focus, or that
    // a screen coming back moved, so goes back. It ends when every one
    // holds, or after a round for each and a few more, so two that can't
    // both be had never move back and forth.
    function settleReopened() {
        if (!reopening)
            return;
        reopening.settling = true;
        reopening.rounds = 0;
        if (reopening.back === null)
            reopening.back = Niri.current;
        reopenSettling.interval = 150;
        reopenSettling.restart();
    }
    function settleReopenedSoon() {
        if (!reopening?.settling)
            return;
        reopenSettling.interval = 150;
        reopenSettling.restart();
    }
    Timer {
        id: reopenSettling
        onTriggered: root.settleNextReopened()
    }
    function settleNextReopened() {
        const going = reopening;
        if (!going?.settling)
            return;
        const ordered = going.placed.filter(one => Niri.windows[one.id])
            .sort((a, b) => root.keptOrder(a.at, b.at));
        if (going.rounds < ordered.length + 3) {
            for (let at = 0; at < ordered.length; at += 1) {
                const beside = root.besideOf(ordered.slice(0, at));
                if (root.placeSession(ordered[at], beside) === "moved") {
                    going.rounds += 1;
                    // Sooner once niri says what moved (settleReopenedSoon).
                    reopenSettling.interval = 1000;
                    reopenSettling.restart();
                    return;
                }
            }
        }
        root.reopenDone();
    }
    // What waited on the last being placed: windows that reopen themselves
    // put back, a gone screen's workspaces taken to it if it came
    // meanwhile, and Joel's window focused again. Kept while a window given
    // up on may still come, to place it.
    function reopenDone() {
        const going = reopening;
        going.settling = false;
        reopenSettling.stop();
        root.putBackOpen();
        arranging.restart();
        // Only when a session reopened moved it: one another session
        // started leaves Joel's focus where he has it.
        if (going.back !== null && going.focused)
            Niri.act({ FocusWindow: { id: going.back } });
        going.back = null;
        going.focused = false;
        if (going.late.length === 0)
            reopening = null;
    }
    // The order places are kept in: a screen and its workspaces in the
    // order niri numbers them, each column left to right, each tile top
    // down.
    function keptOrder(a, b) {
        return a.output.localeCompare(b.output) || a.workspace - b.workspace
            || a.column - b.column || a.tile - b.tile;
    }

    // The next is begun no sooner than this after the last was, since
    // every pi starting at once is a load Joel skipped Reopen Them All
    // for (Joel, 2026-10-30: "i would just like about a second or two
    // between each individual launch"). Each still waits for the one
    // before to open first, since niri puts a window beside the column
    // focused as it opens, and the next one's place would be focused by
    // then.
    readonly property int reopenPace: 2000
    property real reopenStartedAt: 0
    function reopenSoon() {
        const wait = reopenStartedAt + reopenPace - Date.now();
        if (wait > 0 && reopening && reopening.queue.length > 0) {
            reopenPacing.interval = wait;
            reopenPacing.restart();
        } else {
            reopenNext();
        }
    }
    Timer {
        id: reopenPacing
        onTriggered: root.reopenNext()
    }

    // Reopens every session the last boot took away, each in its own
    // folder, back where it sat: those held on another machine joined
    // again at once, then this machine's one at a time, a screen and its
    // workspaces in the order niri numbers them and each column after the
    // last, each placed beside those back before it (placeOpened). Each
    // waits for niri to open the one before, or half a minute if it never
    // does, and for two seconds after it began (reopenPace). One with
    // nowhere kept opens beside where Joel was, after the rest, and Joel
    // is taken back there at the end. `gone(one, cwd)` is
    // told of one whose folder is gone, which isn't opened. `keptAt(id)`
    // says where one sat, the boot being restored's place by default. A
    // list given while another is being reopened joins its end, so two
    // never open at once and misplace each other.
    // `told`, when given, is each one's first prompt as it opens
    // (reopened), as Reopen Them All tells each to carry on.
    property var reopening: null
    function reopenAll(lost, gone, keptAt, told) {
        const kept = keptAt ?? (id => arrangement[restoring.boot]?.[id] ?? null);
        lost.filter(one => one.machine !== undefined).forEach(one => rejoin(one));
        const here = lost.filter(one => one.machine === undefined);
        lookAtScreens(() => reopenHere(here, gone, kept, told ?? ""));
    }
    // What Reopen Them All tells each session it reopens, so a boot never
    // leaves them waiting to be poked one by one (P9R1KI, 2026-11-02
    // [2c47b9fa], after the 14:50 boot left every session idle until Joel
    // poked each by hand). It's said to pi, not shown in the launcher, so
    // it reads as the harness's own automated turns do.
    readonly property string carryOn: "[Automated message, no human input: the machine "
        + "restarted, and Reopen Them All reopened this session.] Carry on from where you "
        + "left off: your plan's next open item."
    // Each kept place found on its own screen as it's connected now, or a
    // gone screen's after the one in use's (Places.js), then in order.
    function reopenHere(here, gone, kept, told) {
        const order = (a, b) => !a.at || !b.at ? Number(!a.at) - Number(!b.at)
            : root.keptOrder(a.at, b.at);
        const keptPlaces = here.map(one => kept(one.id));
        const places = Places.placed(keptPlaces, screens, Niri.workspace()?.output ?? null);
        const queue = here.map((one, at) => ({ one, at: places[at], kept: keptPlaces[at], gone,
            told })).sort(order);
        if (reopening !== null) {
            reopening.queue = reopening.queue.concat(queue);
            if (reopening.waiting === null && !reopening.looking && !reopenPacing.running)
                reopenSoon();
            return;
        }
        reopening = {
            queue,
            back: Niri.current,
            waiting: null,
            looking: false,
            late: [],
            taken: [],
            placed: [],
            settling: false,
            rounds: 0,
            focused: false
        };
        reopenNext();
    }

    // Sessions the harness hands over to reopen, as `quest restore` and
    // `quest spawn` do rather than open every window at once, so one
    // placer puts each where it belongs (85PKB2, 2026-10-30 [020fe1dd]:
    // six restored at once came out reversed, every pi starting together).
    // A JSON object whose `sessions` lists each {id, cwd}, rather than a bare
    // list, since `qs ipc call` doesn't pass an argument starting with `[`
    // through whole (2026-10-31). Each is reopened as Reopen Them All does, at
    // its place kept this boot or else the restored boot's, else beside
    // where Joel is. It answers "taken" at once, so the harness knows the
    // shell has them; a shell that doesn't answer leaves it to open them
    // itself. It's asked through the sessions target below.
    function handedOver(sessions) {
        let read = null;
        try {
            read = JSON.parse(sessions);
        } catch (error) {
            read = null;
        }
        const listed = Array.isArray(read) ? read
            : Array.isArray(read?.sessions) ? read.sessions : null;
        if (listed === null)
            return "refused: not a list of sessions";
        const given = listed.filter(one => one !== null && typeof one === "object"
            && /^[A-Za-z0-9._-]+$/.test(String(one.id ?? ""))
            && typeof one.cwd === "string" && one.cwd.startsWith("/"))
            .map(one => ({ id: String(one.id), cwd: one.cwd, fresh: one.fresh === true,
                quest: typeof one.quest === "string" ? one.quest : "",
                from: typeof one.from === "string" ? one.from : "",
                quiet: one.focus !== true }));
        if (given.length === 0)
            return "refused: no session with an id and a folder";
        console.info(`sessions: reopening ${given.length} handed over`);
        reopenAll(given, (one, cwd) => console.info(`sessions: ${one.id} not reopened, `
            + `since its folder, ${cwd}, is gone`),
            id => arrangement[bootId]?.[id] ?? arrangement[restoring.boot]?.[id] ?? null);
        return given.length === listed.length ? "taken"
            : `taken ${given.length} of ${listed.length}`;
    }
    function reopenNext() {
        if (!reopening) {
            root.putBackOpen();
            arranging.restart();
            return;
        }
        const next = reopening.queue[0];
        if (!next) {
            root.settleReopened();
            return;
        }
        reopening.queue = reopening.queue.slice(1);
        reopening.looking = true;
        lookFor([next.one.cwd], there => {
            reopening.looking = false;
            if (!there[0]) {
                next.gone?.(next.one, next.one.cwd);
                reopenNext();
                return;
            }
            // Where it goes is focused first, so it most often opens there
            // and is seldom seen to move, but for one another session
            // started, which never moves the focus.
            const quiet = next.one.quiet === true;
            if (!quiet)
                reopening.focused = true;
            if (!quiet && next.at) {
                // A screen gone was given the screen in use (Places.js);
                // with none known, it stays on the one in use.
                if (Object.values(Niri.workspaces).some(one => one.output === next.at.output))
                    Niri.act({ FocusMonitor: { output: next.at.output } });
                Niri.act({ FocusWorkspace: { reference: { Index: next.at.workspace } } });
                Niri.act({ FocusColumnLast: {} });
            } else if (!quiet && reopening.back !== null) {
                Niri.act({ FocusWindow: { id: reopening.back } });
            }
            // Placed by its connector and the number it's given now, so it
            // goes beside those given the same screen, a gone one's too.
            const at = next.at ? { output: next.at.output, workspace: next.at.workspace,
                column: next.at.column, tile: next.at.tile ?? 1 } : null;
            reopening.waiting = { known: Object.keys(Niri.windows), at, opened: null,
                away: next.at?.away ?? null, kept: next.kept ?? null, quiet,
                quest: next.one.quest ?? "", from: next.one.from ?? "" };
            reopenStartedAt = Date.now();
            reopened(next.one, next.one.cwd, next.told);
            reopenPatience.restart();
        });
    }
    // Whether Reopen Them All is about to open, opening or placing one, or
    // looking at the screens first, which any other move would misplace.
    // Not `placing`, which OpenBelow.qml's property of that name would
    // answer to where it's called unqualified.
    function placingNow() {
        return screensWaiting.length > 0 || reopening !== null
            && (reopening.queue.length > 0 || reopening.waiting !== null || reopening.looking
                || reopening.settling);
    }

    // Workspaces given the screen in use in place of a kept screen that was
    // gone, by that screen, each with the number it was kept at there (the
    // `away` Places.js gives a place): taken to the screen when it's
    // connected, once nothing's being placed. A screen can come well after
    // niri starts, as the monitor came 52 s after it on 2026-11-02, and niri
    // keeps a workspace that took a window while its screen was away on the
    // screen it took it on, so without this they'd stay there.
    property var awayFrom: ({})
    function noteAway(away, id) {
        if (!away || id === null || id === undefined)
            return;
        const kept = awayFrom[away.screen] ?? [];
        if (kept.some(one => one.id === id))
            return;
        const next = Object.assign({}, awayFrom);
        next[away.screen] = kept.concat([{ id, workspace: away.workspace }]);
        awayFrom = next;
    }
    function comeHome() {
        if (root.placingNow() || Object.keys(awayFrom).length === 0)
            return;
        const going = Places.homeward(awayFrom, screens, Object.values(Niri.workspaces));
        if (going.home.length === 0)
            return;
        const next = {};
        for (const screen in awayFrom) {
            if (!going.home.includes(screen))
                next[screen] = awayFrom[screen];
        }
        awayFrom = next;
        const back = Niri.current;
        going.moves.forEach(move => Niri.act({ MoveWorkspaceToMonitor: { output: move.output,
            reference: { Id: move.id } } }));
        // Then each to its index, lowest first, so each set stays put.
        going.moves.forEach(move => Niri.act({ MoveWorkspaceToIndex: { index: move.index,
            reference: { Id: move.id } } }));
        // A screen just connected keeps its empty workspace while it's the
        // active one there, so the first moved is focused, which lets niri
        // drop the empty one, and then Joel's window again.
        const first = going.moves[0];
        if (first !== undefined) {
            Niri.act({ FocusWorkspace: { reference: { Id: first.id } } });
            if (back !== null)
                Niri.act({ FocusWindow: { id: back } });
        }
    }

    // Reopen Them All, as Agents' row runs it: every session the last boot
    // took away, each told to carry on, and then the windows that reopen
    // themselves put back where they sat (P9R1KI, 2026-11-02 [926e389d],
    // after a session that shared Chrome's workspace came back alone, and
    // Joel moved it by hand). Only Reopen Them All puts them back, since a
    // kept place means something only once the sessions are back.
    function reopenThemAll(lost, gone) {
        const boot = root.arrangedBefore();
        const kept = Array.isArray(arrangedOthers[boot]) ? arrangedOthers[boot] : [];
        puttingBack = kept.length > 0 ? { boot, kept, used: [], back: [], seen: [] } : null;
        root.reopenAll(lost, gone, null, carryOn);
    }
    // Each window but kitty's, as it first shows tiled this boot, matched to
    // one kept by its program and title (Places.js's matchOther) and moved
    // once, beside the windows kept with it (whereBack), with the keys going
    // with it, as Joel just opened it. Those already open are put back once
    // the sessions are, and Joel's window focused again after.
    property var puttingBack: null
    function putBackOpen() {
        if (puttingBack === null || root.placingNow())
            return;
        const back = Niri.current;
        const moved = Object.values(Niri.windows).sort((a, b) => a.id - b.id)
            .filter(window => root.putBack(window));
        if (moved.length > 0 && back !== null)
            Niri.act({ FocusWindow: { id: back } });
    }
    function putBack(window) {
        const going = puttingBack;
        const key = String(window.id);
        if (going === null || window.app_id === "kitty" || going.seen.includes(key)
            || !window.layout?.pos_in_scrolling_layout)
            return false;
        going.seen.push(key);
        const index = Places.matchOther(going.kept, going.used, window.app_id ?? "",
            window.title ?? "");
        if (index < 0)
            return false;
        going.used.push(index);
        const at = going.kept[index];
        const beside = root.besideOf(going.back);
        const found = Places.whereBack(at, beside);
        const output = found.output ?? Places.connectorOf(at, screens)
            ?? Niri.workspace()?.output ?? null;
        const where = found.make ? Object.assign({}, found, { output }) : found;
        const workspace = Niri.workspace(window.workspace_id);
        const column = window.layout.pos_in_scrolling_layout[0];
        let into = workspace?.id ?? null;
        if (workspace && !Places.holds(where, { workspace, column }, beside))
            into = root.moveTo(window, where, workspace, column, beside);
        if (into !== null && Places.connectorOf(at, screens) === undefined)
            root.noteAway({ screen: at.screen || at.output, workspace: at.workspace }, into);
        going.back.push({ id: window.id, at });
        if (going.used.length === going.kept.length)
            puttingBack = null;
        return true;
    }
    // A session's window Reopen Them All placed, with the place it was kept
    // at, for the windows put back beside it. Known by its window, since
    // its pi's record can come a minute and more after the window does.
    function besideKept(id, kept) {
        if (puttingBack !== null && kept)
            puttingBack.back.push({ id, at: kept });
    }
    // Each window given, by its kept place, as niri has it now: those back
    // from the boot being put back, its sessions and the windows put back so
    // far, or the sessions Reopen Them All placed.
    function besideOf(windows) {
        const beside = [];
        for (const one of windows) {
            const window = Niri.windows[one.id];
            const workspace = window ? Niri.workspace(window.workspace_id) : null;
            if (workspace)
                beside.push({ id: one.id, at: one.at, workspace,
                    column: window.layout?.pos_in_scrolling_layout?.[0] ?? 0 });
        }
        return beside;
    }
    // A screen's last workspace, the empty one niri keeps after the rest.
    function lastOn(output) {
        const its = Object.values(Niri.workspaces).filter(one => one.output === output)
            .sort((a, b) => a.idx - b.idx);
        const last = its[its.length - 1];
        return last && last.active_window_id === null ? last.id : null;
    }

    // A session whose window never opens in half a minute is given up on,
    // and placed if it comes later (reopenedWindow). One whose window opened
    // but couldn't be placed yet, as while its screen is away, is left to
    // the check once the last is placed.
    Timer {
        id: reopenPatience
        interval: 30000
        onTriggered: {
            const waiting = root.reopening?.waiting;
            if (waiting && waiting.opened === null)
                root.reopening.late.push(waiting);
            else if (waiting && waiting.at !== null)
                root.reopening.placed.push({ id: waiting.opened, at: waiting.at,
                    away: waiting.away });
            if (root.reopening)
                root.reopening.waiting = null;
            root.reopenNext();
        }
    }

    // Go to a session: its window, or, when niri's isn't known, kitty's,
    // which kitty raises if niri lets it. A held session in no window is
    // attached in a new one, and only then: Go To never makes a second
    // window of a session (Joel: "seeing multiples of the same thing
    // gives me executive dysfunction").
    function goTo(session) {
        if (!session.here && !session.window && !session.kitty) {
            joinElsewhere(session);
            return;
        }
        if (session.window)
            Niri.act({ FocusWindow: { id: session.window.id } });
        else if (session.kitty)
            Niri.spawn(["kitten", "@", "--to", session.kitty.socket, "focus-window",
                "--match", `id:${session.kitty.window}`]);
        else if (session.held)
            attach(session.held);
    }

    // Joins a session held on another machine in a new window here, by
    // the words its node gives for it, run as given (Jitsusama/mycelium:
    // `ssh -t -- <machine> mycelium ... attach <id>`), and only once: the
    // node names the window it joins, which Go To then goes to. With no
    // words, as when it isn't held or its machine is out of reach, nothing
    // opens, and the caller is told why.
    signal unjoined(var session)
    function joinElsewhere(session) {
        const words = session.attach;
        const joining = `${session.machine}:${session.address}`;
        if (!words || (Array.isArray(words) && words.length === 0)) {
            unjoined(session);
            return;
        }
        if (Date.now() - (attaching[joining] ?? 0) < attachPatience)
            return;
        attaching = Object.assign({}, attaching, { [joining]: Date.now() });
        const command = Array.isArray(words) ? words.map(String) : ["zsh", "-lic", String(words)];
        Niri.spawn(["kitty", "-1", "--instance-group", "pi-sessions"].concat(command));
        Report.say(`joined session ${session.address} on ${session.machine}`);
    }

    // Attaches what is held as `id` in a new window of the sessions' kitty.
    // The node names the window within moments of its joining; until it
    // does, asking again opens nothing more.
    readonly property int attachPatience: 5000
    property var attaching: ({})
    function attach(id) {
        if (!holds || Date.now() - (attaching[id] ?? 0) < attachPatience)
            return;
        attaching = Object.assign({}, attaching, { [id]: Date.now() });
        Niri.spawn(["kitty", "-1", "--instance-group", "pi-sessions",
            "zsh", "-lic", 'exec mycelium --realm "$0" attach "$1"', realm, id]);
        Report.say(`attached session ${id}`);
    }

    // Where a session may start: each machine the realm's node places work
    // on, this one first and the rest by name, so none moves as reach comes
    // and goes, each saying why it can't take one now, or which condition
    // it would rather not start one under; and `fits`, the machine
    // Wherever It Fits starts it on. All of it from one placement asked of
    // the node with nothing needed, again each time the launcher opens,
    // since a machine's state changes as it goes (Jitsusama/mycelium's
    // `place`). On a machine in no realm it holds nothing, and on one whose
    // node links no other it holds this one alone, so Start a Session has
    // no step there.
    property var places: []
    property string fits: ""
    readonly property bool choosesMachine: places.length > 1
    function lookAround() {
        if (holds && !placing.running)
            placing.running = true;
    }
    Process {
        id: placing
        command: ["mycelium", "--realm", root.realm, "place"]
        stdout: StdioCollector {
            onStreamFinished: root.placed(text)
        }
    }
    function placed(text) {
        let placement;
        try {
            placement = JSON.parse(text)?.Ok?.Placement;
        } catch (error) {
            return;
        }
        if (!placement)
            return;
        const chosen = placement.placed?.machine ?? "";
        const misses = placement.placed?.passed ?? placement.pending?.misses
            ?? placement.refused?.misses ?? [];
        const all = (chosen !== "" ? [{ machine: chosen, because: [] }] : []).concat(misses);
        const next = all.map(one => ({
            name: String(one.machine),
            cannot: (one.because ?? []).map(cannot).filter(why => why !== "")[0] ?? "",
            rather: (one.because ?? []).map(because => because?.rather_not?.why ?? "")
                .filter(why => why !== "")[0] ?? ""
        })).sort((a, b) => Number(b.name === machine) - Number(a.name === machine)
            || a.name.localeCompare(b.name));
        const said = JSON.stringify([next, chosen]);
        if (said === placesSaid)
            return;
        placesSaid = said;
        places = next;
        fits = chosen;
    }
    property string placesSaid: ""

    // Why a machine can't take a session now, as a placement says it, or
    // nothing where it can: out of reach, or a taint of its own refusing.
    function cannot(because) {
        if (because === "unreachable")
            return "out of reach";
        if (because?.refuses)
            return because.refuses.why ?? because.refuses.key ?? "";
        if (because?.lacks)
            return "it isn't what the work needs";
        return "";
    }

    // Starts a session on a quest on `on`, a machine of the realm: here as
    // start() does, or through the node, which starts it held there and
    // answers the words that join it, run in a new window of the sessions'
    // kitty. With `on` empty the node places it, as Wherever It Fits does.
    // The folder is the quest's, found at the same place under the home of
    // the machine it starts on, or that machine's home.
    function startOn(on, quest, folder) {
        if (on === machine) {
            start(quest, folder);
            return;
        }
        if (!holds || starting.running)
            return;
        starting.asked = { on: on, quest: quest };
        starting.command = ["mycelium", "--realm", realm, "start"]
            .concat(on !== "" ? ["--need", `machine=${on}`] : [])
            .concat(["--folder", folder, "--", "env", `QUEST_WORKFLOW_AUTOLOAD_ID=${quest}`, "pi"]);
        starting.running = true;
    }
    Process {
        id: starting
        property var asked: null
        stdout: StdioCollector {
            onStreamFinished: root.started(starting.asked, text)
        }
    }
    function started(asked, text) {
        let answer;
        try {
            answer = JSON.parse(text);
        } catch (error) {
            answer = null;
        }
        const begun = answer?.Ok?.Started;
        if (!begun || !Array.isArray(begun.attach)) {
            const why = typeof answer?.Err === "string" ? answer.Err : "its node didn't answer";
            Report.say(`couldn't start a session on ${asked.quest}: ${why}`);
            return;
        }
        Niri.spawn(["kitty", "-1", "--instance-group", "pi-sessions"].concat(begun.attach));
        Report.say(`started a session on ${asked.quest} on ${begun.machine}`);
    }

    // Where each session on the mesh takes words and acts from, by its
    // address: the harness's steer inbox, which every session reads, an
    // orchestrator or not (mesh-workflow's steer()).
    readonly property string inboxes: `${state}/steer`

    // Leaves a steer in a session's inbox, under a name it skips until it's
    // whole: `{ text, id }` talks to it, `{ act }` pauses, resumes or stops
    // it.
    // A session on another machine is told through the realm's node,
    // which carries it over the link to that machine's inbox; one here is
    // told by its file alone, so it never waits on the node.
    function steer(address, fields) {
        const name = `${Date.now()}-${Math.random().toString(36).slice(2, 10)}.json`;
        const body = JSON.stringify(Object.assign({ at: new Date().toISOString(), from: "joel" },
            fields));
        const upper = String(address).toUpperCase();
        const session = running.find(one => String(one.address).toUpperCase() === upper);
        if (session && !session.here) {
            steering = steering.concat([{ session: session, body: body }]);
            steerNext();
            return;
        }
        Niri.spawn(["sh", "-c",
            'mkdir -p "$1" && printf %s "$3" > "$1/$2.tmp" && mv "$1/$2.tmp" "$1/$2"',
            "steer", `${inboxes}/${String(address).toUpperCase()}`, name, body]);
    }

    // The steers waiting for the node, in turn, since one process answers
    // one at a time; one it refuses, or that never reaches the node, is
    // said by `unsteered` with the node's reason, or none.
    property var steering: []
    signal unsteered(var session, string why)
    function steerNext() {
        if (steerer.running || steerer.asked !== null || steering.length === 0)
            return;
        const next = steering[0];
        steering = steering.slice(1);
        steerer.asked = next;
        steerer.command = ["sh", "-c", 'printf %s "$3" | mycelium --realm "$1" steer "$2"',
            "steer", realm, String(next.session.address).toUpperCase(), next.body];
        steerer.running = true;
    }
    function steered(asked, text) {
        let answer = null;
        try {
            answer = JSON.parse(text.trim().split("\n").pop() ?? "");
        } catch (error) {
            answer = null;
        }
        if (answer?.Ok?.Steered) {
            Report.say(`steered ${asked.session.address} on ${answer.Ok.Steered.machine}`);
            return;
        }
        const why = typeof answer?.Err === "string" ? answer.Err : "";
        Report.say(`unsteered ${asked.session.address}: ${why}`);
        unsteered(asked.session, why);
    }
    Process {
        id: steerer
        property var asked: null
        onRunningChanged: Qt.callLater(root.steerNext)
        stdout: StdioCollector {
            onStreamFinished: {
                const one = steerer.asked;
                steerer.asked = null;
                if (one)
                    root.steered(one, text);
                Qt.callLater(root.steerNext);
            }
        }
    }

    // Talks to a session: the words go with an id, so the session writes
    // what became of them where the launcher reads its talks.
    function talk(address, words) {
        const text = words.trim();
        if (text === "")
            return false;
        steer(address, { text: text, id: `talk-${Date.now()}` });
        return true;
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
    // Return goes to the session as Go To does, back through `sessions`
    // below, so one on another machine is joined from here (pi-wants-you,
    // home.nix), unless
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
            Niri.spawn(["pi-wants-you", session.address, known,
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

    // What's outside the shell asks of sessions: Go To, as a want's notice
    // does, the session by an address it answers to, if it still runs; and
    // reopen, as the harness hands over the sessions it restores
    // (handedOver).
    IpcHandler {
        target: "sessions"

        function reopen(sessions: string): string {
            return root.handedOver(sessions);
        }

        function goTo(address: string): void {
            const upper = address.toUpperCase();
            const session = root.running.find(one => (one.presence?.addresses ?? [one.address])
                .some(each => String(each).toUpperCase() === upper));
            if (session)
                root.goTo(session);
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
    // When this boot began, in milliseconds since the epoch, read once.
    FileView {
        id: uptime
        path: "/proc/uptime"
        blockLoading: true
        printErrors: false
    }
    readonly property real bootedAt: Date.now()
        - Number(uptime.text().split(" ")[0] || 0) * 1000

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
    // The kitty window the node says a session is in, from its `window`.
    function kittyOfWindow(window) {
        return window && window.terminal === "kitty" && window.within && Number(window.id) > 0
            ? { socket: window.within, window: Number(window.id) } : null;
    }
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

    // Each running session's record, { file, presence, kitty, held }: as
    // the node has it while it answers, else each from the folder whose
    // process runs. A held session is in the window the node says joined
    // its holder last, or none, since its process has lost its own. Made
    // again when a record changes, the node tells of a change or a window
    // comes or goes, and kept as it was when nothing in it did, so what
    // reads it isn't told for nothing.
    property var live: []
    property string liveSaid: ""
    function relive() {
        const next = [];
        // A node from before it passed records on says too little to show.
        if (!fromFolder) {
            for (const id of Object.keys(node).sort()) {
                const session = node[id];
                const presence = session.presence;
                const state = String(session.state ?? "").toLowerCase();
                const out = state === "unreachable";
                const running = state === "busy" || state === "idle" || out;
                const kitty = kittyOfWindow(session.window);
                const here = (session.machine ?? machine) === machine;
                const changed = Date.parse(session.at ?? "") || 0;
                if (!running || !presence || (presence.addresses ?? []).length === 0)
                    continue;
                // One elsewhere the node last heard of before this boot is
                // long gone, so a machine away for days leaves nothing.
                if (here)
                    next.push({
                        file: id,
                        presence: presence,
                        kitty: session.held ? kitty : kitty ?? kittyOf(presence),
                        held: session.held ?? null
                    });
                else if (changed >= bootedAt)
                    next.push({
                        file: id,
                        presence: presence,
                        kitty: kitty,
                        held: null,
                        machine: String(session.machine),
                        here: false,
                        attach: out ? null : session.attach ?? null,
                        reachable: !out,
                        changed: changed
                    });
            }
        } else {
            if (!folderRead())
                return;
            for (const file of Object.keys(records).sort()) {
                const presence = records[file];
                if ((presence.addresses ?? []).length > 0 && alive(presence.owner))
                    next.push({
                        file: file,
                        presence: presence,
                        kitty: kittyOf(presence),
                        held: null
                    });
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
        // A node's first run writes it after the shell has started.
        watchChanges: true
        onFileChanged: reload()
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
    readonly property string nodePath: realm !== "" && Quickshell.env("XDG_RUNTIME_DIR")
        ? `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/${realm}.sock` : ""
    // A fresh connection each time, since a Socket refused once never
    // connects again, however it's asked (Quickshell's socket.cpp keeps the
    // refused one), as when a switch restarts the node and the shell asks
    // before it listens; then Now read the folder until the shell restarted.
    property bool linking: true
    // A watch always carries the sessions; the groups it names are those
    // whose messages it carries too, and the shell wants none of them, so
    // it names none: a watch with no groups hears the sessions alone, and
    // with no `recent`, nothing from before it (mycelium#47).
    LazyLoader {
        active: root.linking && root.nodePath !== ""

        Socket {
            id: mycelium
            path: root.nodePath
            connected: true
            onConnectionStateChanged: {
                if (mycelium.connected) {
                    mycelium.write(`${JSON.stringify({ Watch: {} })}\n`);
                    mycelium.flush();
                    return;
                }
                if (root.node !== null) {
                    root.node = null;
                    root.relive();
                }
                relink.restart();
            }
            onError: relink.restart()
            parser: SplitParser {
                onRead: line => root.nodeSaid(line)
            }
        }
    }
    // A node not yet up, or restarting, is asked again a few seconds on.
    Timer {
        id: relink
        interval: 5000
        onTriggered: {
            root.linking = false;
            root.linking = true;
        }
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
        } else if (typeof event.Err === "string") {
            // A watch refused leaves the folder read, which works but
            // costs a watcher a session, so it's said where it's seen.
            Report.say(`the node refused the watch: ${event.Err}`);
            return;
        } else if (event.SessionForgotten) {
            next = Object.assign({}, node ?? {});
            delete next[event.SessionForgotten.id];
        } else {
            return;
        }
        node = next;
        relive();
    }

    // Each record is read from its file, watched, only while the folder is
    // where sessions are known from: while the node answers, it carries
    // every record itself, so a watcher per session would only cost the
    // shell a file each (the 2026-10-29 crash came of readers per
    // session). A node lost makes them once, as many as there are records.
    // The folder isn't watched either then, since every session's record
    // is written there as often as it changes: fifteen times a minute woke
    // the shell for a folder nothing read (2026-10-31).
    readonly property bool fromFolder: node === null
        || !Object.values(node).every(session => session.presence)
    LazyLoader {
        id: presenceFolder
        active: root.fromFolder

        FolderListModel {
            folder: `file://${root.state}/mesh-workflow/presence`
            showDirs: false
            nameFilters: ["*.json"]
            onStatusChanged: root.relive()
        }
    }
    // Whether every record in the folder is read, so a node lost leaves
    // what it said shown until the folder says it all, never a moment with
    // no sessions while the folder is listed.
    function folderRead() {
        const listing = presenceFolder.item;
        return listing !== null && listing.status === FolderListModel.Ready
            && presenceViews.count === listing.count;
    }
    Instantiator {
        id: presenceViews
        model: root.fromFolder ? presenceFolder.item : null
        // A session that ends takes its file with it, and its state goes too.
        onObjectRemoved: (index, object) => root.forget(object.path)
        // A record may be read before it's counted, so the last is shown
        // once it is.
        onCountChanged: root.relive()

        FileView {
            required property string filePath
            path: filePath
            // Read as it's made, so a node lost leaves no moment with no
            // sessions.
            blockLoading: true
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

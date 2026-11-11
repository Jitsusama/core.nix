pragma Singleton
// pi's sessions, for the launcher's Agents and the pi piece: every pi
// running, the one whose window was used last first, with its quest's whole
// title, and whether it's working or wants Joel, as each says; and a
// notice once for each want, withdrawn when the want ends.
//
// A session is what its own record says: the file each keeps in the mesh's
// presence folder while it runs. The machine's mycelium node passes each
// record on, and says the moment a session starts, changes or ends, from
// its process rather than a clock (Jitsusama/mycelium), so the sessions
// are the node's, and nothing else: the shell reads no presence file and
// looks at no process. While the node doesn't answer, what it last said
// stays. Nothing is read from a window's title, which a
// program can set, a terminal can cut short and two sessions can share
// (Joel, 2026-10-27: "we should have a better system for agent session
// awareness that isn't so fragile"). Each reader here goes through
// `running` and the functions beside it, so neither way shows past them.
//
// A session's window is the one niri lists by the app id it was opened
// with, which the node's record says with the window (`window.app_id`,
// told it by whoever opened the window), or for a window opened without
// one, through the kitty its record names (`terminal`, or the kitty its
// process was started in), which says which of its windows the session is
// in, while niri lists kitty's windows by its process (windowOf). niri's
// windows come ready from the windows daemon (modules/windows), so a
// retitle, or the keys moving, makes no list here again. Its
// quest's whole title and its folder are as its
// record says them (`questInfo` and `cwd`, which the harness publishes),
// or for a record from before it did, as its window's label cuts them;
// no README is read for them. Nothing is asked of pi;
// a want sounds, and For You and the margin say it (onWantingChanged).
import Quickshell
import Quickshell.Io
import QtQuick
import "PiSource.js" as Pi
import "Places.js" as Places
import "SessionWindows.js" as SessionWindows
import "Waiting.js" as Waiting

Singleton {
    id: root

    // Where the harness keeps its state, by the XDG base directories, as it
    // finds them.
    readonly property string home: Quickshell.env("HOME")
    readonly property string state: `${Quickshell.env("XDG_STATE_HOME") || `${home}/.local/state`}/pi/agentic-harness.pi`

    // Each session running: its window or null, its address, its quest's
    // title and its folder as its record says them (named()), its record,
    // its record, the kitty window it runs in or null, the id its holder
    // keeps it by or null, the machine it runs on and whether that's this
    // one, the words that attach it from here or null, whether its
    // machine can be reached, and when it last changed. In inUse's order
    // as it stood when they were last made, which the focus moving doesn't
    // make again (SessionWindows.js), so what shows them in that order, or
    // picks one by it, asks inUse itself.
    readonly property var running: {
        const windows = niriWindows;
        return inUse(live.map(record => shown(record, windows.pids, windows.opened)));
    }
    // Sessions in the order they're shown and picked in: this machine's
    // first, the one whose window was used last first, then those with
    // none; then those elsewhere, the one that changed last first. When a
    // window was used is read from the daemon's record as it is now, which
    // the daemon changes in place, so nothing reading this is made again
    // as the focus moves (Y2YBTS, 2026-11-10 [dda1b827]).
    function inUse(sessions) {
        const when = window => window ? windowsDaemon.held[window.key]?.order ?? window.order ?? 0
            : 0;
        return sessions.slice().sort((a, b) => Number(b.here) - Number(a.here)
            || (a.here ? when(b.window) - when(a.window) : b.changed - a.changed));
    }
    // Each session parked here (the harness's park-workflow): ended while
    // idle to give back its memory, until its holder starts it again in
    // its window, for a message that gives it a turn or a key typed there.
    // Shown as each running one is, but with no window known, since its
    // process, and the window its node knew it by, have gone; in the order
    // the records are. Never among `running`, so nothing counts it as
    // running or starts another session in its place.
    readonly property var parked: resting.map(record => shown(record, {}))
    // Each session here that ended this boot and isn't parked, closed or
    // gone with its process, the one that ended last first, by id when
    // two ended at once: shown as each parked one is, with `id`, its
    // `quest` where it had one, and `ended`, when the node last heard of
    // it, in milliseconds, kept a number since the node gives one and each
    // opening reads it: writing it out as a time only for Agents' Recent
    // to read it back at every change cost a date's sums twice a session
    // (2026-11-10). What a closed session's record can't tell, as
    // when it was lost or whether it was carried, isn't said. Agents'
    // Recent reads it beside toReopen, the last boot's.
    readonly property var closed: ending.map(record => Object.assign(shown(record, {}), {
        id: String(record.presence.session ?? record.file),
        quest: record.presence.questInfo?.id ?? record.presence.quest ?? null,
        ended: record.changed
    })).sort((one, other) => other.ended - one.ended || one.id.localeCompare(other.id))

    // A session as `running` lists it, from its record, with niri's windows
    // by app id when they're at hand (windowOf).
    function shown(record, kitties, opened) {
        const presence = record.presence;
        const address = (presence.addresses ?? [])[0];
        const name = named(presence);
        return {
            window: windowOf(record, kitties, opened),
            address: address,
            title: name.title || address,
            folder: name.folder,
            presence: presence,
            kitty: record.kitty,
            held: record.held,
            machine: record.machine ?? machine,
            here: record.here ?? true,
            attach: record.attach ?? null,
            reachable: record.reachable ?? true,
            changed: record.changed ?? 0
        };
    }

    // A session parked, by any address it goes by, or null.
    function parkedAs(address) {
        const upper = String(address).toUpperCase();
        return parked.find(one => (one.presence?.addresses ?? [one.address])
            .some(each => String(each).toUpperCase() === upper)) ?? null;
    }
    readonly property int working: running.filter(session => session.presence.busy).length
    // Those waiting on Joel, as each says while it does: { kind, since,
    // said }, kind a question, an approval, blocked or an action, and only
    // while it's truly his (Waiting.js): an action while its quest has
    // something he can do now, and a question only from a session no
    // other orchestrates. The oldest first. One parked still wants him:
    // its presence keeps its want while its process gives back its memory
    // (the harness's peer.ts), so it waits in For You until he answers.
    readonly property var wanting: running.concat(parked)
        .filter(session => asks(session))
        .sort((a, b) => String(a.presence.wants.since ?? "").localeCompare(String(b.presence.wants.since ?? "")))
    // The windows of those waiting on Joel, as one string, so a session's
    // other changes (working or not, its step) leave it the same and don't
    // remake the windows that read it.
    readonly property string waitingWindows: wanting
        .filter(session => session.window)
        .map(session => session.window.id).join(" ")

    // How long a pi may take to start, which the launcher waits for on a
    // row it shows as a wake or a start will leave it, before putting it
    // back: pi loads its extensions and its whole log before it says it's
    // up, far slower than a daemon's answer.
    readonly property int startsWithin: 30000

    // Starts a session on a quest, in `folder`, as the harness's own spawn
    // does: in the kitty that holds the sessions, through a login shell,
    // so the session has the account's whole environment, with the quest
    // loaded as it starts. Quests, a closed orchestrator and Agents' Start
    // a Session all start one this way, so a session started anywhere is the
    // same. One that `orchestrates` marks its quest as orchestrating once
    // the quest loads, as the quest tool's orchestrate does, which the
    // harness does for it, since the shell never writes a quest.
    // pi run without a login shell can't reach its model, and exits at
    // once, so a window started that way blinked and went (Joel,
    // 2026-10-28).
    function start(quest, folder, orchestrates) {
        const told = orchestrates ? ["QUEST_WORKFLOW_ORCHESTRATE=1"] : [];
        root.inSessions("pi-session", ["--directory", folder,
            "env", `QUEST_WORKFLOW_AUTOLOAD_ID=${quest}`].concat(told,
            ["zsh", "-lic", held("pi")]));
        Report.say(`started a session ${orchestrates ? "orchestrating" : "on"} ${quest}`);
    }

    // Starts a session told `words` as it opens, as Google's sign-in is
    // "/google-auth" and Ask's carried question its own, in `folder`,
    // through a login shell for the same reason. The words go as the
    // shell's $0, so nothing in them is read as the shell's own.
    function tell(words, folder, program) {
        root.inSessions("pi-session", ["--directory", folder,
            "zsh", "-lic", held(`${program ?? "pi"} -- "$0"`), words]);
    }

    // Opens a window of the sessions' own kitty, running `rest`, and
    // answers the app id it opens with: `kind` and a number of its own,
    // pi-spawned for one another session started, which niri opens
    // without the focus (modules/niri/config.kdl), and pi-session for the
    // rest (Niri.qml's heard). Its process is told the same app id, in
    // MYCELIUM_APP_ID, so a session held in it says it with its window,
    // and its record names that window as niri lists it (windowOf):
    // one kitty draws every window opened here, so its own ids mean
    // nothing to niri, and ranking its windows against niri's by order
    // went wrong as windows came and went. One opened `into` a workspace
    // and column ({ workspace, column }) has niri told to open it there.
    // Every window of that kitty opens here, so how it's started is said
    // once. It starts in agents.slice (modules/slices), weighed beside
    // Joel's programs rather than among them; kitty puts each window's
    // scope in the slice it runs in, so every session goes with it. One
    // opened aside, or where it goes, goes without the activation token
    // niri hands what it spawns, which kitty would spend on taking the
    // focus once its window shows.
    // They're numbered on from the moment this shell started, in
    // milliseconds, not from nought: a shell started again, as every switch
    // does, would otherwise name its first window as one still open from
    // before, and windowOf would take either for the other. No shell opens
    // a window a millisecond, so a later start counts past every number an
    // earlier one gave.
    readonly property real startedAt: Date.now()
    property real openings: startedAt
    function inSessions(kind, rest, into, named) {
        openings++;
        const classed = named ?? `${kind}.${openings}`;
        const placed = into !== null && into !== undefined;
        if (placed) {
            const asked = { app_id: classed, reference: { Id: into.workspace } };
            if (into.column !== null)
                asked.column = into.column;
            Niri.act({ OpenNextWindowOn: asked });
        }
        const untold = kind === "pi-spawned" || placed
            ? ["env", "-u", "XDG_ACTIVATION_TOKEN", "-u", "DESKTOP_STARTUP_ID"] : [];
        Niri.spawn(untold.concat(["systemd-run", "--user", "--scope", "--collect", "--quiet",
            "--slice=agents.slice", "--", "env", `MYCELIUM_APP_ID=${classed}`, "kitty", "-1",
            "--instance-group", "pi-sessions", "--class", classed], rest));
        return classed;
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
    // One run unheld isn't told its window's app id, which every program
    // it starts would inherit, as a session it spawns would, and say as
    // its own window's; the holder keeps it from its program likewise.
    function held(command) {
        return holds ? `exec mycelium --realm ${realm} hold -- ${command}`
            : `exec env -u MYCELIUM_APP_ID ${command}`;
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

    // Whether each folder a closed session would reopen in is there, as it
    // was last looked for: true or false, and absent until it has been. The
    // place listing them looks as it's shown (lookForFolders), never trusting
    // what the shell found at its start, so a row says its folder is gone
    // before Return rather than a notice saying so after (VKCF3Z,
    // 2026-11-07 [4a00987e]).
    property var folders: ({})
    function lookForFolders(cwds) {
        const asked = cwds.filter((cwd, at) => cwd !== "" && cwds.indexOf(cwd) === at);
        if (asked.length === 0)
            return;
        lookFor(asked, there => heardFolders(asked, there));
    }
    function heardFolders(asked, there) {
        if (asked.every((cwd, at) => folders[cwd] === there[at]))
            return;
        const next = Object.assign({}, folders);
        asked.forEach((cwd, at) => {
            next[cwd] = there[at];
        });
        folders = next;
    }
    function folderGone(cwd) {
        return folders[cwd] === false;
    }

    // Opens a closed session again where it left off, in a window of its
    // own in `cwd`, held as every start is. One pi never saved, as it
    // saves none before its first answer, is `fresh` in restore.json
    // (85PKB2, 2026-11-01 [e51c350d]): `pi --session` finds no record of
    // it and exits at once, so it starts under its own id, with its quest
    // loaded as start() loads one, since only a record brings one back.
    // `told`, when given, is pi's first prompt, sent as it opens, as the
    // shell's $0 so nothing in it is read as the shell's own.
    function reopened(one, cwd, told, into) {
        const fresh = one.fresh === true;
        const quest = fresh && typeof one.quest === "string" && one.quest !== ""
            ? ["env", `QUEST_WORKFLOW_AUTOLOAD_ID=${one.quest}`] : [];
        const model = one.model ? ` --model ${one.model}` : "";
        const pi = (fresh ? `pi --session-id ${one.id}` : `pi --session ${one.id}`) + model;
        const telling = typeof told === "string" && told !== "";
        // Classed as a session's either way, so a window Joel opens
        // meanwhile isn't taken for it; one another session started opens
        // aside.
        const kind = one.quiet === true ? "pi-spawned" : "pi-session";
        const classed = root.inSessions(kind, ["--directory", cwd].concat(quest,
            ["zsh", "-lic", held(telling ? `${pi} -- "$0"` : pi)], telling ? [told] : []), into);
        Report.say(`${fresh ? "started unsaved" : "resumed"} session ${one.id}`);
        return classed;
    }

    // The sessions the last boot's end took away, to reopen after a cold
    // boot (Joel, 2026-10-26: one act brings back every session live at
    // shutdown, each in its own window and folder, never one from before
    // that boot). The harness keeps them in restore.json (its #866),
    // rewritten whole as each pi starts and ends, with the boot they ran
    // in, so a reopened one leaves the list within seconds. Only another
    // boot's are offered, and only until Joel puts that boot off, but for
    // those offered apart (`apart`). This boot is
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
    // safe to hand pi's command line and a folder to open in. One that ran
    // hosted (`hosted`) is left out: its host's own unit brings it back, so
    // reopening it would start it twice (N4S6OF, 2026-11-09).
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
        // One another machine carried here, or pi's host gave back, says
        // where from and when; one saying it badly is no last boot's
        // either, so it's left out (Places.saysApart).
        return {
            boot: typeof read.boot === "string" ? read.boot : "",
            sessions: read.sessions.filter(one => one !== null && typeof one === "object"
                && /^[A-Za-z0-9._-]+$/.test(String(one.id ?? "")) && Places.saysApart(one)
                && one.hosted !== true
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

    // The last boot's sessions not back here, and which boot that was, as
    // restore.json says (Places.js). After a boot they wait for Joel:
    // Reopen Them All, or :reopen, brings them back in one act, each where
    // the shell kept it, and nothing reopens them before he asks. The shell
    // once reopened them by itself as it first saw niri, which he never
    // wanted (Joel, 2026-11-07 20:2x: "no, i never wanted them automatic,
    // you decided that by yourself"); on 11-01 he had chosen to reopen
    // them by hand.
    readonly property var lastBootOffered: Places.lastBoot(restoring, {
        boot: bootId,
        putOff: putOff,
        back: running.concat(parked).filter(session => session.here)
            .map(session => session.presence.session)
    })
    readonly property string lastBootIs: lastBootOffered.boot
    readonly property var lastBoot: lastBootOffered.sessions

    // The sessions offered apart from the last boot's, whatever the boot,
    // until each runs here, when the harness drops it, and never put off
    // with the last boot's (VKCF3Z, [17db838c]): those other machines
    // carried here as they went away, and those pi's host gave back, so
    // they run in plain pi again (2QZBX9, [44e65ce0]). The newest first,
    // by id when two came at once.
    readonly property var apart: Places.apart(restoring.sessions)

    // Every session there is to reopen: the last boot's, then those apart,
    // but those Joel forgot.
    readonly property var toReopen: lastBoot.concat(apart)
        .filter(one => !forgotten.includes(one.id))

    // The sessions to reopen Joel forgot, by id, kept by the shell as the
    // boot he put off is, since restore.json is the harness's to write. It
    // keeps only those restore.json still names, so it never grows past it.
    property var forgotten: {
        try {
            const ids = JSON.parse(forgottenFile.text()).ids;
            return Array.isArray(ids) ? ids.map(String) : [];
        } catch (error) {
            return [];
        }
    }
    FileView {
        id: forgottenFile
        path: Quickshell.statePath("reopen-forgotten.json")
        blockLoading: true
        printErrors: false
    }
    function forgetReopen(lost) {
        const named = restoring.sessions.map(one => one.id);
        forgotten = forgotten.filter(id => named.includes(id)).concat([lost.id]);
        forgottenFile.setText(JSON.stringify({ ids: forgotten }));
    }

    // The landings Joel has read, by key (Landed.js's landingKey), and when
    // he first read each, in milliseconds, which For You's History says,
    // "read 14:02"; one read before the shell kept when has none. Kept by
    // the shell, since a session's presence is the harness's to write.
    // Whoever marks one read says which are still held (`held`), so what's
    // kept never grows past them.
    property var landingsRead: landingsKept().keys
    property var landingsReadAt: landingsKept().at
    function landingsKept() {
        try {
            const kept = JSON.parse(landingsReadFile.text());
            const at = kept.at !== null && typeof kept.at === "object" ? kept.at : {};
            return { keys: Array.isArray(kept.keys) ? kept.keys.map(String) : [], at: at };
        } catch (_error) {
            return { keys: [], at: {} };
        }
    }
    FileView {
        id: landingsReadFile
        path: Quickshell.statePath("landings-read.json")
        blockLoading: true
        printErrors: false
    }
    function readLandings(keys, held) {
        const now = Date.now();
        const at = {};
        landingsRead = held.filter(key => !keys.includes(key)).concat(keys);
        for (const key of landingsRead) {
            const was = landingsReadAt[key] ?? (keys.includes(key) ? now : null);
            if (typeof was === "number")
                at[key] = was;
        }
        keepLandingsRead(at);
    }
    function keepLandingsRead(at) {
        landingsReadAt = at;
        landingsReadFile.setText(JSON.stringify({ keys: landingsRead, at: at }));
    }

    // One of them, in its own folder only: when that's gone, nothing is
    // opened, and `gone(cwd)` is told so the caller says so, since there's
    // no quest here to start afresh. Its row said so already unless the
    // folder went since it was listed, so it says so from now on too.
    function reopen(lost, gone) {
        if (lost.machine !== undefined) {
            rejoin(lost);
            return;
        }
        lookFor([lost.cwd], there => {
            heardFolders([lost.cwd], there);
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
            root.inSessions("pi-session",
                (there ? ["--directory", lost.cwd] : []).concat(lost.attach));
            Report.say(`joined session ${lost.address} on ${lost.machine} again`);
        };
        if (lost.cwd === "")
            open(false);
        else
            lookFor([lost.cwd], there => open(there[0]));
    }

    function putOffLastBoot() {
        putOff = lastBootIs;
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
    // Each kept place says whether its session `leads`, orchestrating one
    // of the others as it was kept, so Reopen Them All begins it first
    // (Places.js's leadersFirst).
    function arrangedNow() {
        const now = {};
        const leaders = running.concat(parked)
            .map(session => String(session.presence?.orchestratedBy ?? "").toUpperCase())
            .filter(address => address !== "");
        running.forEach(session => {
            const id = session.presence.session;
            const at = typeof session.window?.column === "number"
                ? [session.window.column, session.window.down] : null;
            const workspace = session.window ? Niri.workspace(session.window.workspace) : null;
            const leads = (session.presence.addresses ?? [session.address])
                .some(address => leaders.includes(String(address).toUpperCase()));
            if (session.here && typeof id === "string" && at && workspace)
                now[id] = { screen: screens[workspace.output] ?? "",
                    output: workspace.output ?? "", workspace: workspace.idx,
                    column: at[0], tile: at[1], leads };
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
    // A session's window found, as when a kitty has said which of its
    // windows is whose, can be what a session handed over waits for to
    // go beside its kin (placeQuietly), one opened behind the lock among
    // them (placeBehind). One moved onto pi's host has its viewer opened
    // where it was (viewKept).
    onRunningChanged: {
        arranging.restart();
        root.placeOpened();
        root.viewKept();
        if (root.placeable() && (root.reopening?.behind.length ?? 0) > 0)
            root.placeBehind();
    }
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
            root.noteGone();
            if (Object.keys(root.awayFrom).length > 0)
                arranging.restart();
            root.placeOpened();
            root.settleReopenedSoon();
        }
        // niri gives the keys back to a window only once it has let the
        // lock go itself (lockGone).
        function onFocusedChanged() {
            if (!root.behindTheLock && !root.lockLetGo && Niri.focused !== null)
                root.lockGone();
        }
        // A workspace focused or one gone changes where a moved session's
        // viewer may open (viewKept).
        function onRevisionChanged() {
            root.viewKept();
        }
        // The window a session being reopened opens in, placed once niri
        // has laid it out, and the next begun; never a terminal of Joel's.
        function onWindowOpenedOrChanged(window) {
            if (root.reopening && window.session === true)
                root.reopenedWindow(window.id);
            root.placeOpened();
            root.settleReopenedSoon();
            if (root.puttingBack !== null && !root.placingNow())
                root.putBack(window);
        }
    }
    // Whose a new window is: the session's opened with its app id
    // (inSessions), the one being reopened now or one given up on before
    // its window came, which goes to its own workspace. Any other, a
    // start or a terminal of Joel's meanwhile, is none of theirs. A slow
    // first kitty after a cold boot once had its window taken for the next
    // one's, by the order they came in, and both misplaced.
    function reopenedWindow(id) {
        const going = reopening;
        const opened = Niri.windows[id]?.opened ?? "";
        const late = going.late.find(one => one.classed === opened);
        if (opened !== "" && late) {
            console.info(`sessions: window ${id} came late, ${late.id}'s by its app id`);
            going.late = going.late.filter(one => one !== late);
            root.besideKept(id, late.kept);
            if (!root.placeable()) {
                going.behind.push({ entry: { id, at: late.at, away: late.away,
                    quiet: late.quiet === true }, waiting: late });
                return;
            }
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
                entry.landed = root.placeSession(entry, root.besideOf(going.placed)) === "held";
            if (entry.at !== null)
                going.placed.push(entry);
            if (going.settling)
                root.settleReopenedSoon();
            else if (!running)
                root.settleReopened();
            return;
        }
        const waiting = going.waiting;
        if (waiting && waiting.opened === null && opened !== "" && waiting.classed === opened)
            waiting.opened = id;
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
        // Behind the lock it's placed once the lock is let go, and the next
        // is begun now, so every session handed over meanwhile runs.
        if (!root.placeable()) {
            reopening.behind.push({ entry, waiting });
            root.besideKept(entry.id, waiting.kept);
            reopening.waiting = null;
            reopenPatience.stop();
            reopenSoon();
            return;
        }
        if (!waiting.said) {
            waiting.said = true;
            console.info(`sessions: ${waiting.id} opened as window ${entry.id} at `
                + root.sitsSaid(entry.id));
        }
        if (entry.at !== null) {
            const placed = root.placeSession(entry, root.besideOf(reopening.placed));
            if (placed === "waiting")
                return;
            // One that opened where it goes has landed, and settling never
            // moves it again (settleNextReopened). The entry is the list's
            // own, never bound, so marking it is no binding's write.
            entry.landed = placed === "held";
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
    // A session's kin among `others`: the session that asked for it
    // (`from`), since an orchestrator starts its sessions where it is
    // (Joel, 2026-11-07), else one on its own quest.
    // Its orchestrator comes first, by the address the harness hands it
    // over with, else the one a session on its own quest says: one an
    // orchestrator starts for another's tree goes with that tree, never with
    // the starter (P9R1KI, 2026-11-08 [38061f98]).
    function kinOf(others, one) {
        const by = root.orchestratorOf(others, one);
        return others.find(session => by !== "" && session.presence.orchestrates === by)
            ?? others.find(session => one.from !== "" && session.presence.session === one.from)
            ?? others.find(session => one.quest !== "" && session.presence.quest === one.quest);
    }
    function orchestratorOf(others, one) {
        if (typeof one.orchestratedBy === "string" && one.orchestratedBy !== "")
            return one.orchestratedBy;
        const sibling = others.find(session => one.quest !== ""
            && session.presence.quest === one.quest
            && typeof session.presence.orchestratedBy === "string");
        return sibling?.presence.orchestratedBy ?? "";
    }
    // Where a session sits, as underOrchestrator reads it: its workspace's
    // id, its column and its quest's title, as its window names it.
    function standing(session) {
        return { workspace: session.window.workspace,
            column: session.window.column ?? 0,
            title: root.named(session.presence).title };
    }
    // Where a session reopened or handed over opens, { workspace, column }
    // by the workspace's id and the column from 1, or null to leave it to
    // niri and the moves after: where it was kept, as the column whereBack
    // says on a workspace there now (Places.opensAt), or, for one given a
    // workspace of its own, last on the one at its number on its screen,
    // else on that screen's last, as focusing them by number did; else its kin's
    // workspace, wherever niri puts it there. Each is opened once the one
    // before it is laid out, so the screen's workspaces are as it left them. niri opens a window on a
    // workspace asked for only by OpenNextWindowOn (core's
    // opens-next-window-on.patch), so it never shows where Joel is first
    // and moves, and nothing takes his focus to put it in place (Joel,
    // 2026-11-07: sessions started in the wrong workspaces). One kept on a
    // workspace not there now, or with a kin whose window isn't told apart
    // from its kitty's yet, gives none, and is moved after.
    function opensOn(one, at, beside) {
        if (at !== null) {
            const where = Places.whereBack(at, beside);
            if (!where.make)
                return { workspace: where.join, column: Places.opensAt(where, beside) };
            // The screen's empty workspace, niri's last, so a workspace kept
            // is never taken for one of Joel's by a number it had last boot.
            const its = Object.values(Niri.workspaces).filter(one => one.output === at.output)
                .sort((a, b) => a.idx - b.idx);
            const empty = root.lastOn(at.output);
            const there = (empty !== null ? Niri.workspace(empty) : null) ?? its[its.length - 1];
            if (!there)
                return null;
            const columns = Object.values(Niri.windows)
                .filter(one => one.workspace_id === there.id)
                .map(one => one.layout?.pos_in_scrolling_layout?.[0] ?? 0);
            return { workspace: there.id, column: Math.max(0, ...columns) + 1 };
        }
        const others = root.inUse(root.running).filter(session => session.here
            && (session.window !== null || session.kitty !== null)
            && session.presence.session !== one.id);
        const asked = { from: one.from ?? "", quest: one.quest ?? "",
            orchestratedBy: one.orchestratedBy ?? "" };
        const kin = root.kinOf(others, asked);
        if (!kin?.window || !Niri.workspace(kin.window.workspace))
            return null;
        // Under its orchestrator, at its place in title order there.
        const by = root.orchestratorOf(others, asked);
        if (by !== "" && kin.presence.orchestrates === by) {
            const under = others.filter(session => session.window
                && session.presence.orchestratedBy === by).map(root.standing);
            return Places.underOrchestrator(one.title ?? "", root.standing(kin), under);
        }
        return { workspace: kin.window.workspace, column: null };
    }
    function placeQuietly(id, waiting) {
        const window = Niri.windows[id];
        if (!window || !Niri.workspace(window.workspace_id))
            return "waiting";
        // Its kin among the sessions here with a window, or in a kitty whose
        // window isn't known yet: one whose node hasn't said its window,
        // or one with no app id in a kitty that says which window is whose
        // only once asked again, which this one's window opening in it has
        // just made stale. That kin's window is waited for (running, below),
        // rather than the session left where it opened, on whatever
        // workspace Joel is on (P9R1KI, 2026-11-03: nine of ten builder
        // sessions landed there).
        const others = root.inUse(root.running).filter(session => session.here
            && (session.window !== null || session.kitty !== null)
            && session.window?.id !== id && session.presence.session !== waiting.id);
        const kin = root.kinOf(others, waiting);
        if (kin && kin.window === null)
            return "waiting";
        const into = kin?.window.workspace ?? window.workspace_id;
        if (into === window.workspace_id)
            return "held";
        Niri.act({ MoveWindowToWorkspace: { window_id: id, reference: { Id: into },
            focus: false } });
        return "moved";
    }
    // Puts a session reopened where whereBack says, beside the sessions
    // back before it (Places.js), when it isn't there already: "held" when
    // it was, "standing" when it was but on a screen standing in for its
    // own, "moved", or "waiting" while niri hasn't laid it out or its
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
        // One that holds on a screen standing in for its own, gone as it
        // opened, only stands there: it hasn't landed, so settling places
        // it again once its screen is back. Taken as landed, two sessions
        // kept on the monitor stayed on the laptop's screen after every
        // screen dropped and came back (screens-keep-places, 2026-11-09).
        if (holds) {
            root.noteAway(entry.away, workspace.id);
            const own = entry.at?.output ?? "";
            return own === "" || workspace.output === own ? "held" : "standing";
        }
        const into = entry.quiet === true ? root.moveQuietly(window, where, workspace)
            : root.moveTo(window, where, workspace, column, beside);
        if (into === null)
            return "waiting";
        console.info(`sessions: window ${entry.id} moved from ${workspace.output} `
            + `${workspace.idx}/${column} toward ${JSON.stringify(where)}, kept at `
            + root.keptSaid(entry.at));
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
    // every window is placed (focusedToPlace).
    function moveTo(window, where, workspace, column, beside) {
        const id = window.id;
        if (where.make) {
            const into = root.madeFor(id, where, workspace);
            if (into === null)
                return null;
            if (into !== workspace.id) {
                root.focusedToPlace();
                Niri.act({ MoveWindowToWorkspace: { window_id: id, reference: { Id: into },
                    focus: true } });
            }
            Niri.act({ MoveWorkspaceToIndex: { index: where.index, reference: { Id: into } } });
            return into;
        }
        if (workspace.id === where.join) {
            // One opened in its column, as opensOn asks, is only taken into
            // the column left of it when it goes under, which needs no focus.
            const to = Places.columnTo(where, column, beside);
            if (to !== column) {
                root.focusedToPlace();
                Niri.act({ FocusWindow: { id } });
                Niri.act({ MoveColumnToIndex: { index: to } });
            }
        } else {
            root.focusedToPlace();
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
    // Where a session was kept, a workspace and a window sit, as the
    // placement log says them, so a reopen on Joel's machine says what it
    // asked and what niri did: on 2026-11-07 five of 57 came back misplaced
    // after the reboot, and nothing said why.
    function keptSaid(at) {
        return at ? `${at.output} ${at.workspace}/${at.column}` : "nowhere";
    }
    function spaceSaid(id) {
        const workspace = Niri.workspace(id);
        return workspace ? `${workspace.output} ${workspace.idx}` : `workspace ${id}`;
    }
    function sitsSaid(id) {
        const window = Niri.windows[id];
        const workspace = window ? Niri.workspace(window.workspace_id) : null;
        return workspace ? `${workspace.output} ${workspace.idx}/`
            + `${window.layout?.pos_in_scrolling_layout?.[0] ?? "?"}` : "nowhere yet";
    }

    // Says Reopen Them All moved the focus to put a window in place, so
    // Joel's window is focused again once every one is placed. A session
    // that opened where it goes (opensOn) never moves it.
    function focusedToPlace() {
        if (reopening !== null)
            reopening.focused = true;
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
        if (!reopening || !root.placeable())
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
        // Behind the lock every move would be refused; placeBehind settles
        // them once it's let go.
        if (!going?.settling || !root.placeable())
            return;
        const ordered = going.placed.filter(one => Niri.windows[one.id])
            .sort((a, b) => root.keptOrder(a.at, b.at));
        if (going.rounds < ordered.length + 3) {
            for (let at = 0; at < ordered.length; at += 1) {
                // A window that has landed stays wherever it is now, so a
                // move Joel or a session makes after it lands sticks (Joel,
                // 2026-11-09: one placer, "and his own moves stick"). It
                // still counts beside the ones after it, by its id.
                if (ordered[at].landed)
                    continue;
                const beside = root.besideOf(ordered.slice(0, at));
                const placed = root.placeSession(ordered[at], beside);
                ordered[at].landed = placed === "held";
                if (placed === "moved") {
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
    // up on may still come, or one opened behind the lock waits for its
    // kin's window, to place it.
    function reopenDone() {
        const going = reopening;
        console.info(`sessions: reopening settled after ${going.rounds} rounds of moves`);
        going.settling = false;
        reopenSettling.stop();
        // Sessions handed over meanwhile are reopened first, and this is
        // done once they're placed too: dropped here, they never opened
        // (2026-11-04, a second spawn 0.7 s after the first).
        if (going.waiting !== null || going.looking)
            return;
        if (going.queue.length > 0) {
            if (!reopenPacing.running)
                reopenSoon();
            return;
        }
        root.putBackOpen();
        arranging.restart();
        // Only when a session reopened moved it: one another session
        // started leaves Joel's focus where he has it.
        if (going.back !== null && going.focused)
            Niri.act({ FocusWindow: { id: going.back } });
        going.back = null;
        going.focused = false;
        if (going.late.length === 0 && going.behind.length === 0)
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
    // And none is begun while the machine queue says the machine has no
    // room (Places.mayBegin), looked at again this often, since eighty
    // sessions begun two seconds apart still swamped it after a boot.
    readonly property int roomPace: 5000
    function reopenSoon() {
        const wait = reopenStartedAt + reopenPace - Date.now();
        const more = !!reopening && reopening.queue.length > 0;
        if (wait > 0 && more) {
            reopenPacing.interval = wait;
            reopenPacing.restart();
        } else if (more && !hasRoom()) {
            if (!reopening.heldForRoom)
                Report.say(`reopening waits for room, ${reopening.queue.length} to go`);
            reopening.heldForRoom = true;
            reopenPacing.interval = roomPace;
            reopenPacing.restart();
        } else {
            if (more)
                reopening.heldForRoom = false;
            reopenNext();
        }
    }
    // The machine queue's verdict, read as it is now: it's written every few
    // seconds, and only a reopening asks.
    FileView {
        id: machineQueue
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/jobs/queue.json`
        blockLoading: true
        printErrors: false
    }
    function hasRoom() {
        machineQueue.reload();
        return Places.mayBegin(machineQueue.text(), Date.now() / 1000);
    }
    // It asks again rather than taking the next at once, since a timer
    // this long is a coarse one, which may fire up to a twentieth of its
    // interval early: one did, beginning a session 1.99 s after the last.
    Timer {
        id: reopenPacing
        onTriggered: root.reopenSoon()
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
        const queue = Places.leadersFirst(here.map((one, at) => ({ one, at: places[at],
            kept: keptPlaces[at], gone, told })).sort(order));
        if (reopening !== null) {
            reopening.queue = Places.queuedOnce(reopening.queue, queue, reopening.begun);
            // Settled again once these are placed too, beside the rest.
            reopening.settling = false;
            reopenSettling.stop();
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
            placed: [],
            behind: [],
            settling: false,
            rounds: 0,
            focused: false,
            // Each it has begun, by id, so one handed over again isn't.
            begun: []
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
                // Who orchestrates it and its title, as its record will say
                // them, so it opens with its orchestrator's sessions in
                // title order (85PKB2, 2026-11-08 [051e6eea]).
                orchestratedBy: typeof one.orchestratedBy === "string"
                    && /^[A-Z0-9]+$/.test(one.orchestratedBy) ? one.orchestratedBy : "",
                title: typeof one.title === "string" ? one.title : "",
                // The model a restart asks for (the harness's quest spawn), passed
                // to pi as it's said, and dropped when it's anything but a name.
                model: typeof one.model === "string" && /^[A-Za-z0-9._:/-]+$/.test(one.model)
                    ? one.model : "",
                quiet: one.focus !== true }));
        if (given.length === 0)
            return "refused: no session with an id and a folder";
        console.info(`sessions: reopening ${given.length} handed over`);
        reopenAll(given, (one, cwd) => console.info(`sessions: ${one.id} not reopened, `
            + `since its folder, ${cwd}, is gone`),
            id => Places.keptFor(id, { now: arrangement[bootId],
                restored: arrangement[restoring.boot] }));
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
        // One already running or parked here is never begun again, as a
        // restart watch may have begun it meanwhile, since two pis on one
        // log would each write over the other.
        if (root.running.concat(root.parked).some(session => session.here
            && session.presence.session === next.one.id)) {
            console.info(`sessions: ${next.one.id} not reopened, since it runs here`);
            reopenNext();
            return;
        }
        reopening.begun.push(next.one.id);
        reopening.looking = true;
        lookFor([next.one.cwd], there => {
            reopening.looking = false;
            heardFolders([next.one.cwd], there);
            if (!there[0]) {
                next.gone?.(next.one, next.one.cwd);
                reopenNext();
                return;
            }
            // Placed by its connector and the number it's given now, so it
            // goes beside those given the same screen, a gone one's too.
            const at = next.at ? { output: next.at.output, workspace: next.at.workspace,
                column: next.at.column, tile: next.at.tile ?? 1 } : null;
            // It opens where it goes when that's a workspace there now.
            // Else where it goes is focused first, so it most often opens
            // there and is seldom seen to move, but for one another session
            // started, which never moves the focus.
            const quiet = next.one.quiet === true;
            const into = quiet || at !== null
                ? root.opensOn(next.one, at, root.besideOf(reopening.placed)) : null;
            const focuses = !quiet && into === null;
            console.info(`sessions: reopening ${next.one.id}, kept at ${root.keptSaid(at)}, `
                + (into !== null
                    ? `asked on ${root.spaceSaid(into.workspace)} column ${into.column ?? "last"}`
                    : focuses ? "its place focused" : "beside Joel"));
            if (focuses)
                reopening.focused = true;
            if (focuses && next.at) {
                // A screen gone was given the screen in use (Places.js);
                // with none known, it stays on the one in use.
                if (Object.values(Niri.workspaces).some(one => one.output === next.at.output))
                    Niri.act({ FocusMonitor: { output: next.at.output } });
                Niri.act({ FocusWorkspace: { reference: { Index: next.at.workspace } } });
                Niri.act({ FocusColumnLast: {} });
            } else if (focuses && reopening.back !== null) {
                Niri.act({ FocusWindow: { id: reopening.back } });
            }
            reopening.waiting = { classed: null, at, opened: null, said: false,
                away: next.at?.away ?? null, kept: next.kept ?? null, quiet,
                quest: next.one.quest ?? "", from: next.one.from ?? "", id: next.one.id,
                orchestratedBy: next.one.orchestratedBy ?? "" };
            // The pace is counted from once it's begun and said, so the next
            // is said two seconds after however long this one took to start.
            // Its window's app id, which niri says as it opens, after this.
            reopening.waiting.classed = reopened(next.one, next.one.cwd, next.told, into);
            reopenStartedAt = Date.now();
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
    // Each workspace's screen and number as last seen with that screen
    // connected, by niri's id. A screen going, as at an undock, sends its
    // workspaces to another, and niri gives one back only if nothing opened
    // in it meanwhile: on 2026-11-03 a session handed over while Joel was
    // undocked made his first workspace the laptop's own. So each workspace
    // moved for its screen going is noted away from it (awayFrom), and goes
    // back in its place when the screen comes. One noted already keeps its
    // first screen, so a laptop shut on its dock never takes the monitor's.
    property var workspacesWere: ({})
    function noteGone() {
        const now = Object.values(Niri.workspaces);
        const outputs = now.map(one => one.output ?? "");
        const noted = Object.values(root.awayFrom)
            .reduce((all, kept) => all.concat(kept.map(one => one.id)), []);
        const next = {};
        for (const one of now) {
            const was = root.workspacesWere[one.id];
            const left = was !== undefined && !outputs.includes(was.output);
            if (left && was.output !== (one.output ?? "") && !noted.includes(one.id))
                root.noteAway({ screen: root.screens[was.output] || was.output,
                    workspace: was.idx }, one.id);
            next[one.id] = left ? was : { output: one.output ?? "", idx: one.idx };
        }
        root.workspacesWere = next;
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
        const moving = going.moves.filter(move => move.there !== true);
        moving.forEach(move => Niri.act({ MoveWorkspaceToMonitor: { output: move.output,
            reference: { Id: move.id } } }));
        // Then each to its index, lowest first, so each set stays put.
        going.moves.forEach(move => Niri.act({ MoveWorkspaceToIndex: { index: move.index,
            reference: { Id: move.id } } }));
        // A screen just connected keeps its empty workspace while it's the
        // active one there, so the first moved is focused, which lets niri
        // drop the empty one, and then Joel's window again.
        const first = moving[0];
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
    // `keptAt`, when given, says where each sat, else where the shell kept
    // it in the boot being restored.
    function reopenThemAll(lost, gone, keptAt) {
        const boot = root.arrangedBefore();
        const kept = Array.isArray(arrangedOthers[boot]) ? arrangedOthers[boot] : [];
        puttingBack = kept.length > 0 ? { boot, kept, used: [], back: [], seen: [] } : null;
        root.reopenAll(lost, gone, keptAt, carryOn);
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
        // One kept on a workspace of its own has none while another window
        // shares it: none of those back before it does, or it would join
        // them. Held there, a chat kept alone between two sessions' stayed
        // with Joel's windows on the first, as it opened.
        const shared = where.make && Object.values(Niri.windows)
            .some(other => other.id !== window.id && other.workspace_id === window.workspace_id);
        if (workspace && (shared || !Places.holds(where, { workspace, column }, beside)))
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

    // Whether the lock is held (shell.qml binds it from Lock.qml). niri
    // refuses every move while locked and says nothing of the lock over
    // its socket, and the lock's own word that it's let go comes before
    // niri has had it, so the windows of sessions opened behind it are
    // placed only once niri gives the keys back to a window, or a second
    // after the lock is let go when none takes them, as on a workspace
    // with no window.
    property bool behindTheLock: false
    property bool lockLetGo: true
    onBehindTheLockChanged: {
        if (behindTheLock) {
            lockLetGo = false;
            lockLettingGo.stop();
        } else {
            lockLettingGo.restart();
        }
    }
    Timer {
        id: lockLettingGo
        interval: 1000
        onTriggered: root.lockGone()
    }
    function lockGone() {
        if (root.behindTheLock || root.lockLetGo)
            return;
        lockLettingGo.stop();
        root.lockLetGo = true;
        root.placeBehind();
    }
    function placeable() {
        return !root.behindTheLock && root.lockLetGo;
    }
    // Each session opened behind the lock, placed as it would have been:
    // one with a place kept by the check once the last is placed, one
    // another session started with its quest's sessions. One whose kin is
    // in a kitty that hasn't yet said which window is whose, as one started
    // from the sessions' kitty beside its sender, stays held until it has
    // (onRunningChanged), rather than left wherever Joel's focus was.
    function placeBehind() {
        const going = root.reopening;
        if (!going)
            return;
        const behind = going.behind;
        going.behind = [];
        for (const held of behind) {
            if (!Niri.windows[held.entry.id])
                continue;
            if (held.entry.at !== null)
                going.placed.push(held.entry);
            else if (held.entry.quiet
                && root.placeQuietly(held.entry.id, held.waiting) === "waiting")
                going.behind.push(held);
        }
        if (going.queue.length === 0 && going.waiting === null && !going.looking
            && !reopenPacing.running)
            root.settleReopened();
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
            if (waiting)
                console.info(`sessions: ${waiting.id} ${waiting.opened === null
                    ? "opened no window" : "wasn't placed"} in 30 s`);
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
    // gives me executive dysfunction"). A hosted one, which pi's host runs
    // under a unit of its own with no window, is viewed in a new one by the
    // words its node gives (Jitsusama/mycelium: `attach --host H --session
    // ID`), named for it alone (viewerOf), so niri's word on that window is
    // what Go To raises next time; with no words it tells `unhosted`.
    signal unhosted(var session)
    function goTo(session) {
        if (session.presence?.hosted && !session.window) {
            view(session);
            return;
        }
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
        root.inSessions("pi-session", command);
        Report.say(`joined session ${session.address} on ${session.machine}`);
    }

    // Views a hosted session in a new window of the sessions' kitty, by its
    // node's words, once until niri lists its window, as `attach` does;
    // `into` a place kept for it (viewKept), which opens without the focus,
    // or, for Go To, where niri opens one, focused, since Joel asked for it.
    function view(session, into) {
        const words = session.attach;
        const named = viewerOf(session.presence);
        if (!Array.isArray(words) || words.length === 0 || named === "") {
            unhosted(session);
            return;
        }
        if (Date.now() - (attaching[named] ?? 0) < attachPatience)
            return;
        attaching = Object.assign({}, attaching, { [named]: Date.now() });
        root.inSessions("pi-hosted", words.map(String), into ?? null, named);
        Report.say(`viewing hosted session ${session.address}`);
    }

    // Where the windows of sessions moving onto pi's host were, by pi's
    // session id, so each one's viewer opens there once it runs hosted,
    // rather than nowhere until Joel goes to it (OSPHT4's cutover; 2QZBX9,
    // [906a6c10]). The cutover asks for each (keepPlace) just before it
    // tells the session to move, which closes its window. Kept in a file,
    // so a shell started again between the two still opens it.
    property var keptPlaces: keptPlacesOf(keptPlacesFile.text())
    function keptPlacesOf(text) {
        try {
            const kept = JSON.parse(text);
            return kept !== null && typeof kept === "object" && !Array.isArray(kept) ? kept : {};
        } catch (_error) {
            return {};
        }
    }
    FileView {
        id: keptPlacesFile
        path: Quickshell.statePath("kept-places.json")
        blockLoading: true
        printErrors: false
    }
    function keepPlaces(kept) {
        keptPlaces = kept;
        keptPlacesFile.setText(JSON.stringify(kept));
    }
    // Keeps where the window of the session here that pi knows as `id`
    // sits, answering "kept", or "no window to keep" for one with no
    // window niri has laid out, so the cutover can list the viewers it
    // couldn't put back.
    function keepPlace(id) {
        const session = running.find(one => one.here && one.presence.session === id);
        const place = /^[A-Za-z0-9-]+$/.test(id) ? Places.placeOf(session?.window, Date.now())
            : null;
        if (place === null)
            return "no window to keep";
        keepPlaces(Object.assign({}, keptPlaces, { [id]: place }));
        console.info(`sessions: kept ${id}'s place, ${place.workspace}/${place.column}`);
        return "kept";
    }
    // Opens the viewer of each session kept that now runs hosted where it
    // was, never on the workspace Joel is on and never with his focus
    // (Places.viewerPlace), and lets go of what's done or a day old
    // (Places.viewersDue). Asked again as niri's focus moves, so one waiting
    // for Joel to leave its workspace opens once he has.
    function viewKept() {
        if (Object.keys(keptPlaces).length === 0)
            return;
        const due = Places.viewersDue(keptPlaces, running, Date.now(), Niri.workspaces);
        for (const one of due.open)
            view(one.session, one.into);
        if (Object.keys(due.kept).length !== Object.keys(keptPlaces).length)
            keepPlaces(due.kept);
    }

    // The app id a hosted session's viewer opens with, by pi's session id,
    // which stays the same as its host restarts, or "" for one not hosted.
    function viewerOf(presence) {
        const id = presence?.hosted ? String(presence.session ?? "") : "";
        return /^[A-Za-z0-9-]+$/.test(id) ? `pi-hosted.${id}` : "";
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
        root.inSessions("pi-session", ["zsh", "-lic", 'exec mycelium --realm "$0" attach "$1"',
            realm, id]);
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
    // the machine it starts on, or that machine's home. With `quest` empty
    // it starts on none, as Machines' Start a Session Here does; `among`
    // names the realm whose node is asked, this machine's when left out.
    function startOn(on, quest, folder, among) {
        if (on === machine) {
            start(quest, folder);
            return;
        }
        if (!holds || starting.running)
            return;
        starting.asked = { on: on, quest: quest };
        starting.command = ["mycelium", "--realm", among || realm, "start"]
            .concat(on !== "" ? ["--need", `machine=${on}`] : [])
            .concat(["--folder", folder, "--"],
                quest !== "" ? ["env", `QUEST_WORKFLOW_AUTOLOAD_ID=${quest}`] : [], ["pi"]);
        starting.running = true;
    }
    Process {
        id: starting
        property var asked: null
        stdout: StdioCollector {
            onStreamFinished: root.started(starting.asked, text)
        }
    }
    // " on QEST-..." for a session started on a quest, or nothing.
    function questNamed(quest) {
        return quest !== "" ? ` on ${quest}` : "";
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
            Report.say(`couldn't start a session${questNamed(asked.quest)}: ${why}`);
            return;
        }
        root.inSessions("pi-session", begun.attach);
        Report.say(`started a session${questNamed(asked.quest)} on ${begun.machine}`);
    }

    // Puts a steer in a session's inbox: `{ text, id }` talks to it,
    // `{ act }` pauses, resumes or stops it. The realm's node is asked on
    // the connection the shell keeps to it, here or elsewhere alike: it
    // writes the inbox of the machine running the session, one parked
    // included, carrying it over the link when that's another machine's
    // (mycelium's PROTOCOL.md, The Daemon Interface). One it refuses, or
    // that finds no node to ask, is said by `unsteered` with the node's
    // reason, or none; `then`, when given, is told whether it was put.
    signal unsteered(var session, string why)
    function steer(address, fields, then) {
        const upper = String(address).toUpperCase();
        PiSource.steer(upper, fields, answer => {
            const machine = answer?.steered?.machine;
            if (machine) {
                Report.say(`steered ${upper} on ${machine}`);
            } else {
                const why = String(answer?.error?.message ?? "");
                Report.say(`unsteered ${upper}: ${why}`);
                unsteered(running.find(one => String(one.address).toUpperCase() === upper)
                    ?? parkedAs(upper) ?? { address: upper }, why);
            }
            if (then)
                then(Boolean(machine));
        });
    }

    // Talks to a session: the words go with an id, so the session writes
    // what became of them where the launcher reads its talks.
    function talk(address, words) {
        const text = words.trim();
        if (text === "")
            return false;
        // Talk gives a parked session a turn, as a question would, so it's
        // woken to read it, once its words are in its inbox.
        const resting = parkedAs(address);
        steer(address, { text: text, id: `talk-${Date.now()}` }, put => {
            if (put && resting)
                wake(resting);
        });
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

    // A want new to this shell sounds once, softly, and is said nowhere
    // else: For You lists it as a thing to do (launcher/Now.qml's
    // wantRow) and the margin's mark says it while it stands, so a notice
    // as well said one ask three times (Joel said yes to For You,
    // 2026-11-08). It sounds unless Joel is in that window already, and not
    // for one that stood before the shell began. Each is known by its
    // session and its want's own id or start (Waiting.wantKey), never by
    // its words, so one rewording itself doesn't sound again. It's kept
    // known: a presence file replaced by rename leaves its folder for a
    // moment, which would otherwise make a want that never ended look new.
    readonly property real began: Date.now()
    property var told: ({})
    function key(session) {
        return Waiting.wantKey(session.address, session.presence.wants);
    }
    onWantingChanged: {
        for (const session of wanting) {
            const known = key(session);
            if (told[known])
                continue;
            told[known] = true;
            const since = Date.parse(session.presence.wants.since ?? "");
            const window = session.window ? String(session.window.id) : "";
            if (window === String(Niri.current) || (since < began))
                continue;
            // Never while notices are held.
            if (Options.sounds.wants !== "" && !Quiet.on)
                Niri.spawn([Options.sounds.player, "--volume=0.5", Options.sounds.wants]);
        }
    }

    // What's outside the shell asks of sessions: reopen, as the harness
    // hands over the sessions it restores (handedOver).
    IpcHandler {
        target: "sessions"

        function reopen(sessions: string): string {
            return root.handedOver(sessions);
        }

        // keepPlace, as the cutover asks it before moving a session onto
        // pi's host, by pi's session id.
        function keepPlace(session: string): string {
            return root.keepPlace(session);
        }
    }

    // Whether a session's want is truly on Joel, rather than one its
    // presence mirrors from a file of things he can't do yet, or a
    // question put to its orchestrator.
    function asks(session) {
        return session.presence !== null
            && Waiting.asksJoel(session.presence, WaitingOnYou.standing);
    }

    // What a want is, in words after the session's name: "needs you",
    // the one phrase for waiting on Joel everywhere, and a yes or a stuck
    // run said within it (VKCF3Z, 2026-11-06), as the vocabulary has them.
    function wanted(wants) {
        const said = Words.of("agents:wants").said ?? {};
        return String(said[wants.kind] ?? said.question ?? "");
    }

    // A session's quest's whole title and its folder's name, as its record
    // says them: `questInfo.title` and the last part of `cwd`, as the
    // harness publishes them from #1006 on. A record from before then, as
    // a parked one's stays until it wakes, is named by its window's label,
    // cut short as it is.
    function named(presence) {
        const label = labelled(typeof presence.title === "string" ? presence.title : "");
        const title = presence.questInfo?.title;
        const cwd = typeof presence.cwd === "string" ? presence.cwd.replace(/\/+$/, "") : "";
        return {
            title: typeof title === "string" && title !== "" ? title : label.cut,
            folder: cwd !== "" ? cwd.slice(cwd.lastIndexOf("/") + 1) : label.folder
        };
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

    // This boot, which a record's owner names the boot it began in, as
    // pi's source reads it.
    readonly property string bootId: PiSource.bootId

    // The kitty window a session runs in, { socket, window }, or null: as
    // the node says a held one's is, or as its record's terminal says. One
    // whose record names no terminal is held, its window the node's
    // (2026-11-06: 43 of the 53 running, and every one of them held).
    readonly property var kittySocket: /^unix:@kitty-(\d+)$/
    // The kitty window the node says a session is in, from its `window`.
    function kittyOfWindow(window) {
        return window && window.terminal === "kitty" && window.within && Number(window.id) > 0
            ? { socket: window.within, window: Number(window.id) } : null;
    }
    function kittyOf(presence) {
        const terminal = presence.terminal;
        return terminal && typeof terminal === "object" && terminal.driverId === "kitty"
            && terminal.scope && Number(terminal.value) > 0
            ? { socket: terminal.scope, window: Number(terminal.value) } : null;
    }

    // Each running session's record, { file, presence, kitty, opened, held },
    // as pi's source sorts the node's (PiSource.js), `opened` the app id its
    // window was opened with, where the node says it was (inSessions), or
    // "". A held session is in the window the node says joined its holder
    // last, or none, since its process has lost its own. Made again when the source sorts them
    // again or a window comes or goes, and kept as it was when nothing in
    // it did, so what reads it isn't told for nothing.
    property var live: []
    // Whether live, resting and ending have been made once, since each is
    // made again only when what it holds moves.
    property bool relived: false
    // Each parked session's record, { file, presence, kitty, held }, as
    // `live` keeps the running ones'.
    property var resting: []
    // Each record of a session here that ended this boot, unparked, as
    // `resting` keeps the parked ones', with when it changed last.
    property var ending: []
    function relive() {
        const kitted = record => {
            const kitty = kittyOfWindow(record.window);
            const kept = Object.assign({}, record, {
                kitty: record.here === false || record.held
                    ? kitty : kitty ?? kittyOf(record.presence),
                opened: String(record.window?.app_id ?? "")
            });
            delete kept.window;
            return kept;
        };
        const next = PiSource.live.map(kitted);
        const sleeping = PiSource.resting.map(kitted);
        const ended = PiSource.ending.map(kitted);
        // Only the list a change moved is set again, so what reads the
        // others isn't told: one session's step moving had made the 36
        // parked and 36 closed of Joel's 113 shown again (2026-11-09).
        // Compared in place rather than as text, since the text of his
        // sessions, about 280,000 characters, started a garbage
        // collection each time (as PiSource.js's alike says).
        const first = !relived;
        relived = true;
        if (first || !Pi.alike(resting, sleeping))
            resting = sleeping;
        if (first || !Pi.alike(ending, ended))
            ending = ended;
        if (!first && Pi.alike(live, next))
            return;
        const by = {};
        for (const record of next) {
            for (const address of record.presence.addresses)
                by[address] = record.presence;
        }
        present = by;
        live = next;
        ask();
    }

    // niri's windows as the windows daemon serves them, each its record
    // (SessionWindows.js says which fields Sessions reads), kept through a
    // gap in the daemon, as what it last said still stands.
    Daemon {
        id: windowsDaemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
        kind: "window"
        answersWhole: true
        onHeard: (changed, gone) => root.windowsHeard(changed, gone)
    }
    readonly property var windowsFrom: windowsDaemon

    // niri's windows by the process drawing them, each kitty's oldest first,
    // and by the app id each was opened with, the oldest of each, for
    // `running`. They're made again only when a window comes, goes or
    // moves, rather than for every change to a session and looked through
    // for every session, which with a hundred sessions made a hundred lists
    // a change; a retitle, or the focus moving, makes nothing again.
    property var niriWindows: ({ pids: {}, opened: {} })
    // The records `niriWindows` was made from, by key.
    property var windowsKept: ({})
    function windowsHeard(changed, gone) {
        let came = gone.length > 0;
        let moved = came;
        for (const key of changed) {
            const record = windowsDaemon.held[key];
            came = came || !(key in windowsKept);
            moved = moved || SessionWindows.moved(windowsKept[key], record);
        }
        if (!moved)
            return;
        windowsKept = Object.assign({}, windowsDaemon.held);
        niriWindows = SessionWindows.indexed(Object.values(windowsKept));
        // A window that comes or goes can be a session's ending, or a
        // kitty's windows changing.
        if (came) {
            relive();
            ask();
        }
    }

    // niri's windows by the process drawing them, each kitty's oldest first.
    function kittyWindows() {
        return niriWindows.pids;
    }

    // A session's niri window: the one niri lists by the app id its window
    // was opened with (`opened`), or a hosted one's viewer by its own
    // (viewerOf), since kitty's own ids mean nothing to niri. A window
    // opened without one, before the shell named them or by Joel by hand,
    // is the one window of the kitty it runs in, or of a
    // kitty drawing several, the nth of kitty's as niri's nth of that
    // kitty's, each newer than the last, once kitty has said which of its
    // windows holds the session and has as many as niri shows. That ranking
    // goes wrong as windows come and go, so it's only for windows with no
    // app id, and goes once every session's window has one. Without the
    // windows by app id, as for a parked or closed session, whose lists
    // needn't be made again each time niri's focus moves, the windows
    // daemon's are looked through.
    function windowOf(record, kitties, opened) {
        const named = record.opened || viewerOf(record.presence);
        if (named !== "" && opened)
            return opened[String(named)] ?? null;
        if (named !== "")
            return Object.values(windowsKept).find(window => window.opened === named) ?? null;
        const kitty = record.kitty;
        const pid = kitty ? (kittySocket.exec(kitty.socket) ?? [])[1] : undefined;
        const windows = pid ? kitties[pid] ?? [] : [];
        if (windows.length === 1)
            return windows[0];
        const listed = ranks[kitty?.socket ?? ""];
        if (!listed || listed.windows !== windows.length || listed.of[kitty.window] === undefined)
            return null;
        return windows[listed.of[kitty.window]];
    }

    // What each kitty with several windows, one of them a session's with no
    // app id, says of them: how many, and in which of them each of its
    // windows sits. Asked again only when niri's windows of that kitty
    // change, one kitty at a time.
    property var ranks: ({})
    property var asked: ({})
    property var asking: []
    function ask() {
        const kitties = kittyWindows();
        const sockets = {};
        for (const record of live) {
            if (record.kitty && record.opened === "")
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
        // The hidden window that keeps the sessions' kitty running between
        // sessions (home.nix's sessions-kitty) is one niri never lists, so
        // it's left out, or every window after it would be taken for the
        // next one niri has.
        const shown = windows.filter(window => window.wm_class !== "pi-sessions-kept");
        const of = {};
        shown.sort((a, b) => a.id - b.id).forEach((window, rank) => {
            for (const tab of window.tabs ?? []) {
                for (const inside of tab.windows ?? [])
                    of[inside.id] = rank;
            }
        });
        const next = Object.assign({}, ranks);
        next[socket] = { windows: shown.length, of: of };
        ranks = next;
    }

    // The machine's mycelium node, for the realm the machine is in, as pi's
    // source keeps it: the realm's name and this machine's, which the node
    // names each session's machine by.
    readonly property string realm: PiSource.realm
    readonly property string machine: PiSource.machine

    // The node's sessions, as pi's source holds them (PiSource.qml): its
    // socket, its link, and the folder of parked sessions' markers. Sorted
    // again as the source sorts them, into the windows they run in here.
    readonly property string nodePath: PiSource.nodePath
    readonly property var node: PiSource.node
    readonly property string parkedFolder: PiSource.parkedFolder
    Connections {
        target: PiSource
        function onSorted() {
            root.relive();
        }
    }

    // Wakes a parked session, as a peer sending it a turn does: anything
    // written to the file its marker names has its holder start it again
    // in its window, written in place, since the holder reads it whole and
    // deletes it (the harness's askWake). It starts with its steer inbox
    // read, so words left there first reach it as it wakes.
    function wake(session) {
        const id = String(session?.presence?.session ?? "");
        const wakes = PiSource.wakes(id);
        if (wakes === "")
            return;
        Niri.spawn(["sh", "-c", 'printf "%s\\n" "$2" > "$1"', "wake", wakes,
            JSON.stringify({ session: id, messageId: "joel", at: new Date().toISOString() })]);
        Report.say(`woke session ${session.address}`);
    }
    // Each running session's record, by every address it answers to.
    property var present: ({})
}

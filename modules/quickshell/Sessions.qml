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
// For You and the margin say a want, and one stopped on Joel's keys
// sounds and is noticed (onStoppedChanged).
import Quickshell
import Quickshell.Io
import QtQuick
import "PiSource.js" as Pi
import "Places.js" as Places
import "Reopening.js" as Reopening
import "SessionWindows.js" as SessionWindows
import "Viewers.js" as Viewers
import "Waiting.js" as Waiting
import "launcher/Order.js" as Order

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
    // Shown as each running one is, in the window its holder keeps where
    // the node knows it, though its process has gone; in the order the
    // records are. Never among `running`, so nothing counts it as
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
    // other orchestrates. The oldest first, those asking at once by
    // address (Waiting.js's askedBefore). One parked still wants him:
    // its presence keeps its want while its process gives back its memory
    // (the harness's peer.ts), so it waits in For You until he answers.
    // One Joel put off for an hour, as the agents daemon says it is
    // (`putOff`), waits on him again once its hour is over.
    readonly property var wanting: running.concat(parked)
        .filter(session => asks(session) && !putOff[String(session.address).toUpperCase()])
        .sort(Waiting.askedBefore)
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
    // Where the harness's config says sessions Joel starts run in one of
    // pi's hosts (5KBYD7's start hosted, harness #1434), its decider is
    // asked first, and one it hosts opens only a viewer (startAsked).
    function start(quest, folder, orchestrates) {
        if (startsHosted) {
            startAsked.createObject(root, { quest, folder, orchestrates });
            return;
        }
        startWindow(quest, folder, orchestrates);
    }
    function startWindow(quest, folder, orchestrates) {
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
    // focus once its window shows. One Go To opens `focused` is focused by
    // the shell itself once niri lists it (focusOpened), since taking Joel
    // to it is what he asked, and the token can't take him: the kitty that
    // draws the window was started long before, by its unit, and the token
    // stays with the client asking it, so the window opened where it was
    // told, unfocused (Joel, 2026-11-18 10:44: "it isn't coming to the
    // forefront").
    // They're numbered on from the moment this shell started, in
    // milliseconds, not from nought: a shell started again, as every switch
    // does, would otherwise name its first window as one still open from
    // before, and windowOf would take either for the other. No shell opens
    // a window a millisecond, so a later start counts past every number an
    // earlier one gave.
    readonly property real startedAt: Date.now()
    property real openings: startedAt
    // How a window is asked of that kitty: over its own socket, or the old
    // way only while the unit's own process is a kitty from before it
    // listened, as the script beside this file says, where it's linted and
    // run by a case of its own (the-sessions-kitty-is-asked-only-its-own-way).
    readonly property string opening: String(Qt.resolvedUrl("opens-in-sessions.sh"))
        .replace(/^file:\/\//, "")
    // The window the last Go To opened focused, until niri lists it: its
    // app id, when it was asked, and the window that had the focus then.
    // Only the last, so a later Go To takes the place of an earlier one
    // still opening, which then opens where it was told and no more.
    property var goingTo: null
    // How long a Go To's window may take to list and still be focused:
    // long enough for the sessions kitty to start again under it, since
    // a Joel who moves the focus himself meanwhile stops it anyway. Cases
    // shorten it to see one too late.
    property int goToPatience: 30000
    // Focuses the window Go To opened once niri lists it, unless the focus
    // has moved since he asked, as when he went on to other work himself,
    // or it came too late, either of which leaves it where it opened, said.
    function focusOpened(window) {
        const going = goingTo;
        if (going === null || window.opened !== going.app)
            return;
        goingTo = null;
        const waited = Date.now() - going.at;
        if (Niri.focused !== going.from)
            Report.say(`Go To's window ${window.id} left where it opened: the focus moved`);
        else if (waited >= goToPatience)
            Report.say(`Go To's window ${window.id} left where it opened: it took ${waited} ms`);
        else
            Niri.act({ FocusWindow: { id: window.id } });
    }
    function inSessions(kind, rest, into, named) {
        openings++;
        const classed = named ?? `${kind}.${openings}`;
        const placed = into !== null && into !== undefined;
        if (placed) {
            const asked = { app_id: classed, reference: { Id: into.workspace } };
            if (into.column !== null)
                asked.column = into.column;
            Niri.act({ OpenNextWindowOn: asked });
            if (into.focused === true)
                goingTo = { app: classed, at: Date.now(), from: Niri.focused };
        }
        const untold = kind === "pi-spawned" || (placed && into.focused !== true)
            ? ["env", "-u", "XDG_ACTIVATION_TOKEN", "-u", "DESKTOP_STARTUP_ID"] : [];
        Niri.spawn(untold.concat(["sh", opening,
            `${Quickshell.env("XDG_RUNTIME_DIR")}/pi-sessions/kitty.sock`, classed], rest));
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
        return holds ? `exec mycelium --realm ${realm} holds add -- ${command}`
            : `exec env -u MYCELIUM_APP_ID ${command}`;
    }
    // What a hosted session's viewer runs: the node's words, kept by the
    // realm's holder as a start is, so the node knows the window it's in
    // and its host can let the conversation go into plain pi in the same
    // window rather than closing it (OSPHT4, 2026-11-11 [56e0fd87]). On a
    // machine in no realm they run as they are.
    function viewing(words) {
        return holds ? ["mycelium", "--realm", realm, "holds", "add", "--"].concat(words)
            : words;
    }

    // Reopens a session that closed, where it left off: pi told the
    // session, in the folder it ran in, which its file's first line says,
    // as the harness restores one. With no file to say it, a session is
    // started afresh on the quest instead, in `folder`. A session's folder
    // can be gone by now, a tree since given back, and kitty given a
    // missing folder opens in the home folder without a word, where pi
    // starts a new session apart from the old one (one of Joel's
    // orchestrators, 2026-10-29). So the folder is looked for first; when
    // it's gone, the quest starts afresh in `folder` if that's there, and
    // `gone(cwd, startedIn)` is told either way, `startedIn` empty when
    // nothing could be opened, so the caller says so.
    readonly property string sessionFiles: `${home}/.pi/agent/sessions`

    // One at a time, in the order asked, so a second asked before the
    // first's file is read waits rather than taking its place.
    property var resuming: []
    // `kept`, where given, asks only for the session where it left off,
    // never one started afresh in its folder: its `done` is told the
    // folder it reopened in, and `gone` is told "" for where it started
    // when there's no session file, or no folder it ran in, to open.
    function resume(id, quest, folder, gone, kept) {
        resuming = resuming.concat([{ id: id, quest: quest, folder: folder, gone: gone,
            kept: kept ?? null }]);
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
                reopened({ id: asked.id, env: asked.kept?.env ?? null }, cwd);
                asked.kept?.done?.(cwd);
                return;
            }
            if (asked.kept) {
                asked.gone?.(cwd, "");
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

    // Opens a closed session again where it left off, in a window of its
    // own in `cwd`, held as every start is. One pi never saved, as it
    // saves none before its first answer, is `fresh` in restore.json
    // (85PKB2, 2026-11-01 [e51c350d]): `pi --session` finds no record of
    // it and exits at once, so it starts under its own id, with its quest
    // loaded as start() loads one, since only a record brings one back.
    // `told`, when given, is pi's first prompt, sent as it opens, as the
    // shell's $0 so nothing in it is read as the shell's own. One its
    // decider kept out of a host starts told why, so its pi says so rather
    // than asking again (decided below).
    function reopened(one, cwd, told, into) {
        const { env, pi } = resumeWords(one);
        const fresh = one.fresh === true;
        const telling = typeof told === "string" && told !== "";
        // Classed as a session's either way, so a window Joel opens
        // meanwhile isn't taken for it; one another session started opens
        // aside.
        const kind = one.quiet === true ? "pi-spawned" : "pi-session";
        const classed = root.inSessions(kind, ["--directory", cwd].concat(env,
            ["zsh", "-lic", held(telling ? `${pi} -- "$0"` : pi)], telling ? [told] : []), into);
        Report.say(`${fresh ? "started unsaved" : "resumed"} session ${one.id}`);
        return classed;
    }
    // The words that start `one` again where it left off, as reopened and
    // reopenedHeld run them: `env`, what it's started with, and `pi`.
    function resumeWords(one) {
        const fresh = one.fresh === true;
        const quest = fresh && typeof one.quest === "string" && one.quest !== ""
            ? [`QUEST_WORKFLOW_AUTOLOAD_ID=${one.quest}`] : [];
        const stayed = typeof one.stayed === "string"
            ? [`QUEST_WORKFLOW_SPAWN_STAYED=${one.stayed}`] : [];
        const assigned = quest.concat(root.startedWith(one), stayed);
        const model = one.model ? ` --model ${one.model}` : "";
        return { env: assigned.length > 0 ? ["env"].concat(assigned) : [],
            pi: (fresh ? `pi --session-id ${one.id}` : `pi --session ${one.id}`) + model };
    }
    // Starts `one` again where it left off, held as every start is but
    // detached, so no window opens for it and Go To joins it, as Reopen
    // Them All brings back one that had no window in the boot
    // (Reopening.broughtBack). Its folder goes as the shell's own `$1`, so
    // nothing in it is read as the shell's words, and `told` as `$0`, as
    // reopened sends it. When it started is kept, so it isn't begun again
    // before its pi says it runs (Reopening.startedHeldLately).
    function reopenedHeld(one, cwd, told) {
        const { env, pi } = resumeWords(one);
        const telling = typeof told === "string" && told !== "";
        Niri.spawn(env.concat(["zsh", "-lic", `exec mycelium --realm ${realm} holds add `
            + `--detached --folder "$1" -- ${telling ? `${pi} -- "$0"` : pi}`,
            telling ? told : "zsh", cwd]));
        root.startedHeld = Object.assign({}, root.startedHeld, { [one.id]: Date.now() });
        Report.say(`${one.fresh === true ? "started unsaved" : "resumed"} session ${one.id} `
            + "without a window");
    }
    // The sessions started without a window, each to when, in milliseconds
    // (reopenedHeld).
    property var startedHeld: ({})

    // What a session was started with that it's started with again, from
    // its record's `env` as the harness writes it in restore.json and hands
    // it over (its #1345): the fence, PI_SANDBOX and PI_SPAWNER alone, each
    // a plain word, so a fenced session never comes back unfenced (85PKB2,
    // 2026-11-11, after Reopen Them All did so at 11:07). Anything else is
    // left out, and said, since nothing fails silently.
    readonly property var passedOn: ["PI_SANDBOX", "PI_SPAWNER"]
    function startedWith(one) {
        const given = one.env;
        if (given === undefined || given === null)
            return [];
        const names = typeof given === "object" && !Array.isArray(given) ? Object.keys(given)
            : [String(given)];
        const kept = root.passedOn.filter(name => names.includes(name)
            && typeof given[name] === "string" && /^[A-Za-z0-9._-]+$/.test(given[name]));
        const left = names.filter(name => !kept.includes(name));
        if (left.length > 0)
            console.warn(`sessions: ${one.id} started without ${left.join(", ")} from its `
                + "record, since only PI_SANDBOX and PI_SPAWNER, each a plain word, are passed on");
        return kept.map(name => `${name}=${given[name]}`);
    }

    // The sessions the last boot's end took away, to reopen after a cold
    // boot (Joel, 2026-10-26: one act brings back every session live at
    // shutdown, each in its own window and folder, never one from before
    // that boot), as the agents daemon serves them (modules/agents's
    // reopen.rs). It works them out of the harness's restore.json, what
    // runs here and what Joel said of them, so the shell reads no file and
    // works none of it out (Joel, 2026-11-10: "the daemons need to be the
    // data layer"). Each is `reopen:agents:<id>`: the session as
    // restore.json keeps it, the way it came and its place in the list.
    // `restoring:agents` names the boot the file is of, which the places
    // kept are by. Each comes worded as its row says it, under `row`, and
    // `to-reopen:agents` words the place, its folders looked for as the
    // daemon makes the list, so the launcher looks for none as it opens
    // and still says a folder is gone before Return (VKCF3Z, 2026-11-07
    // [4a00987e]). With the daemon not answering, none is offered. The
    // same watch takes each session's `want`, by which the bell rings
    // (`stopped`), unpaced, since the bell means something needs Joel now.
    Daemon {
        id: reopenKind

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/agents.sock`
        kind: ["reopen", "restoring", "to-reopen", "arrangement", "kept-places", "want"]
        watching: ({ place: "agents" })
        answersWhole: true
    }
    readonly property var offered: {
        void reopenKind.revision;
        return Order.sorted(Object.values(reopenKind.held).filter(one => one.kind === "reopen"),
            Order.smaller(one => one.order));
    }
    // Each one's row as the daemon words it, by its id: its name, side,
    // note, whether its folder is gone and the host that gave it back.
    readonly property var reopenRows: {
        const rows = {};
        for (const one of offered)
            rows[one.session?.id] = one.row ?? {};
        return rows;
    }
    // Sessions to Reopen's own words: its count, where they came from,
    // and Reopen Them All's side while any folder is gone.
    readonly property var reopenPlace: {
        void reopenKind.revision;
        return reopenKind.held["to-reopen:agents"]?.toReopen ?? {};
    }
    readonly property var restoring: {
        void reopenKind.revision;
        return { boot: String(reopenKind.held["restoring:agents"]?.restoring?.boot ?? "") };
    }

    // The last boot's sessions not back here. After a boot they wait for
    // Joel: Reopen Them All, or :reopen, brings them back in one act, each
    // where the shell kept it, and nothing reopens them before he asks.
    // The shell once reopened them by itself as it first saw niri, which
    // he never wanted (Joel, 2026-11-07 20:2x: "no, i never wanted them
    // automatic, you decided that by yourself"); on 11-01 he had chosen
    // to reopen them by hand.
    readonly property var lastBoot: offered.filter(one => one.way === "boot")
        .map(one => root.wayCame(one))

    // The sessions offered apart from the last boot's, whatever the boot,
    // until each runs here, and never put off with the last boot's
    // (VKCF3Z, [17db838c]): those other machines carried here as they went
    // away, and those pi's host gave back, so they run in plain pi again
    // (2QZBX9, [44e65ce0]). The newest first, as the daemon orders them.
    readonly property var apart: offered.filter(one => one.way !== "boot")
        .map(one => root.wayCame(one))
    // Each offered session with the way it came, which decides whether one
    // kept nowhere comes back without a window (Reopening.broughtBack): on
    // 2026-11-21 the session alone went on, and two kept nowhere got one.
    function wayCame(one) {
        return Object.assign({}, one.session, { way: one.way });
    }

    // Every session there is to reopen: the last boot's, then those apart.
    readonly property var toReopen: lastBoot.concat(apart)

    // Joel forgets one of them, which the daemon keeps, since restore.json
    // is the harness's to write.
    function forgetReopen(lost) {
        reopenKind.ask({ ask: "forget", session: lost.id });
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
        // A hosted one is gone to, which views it, since its host runs it.
        // One no host serves yet, as just after a crash, is viewed by its
        // entry's own words, focused where Go To would open it, to wait
        // there for its host as the walk's do; or said, never a press that
        // does nothing.
        if (lost.hosted === true) {
            const session = root.hostedHere(lost.id);
            if (session)
                goTo(session);
            else
                reopenWaiting(lost);
            return;
        }
        lookFor([lost.cwd], there => {
            if (there[0])
                reopened(lost, lost.cwd);
            else
                gone?.(lost.cwd, "");
        });
    }

    // A hosted one asked for by itself before its host serves it: its
    // waiting viewer, raised when niri lists it already, never a second.
    // Its entry's words are run as they are, as Reopen Them All runs them
    // (reopenViewer), since the harness writes the entry whole.
    function reopenWaiting(lost) {
        const named = viewerNamed(lost.id);
        const words = Viewers.waitingWords(lost);
        const listed = listedNamed(named);
        if (listed) {
            Niri.act({ FocusWindow: { id: listed.id } });
            return;
        }
        const said = { address: String(reopenRows[lost.id]?.name ?? lost.id), machine: "" };
        if (named === "") {
            console.info(`sessions: ${lost.id} not viewed, since no app id could carry its id`);
            unhosted(said, "unnamed");
            return;
        }
        if (words === null) {
            console.info(`sessions: ${lost.id} not viewed, since no host serves it `
                + "and its entry gives no words to wait with");
            unhosted(said, "hostless");
            return;
        }
        // Said before any place is worked out, which can name a workspace,
        // for a window that isn't asked for.
        if (heldBack(lost.id, named, said))
            return;
        const into = root.opensOn(lost, null, []);
        const title = Viewers.viewerTitle("", lost.title, lost.cwd);
        viewWith(words, named, into === null ? null
            : Object.assign({}, into, { focused: true }), title);
        console.info(`sessions: viewing ${lost.id} to wait for its host`);
    }
    // Whether a viewer `named` is held back, as one asked for moments ago
    // that niri hasn't listed yet (viewerComing), said opening. Asked
    // only once a listed viewer would have been raised.
    function heldBack(id, named, said) {
        if (!viewerComing(named))
            return false;
        console.info(`sessions: ${id} not viewed again, since its viewer is asked for already`);
        unhosted(said, "opening");
        return true;
    }

    // A window the last boot had joined to a session another machine
    // holds is joined again by its words, in its folder here when that's
    // still there, or gone to if a window here has joined it since, never
    // opened twice.
    // `quiet` when Joel didn't ask for this one by itself, as Reopen Them All
    // rejoins another machine's: its window opens where opensOn says, on its
    // orchestrator's workspace, never on the one he's on (Joel, 2026-11-13).
    function rejoin(lost, quiet) {
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
                (there ? ["--directory", lost.cwd] : []).concat(lost.attach),
                quiet === true ? root.opensOn(lost, null, null) : null);
            Report.say(`joined session ${lost.address} on ${lost.machine} again`);
        };
        if (lost.cwd === "")
            open(false);
        else
            lookFor([lost.cwd], there => open(there[0]));
    }

    // Joel says Not Now to the last boot's, which the daemon keeps, so they
    // stay put off however often the shell starts on this boot.
    function putOffLastBoot() {
        reopenKind.ask({ ask: "put-off" });
    }

    // Joel says Not Now to what a session asks him, which the daemon puts
    // off for an hour (wants.rs's PutOff), so it leaves For You, rings for
    // nothing, and is back once the hour's over if it still stands.
    // It names the ask he saw (`known`), so a newer one the session asks
    // meanwhile is refused and still needs him.
    function putOffWant(address, known) {
        reopenKind.ask({ ask: "not-now", address: String(address), known: String(known ?? "") });
    }
    // Who the daemon says the want of the session at `address` is, or "".
    function wantKnown(address) {
        return String(wantRecords[String(address).toUpperCase()]?.known ?? "");
    }
    // The daemon's want records by address, in capitals.
    readonly property var wantRecords: {
        void reopenKind.revision;
        const by = {};
        for (const one of Object.values(reopenKind.held)) {
            if (one.kind === "want")
                by[String(one.address).toUpperCase()] = one;
        }
        return by;
    }
    // The addresses whose wants the daemon says are put off, in capitals.
    readonly property var putOff: {
        const by = {};
        for (const address of Object.keys(wantRecords)) {
            if (wantRecords[address].putOff === true)
                by[address] = true;
        }
        return by;
    }

    // Where each session's window sat, by the session's own id, each boot's
    // (this one's and the one before), and every other window's place beside
    // them by its program and title (`others`), kept by the agents daemon
    // once the windows have been still a moment (modules/agents's
    // arrangement.rs), so Reopen Them All puts every one back on its
    // workspace in its order after a cold boot.
    readonly property var arrangementRead: {
        void reopenKind.revision;
        const read = reopenKind.held["arrangement:agents"]?.arrangement;
        return read?.v === 1 && read.boots && typeof read.boots === "object" ? read : {};
    }
    readonly property var arrangement: arrangementRead.boots ?? {}
    readonly property var arrangedOthers: arrangementRead.others
        && typeof arrangementRead.others === "object" ? arrangementRead.others : {}
    // What a reopen puts the sessions by, each boot's places by session id
    // and the boot being restored, said for the test as it changes, since a
    // reopen asked before both are heard puts every session nowhere (at
    // 9bceeeda and a9f253f9, by 2026-11-19). Nothing while reports are off.
    readonly property string placesHeard: {
        if (!Options.reports)
            return "";
        const kept = Object.keys(arrangement).sort()
            .map(boot => `${boot}: ${Object.keys(arrangement[boot] ?? {}).sort().join(" ")}`)
            .join(", ");
        return `heard places kept for ${kept || "no boot"}, restoring ${restoring.boot || "no boot"}`;
    }
    onPlacesHeardChanged: {
        if (placesHeard !== "")
            Report.say(placesHeard);
    }
    // The boot kept beside this one, as the daemon keeps it: the boot being
    // restored when its places are kept, else the other kept.
    function arrangedBefore() {
        if (restoring.boot !== "" && restoring.boot !== bootId && arrangement[restoring.boot])
            return restoring.boot;
        return Object.keys(arrangement).find(boot => boot !== bootId) ?? "";
    }
    // Each connector's screen as niri describes it (Places.js), asked for
    // as the windows settle and as they're reopened, since a dock or a boot
    // can move a screen to another connector. Kept as last said when niri
    // doesn't answer. niri never answers a request it refused, nor one
    // sent on a connection that closed, so Reopen Them All waits no more
    // than screensPatience on it, rather than never reopening at all.
    property var screens: ({})
    // The screens connected now, of those last said (Places.connected), for
    // anything sending a window or a workspace to one.
    function screensThere() {
        return Places.connected(screens, Object.values(Niri.workspaces ?? {}));
    }
    property var screensWaiting: []
    function lookAtScreens(then) {
        screensWaiting = screensWaiting.concat([then]);
        if (screensWaiting.length > 1)
            return;
        screensPatience.restart();
        Niri.request("Outputs", answer => {
            screensUnheard = 0;
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
    // Looks in a row niri has left unanswered. A screen that came while
    // niri lagged, as at a redock on 2026-11-15, was never seen, so the
    // workspaces waiting for it (comeHome) stayed away with nothing to look
    // again: so while any wait, an unanswered look is tried again, for
    // about a minute, a niri that never answers being one gone.
    property int screensUnheard: 0
    Timer {
        id: screensPatience
        interval: 2000
        onTriggered: {
            root.screensLooked();
            if (Object.keys(root.awayFrom).length === 0 || root.screensUnheard >= 15)
                return;
            root.screensUnheard += 1;
            console.info(`sessions: niri didn't say its screens in 2 s, so looking again`);
            arranging.restart();
        }
    }
    Timer {
        id: arranging
        interval: 2000
        onTriggered: root.lookAtScreens(() => {
            root.comeHome();
            root.putBackOpen();
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
    // Joel going away lets a moved session's kept viewer open (viewKept).
    Connections {
        target: Away
        function onNowChanged() {
            root.viewKept();
        }
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
            root.viewerCame(window);
            root.focusOpened(window);
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
            root.letGoWaitedFor(late);
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
        if (waiting && waiting.opened === null && opened !== "" && waiting.classed === opened) {
            waiting.opened = id;
            root.letGoWaitedFor(waiting);
        }
    }
    // Lets go the place kept for a viewer the walk opened to wait for its
    // host, once its window has come: kept, viewKept would open a second
    // one by itself once the host serves it, as on 20 November.
    function letGoWaitedFor(waiting) {
        if (waiting.letGo !== true || keptPlaces[waiting.id] === undefined)
            return;
        const kept = Object.assign({}, keptPlaces);
        delete kept[waiting.id];
        keepPlaces(kept);
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
    // niri and the moves after. One with an orchestrator goes home to it
    // first, whatever its kept place says (Joel, 2026-11-13 16:32: "agent
    // sessions only land on the workspace of their orchestrator"): its
    // home is the address Places.homeOf reads, and that workspace is the
    // one named for it, else the one its orchestrator's window sits on,
    // found by who it is rather than by any number a boot gave it; one
    // going there is `withKin`, so it's placed there after, never moved
    // back to a place kept on a workspace its orchestrator has left. A
    // home not open yet, as when a session comes back before its
    // orchestrator, is made on the screen's empty workspace and named
    // for it, so the orchestrator and its sessions gather there whichever
    // comes first. With no home: where it was kept, as the column whereBack
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
    // from its kitty's yet, gives none, and is moved after. One with
    // neither a kept place nor kin opens on the screen's empty workspace,
    // never on the one Joel is on.
    function opensOn(one, at, beside, home) {
        const others = root.inUse(root.running).filter(session => session.here
            && (session.window !== null || session.kitty !== null)
            && session.presence.session !== one.id);
        const address = home ?? Places.homeOf(one, null);
        const homed = root.homeSpace(one, others, address);
        // One put back at its kept place, not sent home (home ""), still
        // names its workspace for its home should it lead there.
        const naming = home === "" ? Places.homeOf(one, null) : address;
        if (homed !== null)
            return homed;
        // An orchestrator back before its home is named names the workspace
        // it opens on, so its sessions back before its pi says who it is
        // join it rather than make a workspace of their own (homeSpace).
        const into = root.keptOrKin(one, at, beside, others);
        const name = into === null ? "" : into.name || Places.homeToName(one, naming,
            into.workspace, Object.values(Niri.workspaces ?? {}));
        if (name !== "")
            Niri.act({ SetWorkspaceName: { name, workspace: { Id: into.workspace } } });
        return into;
    }
    // Where a session with no home to go to opens, as opensOn says: where
    // it was kept, else beside its kin, else on the screen's empty workspace.
    function keptOrKin(one, at, beside, others) {
        if (at !== null) {
            const where = Places.whereBack(at, beside);
            if (!where.make)
                return { workspace: where.join, column: Places.opensAt(where, beside) };
            // One kept on a workspace named for an orchestrator goes back to
            // the one named so now, whatever its number; else to the
            // screen's empty workspace, niri's last, so a workspace kept is
            // never taken for one of Joel's by a number it had last boot,
            // and that one takes the kept name.
            const back = Reopening.namedBack(at, Object.values(Niri.workspaces));
            const named = back?.join !== undefined ? Niri.workspace(back.join) : null;
            const its = Object.values(Niri.workspaces).filter(one => one.output === at.output)
                .sort((a, b) => a.idx - b.idx);
            const empty = root.lastOn(at.output);
            const emptied = empty !== null ? Niri.workspace(empty) : null;
            const there = named ?? emptied ?? its[its.length - 1];
            if (!there)
                return null;
            const columns = Object.values(Niri.windows)
                .filter(one => one.workspace_id === there.id)
                .map(one => one.layout?.pos_in_scrolling_layout?.[0] ?? 0);
            // Only the empty workspace it found takes the kept name, and
            // only unnamed, as homeToName names: the last with windows may
            // be one Joel named. opensOn sends it, so it's sent once.
            const name = Reopening.keptName(back, there, emptied);
            return { workspace: there.id, column: Math.max(0, ...columns) + 1, name };
        }
        const asked = { from: one.from ?? "", quest: one.quest ?? "",
            orchestratedBy: one.orchestratedBy ?? "" };
        const kin = root.kinOf(others, asked);
        if (!kin?.window || !Niri.workspace(kin.window.workspace))
            return root.quietSpace();
        return { workspace: kin.window.workspace, column: null };
    }
    // The workspace a session goes home to, `home` being its orchestrator's
    // address: the one named so, else its orchestrator's, in title order
    // after it, else one made for it. Null with no home, or with one whose
    // orchestrator's window isn't told apart from its kitty's yet, which is
    // placed once it is (placeQuietly). The orchestrator itself goes to the
    // workspace named for it when its sessions came back first, else to
    // its own kept place, which opensOn names for it.
    function homeSpace(one, others, home) {
        if (home === "")
            return null;
        const named = Object.values(Niri.workspaces).find(space => space.name === home);
        if (one.address === home || one.orchestrates === home)
            return named ? { workspace: named.id, column: null, withKin: true } : null;
        const leader = others.find(session => session.presence.orchestrates === home
            || (session.presence.addresses ?? []).includes(home));
        const standing = leader?.window && Niri.workspace(leader.window.workspace)
            ? root.standing(leader) : null;
        const under = others.filter(session => session.window
            && session.presence.orchestratedBy === home).map(root.standing);
        if (standing !== null && (!named || named.id === standing.workspace))
            return Object.assign(Places.underOrchestrator(one.title ?? "", standing, under),
                { withKin: true });
        if (named)
            return { workspace: named.id, column: null, withKin: true };
        if (leader)
            return null;
        const made = root.quietSpace();
        if (made === null)
            return null;
        Niri.act({ SetWorkspaceName: { name: home, workspace: { Id: made.workspace } } });
        return Object.assign(made, { withKin: true });
    }
    // The empty workspace niri keeps last on the screen in use, where a
    // window opens out of Joel's way; null while that's the one he's on.
    function quietSpace() {
        const output = Niri.workspace()?.output ?? null;
        const empty = output !== null ? root.lastOn(output) : null;
        const there = empty !== null ? Niri.workspace(empty) : null;
        return there && !there.is_focused ? { workspace: there.id, column: null } : null;
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
        // One sent to a workspace by its kept name holds there, though
        // that workspace stands on another screen than the one kept.
        const holds = entry.quiet === true && !where.make ? workspace.id === where.join
            : where.make && Reopening.onNamed(entry.at, workspace)
                || Places.holds(where, { workspace, column }, beside);
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
    // holds, or once each is moved a few times (settleTries), so two that
    // can't both be had never move back and forth. Each counts its own,
    // so one that won't go where it's asked is left where it is and the
    // rest are still placed: one counted for all, a window moved seven
    // times to no effect left the two after it on the wrong screen
    // (screens-keep-places, 2026-11-14).
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
    readonly property int settleTries: 3
    function settleNextReopened() {
        const going = reopening;
        // Behind the lock every move would be refused; placeBehind settles
        // them once it's let go.
        if (!going?.settling || !root.placeable())
            return;
        const ordered = going.placed.filter(one => Niri.windows[one.id])
            .sort((a, b) => root.keptOrder(a.at, b.at));
        for (let at = 0; at < ordered.length; at += 1) {
            // A window that has landed stays wherever it is now, so a
            // move Joel or a session makes after it lands sticks (Joel,
            // 2026-11-09: one placer, "and his own moves stick"). It
            // still counts beside the ones after it, by its id, as does
            // one given up on.
            const entry = ordered[at];
            if (entry.landed || (entry.tries ?? 0) >= root.settleTries)
                continue;
            const beside = root.besideOf(ordered.slice(0, at));
            const placed = root.placeSession(entry, beside);
            entry.landed = placed === "held";
            if (placed === "moved") {
                going.rounds += 1;
                entry.tries = (entry.tries ?? 0) + 1;
                if (entry.tries >= root.settleTries)
                    console.info(`sessions: window ${entry.id} is left where it is after `
                        + `${entry.tries} moves`);
                // Sooner once niri says what moved (settleReopenedSoon).
                reopenSettling.interval = 1000;
                reopenSettling.restart();
                return;
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
    // Counts the pace from now, and reports how long after the last this
    // one was begun, by the shell's own clock: the journal stamps each line
    // as it reads it, and on 15 November stamped two begun by this pace
    // 0.98 s apart, the first after the window kitty opened for it.
    function paceFromNow(id) {
        const now = Date.now();
        Report.say(`began ${id} ${now - reopenStartedAt} ms after the last`);
        reopenStartedAt = now;
    }
    // And none is begun while the machine queue says the machine has no
    // room (Reopening.mayBegin), looked at again this often, since eighty
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
        return Reopening.mayBegin(machineQueue.text(), Date.now() / 1000);
    }
    // It asks again rather than taking the next at once, since a timer
    // this long is a coarse one, which may fire up to a twentieth of its
    // interval early: one did, beginning a session 1.99 s after the last.
    Timer {
        id: reopenPacing
        onTriggered: root.reopenSoon()
    }

    // Each session Reopen Them All opened is told to carry on only as the
    // uplink has room (Reopening.carryOnsDue), since the words make it send,
    // not its window: an idle pi sends a fraction of a kbit/s, and on
    // 2026-11-14 seventy-two told as each opened sent 6.4 Mbit/s of the
    // home line's 8.2 for 17 minutes, past the builders sharing it. Every
    // window still opens as it did. Each is told through its steer inbox
    // once its pi says it runs, and one that never does within
    // carryOnPatience is let go and said. The holder gives a reopened pi
    // its turn four at a time, a turn lasting up to a minute, so seventy-
    // two can take eighteen minutes to run: half an hour lets each come.
    // The uplink's rate lags the window it's measured over, so about one
    // a look goes before a climb shows; their first requests are small
    // beside the line.
    property var carryingOn: ({ pending: [], told: [] })
    readonly property int carryOnPatience: 1800000
    // A session told counts as working this long, in seconds, before its
    // pi idle says it's done: one just told hasn't begun its turn yet.
    readonly property int carryOnSettles: 15
    function pendingOf(one, kept) {
        return { id: one.id, carried: one.carried, leads: kept?.leads === true,
            opened: Date.now() };
    }
    // Whether `one` may be told as it opens: none waits ahead of it, and
    // the uplink has room for one more now.
    function carryOnNow(one, kept) {
        if (carryingOn.pending.length > 0)
            return false;
        const now = Date.now() / 1000;
        // Those told an hour ago are long past counting.
        carryingOn.told = carryingOn.told.filter(told => now - told.at < 3600);
        machineQueue.reload();
        const due = Reopening.carryOnsDue([pendingOf(one, kept)], toldSoFar(now),
            machineQueue.text(), now);
        if (due.length === 0)
            return false;
        carryingOn.told.push({ id: one.id, at: now });
        return true;
    }
    function carryOnLater(one, kept) {
        carryingOn.pending.push(pendingOf(one, kept));
        carryOnSoon();
    }
    function runsHere(id) {
        return running.find(one => one.here && one.presence?.session === id);
    }
    // Those told so far, each idle once its pi has settled and isn't busy.
    function toldSoFar(now) {
        return carryingOn.told.map(one => ({ id: one.id, at: one.at,
            idle: now - one.at > carryOnSettles && !!runsHere(one.id)
                && !runsHere(one.id).presence?.busy }));
    }
    function carryOnSoon() {
        const now = Date.now();
        const runs = runsHere;
        const lost = carryingOn.pending.filter(one => !runs(one.id)
            && now - one.opened > carryOnPatience);
        lost.forEach(one => console.warn(`sessions: ${one.id} never ran, so isn't told to carry on`));
        carryingOn.pending = carryingOn.pending.filter(one => !lost.includes(one));
        const ready = carryingOn.pending.filter(one => runs(one.id));
        const told = toldSoFar(now / 1000);
        machineQueue.reload();
        const asked = ready.map(one => Object.assign({}, one,
            { wants: wanting.some(session => session.presence?.session === one.id) }));
        const due = Reopening.carryOnsDue(asked, told, machineQueue.text(), now / 1000);
        for (const one of due) {
            steer(runs(one.id).address, { text: carryOn, id: `carry-on-${now}-${one.id}` });
            carryingOn.told.push({ id: one.id, at: now / 1000 });
        }
        const telling = due.map(one => one.id);
        carryingOn.pending = carryingOn.pending.filter(one => !telling.includes(one.id));
        if (carryingOn.pending.length > 0) {
            if (due.length === 0 && ready.length > 0 && !carryingOn.heldForRoom)
                Report.say(`carrying on waits for the uplink, ${carryingOn.pending.length} to go`);
            carryingOn.heldForRoom = due.length === 0 && ready.length > 0;
            carryOnPacing.restart();
        } else {
            carryingOn.heldForRoom = false;
        }
    }
    Timer {
        id: carryOnPacing
        interval: root.roomPace
        onTriggered: root.carryOnSoon()
    }

    // Reopens every session the last boot took away, each in its own
    // folder, back where it sat: those held on another machine joined
    // again at once, then this machine's one at a time, a screen and its
    // workspaces in the order niri numbers them and each column after the
    // last, each placed beside those back before it (placeOpened). Each
    // waits for niri to open the one before, or half a minute if it never
    // does, and for two seconds after it began (reopenPace). One with
    // nowhere kept opens beside where Joel was, after the rest, and Joel
    // is taken back there at the end. One whose folder is gone starts its
    // quest afresh where a new session on it starts (reopenAfresh), and
    // `gone(one, cwd)` is told of one with no quest or no such folder,
    // which isn't opened. `keptAt(id)`
    // says where one sat: by default its place this boot while it has one,
    // else the restored boot's (Places.keptFor), so one pi's host gave back
    // this boot opens where its viewer sat, not where the boot before left
    // it (item 11's rollback drill, 2026-11-11). A list given while another
    // is being reopened joins its end, so two never open at once and
    // misplace each other.
    // `told`, when given, is each one's first prompt as it opens
    // (reopened), as Reopen Them All tells each to carry on.
    property var reopening: null
    function reopenAll(lost, gone, keptAt, told) {
        const boots = { now: arrangement[bootId], restored: arrangement[restoring.boot] };
        const kept = keptAt ?? (id => Places.keptFor(id, boots));
        // The other windows its place's boot kept at once, which say which
        // alike screens were two (Places.js's twinsOf).
        const moments = { now: arrangedOthers[bootId], restored: arrangedOthers[restoring.boot] };
        const momentOf = id => keptAt ? [] : moments[Places.keptFrom(id, boots)];
        // Whether the boot being restored kept its places at all, so one
        // kept nowhere had no window rather than went unrecorded
        // (Reopening.broughtBack).
        const arranged = keptAt !== undefined
            || Object.keys(boots.restored ?? {}).length > 0;
        lost.filter(one => one.machine !== undefined).forEach(one => rejoin(one, true));
        const here = lost.filter(one => one.machine === undefined);
        lookAtScreens(() => reopenHere(here, gone, kept, told ?? "", momentOf, arranged));
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
    function reopenHere(here, gone, kept, told, momentOf, arranged) {
        const order = (a, b) => !a.at || !b.at ? Number(!a.at) - Number(!b.at)
            : root.keptOrder(a.at, b.at);
        const keptPlaces = here.map(one => kept(one.id));
        const placeable = keptPlaces.map(Reopening.placeOf);
        const inUse = Niri.workspace()?.output ?? null;
        // Only Reopen Them All tells each to carry on, which marks its walk.
        const walk = told === root.carryOn;
        const places = Places.placed(placeable, root.screensThere(), inUse,
            here.map(one => momentOf(one.id)).map(moment => Array.isArray(moment) ? moment : []));
        // Its windows go back by the screen in use as its own sessions are
        // placed, and to the twin connectors they're given, not as a
        // hand-over meanwhile places others, and what windows back before
        // took where the sessions' say nothing.
        if (puttingBack !== null && walk && !("inUse" in puttingBack)) {
            puttingBack.inUse = inUse;
            puttingBack.given = Reopening.givenBy(placeable, places, puttingBack.given);
        }
        const queue = Reopening.leadersFirst(here.map((one, at) => ({ one, at: places[at],
            kept: placeable[at], unplaced: keptPlaces[at]?.unplaced === true, gone, told,
            back: Reopening.broughtBack(walk, keptPlaces[at], arranged === true,
                Reopening.satHere(one)) }))
            .sort(order));
        if (reopening !== null) {
            reopening.queue = Reopening.queuedOnce(reopening.queue, queue, reopening.begun);
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
    // its place kept this boot or else the restored boot's (reopenAll), else
    // beside where Joel is. It answers "taken" at once, so the harness knows the
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
                // The workspace it goes home to, named for the orchestrator
                // over its quest, else its quest (Places.homeOf).
                workspace: typeof one.workspace === "string"
                    && /^[A-Z0-9]+$/.test(one.workspace) ? one.workspace : "",
                title: typeof one.title === "string" ? one.title : "",
                // The model a restart asks for (the harness's quest spawn), passed
                // to pi as it's said, and dropped when it's anything but a name.
                model: typeof one.model === "string" && /^[A-Za-z0-9._:/-]+$/.test(one.model)
                    ? one.model : "",
                // What it was started with, passed on as reopened says.
                env: one.env,
                quiet: one.focus !== true,
                focused: one.focus === true }));
        if (given.length === 0)
            return "refused: no session with an id and a folder";
        console.info(`sessions: reopening ${given.length} handed over`);
        const gone = (one, cwd) => console.info(`sessions: ${one.id} not reopened, `
            + `since its folder, ${cwd}, is gone`);
        const now = given.filter(one => !one.fresh);
        if (now.length > 0)
            reopenAll(now, gone);
        given.filter(one => one.fresh).forEach(one => deciding.createObject(root, { one, gone }));
        return given.length === listed.length ? "taken"
            : `taken ${given.length} of ${listed.length}`;
    }

    // What asks whether a session the harness starts afresh runs in one of
    // pi's hosts rather than a window of its own (2QZBX9's spawn-decide,
    // harness #1406), given `--session-id` and `--model` after it. It runs
    // through Joel's login shell, as a terminal would, so node is found.
    property var decider: ["zsh", "-lic", 'exec node --no-warnings "$0" "$@"',
        `${home}/.local/share/pi-packages/github.com/Jitsusama/agentic-harness.pi/current`
        + "/scripts/spawn-decide.ts"]
    // How long a decider has to say it's opening one in a host before the
    // window opens as it always did, so a decider that hangs, or isn't
    // installed, costs a spawn three seconds at most (2QZBX9, [00cc6a34]).
    readonly property int decidesWithin: 3000

    // Whether the harness's config says sessions Joel starts run in pi's
    // host: its quest-workflow.startHost, "attended" for it, under sections,
    // where the harness keeps each extension's settings
    // (lib/internal/config/loader.ts; 85PKB2, 2026-11-19 [ff8188f0]). Said
    // nothing of, a start opens its window at once, never asking, since
    // asking costs every start about 550 ms (5KBYD7: 544, 615 and 552 ms,
    // and 856 to orchestrate).
    readonly property bool startsHosted: {
        try {
            const said = JSON.parse(harnessConfig.text() || "{}");
            return said?.sections?.["quest-workflow"]?.startHost === "attended";
        } catch (error) {
            return false;
        }
    }
    FileView {
        id: harnessConfig
        path: `${Quickshell.env("XDG_CONFIG_HOME") || `${root.home}/.config`}`
            + "/pi/agentic-harness.pi/config.json"
        blockLoading: true
        printErrors: false
        // Read again whenever it's written, as Joel's settings change it.
        watchChanges: true
        onFileChanged: reload()
    }

    // One start on a quest asked of the decider, in its folder, as the
    // harness's contract says: `hosted <host> <session>` last, on exit 0,
    // opens a viewer of it in the host, focused, and no window of pi's;
    // kept out (exit 3), failing, or no word of opening in time, its window
    // opens as it always did, told why where the decider says. Once it's
    // said it's opening, no window opens unless it fails, however long the
    // host takes.
    Component {
        id: startAsked

        Process {
            id: begun

            required property string quest
            required property string folder
            required property bool orchestrates
            property bool opening: false
            property bool settled: false
            property string last: ""

            function settle(hosted) {
                if (settled)
                    return;
                settled = true;
                const [said, host, id] = last.split(" ");
                const started = hosted && said === "hosted";
                // A host's name begins with a letter or digit, so it can't
                // pass for one of mycelium's flags.
                const viewer = started && /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(host ?? "")
                    ? root.viewerNamed(id ?? "") : "";
                if (started && viewer === "") {
                    // Its host runs it, so a window here would begin a
                    // second pi on its quest; with no viewer it can name
                    // and attach, it's said started, not viewed.
                    console.info(`sessions: started ${quest} in pi's host ${host ?? "unnamed"}, `
                        + `as ${id ?? "no id"}, but it can't be viewed by that host and id`);
                    Report.say(`started a session ${orchestrates ? "orchestrating" : "on"} ${quest}`
                        + " in pi's host");
                    root.unhosted({ address: quest, machine: "" }, "unviewable");
                    return;
                }
                if (started) {
                    // One listed already is raised, and one still coming
                    // is said (heldBack); otherwise it opens as any viewer
                    // does (viewWith), titled for its quest and folder
                    // until pi titles it, rather than a bare attach whose
                    // window reads "mycelium".
                    const listed = root.listedNamed(viewer);
                    if (listed)
                        Niri.act({ FocusWindow: { id: listed.id } });
                    else if (!root.heldBack(id, viewer, { address: quest, machine: "" }))
                        root.viewWith(["mycelium", "--realm", host.replace(/-attended$/, ""),
                            "attach", "--host", host, "--session", id],
                            viewer, null, Viewers.viewerTitle("", quest, folder));
                    console.info(`sessions: started ${quest} in pi's host ${host}, as ${id}`
                        + (listed ? ", its viewer already open" : ""));
                    Report.say(`started a session ${orchestrates ? "orchestrating" : "on"} ${quest}`
                        + " in pi's host");
                    return;
                }
                if (last.startsWith("quest-workflow.startHost ")) {
                    console.info(`sessions: ${quest} starts in a window, since ${last}`);
                    Report.say(last);
                }
                root.startWindow(quest, folder, orchestrates);
            }

            workingDirectory: folder
            command: ["env", `QUEST_WORKFLOW_AUTOLOAD_ID=${quest}`].concat(root.decider,
                ["--start"], orchestrates ? ["--orchestrate"] : [])
            running: true
            stdout: SplitParser {
                onRead: line => {
                    begun.last = line.trim();
                    if (begun.last.startsWith("opening "))
                        begun.opening = true;
                }
            }
            onExited: code => {
                begun.settle(code === 0);
                begun.destroy();
            }

            property Timer patience: Timer {
                running: true
                interval: root.decidesWithin
                onTriggered: {
                    if (begun.opening || begun.settled)
                        return;
                    begun.settle(false);
                    // Ended by the terminate signal, so it opens nothing.
                    begun.running = false;
                }
            }
        }
    }

    // One fresh session's decision, in its folder with what it starts with.
    // Hosted, it opens nothing, since its host runs it; kept out (exit 3),
    // it opens told why on its last line; anything else, or no word of
    // opening in time, opens it as before. Once it's said it's opening,
    // no window opens unless it fails, however long the host takes.
    Component {
        id: deciding

        Process {
            id: decision

            required property var one
            required property var gone
            property bool opening: false
            property bool settled: false
            property string last: ""

            function settle(way) {
                if (settled)
                    return;
                settled = true;
                if (way === "hosted") {
                    console.info(`sessions: ${one.id} started in pi's host, ${last}`);
                    Report.say(`started session ${one.id} in the background`);
                    return;
                }
                // Kept out with no reason after its word of opening says none.
                const why = last.startsWith("opening ") ? "" : last;
                const stayed = way === "stayed" ? { stayed: why } : {};
                console.info(`sessions: ${one.id} opens in a window, since its decider `
                    + (way === "stayed" ? `kept it out of a host: "${why}"`
                        : way === "late" ? "said nothing in time" : "failed"));
                root.reopenAll([Object.assign({}, one, stayed)], gone);
            }

            workingDirectory: one.cwd
            command: {
                const quest = one.quest !== "" ? [`QUEST_WORKFLOW_AUTOLOAD_ID=${one.quest}`] : [];
                const model = one.model !== "" ? ["--model", one.model] : [];
                return ["env"].concat(quest, root.startedWith(one), root.decider,
                    ["--session-id", one.id], model);
            }
            running: true
            stdout: SplitParser {
                onRead: line => {
                    decision.last = line.trim();
                    if (decision.last.startsWith("opening "))
                        decision.opening = true;
                }
            }
            onExited: code => {
                if (code === 0 && last.startsWith("hosted "))
                    settle("hosted");
                else
                    settle(code === 3 ? "stayed" : "failed");
                decision.destroy();
            }

            property Timer patience: Timer {
                running: true
                interval: root.decidesWithin
                onTriggered: {
                    if (decision.opening || decision.settled)
                        return;
                    decision.settle("late");
                    // Ended by the terminate signal, so it opens nothing.
                    decision.running = false;
                }
            }
        }
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
        if (next.one.hosted === true) {
            reopenViewer(next);
            return;
        }
        // One already running here is never begun again, as a restart
        // watch may have begun it meanwhile, since two pis on one log would
        // each write over the other; but a hosted presence is believed only
        // while its host's own list names it, and one parked here is woken
        // rather than left asleep (Places.foundHere).
        const found = Places.foundHere(next.one.id, root.running, root.parked,
            owner => Places.hostSays(owner, root.hostList(owner)));
        if (found.as === "parked") {
            root.wakeReopened(found.session, next);
            reopenNext();
            return;
        }
        if (found.as === "running") {
            console.info(`sessions: ${next.one.id} not reopened, since it runs here`);
            reopenNext();
            return;
        }
        if (Reopening.startedHeldLately(root.startedHeld, next.one.id, Date.now())) {
            console.info(`sessions: ${next.one.id} not reopened, since it was started `
                + "without a window moments ago");
            reopenNext();
            return;
        }
        if (found.as === "stale")
            console.info(`sessions: ${next.one.id}'s hosted presence is stale, since `
                + `${found.session.presence.owner.host} doesn't list it, so it's reopened`);
        reopening.begun.push(next.one.id);
        reopening.looking = true;
        lookFor([next.one.cwd], there => {
            reopening.looking = false;
            if (!there[0]) {
                root.reopenAfresh(next);
                return;
            }
            // One that had no window as the boot ended comes back without
            // one, held, and the next is begun at the pace; with no realm
            // to hold it in, it opens as any other.
            if (next.back?.window === false && root.holds) {
                const waits = next.told === root.carryOn
                    && !root.carryOnNow(next.one, next.kept);
                console.info(`sessions: reopening ${next.one.id} without a window, `
                    + "since the boot kept none for it");
                root.reopenedHeld(next.one, next.one.cwd, waits ? "" : next.told);
                if (waits)
                    root.carryOnLater(next.one, next.kept);
                paceFromNow(next.one.id);
                root.reopenSoon();
                return;
            }
            // Placed by its connector and the number it's given now, so it
            // goes beside those given the same screen, a gone one's too.
            // Its workspace's name too, which says where it goes better
            // than the number (Reopening.namedBack).
            const at = Reopening.atOf(next.at);
            // It opens where it goes, its orchestrator's workspace first
            // (opensOn), and never where Joel is unless he asked for it by
            // itself, handed over to be focused with no home and no place.
            // Only with no workspace to open on is where it goes focused
            // first, so it opens there and is seldom seen to move.
            // Reopen Them All puts one back at its kept place, ahead of
            // its orchestrator's workspace (Reopening.broughtBack).
            const quiet = next.one.quiet === true;
            const home = next.back?.home === false ? "" : Places.homeOf(next.one, next.kept);
            const asked = next.one.focused === true && at === null && home === "";
            const into = asked ? null
                : root.opensOn(next.one, at, root.besideOf(reopening.placed), home);
            // One going home is placed beside its kin there (placeQuietly),
            // never moved back to a place kept elsewhere.
            const homed = into?.withKin === true;
            const focuses = !quiet && !homed && into === null;
            console.info(`sessions: reopening ${next.one.id}, kept at ${root.keptSaid(at)}, `
                + (home !== "" ? `home ${home}, ` : "")
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
            reopening.waiting = { classed: null, at: homed ? null : at, opened: null,
                said: false, away: next.at?.away ?? null, kept: next.kept ?? null,
                quiet: quiet || homed, quest: next.one.quest ?? "", from: next.one.from ?? "",
                id: next.one.id,
                orchestratedBy: home !== "" ? home : next.one.orchestratedBy ?? "" };
            // The pace is counted from once it's begun and said, so the next
            // is said two seconds after however long this one took to start.
            // Its window's app id, which niri says as it opens, after this.
            // Reopen Them All's carry-on is said as it opens while the
            // uplink has room, else it opens untold and waits its turn
            // (carryOnSoon).
            const waits = next.told === root.carryOn && !root.carryOnNow(next.one, next.kept);
            reopening.waiting.classed = reopened(next.one, next.one.cwd,
                waits ? "" : next.told, into);
            if (waits)
                root.carryOnLater(next.one, next.kept);
            paceFromNow(next.one.id);
            reopenPatience.restart();
        });
    }
    // A hosted session's turn in the walk: its host brought the
    // conversation back, so only its viewer opens, never its pi, at the
    // place the boot kept, quietly, as any other reopened window does: it
    // goes in the one walk, waited for and placed as theirs are, so the
    // columns after it land beside it (Y2YBTS and KP0L6A, [342c50d4]). One
    // whose viewer is open, or that the boot kept no place of, opens none;
    // one it kept as unplaced, floating or untold, had a window, so it
    // opens going home, as a pi kept so does.
    // One whose host isn't serving it yet, as after a crash, while the host
    // comes back, opens there all the same by the words its restore entry
    // says (Viewers.waitingWords), and waits in the window for its host:
    // kept for later instead, it opened by itself nearly two hours after
    // the walk on 20 November, where Joel had moved on. One whose entry
    // says no words, as an older harness's, or one bound for the workspace
    // Joel is on, where a window would catch his eye, has its place kept
    // for viewKept, and the walk goes on: one with words opens once Joel
    // leaves that workspace, within ASKED_FOR, one without once the host
    // serves it and Joel is away. The walk itself opens with Joel there,
    // and wakes one parked in its host, since it runs only when asked: his
    // Reopen Them All, or `sessions reopen`, and reopening a session
    // carries it on. Kept back, a parked one would stay offered, and each
    // further Reopen Them All would do nothing he could see (KP0L6A on
    // #3363); only viewKept, which opens by itself, leaves it asleep.
    function reopenViewer(next) {
        const id = next.one.id;
        const session = root.hostedHere(id);
        const at = Reopening.atOf(next.at);
        const opens = at !== null || next.unplaced === true
            ? root.opensOn(next.one, at, root.besideOf(reopening.placed),
                next.back?.home === false ? "" : undefined)
            : null;
        // One kept as unplaced with nowhere quiet to open, as while Joel is
        // on the screen's empty workspace, is bound for his, so it waits for
        // him to leave (joelThere), as a pi kept so opens beside him.
        const joels = Niri.workspace();
        const into = opens ?? (next.unplaced === true && joels
            ? { workspace: joels.id, column: null } : null);
        // Open only as niri lists it, where its list is trusted, so one
        // the windows daemon's index alone still names isn't taken as open.
        const open = root.listedNow(session?.window);
        if (open || into === null) {
            console.info(`sessions: ${id}'s viewer not opened, since `
                + (open ? "it is open" : next.unplaced === true
                    ? "it has nowhere to open" : `none was kept at ${root.keptSaid(at)}`));
            reopenNext();
            return;
        }
        const served = session !== null && Array.isArray(session.attach)
            && session.attach.length > 0 && viewerOf(session.presence) !== "";
        const named = served ? viewerOf(session.presence) : viewerNamed(id);
        // A viewer an earlier walk opened to wait for its host is found by
        // its app id alone, since no record runs for it yet, and the row
        // stays offered until the host serves it: asked again, the walk
        // would open a second one beside it.
        if (!served && root.openNamed(named)) {
            console.info(`sessions: ${id}'s viewer not opened, since it is open, waiting`);
            reopenNext();
            return;
        }
        const words = named === "" ? null : Viewers.waitingWords(next.one);
        const waiting = served ? null : words;
        const title = Viewers.viewerTitle("", next.one.title, next.one.cwd);
        const joelThere = Niri.workspaces[into.workspace]?.is_focused === true;
        if ((!served && waiting === null) || joelThere) {
            // One kept for Joel to leave is marked asked, served or not, so
            // viewKept opens it once he leaves, by its session's words or
            // its own, rather than once he goes away, perhaps hours later;
            // and there, so the time an ask lasts runs only once he has.
            const place = { workspace: into.workspace, column: into.column, at: Date.now() };
            const marks = !joelThere ? {} : Object.assign({ asked: true, there: true },
                words === null ? {} : { waiting: words, named, title });
            keepPlaces(Object.assign({}, keptPlaces, { [id]: Object.assign(place, marks) }));
            console.info(`sessions: ${id}'s viewer waits for `
                + `${joelThere ? "Joel to leave" : "its host"}, kept at `
                + `${root.spaceSaid(into.workspace)} column ${into.column ?? "last"}`);
            reopenNext();
            return;
        }
        reopening.begun.push(id);
        const asked = served ? view(session, into)
            : viewWith(waiting, named, into, title);
        // One asked of moments ago, by Go To or viewKept, is opening it:
        // that window is the one niri lists, so the walk doesn't wait on it.
        if (!asked) {
            console.info(`sessions: ${id}'s viewer not opened, since niri lists it open `
                + "or one asked for is opening");
            reopenNext();
            return;
        }
        console.info(`sessions: reopening ${id}'s viewer${served ? "" : " to wait for its host"}, `
            + `kept at ${root.keptSaid(at)}, `
            + `asked on ${root.spaceSaid(into.workspace)} column ${into.column ?? "last"}`);
        // A place kept for it before, by a walk Joel was there for or by
        // the cutover, is let go once its window comes (letGoWaitedFor),
        // not as it's asked for: one never come would leave nothing to
        // open it.
        reopening.waiting = { classed: named, at, opened: null, said: false, letGo: !served,
            away: next.at?.away ?? null, kept: next.kept ?? null, quiet: true,
            quest: next.one.quest ?? "", from: next.one.from ?? "", id,
            orchestratedBy: next.one.orchestratedBy ?? "" };
        paceFromNow(id);
        reopenPatience.restart();
    }
    // One parked here, in its holder or in pi's host, which Reopen Them All
    // wakes rather than begins again: told to carry on as it wakes when
    // `next` is told that and the uplink has room, else woken and told
    // once it runs (carryOnLater). Its place is kept as a sleeper's
    // (keepPlaceAsleep), marked asked, since Joel asked for its window, so
    // it opens there as it wakes (Viewers.viewersDue, within ASKED_FOR);
    // where one was kept as it went to sleep, that one is marked asked. Left
    // asleep, 2RCBOJ's parked hold stayed shut after the drill of 15
    // November until it was woken by hand (OSPHT4, [58d7fa8e]).
    function wakeReopened(session, next) {
        const id = next.one.id;
        const at = Reopening.atOf(next.at);
        const slept = keptPlaces[id]?.slept === true ? keptPlaces[id] : null;
        const into = slept ? null
            : root.opensOn(next.one, at, root.besideOf(reopening.placed),
                next.back?.home === false ? "" : undefined);
        if (slept)
            keepPlaces(Object.assign({}, keptPlaces, { [id]: Object.assign({}, slept,
                { at: Date.now(), asleep: true, asked: true }) }));
        else if (into !== null)
            keepPlaces(Object.assign({}, keptPlaces, { [id]: { workspace: into.workspace,
                column: into.column, at: Date.now(), slept: true, asleep: true,
                asked: true } }));
        console.info(`sessions: waking ${id}, parked here, kept at ${root.keptSaid(at)}`);
        const telling = next.told === root.carryOn;
        if (telling && !wokenByViewing(session) && carryOnNow(next.one, next.kept)) {
            steer(session.address, { text: carryOn, id: `carry-on-${Date.now()}-${id}` });
            return;
        }
        wake(session);
        if (telling)
            carryOnLater(next.one, next.kept);
    }

    // A pi host's own list of the sessions it runs, as the harness keeps it
    // in its hosts folder, read as it is now for the host `owner` names
    // (Places.hostListOf), or "" for none. Reopen Them All reads it before
    // it believes a hosted presence, since one can outlive its session
    // while its owner, the host's process, runs on.
    Component {
        id: hostListing

        FileView {
            blockAllReads: true
            printErrors: false
        }
    }
    function hostList(owner) {
        const name = Places.hostListOf(owner);
        if (name === "")
            return "";
        const file = hostListing.createObject(root, { path: `${state}/hosts/${name}` }) as FileView;
        const text = file.text();
        file.destroy();
        return text;
    }

    // One whose folder is gone starts its quest afresh in the folder the
    // harness says a new session on it starts in, and is walked again as
    // that, so it lands where the one it stands in for sat; only with no
    // quest, or no start there, is `gone` told and it left shut (OSPHT4's
    // rollback drill, 2026-11-15 [58d7fa8e], where two stayed shut so).
    // pi resumes a session only in its own folder, asking first in any
    // other, so it starts under an id of its own (Places.afresh).
    function reopenAfresh(next) {
        const quest = next.one.quest ?? "";
        const start = quest === "" || next.one.afreshFor !== undefined ? ""
            : Places.questStart(root.questIndex(), quest);
        if (start === "" || start === next.one.cwd) {
            next.gone?.(next.one, next.one.cwd);
            reopenNext();
            return;
        }
        reopening.looking = true;
        lookFor([start], there => {
            reopening.looking = false;
            if (!there[0]) {
                next.gone?.(next.one, next.one.cwd);
                reopenNext();
                return;
            }
            const id = Places.freshId(Date.now(), Math.random);
            console.info(`sessions: ${next.one.id}'s folder, ${next.one.cwd}, is gone, so `
                + `${quest} starts afresh in ${start}, as ${id}`);
            reopening.queue = [Object.assign({}, next, { one: Places.afresh(next.one, start, id) })]
                .concat(reopening.queue);
            reopenNext();
        });
    }
    // The harness's index of quests, read as it is now, or "" for none.
    // It's big and rewritten often, so it's read, as a host's list is, only
    // as a folder is found gone, never on a timer or a frame.
    function questIndex() {
        const file = hostListing.createObject(root, {
            path: `${state}/quest-workflow/quests.json` }) as FileView;
        const text = file.text();
        file.destroy();
        return text;
    }

    // The session pi's host runs here as `id`, or null.
    function hostedHere(id) {
        return running.find(one => one.here && one.presence?.hosted
            && one.presence.session === id) ?? null;
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
    // screen it took it on, so without this they'd stay there. Each keeps
    // the other screens as it left (`others`, Places.js's othersAt), said by
    // whoever saw it leave, since a reopen notes one minutes after.
    property var awayFrom: ({})
    function noteAway(away, id) {
        if (!away || id === null || id === undefined)
            return;
        const kept = awayFrom[away.screen] ?? [];
        if (kept.some(one => one.id === id))
            return;
        const next = Object.assign({}, awayFrom);
        next[away.screen] = kept.concat([{ id, workspace: away.workspace, output: away.output,
            others: away.others ?? {} }]);
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
    // Says each workspace comeHome holds back once, as it starts to wait,
    // so a screen test that fails shows whether the shell waited.
    property var heldSaid: []
    function sayHeld(waiting) {
        // By the connector it left as well as the screen it waits for,
        // since twins wait for one screen, and one noted on the screen in
        // use before that was heard waits for one that may be it.
        const held = Object.entries(waiting).reduce((all, [screen, kept]) =>
            all.concat(kept.map(one => ({ id: one.id, screen, left: one.output }))), []);
        held.filter(one => !heldSaid.includes(one.id)).forEach(one => console.info(
            `workspace ${one.id} left ${one.left || "a screen"}, `
            + `waits for ${one.screen || "its screen"}`));
        heldSaid = held.map(one => one.id);
    }
    function noteGone() {
        const now = Object.values(Niri.workspaces);
        const outputs = now.map(one => one.output ?? "");
        const noted = Object.values(root.awayFrom)
            .reduce((all, kept) => all.concat(kept.map(one => one.id)), []);
        // Only connectors there as niri last said its workspaces are other
        // screens, since one that comes in the same moment may be the one
        // going, on another connector, and those gone just now are siblings.
        const lefts = root.outputsWere.filter(output => !outputs.includes(output));
        const stayed = outputs.filter(output => root.outputsWere.includes(output));
        const next = {};
        for (const one of now) {
            const was = root.workspacesWere[one.id];
            // One first seen on no screen left none, so it waits for none.
            const left = was !== undefined && was.output !== "" && !outputs.includes(was.output);
            if (left && was.output !== (one.output ?? "") && !noted.includes(one.id))
                root.noteAway({ screen: root.screens[was.output] || was.output, workspace: was.idx,
                    output: was.output,
                    others: Places.othersAt(was.output, stayed, lefts, root.screens) }, one.id);
            next[one.id] = left ? was : { output: one.output ?? "", idx: one.idx };
        }
        root.workspacesWere = next;
        root.outputsWere = [...new Set(outputs)];
    }
    property var outputsWere: []
    function comeHome() {
        if (root.placingNow() || Object.keys(awayFrom).length === 0)
            return;
        const going = Places.homeward(awayFrom, screens, Object.values(Niri.workspaces));
        root.sayHeld(going.waiting);
        if (JSON.stringify(going.waiting) !== JSON.stringify(awayFrom))
            awayFrom = going.waiting;
        going.moves.forEach(move => console.info(move.there === true
            ? `workspace ${move.id} is back on ${move.output}`
            : `workspace ${move.id} goes back to ${move.output}`));
        const back = Niri.current;
        const moving = going.moves.filter(move => move.there !== true);
        // Their order is the windows daemon's to put back, as Joel last had
        // it, since the numbers kept here are from before he moved any while
        // the screen was away (2026-11-11, modules/windows/src/order.rs).
        moving.forEach(move => Niri.act({ MoveWorkspaceToMonitor: { output: move.output,
            reference: { Id: move.id } } }));
        // Each is where it was sent, or found, now, so niri moving it there
        // isn't its screen going again (noteGone), as it was when it went
        // to an alike screen on another connector than the one it left.
        if (going.moves.length > 0) {
            const were = Object.assign({}, root.workspacesWere);
            going.moves.forEach(move => {
                were[move.id] = { output: move.output, idx: Niri.workspace(move.id)?.idx ?? 1 };
            });
            root.workspacesWere = were;
        }
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
    // it this boot or in the boot being restored (reopenAll).
    function reopenThemAll(lost, gone, keptAt) {
        const boot = root.arrangedBefore();
        const kept = Array.isArray(arrangedOthers[boot]) ? arrangedOthers[boot] : [];
        puttingBack = kept.length > 0
            ? { boot, kept, used: [], back: [], seen: [], given: [] } : null;
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
        // Not beside a twin monitor's windows, though they share its screen.
        const found = Places.whereBack(at, beside, Places.twinsOf(at, going.kept));
        const there = root.screensThere();
        const inUse = Reopening.inUseFor(going.inUse, Niri.workspace()?.output ?? null, there);
        // Each window put back gives its twins' connectors, after its
        // sessions', so twin monitors on new connectors take one each.
        const kept = Places.whereKept(at, going.kept, there, { given: going.given, inUse });
        going.given.push({ at, output: kept.output });
        const output = found.output ?? kept.output ?? inUse;
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
        if (into !== null && kept.away !== null)
            root.noteAway(kept.away, into);
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
    // what Go To raises next time; with no words it tells `unhosted`, `why`
    // "hosted", or "unnamed" for an id that can't name a viewer's window.
    // A row's Reopen or a hosted start tells it too, why naming the one
    // that held: "hostless", no host serves it and its entry gives no
    // words to wait with; "opening", its viewer was asked for moments ago
    // (heldBack); "unviewable", started in pi's host under a host or id
    // it can't view by. Each is said in the launcher's foot, since a press
    // is never met with nothing.
    signal unhosted(var session, string why)
    // A window Go To opens goes where the session would reopen (goesTo),
    // never on whichever workspace Joel is on.
    function goTo(session) {
        if (session.presence?.hosted && !root.listedNow(session.window)) {
            // Its viewer niri lists is raised, never a second
            // (listedNamed); one whose window niri no longer lists is
            // viewed afresh. Each refused is said before any place is
            // worked out, which can name a workspace, in the order a row's
            // Reopen says it: one whose viewer can't be named, rather than
            // raising any window niri lists with no app id, one with no
            // words to view it by, then one held back (heldBack).
            const named = viewerOf(session.presence);
            const listed = listedNamed(named);
            const words = session.attach;
            if (listed)
                Niri.act({ FocusWindow: { id: listed.id } });
            else if (named === "" || !Array.isArray(words) || words.length === 0)
                unhosted(session, named === "" ? "unnamed" : "hosted");
            else if (!heldBack(session.address, named, session))
                view(session, root.goesTo(session));
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
            attach(session.held, root.goesTo(session));
        else
            unopened(session);
    }
    // A session here in no window, kitty or holder the shell can reach, as
    // pi run in another terminal is, which Go To can't open: said, never
    // a press that does nothing.
    signal unopened(var session)

    // Where Go To opens a new window of a session, focused, since Joel asked
    // for it: home to its orchestrator's workspace, else beside its kin,
    // else on the screen's empty workspace, as one reopened goes (opensOn).
    // A session held on another machine has no window here for the rule
    // that gathers sessions to find, so one joined where niri opens it
    // stayed on the workspace Joel was on (Joel, 2026-11-15 18:45: "why did
    // this window materialize on my main chrome workspace?"). Null, where
    // niri opens it, only while Joel is on that empty workspace himself.
    function goesTo(session) {
        const presence = session.presence ?? {};
        const word = value => typeof value === "string" ? value : "";
        const one = { id: word(presence.session), address: word(session.address),
            orchestrates: word(presence.orchestrates), title: root.named(presence).title,
            from: "", quest: word(presence.quest), workspace: "",
            orchestratedBy: word(presence.orchestratedBy) };
        const into = root.opensOn(one, null, []);
        return into === null ? null : Object.assign({}, into, { focused: true });
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
        root.inSessions("pi-session", command, root.goesTo(session));
        Report.say(`joined session ${session.address} on ${session.machine}`);
    }

    // Views a hosted session in a new window of the sessions' kitty, by its
    // node's words, once until niri lists its window, as `attach` does;
    // `into` a place kept for it (viewKept, reopenViewer), which opens
    // without the focus, or, for Go To, where goesTo says, focused, since
    // Joel asked for it. Said true once it is asked for.
    function view(session, into) {
        const words = session.attach;
        const named = viewerOf(session.presence);
        if (!Array.isArray(words) || words.length === 0 || named === "") {
            unhosted(session, named === "" ? "unnamed" : "hosted");
            return false;
        }
        const title = Viewers.viewerTitle(session.presence?.title, session.title,
            session.presence?.cwd);
        const asked = viewWith(words.map(String), named, into, title);
        if (asked)
            Report.say(`viewing hosted session ${session.address}`);
        return asked;
    }
    // Opens a viewer, `named`, running `words` in a window titled `title`
    // from its first moment, `into` a place or where niri opens one; or
    // nothing, said false, while one asked of it is still opening: until
    // niri lists its window (viewerCame), or for viewPatience, as long as
    // the walk waits for a window, since the sessions' kitty may be
    // starting again after a crash. Within attachPatience alone, a second
    // would open beside a viewer slow to come. Nor while one is open
    // already (openNamed), which a served session's record may not say
    // yet.
    readonly property int viewPatience: 30000
    function viewWith(words, named, into, title) {
        if (root.openNamed(named) || viewerComing(named))
            return false;
        attaching = Object.assign({}, attaching, { [named]: Date.now() });
        root.inSessions("pi-hosted", viewing(Viewers.titled(title, words)), into ?? null, named);
        return true;
    }
    // A viewer's window niri lists is no longer opening, so the next ask
    // of it, once it has closed, opens one (viewWith); while it's open,
    // openNamed finds it.
    // A place kept for a waiting viewer is let go as it comes, whichever
    // press asked for it: kept, viewKept would open another by itself once
    // Joel closes this one. Not as it's asked for, as letGoWaitedFor
    // isn't: one never come would leave nothing to open it.
    function viewerCame(window) {
        const named = window?.opened ?? "";
        if (!named.startsWith("pi-hosted."))
            return;
        const waited = Object.keys(keptPlaces).filter(id => keptPlaces[id]?.named === named
            && keptPlaces[id]?.waiting !== undefined);
        if (waited.length > 0) {
            const kept = Object.assign({}, keptPlaces);
            for (const id of waited)
                delete kept[id];
            keepPlaces(kept);
            console.info(`sessions: ${waited.join(", ")}'s kept place let go, its viewer come`);
        }
        if (attaching[named] === undefined)
            return;
        const still = Object.assign({}, attaching);
        delete still[named];
        attaching = still;
    }
    // A window no sessions kitty took, as opens-in-sessions.sh says when
    // it gives up, by the app id it was asked under.
    function untaken(named) {
    }

    // Where the windows of sessions moving onto pi's host were, by pi's
    // session id, so each one's viewer opens there once it runs hosted,
    // rather than nowhere until Joel goes to it (OSPHT4's cutover; 2QZBX9,
    // [906a6c10]). The cutover asks for each (keepPlace) just before it
    // tells the session to move, which closes its window. Held here, so
    // the cutover is answered at once, and kept by the agents daemon
    // (kept_places.rs), told the whole of them as they change, so a shell
    // started again between the two still opens it: it takes what the
    // daemon kept once, as it first hears it, under any kept since.
    property var keptPlaces: ({})
    property bool keptPlacesTaken: false
    readonly property var keptPlacesServed: {
        void reopenKind.revision;
        return reopenKind.held["kept-places:agents"]?.places ?? null;
    }
    onKeptPlacesServedChanged: {
        if (keptPlacesTaken || keptPlacesServed === null)
            return;
        keptPlacesTaken = true;
        const keptSince = Object.keys(keptPlaces).length > 0;
        keptPlaces = Object.assign({}, keptPlacesServed, keptPlaces);
        if (keptSince)
            keepPlaces(keptPlaces);
    }
    function keepPlaces(kept) {
        keptPlaces = kept;
        reopenKind.ask({ ask: "keep-places", places: kept });
    }
    // Keeps where the window of the session here that pi knows as `id`
    // sits, answering "kept", or "no window to keep" for one with no
    // window niri has laid out, so the cutover can list the viewers it
    // couldn't put back. One parked here keeps its window while its
    // process is gone, and the cutover wakes it to move it, so it's
    // looked for among the parked too (2QZBX9, 2026-11-11: a parked
    // canary's window closed with no place kept). A parked one's window is
    // found here with niri's windows by kitty, as a running one's is, since
    // `parked` finds only those opened with an app id of their own, and
    // one in a kitty opened without, as the oldest are, had none to keep
    // (2QZBX9, 2026-11-11: B8RBNW's window 435 was on workspace 8).
    function keepPlace(id, marks) {
        const windows = niriWindows;
        const resting = root.resting.find(one => one.presence.session === id);
        const window = running.find(one => one.here && one.presence.session === id)?.window
            ?? (resting ? windowOf(resting, windows.pids, windows.opened) : null);
        const place = /^[A-Za-z0-9-]+$/.test(id) ? Viewers.placeOf(window, Date.now()) : null;
        if (place === null)
            return "no window to keep";
        keepPlaces(Object.assign({}, keptPlaces, { [id]: Object.assign(place, marks ?? {}) }));
        console.info(`sessions: kept ${id}'s place, ${place.workspace}/${place.column}`
            + (marks?.slept ? ", as it goes to sleep" : ""));
        return "kept";
    }
    // keepPlace, for a session going to sleep in pi's host, its window
    // closing so it costs next to nothing: its viewer opens there again
    // once it wakes, whatever wakes it (Viewers.sleeperDue).
    function keepPlaceAsleep(id) {
        return keepPlace(id, { slept: true });
    }
    // Opens the viewer of each session kept that now runs hosted where it
    // was, never on the workspace Joel is on and never with his focus
    // (Viewers.viewerPlace), and lets go of what's done or a day old
    // (Viewers.viewersDue). Asked again as niri's focus moves, so one waiting
    // for Joel to leave its workspace opens once he has, and as he goes
    // away. Only places kept since this shell started open, and only while
    // he is away (Away.qml), so a shell restarted by a switch opens none;
    // but a sleeper's opens as it wakes, him there or not, since waking is
    // what opens it, and one Reopen Them All kept for him to leave its
    // workspace, or with words to wait for its host by, opens once he has
    // left it, within ASKED_FOR of the ask or of his leaving, since he
    // asked for it, unless its viewer is open already. Asked as each
    // session's record changes too, which is how a sleeper is seen to wake.
    function viewKept() {
        if (Object.keys(keptPlaces).length === 0)
            return;
        const due = Viewers.viewersDue(keptPlaces, running.concat(parked), Date.now(),
            { workspaces: Niri.workspaces, since: startedAt, away: Away.now,
                listed: window => root.listedNow(window) });
        // One waiting for its host, asked of moments ago, stays kept.
        const kept = due.kept;
        for (const one of due.open) {
            if (one.session === undefined && root.openNamed(one.place.named)) {
                console.info(`sessions: ${one.id}'s kept viewer not opened, since it is open`);
            } else if (one.session === undefined) {
                const asked = viewWith(one.place.waiting, one.place.named, one.into,
                    one.place.title);
                if (!asked)
                    kept[one.id] = one.place;
                const on = root.spaceSaid(one.into.workspace);
                console.info(`sessions: ${one.id}'s kept viewer `
                    + (asked ? `opened to wait for its host, on ${on}`
                        : "kept, since one asked for is opening"));
            } else if (one.session.held && !one.session.presence?.hosted) {
                attach(one.session.held, one.into);
            } else if (!view(one.session, one.into)
                && !root.openNamed(viewerOf(one.session.presence))) {
                // Refused while one asked for is opening, it stays kept, so a
                // viewer that never opens still leaves its place to open by.
                const id = String(one.session.presence?.session ?? "");
                kept[id] = keptPlaces[id];
                console.info(`sessions: ${id}'s kept viewer kept, since one asked for is opening`);
            }
        }
        // What's let go is said, so a viewer Joel asked for that never
        // opens can be found in the journal.
        const opened = due.open.map(one => one.id ?? one.session?.presence?.session);
        for (const id of Object.keys(keptPlaces).filter(id => kept[id] === undefined
            && !opened.includes(id)))
            console.info(`sessions: ${id}'s kept place let go: its viewer is open, `
                + "its session is back as a plain pi, or it lapsed");
        // A sleeper first seen asleep is marked so in its place, which is
        // kept as any other change is.
        if (JSON.stringify(kept) !== JSON.stringify(keptPlaces))
            keepPlaces(kept);
    }

    // Whether a window opened with app id `named` is open (listedNamed),
    // so a second is never opened beside it.
    function openNamed(named) {
        return listedNamed(named) !== undefined;
    }
    // The window opened with app id `named`, as niri lists it while that
    // list is to be trusted (trustsNiri), else as this file's index of the
    // windows daemon has it. Undefined while neither has it, or for "".
    function listedNamed(named) {
        if (named === "")
            return undefined;
        return trustsNiri() ? Object.values(Niri.windows).find(window => window.opened === named)
            : (niriWindows.opened[named] ?? [])[0];
    }
    // Whether niri's list says which windows are open: while it lists
    // any, being the windows daemon's own, or while niri's own stream
    // stands in for a daemon that isn't answering, which leaves the index
    // (niriWindows) as it last stood. Only while it lists none, before it
    // has heard the daemon, is the index read instead.
    function trustsNiri() {
        return Object.keys(Niri.windows).length > 0 || Niri.standIn === true;
    }
    // Whether `window`, a session's as the index names it, is one niri
    // lists, when that list is trusted.
    function listedNow(window) {
        return window !== null && window !== undefined
            && (!trustsNiri() || String(window.id) in Niri.windows);
    }
    // Whether a viewer `named` was asked for within viewPatience and niri
    // hasn't listed it since (viewerCame).
    function viewerComing(named) {
        return named !== "" && Date.now() - (attaching[named] ?? 0) < viewPatience;
    }
    // The app id a hosted session's viewer opens with, by pi's session id,
    // which stays the same as its host restarts, or "" for one not hosted.
    function viewerOf(presence) {
        return viewerNamed(presence?.hosted ? String(presence.session ?? "") : "");
    }
    // The app id of the viewer of session `id`, or "" for an id no app id
    // could carry, or one beginning with a dash, which mycelium's
    // `--session` could take for a flag.
    function viewerNamed(id) {
        return /^[A-Za-z0-9][A-Za-z0-9-]*$/.test(id) ? `pi-hosted.${id}` : "";
    }

    // Attaches what is held as `id` in a new window of the sessions' kitty,
    // `into` a place kept for it (viewKept, as a sleeper wakes), which
    // opens without the focus, or where niri opens one. The node names the
    // window within moments of its joining; until it does, asking again
    // opens nothing more.
    readonly property int attachPatience: 5000
    property var attaching: ({})
    function attach(id, into) {
        if (!holds || Date.now() - (attaching[id] ?? 0) < attachPatience)
            return;
        attaching = Object.assign({}, attaching, { [id]: Date.now() });
        root.inSessions("pi-session", ["zsh", "-lic", 'exec mycelium --realm "$0" attach "$1"',
            realm, id], into ?? null);
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
    // reason, or none, and the act refused where its caller names it
    // ("wake", "pause"), else "", so a row becoming what one act asked is
    // taken back only for that act; `then`, when given, is told whether it
    // was put, the node's answer, and whether it was sent: one sent with
    // no answer is the node gone before answering, perhaps having written
    // it, and one unsent never left the shell, both apart from a refusal.
    signal unsteered(var session, string why, string act)
    function steer(address, fields, then) {
        const upper = String(address).toUpperCase();
        const sent = PiSource.reaches();
        PiSource.steer(upper, fields, answer => {
            const put = steered(upper, answer, fields?.act ?? "");
            if (then)
                then(put, answer, sent);
        });
    }
    // Says what became of a steer by the node's answer, the agents
    // daemon's to an act it did by one included, and whether it was put.
    function steered(address, answer, act) {
        const upper = String(address).toUpperCase();
        const machine = answer?.steered?.machine;
        if (machine) {
            Report.say(`steered ${upper} on ${machine}`);
            return true;
        }
        const why = String(answer?.error?.message ?? "");
        Report.say(`unsteered ${upper}: ${why}`);
        unsteered(running.find(one => String(one.address).toUpperCase() === upper)
            ?? parkedAs(upper) ?? { address: upper }, why, String(act ?? ""));
        return false;
    }

    // Talks to a session: the words go with an id, so the session writes
    // what became of them where the launcher reads its talks.
    function talk(address, words) {
        const text = words.trim();
        if (text === "")
            return false;
        // Talk gives a parked session a turn, as a question would: the node
        // wakes one parked here once its words are in its inbox.
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

    // A session stopped on Joel's keys, a question or a yes, as the agents
    // daemon serves its want (modules/agents's wants.rs, `rings`), is new
    // to this shell once: it sounds, softly, and its notice
    // says it with Go To its window first (pieces/Pi.qml, said by
    // Arrivals.qml), so Super+N and Return take him to it. The bell means
    // something needs him now, so a run that ended blocked and an item of
    // a session's own, which For You lists, never ring (WCJQSG's pivot,
    // 2026-11-12, on Joel's "my computer constantly dinging instead of
    // being brought to what i need to interact with"), and the notice
    // goes once he's in that window or the want ends, so it never takes
    // him to a session that has moved on. Neither comes while he's in that
    // window already, nor for one that stood before the shell began. Each
    // is known by its session and its want's own id or start, as the
    // daemon says it (`known`), never by its words, so one rewording itself
    // doesn't sound again. It's kept known: a presence file replaced by
    // rename leaves its folder for a moment, which would otherwise make a
    // want that never ended look new. Each ring is said in the journal,
    // so the sounds can be counted by where they came from.
    readonly property real began: Date.now()
    property var told: ({})
    // The newest want the bell rang for, by its key, while it stands and
    // Joel hasn't gone to its window.
    property string rung: ""
    // The wants that ring, as the daemon serves them, by their session's
    // address: none while it doesn't answer, so the shell judges none.
    readonly property var rings: {
        void reopenKind.revision;
        const by = {};
        for (const one of Object.values(reopenKind.held)) {
            if (one.kind === "want" && one.rings === true)
                by[String(one.address).toUpperCase()] = one;
        }
        return by;
    }
    function rang(session) {
        return rings[String(session?.address ?? "").toUpperCase()] ?? null;
    }
    // Those that ring, the oldest want first, as `wanting` lists them.
    readonly property var stopped: running.concat(parked)
        .filter(session => rang(session) !== null)
        .sort((a, b) => String(rang(a).since).localeCompare(String(rang(b).since)))
    readonly property var ringing: stopped.find(session => key(session) === rung) ?? null
    function key(session) {
        return String(rang(session)?.known ?? "");
    }
    onStoppedChanged: {
        for (const session of stopped) {
            const known = key(session);
            if (told[known])
                continue;
            told[known] = true;
            const since = Date.parse(rang(session).since ?? "");
            const window = session.window ? String(session.window.id) : "";
            if (window === String(Niri.current) || (since < began))
                continue;
            rung = known;
            // Never while notices are held, and Arrivals holds its card.
            if (Options.sounds.wants !== "" && !Quiet.on) {
                Niri.spawn([Options.sounds.player, "--volume=0.5", Options.sounds.wants]);
                console.info(`sessions: rang the want bell for ${session.address} `
                    + `(${session.presence?.wants?.kind ?? "a want"})`);
            }
        }
    }
    // Once Joel is in the window of what rang, he's been brought to it.
    Connections {
        target: Niri
        function onCurrentChanged() {
            const window = root.ringing?.window;
            if (window && String(window.id) === String(Niri.current))
                root.rung = "";
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

        // keepPlaceAsleep, as pi's host asks it before it closes the
        // window of a session it puts to sleep, by pi's session id.
        function keepPlaceAsleep(session: string): string {
            return root.keepPlaceAsleep(session);
        }

        // untaken, as opens-in-sessions.sh says a window it was asked
        // for found no sessions kitty to open in.
        function untaken(named: string): void {
            root.untaken(named);
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
    // `kind` is the want's kind, a question, a yes or a stuck run.
    function wanted(kind) {
        const said = Words.of("agents:wants").said ?? {};
        return String(said[kind] ?? said.question ?? "");
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
    // The sessions' kitty listens on a path that names no pid, so its unit
    // writes the pid beside it (home.nix's sessions-kitty). Read again as
    // it's written and as windows come, since a unit started again makes
    // its folder anew, which a watch on the old file misses.
    readonly property string sessionsSocket:
        `unix:${Quickshell.env("XDG_RUNTIME_DIR")}/pi-sessions/kitty.sock`
    FileView {
        id: sessionsKittyFile
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/pi-sessions/kitty.pid`
        blockLoading: true
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
    }
    readonly property string sessionsKittyPid: {
        const read = sessionsKittyFile.text().trim();
        return /^\d+$/.test(read) ? read : "";
    }
    // The pid of the kitty listening on a socket, as niri lists the windows
    // it draws by, or undefined where the socket says none.
    function kittyPid(socket) {
        if (socket === sessionsSocket)
            return sessionsKittyPid || undefined;
        return (kittySocket.exec(socket) ?? [])[1];
    }
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
    // Each list's copies as relive last made them, beside the entries they
    // were made from (Pi.keptBy), so only a session the node's change named
    // is copied again, and alike passes the rest at once. In plain objects,
    // so keeping them notifies nothing.
    readonly property var copied: ({ live: {}, resting: {}, ending: {} })
    function relive() {
        // It reads nothing but its record, which Pi.keptBy relies on.
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
        const next = Pi.keptBy(PiSource.live, copied.live, kitted);
        const sleeping = Pi.keptBy(PiSource.resting, copied.resting, kitted);
        const ended = Pi.keptBy(PiSource.ending, copied.ending, kitted);
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

    // niri's windows by the process drawing them, and by the app id each was
    // opened with, oldest first, for
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
        // kitty's windows changing, the sessions' kitty started again among
        // them.
        if (came) {
            sessionsKittyFile.reload();
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
    // (viewerOf), since kitty's own ids mean nothing to niri; of several
    // opened with one app id, the one its kitty draws. A window
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
        const kitty = record.kitty;
        const pid = kitty ? kittyPid(kitty.socket) : undefined;
        if (named !== "" && opened)
            return SessionWindows.named(opened[String(named)] ?? [], pid);
        if (named !== "")
            return SessionWindows.named(Object.values(windowsKept)
                .filter(window => window.opened === named).sort((a, b) => a.id - b.id), pid);
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
            const pid = kittyPid(socket);
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
    Connections {
        target: PiSource
        function onSorted() {
            root.relive();
        }
    }

    // Wakes a parked session by asking the node, on the connection kept to
    // it, to steer it with nothing but the act: the node writes the wake
    // file of the holder keeping it here, which starts it again in its
    // window with its steer inbox read, so words left there first reach it
    // as it wakes (mycelium's PROTOCOL.md, its steer). One the node can't
    // wake, parked on another machine, is said by `unsteered` with the
    // node's reason; one parked here with no holder this boot is reopened
    // where it left off (reopensUnwoken). One asleep in pi's host with
    // no window is viewed instead, since viewing it is what wakes it there
    // and the node's wake reaches only a holder's file: five such sleepers
    // on 14 November had their wake file in the host's own folder, which
    // the node refuses. It opens where its window was as it went to sleep
    // (keepPlaceAsleep), on Joel's own workspace too since he asked for it,
    // or where niri opens one when no place was kept. One a holder keeps
    // has its place marked asked, since every caller here is Joel's own
    // act (a quest's Return, a Go To), so its window opens there as it
    // wakes (Viewers.viewersDue); a wake nobody here asked for, the node's
    // or a restart's, opens none.
    function wake(session) {
        if (wokenByViewing(session)) {
            const id = String(session.presence?.session ?? "");
            const into = sleptPlace(id);
            if (keptPlaces[id] !== undefined) {
                const kept = Object.assign({}, keptPlaces);
                delete kept[id];
                keepPlaces(kept);
            }
            view(session, into);
            return;
        }
        const id = String(session?.presence?.session ?? "");
        if (keptPlaces[id]?.slept === true)
            keepPlaces(Object.assign({}, keptPlaces, { [id]: Object.assign({}, keptPlaces[id],
                { at: Date.now(), asked: true }) }));
        if (!session?.address)
            return;
        const upper = String(session.address).toUpperCase();
        PiSource.steer(upper, { act: "wake" }, answer => {
            if (!reopensUnwoken(session, answer))
                steered(upper, answer, "wake");
        });
    }
    // Reopens where it left off one parked here that the node says no
    // holder keeps this boot, as when the holder died or the machine started
    // again, since nothing else will ever wake it and Joel asked for it:
    // left to the node's refusal alone, it stayed asleep, said only on the
    // launcher's foot. Only that refusal, and only here, since one a holder
    // still keeps, or one on another machine, opened again here would be
    // the same session run twice. Whether it does is answered.
    // The node says why only in words, so its refusal is known by them
    // (mycelium's steer_here); any other refusal is said as it was.
    readonly property string unheld: "isn't parked with a holder to wake it this boot"
    // Each session reopened so, by its id, with when, so a second Wake
    // before pi is up opens no second: held until it runs here, or for
    // unwokenPatience, far past the slowest pi to load its log.
    property var unwokenReopening: ({})
    readonly property int unwokenPatience: 300000
    // One nothing holds whose record names no fence it can come back under
    // is left asleep and said by this, not as the node's refusal, which
    // blames only its missing holder (#5901's F16).
    signal unfenced(var session)
    function reopensUnwoken(session, answer) {
        const id = String(session?.presence?.session ?? "");
        const why = String(answer?.error?.message ?? "");
        // Only one whose presence names its fence: "none" reopens as it
        // is, and "fence" or "strict" with that word as its PI_SANDBOX, in
        // the reopened pi's own environment alone, so it never comes back
        // looser than it ran; a stage's listing by id keeps it fenced too
        // (the harness's fence-stage.ts). The word is the session's own, so
        // it only ever tightens; any other is left asleep (ISXEG8, 20 Nov
        // [0d92dfe3]) and said as one it might bring back unfenced. And
        // only an id that's a plain word, as handedOver holds it, since
        // it's said in a shell's words.
        const fence = session.presence?.fence;
        const fenced = fence === "fence" || fence === "strict";
        if (!/^[A-Za-z0-9._-]+$/.test(id) || session.here === false || !why.endsWith(unheld))
            return false;
        const upper = String(session.address).toUpperCase();
        if (running.some(one => one.here && one.presence?.session === id)) {
            Report.say(`left ${upper} alone, which runs here already`);
            return true;
        }
        if (fence !== "none" && !fenced) {
            Report.say(`left ${upper} asleep, whose record names no fence it can come back under`);
            unfenced(session);
            return true;
        }
        const asked = unwokenReopening[id];
        if (asked !== undefined && Date.now() - asked < unwokenPatience) {
            Report.say(`left ${upper} alone, which is reopening`);
            return true;
        }
        unwokenReopening = Object.assign({}, unwokenReopening, { [id]: Date.now() });
        const quest = session.presence?.questInfo?.id ?? session.presence?.quest ?? "";
        // Only its own session file, never one started afresh; with none,
        // or no folder to open it in, it's said as the refusal was.
        resume(id, String(quest), String(session.folder ?? ""), () => {
            const left = Object.assign({}, unwokenReopening);
            delete left[id];
            unwokenReopening = left;
            Report.say(`couldn't reopen ${upper}: no session file or folder of its own`);
            unsteered(session, why, "wake");
        }, { done: cwd => Report.say(`reopened ${upper}, which nothing held, in ${cwd}`
                + (fenced ? `, under its fence, ${fence}` : "")),
            env: fenced ? { PI_SANDBOX: fence } : null });
        return true;
    }
    // Go To on `session`, waking `resting` first, its parked record, when
    // it's parked. One only viewing wakes is left to goTo's own viewer,
    // which opens where Joel is and takes focus: wake's would open where it
    // slept, unfocused, and hold goTo's off while it came. Agents' Go To
    // and For You's Deal With It both go through here.
    function wakeToGo(resting, session) {
        if (resting && !wokenByViewing(resting))
            wake(resting);
        goTo(session);
    }
    // Whether `session` sleeps in pi's host with no window, which only
    // viewing it wakes.
    function wokenByViewing(session) {
        return !!session?.presence?.hosted && !session.window && Viewers.parkedHere(session);
    }
    // Where the session pi knows as `id` had its window as it went to
    // sleep, while that workspace is still there, or null.
    function sleptPlace(id) {
        const place = keptPlaces[id];
        const workspace = place ? (Niri.workspaces ?? {})[place.workspace] : undefined;
        return workspace ? { workspace: workspace.id, column: place.column ?? null } : null;
    }
    // Each running session's record, by every address it answers to.
    property var present: ({})
}

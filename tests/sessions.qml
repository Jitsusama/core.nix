pragma ComponentBehavior: Bound
// Drives pi's sessions (modules/quickshell/Sessions.qml) through Quickshell
// itself, with the machine's mycelium node stood in for over the daemon
// interface it speaks, the windows daemon told as its watch is, and niri,
// kitty's kitten and the notices stood in too, over what sessions-state.sh lays out. The sessions are the
// node's and nothing else's: a node not listening as the shell starts, as
// when a switch restarts it, is said as not answering, with no session
// guessed in its place, and is watched once it listens. Every window niri
// shows carries the same title, so nothing can be found by one. A
// session's window is the one niri lists by the app id the node says its
// window was opened with, or for one opened without, through the kitty its
// record's terminal names, by kitty's own list of its several windows. Go
// To focuses that window, or asks kitty when niri's isn't known. One the
// node says is parked is held apart from those running, and one it says
// closed apart again, however each record reads; Wake, or talking to one
// parked, writes the file its marker names. What an earlier boot's end took away is
// offered whole, but one that ran hosted, which its host brings back,
// reopened held in its own folder, until put off, one pi
// never saved started under its own id with its quest, and Reopen Them
// All reopens even when niri never answers its ask for the screens; what
// another machine carried here, or pi's host gave back, is offered after
// it, the newest first, and never put off with it, and a window it had joined to a session another
// machine holds is joined again by its words, once; a closed session
// resumes in its own folder, and one whose folder is gone opens nowhere
// but its quest's start folder, never the home folder. What the node tells
// of sessions on the realm's other machines is held too: each says its
// machine, after this machine's; Go To joins one by the words its node
// gives; one out of reach stays, saying so, and isn't joined; one pi's
// host runs with no window is said and never opened; one the node says
// nothing of how it stands is passed over; and a steer to one is asked
// of the node, its refusal said. One the node says is gone goes; a
// change saying only what differs keeps the rest of what the node said,
// and one to a running session tells nothing of the parked or closed;
// the one whose window the windows daemon says was used last goes first,
// and a window retitled makes no session's window again;
// one the node says has ended goes; and a node lost leaves what it said
// shown. It exits 0 once every case holds, and otherwise 1, saying which
// didn't.
// Its stand-ins are in modules/quickshell/tests/stand-ins.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string state: Quickshell.env("XDG_STATE_HOME")
    property string stage: "unanswered"

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    function addresses() {
        return Sessions.running.map(session => session.address).sort().join(" ");
    }

    function find(address) {
        return Sessions.running.find(session => session.address === address);
    }

    // How often what reads parked and closed sessions was told they moved,
    // so a change to a running one alone is seen to tell neither.
    property var retold: ({ parked: 0, closed: 0 })
    property var retoldBefore: null

    Connections {
        target: Sessions
        function onRunningChanged() {
            Qt.callLater(root.check);
        }
        function onParkedChanged() {
            root.retold.parked += 1;
            Qt.callLater(root.check);
        }
        function onClosedChanged() {
            root.retold.closed += 1;
        }
    }

    Connections {
        target: Sessions.node
        function onStateChanged() {
            Qt.callLater(root.check);
        }
    }

    function parkedAs() {
        return Sessions.parked.map(session => session.address).sort().join(" ");
    }

    // Reopen Them All opens each on a timer of its own, which nothing else
    // here hears.
    Connections {
        target: Niri
        function onSpawnsChanged() {
            Qt.callLater(root.check);
        }
    }

    Component.onCompleted: Qt.callLater(check)

    // A window as the windows daemon serves it (modules/windows), with only
    // what Sessions reads of it, and the title a row would say.
    function windowRecord(id, pid, opened, fields) {
        return Object.assign({ key: `window:${id}`, id, pid, opened,
            title: "\u03c0 - Lighthouse", workspace: null, column: null, down: null, order: 0 },
            fields ?? {});
    }
    // Tells Sessions' watch of the windows daemon, as the daemon does: its
    // whole answer, or a change.
    function windowsTell(said) {
        Sessions.windowsFrom.said(JSON.stringify(Object.assign({ id: "all" }, said)));
    }
    // How often Sessions made its windows by process and app id again.
    property int windowsMade: 0
    Connections {
        target: Sessions
        function onNiriWindowsChanged() {
            root.windowsMade += 1;
        }
    }

    function check() {
        try {
            if (stage === "unanswered")
                unanswered();
            else if (stage === "listed")
                listed();
            else if (stage === "reopening")
                reopening();
            else if (stage === "forgotten")
                forgotten();
            else if (stage === "differing")
                differing();
            else if (stage === "resting")
                resting();
            else if (stage === "ended")
                ended();
            else if (stage === "lost")
                lost();
        } catch (error) {
            fail(error.message);
        }
    }

    // With the node not listening, it's said so, and no session is
    // guessed from a folder or a process; then it listens.
    function unanswered() {
        if (Sessions.node?.state !== "not answering")
            return;
        expect("no session while the node doesn't answer", addresses() === "", addresses());
        // The same three windows niri's stand-in lists (checks.nix), and
        // the one a parked session's holder keeps. Older windows of
        // another kitty were opened with the same app ids, as old spawns
        // share small numbers (2QZBX9, 2026-11-11), and are neither's.
        windowsTell({ records: [windowRecord(10, 500, "kitty"), windowRecord(20, 600, "kitty"),
            windowRecord(17, 900, "pi-session.7", { workspace: 11, column: 1, down: 1 }),
            windowRecord(18, 900, "pi-spawned.9", { workspace: 11, column: 2, down: 1 }),
            windowRecord(21, 600, "pi-session.7", { workspace: 3, column: 2, down: 1 }),
            windowRecord(22, 800, "pi-spawned.9", { workspace: 5, column: 1, down: 1 })] });
        stage = "listed";
        node.active = true;
    }

    // Waits until every running session is listed, the one in a kitty
    // drawing two windows found by its window's app id.
    function listed() {
        const other = find("OTHER2");
        if (addresses() !== "FARAWY LIVE01 NOWIN6 OTHER2 OUTRCH" || !other || !other.window)
            return;
        if (!markersLaid) {
            parking.running = true;
            return;
        }
        const live = find("LIVE01");
        expect("a kitty with one window has it", live.window && live.window.id === 10,
            JSON.stringify(live.window));
        expect("its label shown, cut at its folder",
            live.title === "Live One" && live.folder === "core.nix", `${live.title} ${live.folder}`);
        expect("its quest's title and its folder as its record says them, not its label",
            other.title === "Lighthouse - Keep It Lit" && other.folder === "lamp",
            `${other.title} ${other.folder}`);
        expect("of a kitty's several windows, the one opened with its app id",
            other.window.id === 21, other.window.id);
        expect("its kitty from its record's terminal",
            JSON.stringify(other.kitty) === '{"socket":"unix:@kitty-600","window":4}',
            JSON.stringify(other.kitty));
        expect("one the node says has ended, and one it passes over elsewhere, aren't there",
            Object.keys(Sessions.present).sort().join(" ") === "FARAWY LIVE01 NOWIN6 OTHER2 OUTRCH",
            Object.keys(Sessions.present).sort().join(" "));
        // LIVE01 here, and both elsewhere.
        expect("working, as each record says", Sessions.working === 3, Sessions.working);
        expect("a session is its window", Sessions.isOf(other) === "window:21", Sessions.isOf(other));
        // Asked to keep where a session's window is, as the move onto pi's
        // host asks before it closes it, the shell says whether it could.
        expect("a session's window kept", Sessions.keepPlace("s-OTHER2") === "kept"
            && JSON.stringify(Sessions.keptPlaces["s-OTHER2"] ?? null).startsWith(
                '{"workspace":3,"column":2,'), JSON.stringify(Sessions.keptPlaces));
        const keptNot = ["s-NOWIN6", "r-far", "s-nobody"].map(id => Sessions.keepPlace(id));
        expect("one with no window, or not here, keeps nothing", keptNot.every(answer =>
            answer === "no window to keep") && Object.keys(Sessions.keptPlaces).length === 1,
            keptNot.join(", "));
        Sessions.keepPlaces({});

        Sessions.goTo(other);
        expect("Go To focuses its window", JSON.stringify(Niri.acted) === '{"FocusWindow":{"id":21}}',
            JSON.stringify(Niri.acted));
        const unseen = find("NOWIN6");
        expect("with no window niri shows, none", unseen.window === null,
            JSON.stringify(unseen.window));
        expect("a session with no window is itself", Sessions.isOf(unseen) === "agent:NOWIN6",
            Sessions.isOf(unseen));
        Sessions.goTo(unseen);
        expect("Go To asks its kitty", Niri.spawned.join(" ")
            === "kitten @ --to unix:@kitty-700 focus-window --match id:2", Niri.spawned.join(" "));
        elsewhere();
        parks();
        lastBoot();
        stage = "reopening";
    }

    // Those on the realm's other machines, as the node tells of them.
    function elsewhere() {
        const far = find("FARAWY");
        const out = find("OUTRCH");
        expect("one elsewhere says its machine", !far.here && far.machine === "lighthouse"
            && far.reachable, JSON.stringify(far));
        expect("one out of reach says so", !out.reachable && out.attach === null,
            JSON.stringify(out));
        expect("this machine's first", Sessions.running.slice(0, 3).every(session => session.here),
            Sessions.running.map(session => session.address).join(" "));
        Sessions.goTo(far);
        expect("Go To joins it by its node's words", inSessions(Niri.spawned.join(" "))
            === "kitty -1 --instance-group pi-sessions zsh -lic mycelium attach r-far",
            Niri.spawned.join(" "));
        Sessions.goTo(out);
        expect("one out of reach isn't joined", root.unjoined === "OUTRCH", root.unjoined);
        hosts();
        Sessions.steer("farawy", { act: "pause" });
    }

    // One pi's host runs, with no window: Go To views it by its node's
    // words, in a window named for it, once; niri's window by that name is
    // its own after; and one the node gives no words for is said.
    function hosts() {
        const hosted = {
            address: "HOSTED", here: true, window: null, kitty: null, held: null,
            attach: ["mycelium", "--realm", "harbour", "attach", "--host", "harbour-attended",
                "--session", "s-hosted"],
            presence: { hosted: { host: "harbour-attended", conversation: "s-hosted" },
                session: "s-hosted", addresses: ["HOSTED"] }
        };
        const before = Niri.spawns.length;
        Sessions.goTo(hosted);
        const viewed = Niri.spawned.join(" ");
        expect("a hosted one is viewed by its node's words, held, in a window named for it",
            viewed.includes("MYCELIUM_APP_ID=pi-hosted.s-hosted")
            && viewed.includes("--class pi-hosted.s-hosted mycelium --realm harbour hold -- "
                + "mycelium --realm harbour attach --host harbour-attended --session s-hosted")
            && viewed.endsWith("--session s-hosted"), viewed);
        Sessions.goTo(hosted);
        expect("viewed only once while its window comes", Niri.spawns.length === before + 1,
            Niri.spawns.slice(before).join("\n"));
        const record = { opened: "", kitty: null, presence: hosted.presence };
        const window = { id: 40, opened: "pi-hosted.s-hosted" };
        expect("its viewer is its window, by the name it opened with",
            Sessions.windowOf(record, {}, { "pi-hosted.s-hosted": [window] }) === window,
            JSON.stringify(Sessions.windowOf(record, {}, { "pi-hosted.s-hosted": [window] })));
        Sessions.goTo(Object.assign({}, hosted, { window: window }));
        const raised = JSON.stringify(Niri.acted);
        expect("Go To raises its viewer", raised === '{"FocusWindow":{"id":40}}', raised);
        Sessions.goTo(Object.assign({}, hosted, { address: "WORDLESS", attach: null }));
        expect("one with no words is said, never opened", root.unhosted === "WORDLESS"
            && Niri.spawns.length === before + 1, `${root.unhosted} ${Niri.spawned.join(" ")}`);
    }

    // One the node says is parked is listed apart, never running, however
    // its record reads; Wake writes the file its marker names, whole, in
    // place, and talking to it asks the node to steer its inbox, then,
    // once the node has, wakes it to read it.
    function parks() {
        expect("only the one the node says is parked is parked",
            parkedAs() === "NAPPER", parkedAs());
        // Those the node says are closed, one whose record says parked
        // among them, the latest first, by id when they ended at once, each
        // saying when it ended as the node's number, never a time written
        // out for the launcher to read back.
        const closed = Sessions.closed;
        expect("closed, as the node says each is",
            closed.map(one => one.id).join(" ") === "s-ENDED3 s-STALE7 s-WOKEN8",
            JSON.stringify(closed.map(one => one.id)));
        expect("each saying when it ended, and its address",
            closed.every(one => typeof one.ended === "number" && one.ended > 0
                && one.endedAt === undefined) && closed[0].address === "ENDED3",
            JSON.stringify(closed[0]));
        const napper = Sessions.parked[0];
        expect("one parked is named by its record, in the window its holder keeps",
            napper.title === "Napper" && napper.window?.id === 22, JSON.stringify(napper));
        // The cutover wakes a parked one to move it, so where its window
        // sits is kept first, as a running one's is.
        expect("a parked session's window kept", Sessions.keepPlace("s-NAPPER") === "kept"
            && JSON.stringify(Sessions.keptPlaces["s-NAPPER"] ?? null).startsWith(
                '{"workspace":5,"column":1,'), JSON.stringify(Sessions.keptPlaces));
        Sessions.keepPlaces({});
        Sessions.wake(napper);
        const woken = Niri.spawned;
        expect("Wake writes the file its marker names, in place",
            woken.slice(0, 5).join(" ")
                === `sh -c printf "%s\\n" "$2" > "$1" wake ${root.state}/napper.wake`
            && JSON.parse(woken[5]).session === "s-NAPPER", woken.join(" "));
        talkedAt = Niri.spawns.length;
        Sessions.talk("napper", "what now?");
        expect("a talk writes no inbox itself, nor wakes before the node has it",
            Niri.spawns.length === talkedAt, Niri.spawns.slice(talkedAt).join(" | "));
    }

    // What the shell ran since the talk to the one parked, and whether the
    // talk has been steered and woken it, its wake after the node's answer.
    property int talkedAt: -1
    function wokenSinceTalk() {
        return talkedAt >= 0
            && Niri.spawns.slice(talkedAt).some(one => one.includes("/napper.wake "));
    }
    function talked() {
        const told = steers.find(one => one.ask.address === "NAPPER");
        if (!told || !wokenSinceTalk())
            return false;
        expect("talking to one parked asks the node to steer it, then wakes it",
            told.ask.steer.text === "what now?" && told.ask.steer.from === "joel"
            && !told.wokenBefore, JSON.stringify(told));
        expect("a talk writes no inbox itself",
            !Niri.spawns.slice(talkedAt).some(one => one.includes("/steer/")),
            Niri.spawns.slice(talkedAt).join(" | "));
        return true;
    }

    // The marker of the one parked, naming its wake file, laid once the
    // shell has listed its sessions, in a park folder that wasn't there
    // till then, as the harness makes it only as a session first parks.
    property bool markersLaid: false
    Process {
        id: parking
        onExited: {
            root.markersLaid = true;
            Qt.callLater(root.check);
        }
        command: ["sh", "-c", 'p="$1/pi/agentic-harness.pi/park-workflow/parked"; mkdir -p "$p"'
            + ' && printf \'{"session":"s-NAPPER","wake":"%s/napper.wake"}\' "$1"'
            + ' > "$p/s-NAPPER.json"'
            + ' && printf \'{"session":"s-STALE7","wake":"%s/stale.wake"}\' "$1"'
            + ' > "$p/s-STALE7.json"', "park", root.state]
    }

    // Why each that didn't open as asked didn't, as its caller is told:
    // the id, the folder gone and where it started afresh instead.
    property var unopened: []
    function told(id) {
        return (cwd, startedIn) => {
            unopened = unopened.concat([`${id} ${cwd} ${startedIn}`]);
            Qt.callLater(root.check);
        };
    }

    // The sessions another boot's end took away are offered, but for one
    // with no folder or an id pi's command line can't take; each reopens
    // in its own window and folder, held, and one whose folder is gone
    // since opens nowhere, never in the home folder. A closed session
    // resumes in the folder its own record says, which needn't be the
    // home folder; one whose folder is gone starts its quest afresh in
    // the quest's start folder, or nowhere when it has none.
    function lastBoot() {
        expect("only the whole ones of another boot, none hosted",
            Sessions.lastBoot.map(one => one.id).join(" ")
                === "s-before s-fresh s-given-back s-far",
            JSON.stringify(Sessions.lastBoot));
        // Those another machine carried here or pi's host gave back
        // apart, the newest first, and one saying where from badly, or
        // both ways, left out.
        expect("the carried and given back, newest first",
            Sessions.apart.map(one => one.id).join(" ") === "s-ferry s-freed s-ferried",
            JSON.stringify(Sessions.apart));
        expect("the last boot's, then those apart", Sessions.toReopen.map(one => one.id).join(" ")
            === "s-before s-fresh s-given-back s-far s-ferry s-freed s-ferried",
            JSON.stringify(Sessions.toReopen));
        Sessions.lastBoot.forEach(one => Sessions.reopen(one, told(one.id)));
        // One held elsewhere is joined again once, however often asked.
        Sessions.reopen(Sessions.lastBoot.find(one => one.id === "s-far"), told("s-far"));
        Sessions.resume("s-kept", "QEST-KEPT", "/tmp", told("s-kept"));
        Sessions.resume("s-gone", "QEST-GONE", root.state, told("s-gone"));
        Sessions.resume("s-gone", "QEST-NONE", "", told("s-none"));
        // Reopen Them All asks niri for its screens first, and niri here
        // never answers, as it doesn't a request it refused.
        Sessions.reopenAll([{ id: "s-patient", cwd: "/tmp" }], told("s-patient"), id => null);
        patience.start();
    }
    Timer {
        id: patience
        interval: 10000
        onTriggered: {
            if (!Niri.spawns.some(one => one.endsWith(" pi --session s-patient")))
                root.fail("Reopen Them All waited on niri's answer for good");
        }
    }

    readonly property string kitty: "kitty -1 --instance-group pi-sessions --directory"
    // The sessions' kitty as the shell asks for it, past the scope that
    // starts it in agents.slice and without its class or the app id its
    // process is told, or "" for anything not started there.
    readonly property string scope: "systemd-run --user --scope --collect --quiet "
        + "--slice=agents.slice -- "
    function inSessions(command) {
        const at = command.indexOf(scope);
        return at < 0 ? "" : command.slice(at + scope.length).replace(/ --class \S+/, "")
            .replace(/^env MYCELIUM_APP_ID=\S+ /, "");
    }
    function reopening() {
        const kitties = Niri.spawns.map(command => inSessions(command))
            .filter(command => command.startsWith("kitty ") && !command.includes("attach r-far")
                && !command.includes("attach --host"));
        if (kitties.length < 6 || unopened.length < 3 || !steered || !talked())
            return;
        // Each window is opened with an app id of its own, which its process
        // is told, so the node's record of what it holds names it as niri
        // lists it; a hosted one's viewer, by its session's id.
        const named = Niri.spawns.filter(command => command.includes(scope))
            .map(command => [(/MYCELIUM_APP_ID=(\S+)/.exec(command) ?? [])[1],
                (/ --class (\S+)/.exec(command) ?? [])[1]]);
        expect("each window opened is told the app id it opens with, its own",
            named.every(([told, classed]) => /^pi-((session|spawned)\.\d+|hosted\.s-hosted)$/
                .test(told ?? "")
                && told === classed) && new Set(named.map(([told]) => told)).size === named.length,
            JSON.stringify(named));
        // Numbered on from when this shell started, so a shell started again,
        // as every switch does, names none as a window still open from before.
        const numbers = named.map(([told]) => /\.(\d+)$/.exec(told ?? ""))
            .filter(number => number !== null).map(number => Number(number[1]));
        expect("each window numbered past the moment its shell started",
            numbers.length > 0 && Math.abs(Date.now() - Sessions.startedAt) < 10 * 60 * 1000
                && numbers.every(number => number > Sessions.startedAt),
            `${Sessions.startedAt}: ${numbers.join(" ")}`);
        // Held as every start is, by the realm's node where there is one.
        const opened = command => kitties.some(one => one.startsWith(command[0])
            && one.endsWith(command[1]));
        expect("reopened in its own window and folder",
            opened([`${kitty} /tmp zsh -lic `, " pi --session s-before"]), kitties.join("\n"));
        expect("one pi never saved started under its own id, its quest loaded", opened([
            `${kitty} /tmp env QUEST_WORKFLOW_AUTOLOAD_ID=QEST-FRESH zsh -lic `,
            " pi --session-id s-fresh"]), kitties.join("\n"));
        expect("reopened though niri never said its screens",
            opened([`${kitty} /tmp zsh -lic `, " pi --session s-patient"]), kitties.join("\n"));
        expect("resumed in the folder its record says",
            opened([`${kitty} ${state}/trees/karst zsh -lic `, " pi --session s-kept"]),
            kitties.join("\n"));
        expect("its folder gone, its quest started afresh in the quest's",
            opened([`${kitty} ${state} env QUEST_WORKFLOW_AUTOLOAD_ID=QEST-GONE zsh -lic `, " pi"]),
            kitties.join("\n"));
        expect("one held elsewhere joined again by its words, in its folder here",
            kitties.includes(`${kitty} /tmp ssh -t lighthouse mycelium attach s-far`),
            kitties.join("\n"));
        expect("six opened, none in the home folder nor a gone one", kitties.length === 6
            && !kitties.some(one => one.includes(`--directory ${Quickshell.env("HOME")} `)
                || one.includes("given-back") || one.includes("QEST-NONE")),
            kitties.join("\n"));
        expect("each not opened as asked told why", JSON.stringify(unopened.slice().sort())
            === JSON.stringify(["s-given-back /given-back ",
                `s-gone ${state}/trees/given-back ${state}`, `s-none ${state}/trees/given-back `]),
            JSON.stringify(unopened));
        // Each folder looked for is known after, so a row says one's gone
        // before Return the next time it's listed.
        expect("a folder found gone is known gone", Sessions.folderGone("/given-back")
            && !Sessions.folderGone("/tmp"), JSON.stringify(Sessions.folders));
        // Forgotten, one carried is offered no more, and only those
        // restore.json still names are kept forgotten.
        Sessions.forgetReopen({ id: "s-ferried" });
        expect("a forgotten one is offered no more",
            Sessions.toReopen.map(one => one.id).join(" ")
                === "s-before s-fresh s-given-back s-far s-ferry s-freed",
            JSON.stringify(Sessions.toReopen));
        Sessions.forgotten = ["s-long-gone"];
        Sessions.forgetReopen({ id: "s-ferried" });
        expect("what restore.json no longer names isn't kept forgotten",
            JSON.stringify(Sessions.forgotten) === '["s-ferried"]',
            JSON.stringify(Sessions.forgotten));
        Sessions.putOffLastBoot();
        expect("put off, none", Sessions.lastBoot.length === 0, JSON.stringify(Sessions.lastBoot));
        // Those apart are never put off with the last boot.
        expect("put off, the carried and given back still",
            Sessions.toReopen.map(one => one.id).join(" ") === "s-ferry s-freed",
            JSON.stringify(Sessions.toReopen));
        stage = "forgotten";
        tell({ gone: ["session:s-LIVE01"] });
    }

    // One the node says is gone goes.
    function forgotten() {
        if (addresses() !== "FARAWY NOWIN6 OTHER2 OUTRCH")
            return;
        // Told only that it's working now, it keeps the rest of what the
        // node said of it: running here, now in kitty 600's first window,
        // which nothing named.
        // Its terminal is read shut, so the node tells a new digest.
        stage = "differing";
        tell({
            changed: [{
                key: "session:s-NOWIN6",
                told: "0000000000000001",
                presence: { session: "s-NOWIN6", addresses: ["NOWIN6"], busy: true,
                    terminal: root.terminal("unix:@kitty-600", "1") }
            }]
        });
    }

    function differing() {
        const unseen = find("NOWIN6");
        if (!unseen || !unseen.presence.busy || !unseen.window)
            return;
        expect("a change saying only what differs keeps the rest", unseen.here
            && JSON.stringify(unseen.kitty) === '{"socket":"unix:@kitty-600","window":1}',
            JSON.stringify(unseen));
        // A window opened with no app id, as every one was before the shell
        // named them, is found by kitty's list: its first is niri's first.
        expect("of a kitty's several windows, one unnamed by kitty's own list",
            unseen.window.id === 20, unseen.window.id);
        // Then it rests, as a session's own work moves it, its window
        // the same, with the launcher showing it, since a shut one holds
        // a turn between working and listening back (PiSource.js).
        stage = "resting";
        retoldBefore = Object.assign({}, retold);
        PiSource.showing = true;
        tell({
            changed: [{
                key: "session:s-NOWIN6",
                presence: { session: "s-NOWIN6", addresses: ["NOWIN6"], busy: false,
                    terminal: root.terminal("unix:@kitty-600", "1") }
            }]
        });
    }

    // Only a running session moved, so what reads the parked and the
    // closed isn't told they did.
    function resting() {
        if (find("NOWIN6")?.presence.busy !== false)
            return;
        expect("a running session moving alone tells no parked or closed one",
            retold.parked === retoldBefore.parked && retold.closed === retoldBefore.closed,
            `parked told ${retold.parked - retoldBefore.parked}, closed `
            + `${retold.closed - retoldBefore.closed}`);
        PiSource.showing = false;
        // niri opening OTHER2's window anew, by the app id its record names,
        // is told by the windows daemon, and the session's window follows.
        const windows = Niri.windows;
        delete windows["21"];
        windows["23"] = { id: 23, pid: 600, title: "\u03c0 - Lighthouse", opened: "pi-session.7" };
        windowsTell({ changed: [windowRecord(23, 600, "pi-session.7")], gone: ["window:21"] });
        expect("a window opened anew by its app id is its session's once niri says so",
            find("OTHER2")?.window?.id === 23, JSON.stringify(find("OTHER2")?.window));
        // A window used is told with when, and the one used last goes
        // first where they're shown or picked, while the focus moving,
        // as it does while the launcher is shut, makes nothing again: no
        // windows by process, no sessions running.
        const inUse = () => Sessions.inUse(Sessions.running).map(session => session.address);
        const before = () => inUse().indexOf("NOWIN6") < inUse().indexOf("OTHER2");
        const nowin = find("NOWIN6").window;
        const made = root.windowsMade;
        const running = Sessions.running;
        windowsTell({ changed: [Object.assign({}, nowin, { order: 200000 }),
            windowRecord(23, 600, "pi-session.7", { order: 300000 })] });
        expect("the session whose window was used last goes first", !before(),
            inUse().join(" "));
        windowsTell({ changed: [Object.assign({}, nowin, { order: 400000 })] });
        expect("and the other once its window is used", before(), inUse().join(" "));
        expect("the focus moving makes nothing again",
            root.windowsMade === made && Sessions.running === running,
            `windows made ${root.windowsMade - made} times, running `
            + `${Sessions.running === running ? "kept" : "made again"}`);
        // A window retitled, or the keys leaving it, makes nothing again.
        windowsTell({ changed: [Object.assign({}, nowin, { order: 400000, title: "renamed",
            focused: false })] });
        expect("a window retitled makes no session's window again", root.windowsMade === made,
            `made ${root.windowsMade - made} times`);
        stage = "ended";
        tell({ changed: [{ key: "session:s-OTHER2", state: "ended", standing: "closed",
            told: "0000000000000002" }] });
    }

    // One the node says has ended goes, though its record stays.
    function ended() {
        if (addresses() !== "FARAWY NOWIN6 OUTRCH")
            return;
        stage = "lost";
        node.active = false;
        root.answering.connected = false;
    }

    // A node lost leaves what it said shown, never a moment with none.
    function lost() {
        expect("a node lost leaves what it said", addresses() === "FARAWY NOWIN6 OUTRCH",
            addresses());
        if (Sessions.node?.state !== "not answering")
            return;
        console.log("a session is known by its own record");
        Qt.callLater(() => Qt.exit(0));
    }

    property string unjoined: ""
    property string unhosted: ""
    property bool steered: false
    // Each steer the node was asked, and whether the one parked had been
    // woken by then.
    property var steers: []

    // A steer is asked of the node, whose refusal is said.
    Connections {
        target: Sessions

        function onUnjoined(session) {
            root.unjoined = session.address;
        }

        function onUnhosted(session) {
            root.unhosted = session.address;
        }

        function onUnsteered(session, why) {
            try {
                root.expect("a steer elsewhere is refused by the node, saying why",
                    session.address === "FARAWY" && session.machine === "lighthouse"
                    && why === "lighthouse is out of reach, so FARAWY can't be told now", why);
                const asked = root.steers.find(one => one.ask.address === "FARAWY")?.ask;
                root.expect("the node is asked with the steer, its address as the mesh writes it",
                    asked?.steer?.act === "pause" && asked?.steer?.from === "joel",
                    JSON.stringify(asked));
                root.steered = true;
                Qt.callLater(root.check);
            } catch (error) {
                root.fail(error.message);
            }
        }
    }

    // The node's end of the shell's watch, so the node can tell it more,
    // and be lost.
    property var answering: null

    // Tells the shell's watch a change, as the node does.
    function tell(change) {
        answering.write(`${JSON.stringify(Object.assign({ id: root.watchId }, change))}\n`);
        answering.flush();
    }
    property var watchId: null

    // A stand-in for the machine's node, listening only once asked, which
    // speaks the daemon interface as mycelium's does (its PROTOCOL.md, The
    // Daemon Interface): hello, a watch of sessions answered with each
    // whole, then each change saying only what differs, and a steer, which
    // it refuses for the one elsewhere, as for a machine out of reach.
    SocketServer {
        id: node
        active: false
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/harbour.sock`
        handler: Socket {
            id: asked
            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "node", state: "running" };
                    } else if (ask.ask === "watch" && ask.kind === "session"
                        && ask.keys === undefined) {
                        // The connection watching sessions, which pi's
                        // source makes beside its watches of runs and talks.
                        root.answering = asked;
                        root.watchId = ask.id;
                        answer = { id: ask.id, records: root.sessions() };
                    } else if (ask.ask === "watch" && (ask.kind === "run" || ask.kind === "talk")) {
                        answer = { id: ask.id, records: [] };
                    } else if (ask.ask === "steer") {
                        root.steers = root.steers.concat([
                            { ask: ask, wokenBefore: root.wokenSinceTalk() }]);
                        answer = ask.address === "FARAWY" ? { id: ask.id, error: {
                            code: "refused",
                            message: "lighthouse is out of reach, so FARAWY can't be told now"
                        } } : { id: ask.id, steered: { machine: Sessions.machine } };
                        Qt.callLater(root.check);
                    } else {
                        root.fail(`an ask the shell has no need of: ${line}`);
                        answer = { id: ask.id, error: { code: "unknown-ask", message: line } };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    function terminal(scope, window) {
        return { driverId: "kitty", kind: "kitty-window", hostId: "here", value: window,
            scope: scope };
    }

    // The sessions the node holds, as records of the daemon interface.
    function sessions() {
        const now = new Date().toISOString();
        const record = (id, fields) => Object.assign({
            key: `session:${id}`, kind: "session", heard: Date.now(), id: id, at: now,
            groups: [], title: "", window: null, held: null
        }, fields);
        // One here, standing as the node says: running, or closed once
        // ended, unless it's said otherwise.
        const here = (address, state, presence, standing) => record(`s-${address}`, {
            address: address, machine: Sessions.machine, here: true, state: state,
            standing: standing ?? (state === "ended" ? "closed" : "running"),
            presence: Object.assign({ session: `s-${address}`, addresses: [address] }, presence)
        });
        // One on another machine, running unless the node passes it over.
        const far = (id, address, state, at, attach, standing) => record(id, {
            address: address, machine: "lighthouse", here: false, state: state, at: at,
            attach: attach, standing: standing,
            presence: { addresses: [address], title: `\u03c0 - ${address} - /far`, busy: true }
        });
        return [
            here("LIVE01", "busy", { busy: true, title: "\u03c0 - Live One - core.nix",
                terminal: terminal("unix:@kitty-500", "3") }),
            // In the second of kitty 600's two windows, which the node
            // says was opened as pi-session.7.
            Object.assign(here("OTHER2", "idle", {
                title: "\u03c0 - Lighth\u2026 - wrong",
                questInfo: { id: "QEST-20261106-OTHER2", title: "Lighthouse - Keep It Lit",
                    short: "OTHER2", status: "active" },
                cwd: "/home/me/lamp/", terminal: terminal("unix:@kitty-600", "4")
            }), { window: { terminal: "kitty", id: "4", within: "unix:@kitty-600",
                app_id: "pi-session.7" } }),
            // In a kitty niri shows no window of.
            here("NOWIN6", "idle", { terminal: terminal("unix:@kitty-700", "2") }),
            here("ENDED3", "ended", {}),
            // Parked, as the node tells of one: ended, saying it's parked.
            Object.assign(here("NAPPER", "ended", { state: "parked", busy: false,
                title: "\u03c0 - Napper - core.nix" }, "parked"),
            { window: { terminal: "kitty", id: "1", within: "unix:@kitty-800",
                app_id: "pi-spawned.9" } }),
            // Closed, though each record says parked: its marker gone, or
            // laid before this boot, which the node sees and the shell
            // needn't.
            here("WOKEN8", "ended", { state: "parked" }),
            here("STALE7", "ended", { state: "parked",
                owner: { pid: 4242, bootId: "an-earlier-boot" } }),
            far("r-far", "FARAWY", "busy", now, "mycelium attach r-far", "running"),
            far("r-out", "OUTRCH", "unreachable", now, null, "running"),
            // Last heard of before this boot, so the node says nothing of
            // how it stands.
            far("r-old", "OLDONE", "busy", "2000-01-01T00:00:00Z", "mycelium attach r-old")
        ];
    }

    Timer {
        interval: 30000
        running: true
        onTriggered: root.fail(`still at ${root.stage}: ${root.addresses()}`)
    }
}

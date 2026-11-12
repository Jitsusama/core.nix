pragma ComponentBehavior: Bound
// Drives a session handed over while the screen is locked, beside its
// sender in the sessions' kitty (modules/quickshell/Sessions.qml), with
// the machine's node and niri stood in, and the windows daemon told as its
// watch is, the node telling of the sender's
// session, held in kitty 600, which draws two windows, before it has said
// which of them the sender is in. The handed-over session's window opens
// with the app id the shell gave it, and the lock is let go while the
// sender's window isn't known yet. It's held until the node says the
// sender's window, by the app id it opened with, then goes to the
// sender's workspace without the focus, rather than staying where it
// opened. It exits 0 once that holds, and otherwise 1, saying what didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    property string stage: "listed"

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    function sender() {
        return Sessions.running.find(session => session.address === "SENDER");
    }

    // A move of the handed-over window, as niri was asked for it.
    function moves() {
        return Niri.acts.filter(act => act.MoveWindowToWorkspace?.window_id === 22);
    }

    Connections {
        target: Sessions
        function onRunningChanged() {
            Qt.callLater(root.check);
        }
        function onLockLetGoChanged() {
            Qt.callLater(root.check);
        }
    }
    Connections {
        target: Niri
        function onSpawnsChanged() {
            Qt.callLater(root.check);
        }
        function onActsChanged() {
            Qt.callLater(root.check);
        }
    }

    // The windows as the windows daemon serves them (modules/windows), told
    // to Sessions' watch as the daemon tells it: the two niri's stand-in
    // lists (checks.nix), and later the one handed over.
    function windowRecord(id, opened, workspace) {
        return { key: `window:${id}`, id, pid: 600, opened, workspace, column: null, down: null,
            order: 0 };
    }
    function windowsTell(said) {
        Sessions.windowsFrom.said(JSON.stringify(Object.assign({ id: "all" }, said)));
    }
    Component.onCompleted: {
        windowsTell({ records: [windowRecord(20, "kitty", 2),
            windowRecord(21, "pi-session.3", 2)] });
        Qt.callLater(check);
    }

    function check() {
        try {
            if (stage === "listed")
                listed();
            else if (stage === "opening")
                opening();
            else if (stage === "letGo")
                letGo();
            else if (stage === "told")
                told();
        } catch (error) {
            fail(error.message);
        }
    }

    // The sender is listed, in a kitty of two windows and none named, so
    // its window isn't known; the lock is taken, and a session is handed
    // over beside the sender, as a quest's spawn hands one over.
    function listed() {
        const listed = sender();
        if (!listed)
            return;
        expect("of a kitty's two windows, neither named, the sender's isn't known",
            listed.window === null, JSON.stringify(listed.window));
        Sessions.behindTheLock = true;
        // Handed over fenced, as the harness hands over what a session was
        // started with (its #1345).
        const handed = { sessions: [{ id: "s-handed", cwd: "/tmp", from: "s-sender",
            env: { PI_SANDBOX: "fence", PI_SPAWNER: "on" } }] };
        const answer = Sessions.handedOver(JSON.stringify(handed));
        expect("handed over", answer === "taken", answer);
        stage = "opening";
    }

    // Its window opens in the sessions' kitty, on the workspace Joel was
    // on, with the app id the shell gave it; the lock is let go.
    function opening() {
        const spawn = Niri.spawns.find(command => command.includes("pi --session s-handed"));
        if (!spawn)
            return;
        // The shell's opening tells the window and its process the class
        // it's handed after the sessions kitty's socket.
        const classed = (/\/pi-sessions\/kitty\.sock (\S+) /.exec(spawn) ?? [])[1];
        expect("its window opens with an app id of its own, which its process is told",
            /^pi-(session|spawned)\.\d+$/.test(classed ?? "")
            && spawn.includes('--env "MYCELIUM_APP_ID=$class"'), spawn);
        expect("started again fenced, as it was handed over",
            spawn.includes(" /tmp env PI_SANDBOX=fence PI_SPAWNER=on zsh -lic "), spawn);
        const window = { id: 22, pid: 600, workspace_id: 1, session: true, opened: classed };
        Niri.windows = Object.assign({}, Niri.windows, { "22": window });
        Niri.revision += 1;
        windowsTell({ changed: [windowRecord(22, classed, 1)] });
        Niri.windowOpenedOrChanged(window);
        stage = "letGo";
        Sessions.behindTheLock = false;
    }

    // Once the lock is gone, with its sender's window not known yet, it
    // waits where it opened; then the node says the sender's window.
    function letGo() {
        if (!Sessions.lockLetGo)
            return;
        expect("the sender's window not known until the node says it",
            sender().window === null, JSON.stringify(sender().window));
        expect("not moved before its sender's window is known", moves().length === 0,
            JSON.stringify(moves()));
        stage = "told";
        // Its window is read shut, so the node tells a new digest.
        tell({ changed: [{ key: "session:s-sender", held: "h-sender", told: "0000000000000001",
            window: { terminal: "kitty", id: "4", within: "unix:@kitty-600",
                app_id: "pi-session.3" } }] });
    }

    // Then it goes to its sender's workspace, without the focus.
    function told() {
        if (moves().length === 0)
            return;
        expect("the sender's window is the one opened with its app id",
            sender().window?.id === 21, JSON.stringify(sender().window));
        expect("moved to its sender's workspace, without the focus",
            JSON.stringify(moves()) === JSON.stringify([{ MoveWindowToWorkspace: {
                window_id: 22, reference: { Id: 2 }, focus: false } }]),
            JSON.stringify(moves()));
        expect("nothing focused", !Niri.acts.some(act => act.FocusWindow
            || act.FocusWorkspace || act.FocusMonitor), JSON.stringify(Niri.acts));
        Qt.exit(0);
    }

    // The node's end of the shell's watch of sessions, so it can tell more.
    property var answering: null
    property var watchId: null

    // Tells the shell's watch a change, saying only what differs, as the
    // node does.
    function tell(change) {
        answering.write(`${JSON.stringify(Object.assign({ id: root.watchId }, change))}\n`);
        answering.flush();
    }

    // A stand-in for the machine's node, over the daemon interface, holding
    // the sender's session.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/harbour.sock`
        handler: Socket {
            id: asked
            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    const terminal = { driverId: "kitty", kind: "kitty-window",
                        hostId: "here", value: "4", scope: "unix:@kitty-600" };
                    const sender = { key: "session:s-sender", kind: "session",
                        heard: Date.now(), id: "s-sender", state: "idle",
                        standing: "running", here: true,
                        machine: Sessions.machine, at: new Date().toISOString(),
                        window: null, held: null, presence: { session: "s-sender",
                            addresses: ["SENDER"], terminal: terminal } };
                    let answer = { id: ask.id, version: 1, domain: "node", state: "running" };
                    if (ask.ask === "watch" && ask.kind === "session") {
                        root.answering = asked;
                        root.watchId = ask.id;
                        answer = { id: ask.id, records: [sender] };
                    } else if (ask.ask === "watch") {
                        answer = { id: ask.id, records: [] };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    Timer {
        running: true
        interval: 50000
        onTriggered: root.fail(`still at ${root.stage}: ${JSON.stringify(Niri.acts)}`)
    }
}

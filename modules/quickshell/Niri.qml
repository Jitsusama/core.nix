// The shell's connection to niri, over niri's own socket, so asking costs
// no process: one socket for telling niri what to do, and one that hears
// its events, for what the whole shell follows: the overview, and every
// window and workspace, which niri describes whole as the stream begins
// and then change by change.
//
// Programs the shell opens are spawned by niri rather than by the shell.
// niri gives each one a systemd scope of its own; one the shell started
// itself would leave its helpers in the shell's service, where restarting
// the shell kills them. Google's Chrome wrapper pipes Chrome's output
// through two of those helpers, so a switch that restarted the shell took
// down every Chrome and Slack opened from the launcher.
pragma Singleton
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // Requests asked for before the socket connects, sent once it does, and
    // what hears each one's answer, in the order niri answers them.
    property var waiting: []
    property var answers: []

    // Whether the overview is open. niri says so as the stream begins, and
    // whenever it changes.
    property bool overview: false

    // Every window and workspace by id, as niri's events describe them,
    // changed in place. `revision` counts the changes a list of them would
    // show, a window opening, closing, retitled or focused, so what's drawn
    // from them follows it; a window only moving or resizing says so through
    // layoutsChanged instead, since that happens many times a second.
    property var windows: ({})
    property var workspaces: ({})
    property var focused: null
    property int revision: 0

    // The window the keys were in, by id. niri says no window has focus
    // while a layer such as the launcher has the keys, so it's the focused
    // workspace's active window then; null on an empty workspace.
    readonly property var current: {
        revision;
        const workspace = root.workspace();
        const id = focused ?? (workspace ? workspace.active_window_id : null);
        return id ?? null;
    }

    // Every screencast by its stream's id: a program sharing the screen
    // through the portal (PipeWire), or one recording it straight from niri
    // (WlrScreencopy), as wf-recorder does. Replaced whole on each change,
    // since they're few and change seldom.
    property var casts: ({})

    // Whether niri's last load of its settings failed, which it says as the
    // stream begins and at every reload. niri then keeps the settings it
    // had, so nothing changed in them takes effect, and its own warning
    // shows for a few seconds only: on 2026-10-24 a running niri older than
    // its settings refused every change for four hours while Joel slept.
    // So a critical notice says so until a load succeeds, and why: the
    // installed niri accepting them means the running one is older, and a
    // new sign-in starts the newer; refusing them means they're wrong.
    property bool settingsFailed: false

    onSettingsFailedChanged: {
        const notice = '"$XDG_RUNTIME_DIR/niri-settings-notice"';
        spawn(["sh", "-c", settingsFailed ? [
            `if niri validate > /dev/null 2>&1; then`,
            `  why="Log out and back in to take the new ones, which need the newer window manager."`,
            `else`,
            `  why="The new ones have a mistake in them, which niri validate shows."`,
            `fi`,
            `replace=$(cat ${notice} 2> /dev/null)`,
            `notify-send --app-name=niri --urgency=critical --print-id \\`,
            `  \${replace:+--replace-id="$replace"} \\`,
            `  "The window manager kept its old settings" "$why" > ${notice}`
        ].join("\n") : [
            `[ -s ${notice} ] && busctl --user call org.freedesktop.Notifications \\`,
            `  /org/freedesktop/Notifications org.freedesktop.Notifications \\`,
            `  CloseNotification u "$(cat ${notice})"`,
            `rm -f ${notice}`
        ].join("\n")]);
    }

    // A window opened, or changed what it shows; one closed; some moved.
    signal windowOpenedOrChanged(var window)
    signal windowClosed(var id)
    signal layoutsChanged
    // niri took a screenshot, saved here unless it was only copied.
    signal screenshotCaptured(var path)

    // The workspace a window is on, or the focused one.
    function workspace(id: var): var {
        if (id !== undefined)
            return workspaces[id] ?? null;
        for (const key in workspaces)
            if (workspaces[key].is_focused)
                return workspaces[key];
        return null;
    }

    function hear(event: var): void {
        if (event.OverviewOpenedOrClosed) {
            overview = event.OverviewOpenedOrClosed.is_open;
        } else if (event.WindowsChanged) {
            const all = {};
            focused = null;
            for (const window of event.WindowsChanged.windows) {
                all[window.id] = window;
                if (window.is_focused)
                    focused = window.id;
            }
            windows = all;
            revision++;
        } else if (event.WindowOpenedOrChanged) {
            const window = event.WindowOpenedOrChanged.window;
            windows[window.id] = window;
            // One that's focused takes focus from every other, as niri's IPC
            // documents, without an event for each of them.
            if (window.is_focused) {
                focused = window.id;
                for (const id in windows)
                    windows[id].is_focused = windows[id].id === focused;
            }
            revision++;
            windowOpenedOrChanged(window);
        } else if (event.WindowClosed) {
            const id = event.WindowClosed.id;
            delete windows[id];
            if (focused === id)
                focused = null;
            revision++;
            windowClosed(id);
        } else if (event.WindowFocusChanged) {
            focused = event.WindowFocusChanged.id;
            for (const id in windows)
                windows[id].is_focused = windows[id].id === focused;
            revision++;
        } else if (event.WindowFocusTimestampChanged) {
            const change = event.WindowFocusTimestampChanged;
            if (windows[change.id])
                windows[change.id].focus_timestamp = change.focus_timestamp;
            // Lists of windows put the one used last first.
            revision++;
        } else if (event.WindowUrgencyChanged) {
            const change = event.WindowUrgencyChanged;
            if (windows[change.id])
                windows[change.id].is_urgent = change.urgent;
            revision++;
        } else if (event.WindowLayoutsChanged) {
            for (const [id, layout] of event.WindowLayoutsChanged.changes)
                if (windows[id])
                    windows[id].layout = layout;
            layoutsChanged();
        } else if (event.WorkspacesChanged) {
            const all = {};
            for (const workspace of event.WorkspacesChanged.workspaces)
                all[workspace.id] = workspace;
            workspaces = all;
            revision++;
        } else if (event.WorkspaceActivated) {
            const change = event.WorkspaceActivated;
            const output = workspaces[change.id] ? workspaces[change.id].output : null;
            for (const id in workspaces) {
                const workspace = workspaces[id];
                if (workspace.output === output)
                    workspace.is_active = workspace.id === change.id;
                if (change.focused)
                    workspace.is_focused = workspace.id === change.id;
            }
            revision++;
        } else if (event.CastsChanged) {
            const all = {};
            for (const cast of event.CastsChanged.casts)
                all[cast.stream_id] = cast;
            casts = all;
        } else if (event.CastStartedOrChanged) {
            const cast = event.CastStartedOrChanged.cast;
            casts = Object.assign({}, casts, { [cast.stream_id]: cast });
        } else if (event.CastStopped) {
            const all = Object.assign({}, casts);
            delete all[event.CastStopped.stream_id];
            casts = all;
        } else if (event.ConfigLoaded) {
            settingsFailed = event.ConfigLoaded.failed;
        } else if (event.ScreenshotCaptured) {
            screenshotCaptured(event.ScreenshotCaptured.path ?? null);
        } else if (event.WorkspaceActiveWindowChanged) {
            const change = event.WorkspaceActiveWindowChanged;
            if (workspaces[change.workspace_id])
                workspaces[change.workspace_id].active_window_id = change.active_window_id;
            revision++;
        }
    }

    function act(action: var): void {
        request({ Action: action }, null);
    }

    // Asks niri something, such as "Outputs", and hands what it says to
    // `answer`; or tells it something, with nothing to hear back.
    function request(asked: var, answer: var): void {
        waiting.push(JSON.stringify(asked));
        answers.push(answer);
        if (socket.connected)
            send();
        else
            socket.connected = true;
    }

    function spawn(command: var): void {
        act({ Spawn: { command: command } });
    }

    function send(): void {
        for (const request of waiting)
            socket.write(request + "\n");
        waiting = [];
        socket.flush();
    }

    // What's drawn from the windows names each one's program by its desktop
    // entry, and gives a program its windows by it (`revision`). Quickshell
    // reads the entries in the background as the shell starts, and niri
    // describes its windows as soon as the stream begins, so a list drawn
    // from that first description could name no program, and give none its
    // windows, until niri next said something. The entries arriving, or
    // changing, are a change those lists show.
    Connections {
        target: DesktopEntries

        function onApplicationsChanged(): void {
            root.revision++;
        }
    }

    Socket {
        id: socket
        path: Quickshell.env("NIRI_SOCKET")
        connected: true
        onConnectionStateChanged: {
            // Requests sent on a connection that closed are never answered,
            // so only the unsent keep a place in line.
            if (!connected)
                root.answers = root.answers.slice(root.answers.length - root.waiting.length);
            else if (root.waiting.length > 0)
                root.send();
        }
        parser: SplitParser {
            onRead: line => {
                const reply = JSON.parse(line);
                const answer = root.answers.shift();
                if (reply.Err)
                    console.warn("niri refused a request:", reply.Err);
                else if (answer)
                    answer(reply.Ok);
            }
        }
    }

    Socket {
        id: events
        path: Quickshell.env("NIRI_SOCKET")
        connected: true
        onConnectionStateChanged: {
            if (connected) {
                write('"EventStream"\n');
                flush();
            }
        }
        parser: SplitParser {
            onRead: line => {
                const event = JSON.parse(line);
                if (!("Ok" in event))
                    root.hear(event);
            }
        }
    }
}

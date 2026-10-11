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

    // Requests asked for before the socket connects, sent once it does.
    property var waiting: []

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

    // A window opened, or changed what it shows; one closed; some moved.
    signal windowOpenedOrChanged(var window)
    signal windowClosed(var id)
    signal layoutsChanged

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
            if (window.is_focused)
                focused = window.id;
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
        } else if (event.WorkspaceActiveWindowChanged) {
            const change = event.WorkspaceActiveWindowChanged;
            if (workspaces[change.workspace_id])
                workspaces[change.workspace_id].active_window_id = change.active_window_id;
        }
    }

    function act(action: var): void {
        waiting.push(JSON.stringify({ Action: action }));
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

    Socket {
        id: socket
        path: Quickshell.env("NIRI_SOCKET")
        connected: true
        onConnectionStateChanged: {
            if (connected && root.waiting.length > 0)
                root.send();
        }
        parser: SplitParser {
            onRead: line => {
                const reply = JSON.parse(line);
                if (reply.Err)
                    console.warn("niri refused a request:", reply.Err);
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

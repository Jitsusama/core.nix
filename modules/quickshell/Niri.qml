// The shell's connection to niri, over niri's own socket, so asking costs
// no process, and what the whole shell follows of it: the overview, and
// every window and workspace, held as the windows daemon (modules/windows)
// tells them, in niri's own shape (NiriRecords.js), rather than heard from
// niri's own event stream. niri says every tile moving at each resize and
// scroll, and each window's every retitle and use, and the shell woke for
// each, launcher shut or open; the daemon tells only the fields read here,
// and tiles only while something draws them (wantTiles), so the shell
// wakes for less than niri says. While the daemon isn't answering, niri's
// own stream stands in for it, so nothing here stops for a daemon down.
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
import Quickshell.Wayland
import QtQuick
import "NiriRecords.js" as NiriRecords

Singleton {
    id: root

    // How a surface Joel opens asks niri for the keys: on demand, once
    // niri says it gives them as a mapped surface asks
    // (modules/niri/on-demand-when-asked.patch), so the newest he opened
    // has them and a click on a window takes them; exclusively until then.
    // A switch restarts the shell under the niri already running, and an
    // older niri never gives a mapped surface the keys on demand.
    readonly property int asksForKeys: Quickshell.env("NIRI_ON_DEMAND_WHEN_ASKED") === "1"
        ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.Exclusive

    // Whether niri powers one output off by name, keeping it docked
    // (modules/niri/powers-off-each-output.patch), and says which are off
    // as each output's powered_off. Asked of the niri running, as the keys
    // are, since a switch restarts the shell under the niri it had.
    readonly property bool powersOffEach: Quickshell.env("NIRI_POWERS_OFF_EACH_OUTPUT") === "1"

    // Requests asked for before the socket connects, sent once it does, and
    // what hears each one's answer, in the order niri answers them.
    property var waiting: []
    property var answers: []

    // Whether the overview is open. niri says so as the stream begins, and
    // whenever it changes.
    property bool overview: false

    // Every window and workspace by id, as the windows daemon tells them,
    // in niri's shape, changed in place. `revision` counts the changes a list of them would
    // show, a window opening, closing, retitled or used, so what's drawn
    // from them follows it; a window only moving or resizing says so through
    // layoutsChanged instead, since that happens many times a second. The
    // keys moving, to a window or to a workspace's other window, change
    // only `focused` and `current`, which say so themselves: nothing lists
    // by a window's is_focused, yet those were nearly half of what niri said
    // as Joel worked (17 of 36 events in a minute, 2026-11-10), and each
    // made every list again, Sessions' every session among them. A window
    // used, which niri says a moment after the keys stay in it, is kept in
    // place and said to nothing: what puts the window used last first reads
    // it from the windows daemon (modules/windows), and what reads it here
    // reads it when it's asked, as Forward.js does.
    property var windows: ({})
    property var workspaces: ({})
    property var focused: null
    property int revision: 0
    // Each program's newest window, by its app id in lower case, made again
    // only as the windows a list shows change, so a row going to a program
    // looks it up rather than walking every window (NiriRecords.js's byApp).
    readonly property var byApp: {
        void revision;
        return NiriRecords.byApp(windows);
    }

    // The window the keys were in, by id. niri says no window has focus
    // while a layer such as the launcher has the keys, so it's the focused
    // workspace's active window then; null on an empty workspace.
    // Followed after each event, and set only when it differs, so what
    // reads it wakes only as it moves.
    property var current: null
    function follow(): void {
        const workspace = root.workspace();
        const id = focused ?? (workspace ? workspace.active_window_id : null) ?? null;
        if (current !== id)
            current = id;
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

    // A window opened, changed what it shows, or given the keys; one
    // closed; some moved in their layout, or, while tiles are wanted, any
    // tile did.
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

    // A window as the shell knows it. A kitty window opened for a session is
    // classed apart, with a number of its own (Sessions.qml's inSessions):
    // pi-spawned for one another session started, so niri's rule opens it
    // without taking the focus from Joel (modules/niri/config.kdl), and
    // pi-session for the rest; a hosted conversation's viewer is pi-hosted
    // with its session's id (NiriRecords.js's isSessionApp, as the windows
    // daemon judges it). Each is kitty's all the same to every list,
    // mark and key the shell has, and says it holds a session (`session`).
    // The app id it opened with is kept as `opened`, which names it apart
    // from every other window of the same kitty.
    function heard(window: var): var {
        const app = window.app_id ?? "";
        window.opened = app;
        if (NiriRecords.isSessionApp(app)) {
            window.app_id = "kitty";
            window.session = true;
        }
        return window;
    }

    // The windows daemon's socket, and what it tells, as NiriRecords
    // holds it: Niri.qml's windows, workspaces and casts are the ones held
    // there, published here as they change.
    readonly property string daemonSocket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
    property var kept: NiriRecords.held()

    // What draws tiles, by name, as notices do while they're up, beside
    // the window under them; the daemon tells tiles only while one does.
    property var tilesFor: ({})
    readonly property bool tilesWanted: Object.keys(tilesFor).length > 0
    // Whether the tiles held are current: once the daemon's watch of them
    // has answered, or while niri's own stream, which says them, stands in.
    readonly property bool tilesReady: standIn || tilesHeard.answeredThisWatch
    function wantTiles(who: string, wanted: bool): void {
        if ((who in tilesFor) === wanted)
            return;
        const now = Object.assign({}, tilesFor);
        if (wanted)
            now[who] = true;
        else
            delete now[who];
        tilesFor = now;
    }

    // What sits over one window's tile, by name, as the cat sits over the
    // window with the keys: that tile alone is told, so a window resized
    // or scrolled beside it wakes nothing (2026-11-12: the cat watching
    // every tile made the shut shell hear each of 20 resizes).
    property var tileFor: ({})
    readonly property var tilesAhead: Array.from(new Set(Object.values(tileFor)))
        .map(id => `layout:${id}`).sort()
    function wantTile(who: string, id: var): void {
        const wanted = id === undefined ? null : id;
        if ((tileFor[who] ?? null) === wanted)
            return;
        const now = Object.assign({}, tileFor);
        if (wanted === null)
            delete now[who];
        else
            now[who] = wanted;
        tileFor = now;
    }

    // A tile no watch follows is let go, as it would be wrong once its
    // window moved unheard: sitting over it later waits the moment its
    // watch takes to answer, rather than drawing where it was.
    function forgetTiles(): void {
        if (tilesWanted)
            return;
        const following = new Set(tilesAhead);
        const stale = Object.keys(tilesHeard.held).filter(key => !following.has(key));
        if (stale.length > 0)
            tilesHeard.took([], stale, false);
    }
    onTilesAheadChanged: forgetTiles()
    onTilesWantedChanged: forgetTiles()

    // Whether niri's own stream stands in for the daemon, since it isn't
    // answering, so what it held is made again from the daemon's whole
    // answer once it is.
    readonly property bool standIn: windowsHeard.state === "not answering"
    property bool stoodIn: false
    onStandInChanged: {
        if (!standIn)
            return;
        stoodIn = true;
        console.warn("niri: the windows daemon isn't answering; niri's own events stand in");
    }

    // What the daemon told, the keys changed and those gone, taken in and
    // said as niri's events said it.
    function took(daemon: var, changed: var, gone: var): void {
        if (standIn)
            return;
        const records = changed.map(key => daemon.held[key]).filter(record => record);
        root.publish(NiriRecords.take(kept, records, gone));
    }

    // Everything the daemon holds, taken anew, as once it answers again
    // after niri's stream stood in.
    function retake(): void {
        stoodIn = false;
        kept = NiriRecords.held();
        const records = [windowsHeard, placesHeard, tilesHeard]
            .reduce((all, daemon) => all.concat(Object.values(daemon.held)), []);
        const told = NiriRecords.take(kept, records, []);
        told.listed = true;
        root.publish(told);
    }

    function publish(told: var): void {
        if (windows !== kept.windows)
            windows = kept.windows;
        if (workspaces !== kept.workspaces)
            workspaces = kept.workspaces;
        if (told.casts)
            casts = kept.casts;
        overview = kept.overview;
        settingsFailed = kept.settingsFailed;
        if (focused !== kept.focused)
            focused = kept.focused;
        if (told.listed)
            revision++;
        follow();
        for (const window of told.opened)
            windowOpenedOrChanged(window);
        for (const id of told.closed)
            windowClosed(id);
        if (told.layouts)
            layoutsChanged();
        if (told.shot)
            screenshotCaptured(told.path);
    }

    // Each window's fields read here, told only as one changes.
    Daemon {
        id: windowsHeard

        socket: root.daemonSocket
        kind: "window"
        watching: ({ fields: NiriRecords.WINDOW_FIELDS })
        answersWhole: true
        onHeard: (changed, gone) => root.took(windowsHeard, changed, gone)
        onAnsweredThisWatchChanged: {
            if (answeredThisWatch && root.stoodIn)
                root.retake();
        }
    }

    // The workspaces, the window niri says has the keys, the desktop's
    // overview and settings, the casts and the last screenshot, few and
    // seldom changed, so each is told whole.
    Daemon {
        id: placesHeard

        socket: root.daemonSocket
        kind: ["workspace", "focus", "desktop", "cast", "screenshot"]
        answersWhole: true
        onHeard: (changed, gone) => root.took(placesHeard, changed, gone)
    }

    // Each window's tile, only while something draws tiles, and the one
    // something sits over.
    Daemon {
        id: tilesHeard

        socket: root.daemonSocket
        kind: "layout"
        live: root.tilesWanted
        ahead: root.tilesAhead
        answersWhole: true
        onHeard: (changed, gone) => root.took(tilesHeard, changed, gone)
    }

    // niri's own events, as its stream says them, while the daemon isn't
    // answering.
    function hear(event: var): void {
        if (event.OverviewOpenedOrClosed) {
            overview = event.OverviewOpenedOrClosed.is_open;
        } else if (event.WindowsChanged) {
            const all = {};
            focused = null;
            for (const window of event.WindowsChanged.windows) {
                all[window.id] = root.heard(window);
                if (window.is_focused)
                    focused = window.id;
            }
            windows = all;
            revision++;
        } else if (event.WindowOpenedOrChanged) {
            const window = root.heard(event.WindowOpenedOrChanged.window);
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
        }
        follow();
    }

    function act(action: var): void {
        request({ Action: action }, null, null);
    }

    // Asks niri something, such as "Outputs", and hands what it says to
    // `answer`; or tells it something, with nothing to hear back. Where
    // niri refuses it, `refused` is handed niri's own words for why, so
    // what asked can say so; where nothing asked to hear, the shell's log
    // has them.
    function request(asked: var, answer: var, refused: var): void {
        waiting.push(JSON.stringify(asked));
        answers.push({ answer: answer ?? null, refused: refused ?? null });
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
                const asked = root.answers.shift() ?? { answer: null, refused: null };
                if (reply.Err === undefined) {
                    if (asked.answer)
                        asked.answer(reply.Ok);
                } else if (asked.refused) {
                    asked.refused(String(reply.Err));
                } else {
                    console.warn("niri refused a request:", reply.Err);
                }
            }
        }
    }

    Socket {
        id: events
        path: Quickshell.env("NIRI_SOCKET")
        connected: root.standIn
        onConnectionStateChanged: {
            if (connected) {
                write('"EventStream"\n');
                flush();
            }
        }
        parser: SplitParser {
            onRead: line => {
                root.streamHeard += 1;
                const event = JSON.parse(line);
                if (!("Ok" in event))
                    root.hear(event);
            }
        }
    }

    // How many lines niri's stream has said while it stood in.
    property int streamHeard: 0

    // Every line heard of niri's windows and places, from the windows
    // daemon or niri's own stream, for the check that the shut shell hears
    // nothing of a window resized.
    function linesHeard(): int {
        return windowsHeard.linesHeard + placesHeard.linesHeard + tilesHeard.linesHeard
            + streamHeard;
    }
}

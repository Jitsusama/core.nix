// Opens the next window just below the one Joel had focused, in its column,
// so windows stack into a grid as they open. Every other window opens as
// niri opens it, in a column of its own.
//
// niri can't be told where a window will open (its FAQ, "Can I open a window
// directly in the current column"), so this watches niri's events for the
// new window, folds it into the column on its left, which is the focused
// one, and moves it up until it sits just below the window that was
// focused. It talks to niri over niri's own socket rather than running
// anything, listens only while it waits, and gives up after ten seconds or
// as soon as the new window isn't where it expects, leaving it as niri put
// it.
//
// It isn't called Grid, because QtQuick has a Grid of its own, and a file
// importing QtQuick would get that one instead without a word.
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: root

    property bool waiting: false
    // Every window niri has, by id, as its events describe them.
    property var windows: ({})
    // The windows there were when it began to wait, so the new one stands out.
    property var known: null
    // The window that was focused when asked, which the new one goes below.
    property var anchor: null
    // The new window, once it's being folded into the anchor's column.
    property var placing: null
    property var focused: null

    function below(program: string): void {
        windows = {};
        known = null;
        anchor = null;
        placing = null;
        waiting = true;
        expiry.restart();
        if (program !== "")
            Niri.spawn([program]);
    }

    function finish(): void {
        waiting = false;
        expiry.stop();
    }

    function position(id: var): var {
        const window = windows[id];
        return window ? window.layout.pos_in_scrolling_layout : null;
    }

    // niri opens a window in a new column to the right of the focused one,
    // so a tiled window on the anchor's workspace, one column to its right,
    // is folded in; anything else is left where niri put it.
    function opened(window: var): void {
        const anchored = windows[anchor];
        const here = position(anchor);
        const there = window.layout.pos_in_scrolling_layout;
        const beside = anchored && here && there && !window.is_floating
            && window.workspace_id === anchored.workspace_id && there[0] === here[0] + 1;
        if (!beside) {
            finish();
            return;
        }
        placing = window.id;
        Niri.act({ ConsumeOrExpelWindowLeft: { id: window.id } });
    }

    // Folded in, the new window sits at the bottom of the column, so it
    // moves up to just below the anchor. niri moves only the focused window
    // up, and the new one has focus unless something took it, in which case
    // it stays at the bottom rather than moving the wrong window.
    function settle(): void {
        const here = position(anchor);
        const there = position(placing);
        if (!here || !there || there[0] !== here[0])
            return;
        if (focused === placing)
            for (let move = there[1] - (here[1] + 1); move > 0; move--)
                Niri.act({ MoveWindowUp: {} });
        finish();
    }

    function handle(event: var): void {
        if (event.WindowsChanged) {
            const all = {};
            for (const window of event.WindowsChanged.windows) {
                all[window.id] = window;
                if (window.is_focused)
                    focused = window.id;
            }
            windows = all;
            if (known === null) {
                known = Object.assign({}, all);
                anchor = focused;
                if (anchor === null)
                    finish();
            }
        } else if (event.WindowOpenedOrChanged) {
            const window = event.WindowOpenedOrChanged.window;
            windows[window.id] = window;
            if (window.is_focused)
                focused = window.id;
            if (known !== null && placing === null && !(window.id in known))
                opened(window);
            else if (window.id === placing)
                settle();
        } else if (event.WindowLayoutsChanged) {
            for (const [id, layout] of event.WindowLayoutsChanged.changes)
                if (windows[id])
                    windows[id].layout = layout;
            if (placing !== null)
                settle();
        } else if (event.WindowFocusChanged) {
            focused = event.WindowFocusChanged.id;
        } else if (event.WindowClosed) {
            const id = event.WindowClosed.id;
            delete windows[id];
            if (id === anchor || id === placing)
                finish();
        }
    }

    Timer {
        id: expiry
        interval: 10000
        onTriggered: root.finish()
    }

    // niri's event stream, open only while waiting. Its first events
    // describe every window, which is how the anchor is found. Actions go
    // through Niri, since a connection that has asked for the event stream
    // answers nothing else.
    Socket {
        path: Quickshell.env("NIRI_SOCKET")
        connected: root.waiting
        onConnectionStateChanged: {
            if (connected) {
                write("\"EventStream\"\n");
                flush();
            }
        }
        parser: SplitParser {
            onRead: line => {
                const event = JSON.parse(line);
                if (!("Ok" in event))
                    root.handle(event);
            }
        }
    }

    // qs ipc call grid below kitty opens kitty below the focused window; an
    // empty name places whichever window opens next.
    IpcHandler {
        target: "grid"

        function below(program: string): void {
            root.below(program);
        }
    }
}

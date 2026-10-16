// Opens the next window just below the one Joel had focused, in its column,
// so windows stack into a grid as they open. Every other window opens as
// niri opens it, in a column of its own.
//
// niri can't be told where a window will open (its FAQ, "Can I open a window
// directly in the current column"), so this watches for the new window in
// what Niri hears, folds it into the column on its left, which is the
// focused one, and moves it up until it sits just below the window that was
// focused. It talks to niri over niri's own socket rather than running
// anything, and gives up after ten seconds or as soon as the new window
// isn't where it expects, leaving it as niri put it.
//
// It isn't called Grid, because QtQuick has a Grid of its own, and a file
// importing QtQuick would get that one instead without a word.
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: root

    property bool waiting: false
    // The windows there were when it began to wait, so the new one stands out.
    property var known: ({})
    // The window that was focused when asked, which the new one goes below.
    property var anchor: null
    // The new window, once it's being folded into the anchor's column.
    property var placing: null

    function below(program: string): void {
        under(Niri.current, program === "" ? [] : [program]);
    }

    // Opens what the command runs below the window given, focusing it first
    // when it isn't, since niri opens a window beside the focused one; an
    // empty command places whichever window opens next.
    function under(window: var, command: var): void {
        known = Object.assign({}, Niri.windows);
        anchor = window;
        placing = null;
        waiting = anchor !== null;
        if (waiting) {
            expiry.restart();
            if (Niri.focused !== anchor)
                Niri.act({ FocusWindow: { id: anchor } });
        }
        if (command.length > 0)
            Niri.spawn(command);
    }

    function finish(): void {
        waiting = false;
        expiry.stop();
    }

    function position(id: var): var {
        const window = Niri.windows[id];
        return window && window.layout ? window.layout.pos_in_scrolling_layout : null;
    }

    // niri opens a window in a new column to the right of the focused one,
    // so a tiled window on the anchor's workspace, one column to its right,
    // is folded in; anything else is left where niri put it.
    function opened(window: var): void {
        const anchored = Niri.windows[anchor];
        const here = position(anchor);
        const there = window.layout ? window.layout.pos_in_scrolling_layout : null;
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
        if (Niri.focused === placing)
            for (let move = there[1] - (here[1] + 1); move > 0; move--)
                Niri.act({ MoveWindowUp: {} });
        finish();
    }

    Connections {
        target: Niri
        enabled: root.waiting

        function onWindowOpenedOrChanged(window) {
            if (root.placing === null && !(window.id in root.known))
                root.opened(window);
            else if (window.id === root.placing)
                root.settle();
        }

        function onLayoutsChanged() {
            if (root.placing !== null)
                root.settle();
        }

        function onWindowClosed(id) {
            if (id === root.anchor || id === root.placing)
                root.finish();
        }
    }

    Timer {
        id: expiry
        interval: 10000
        onTriggered: root.finish()
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

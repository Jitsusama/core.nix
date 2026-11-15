// What a key's sources found after its search, gathered and taken in once
// the shell is back at its event loop, so answers heard together search
// again together (Launcher.qml's foundMore): each searched every source
// again and laid the list out, four times in 55 ms after Joel's "kee",
// holding his next key (perf on his shell, 2026-11-14). tests/gathering.qml
// holds it in Qt's own engine.
import QtQuick

QtObject {
    id: gathering

    // What's gathered is taken in, for what was typed as the last came, and
    // fresh when any of them came fresh.
    signal takeIn(bool fresh, string text)

    property var due: null

    // A source found more for `text`.
    function found(fresh, text) {
        gathering.due = { fresh: fresh || (gathering.due?.fresh ?? false), text: text };
        Qt.callLater(gathering.takeInNow);
    }

    // A search under way, a key's own, takes in all that's gathered, so none
    // is left waiting after it.
    function drop() {
        gathering.due = null;
    }

    function takeInNow() {
        const due = gathering.due;
        gathering.due = null;
        if (due !== null)
            gathering.takeIn(due.fresh, due.text);
    }
}

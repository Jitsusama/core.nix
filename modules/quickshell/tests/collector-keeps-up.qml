// Whether the collector keeps up with one long call, in the Qt the shell is
// built with (modules/quickshell/qtdeclarative-collector-keeps-up.patch). The
// shell hears the clipboard history's Emoji in one call of about a second at
// its start, and Qt 6.12.0 steps a cycle only from the event loop, so every
// chunk that call ran out of was a new one, each left holding a few of the
// rows it kept: in the weigh lab the heap started at 143 MB, against 73 with
// the patch, and stayed about 50 MB heavier through the churn after. The
// `the-collector-keeps-up` check runs this as the shell: with a heap already
// holding what a started shell holds, one call making 1.5 million objects
// around a scattered few it keeps grows its memory by under 300 MB (207 on
// a builder), where Qt 6.12.0 grew it by 419. It exits 1, saying what was wrong, if it
// doesn't.
import Quickshell
import QtQuick
import "Memory.js" as Memory

ShellRoot {
    // What a started shell holds, its rows and their parts, made before the
    // call so the collector has a heap to walk that takes it longer than a
    // step of its own.
    property var standing: []

    Component.onCompleted: {
        for (let at = 0; at < 300000; at += 1)
            standing.push({ key: `row ${at}`, at: at, parts: [at, at + 1] });
        gc();
        Qt.callLater(hear);
    }

    // One long call, as the shell's hearing of an answer is: what it keeps
    // is one in fifty of what it makes, so every chunk holds a few.
    function hear() {
        const before = Memory.anonymous();
        const kept = [];
        for (let at = 0; at < 1500000; at += 1) {
            const made = { key: `made ${at}`, at: at, parts: [at, at + 1] };
            if (at % 50 === 0)
                kept.push(made);
        }
        gc();
        const grew = Math.round((Memory.anonymous() - before) / (1024 * 1024));
        if (kept.length !== 30000 || standing.length !== 300000) {
            console.error(`kept ${kept.length} and stood ${standing.length}`);
            Qt.callLater(() => Qt.exit(1));
            return;
        }
        if (grew >= 300) {
            console.error(`one long call grew memory by ${grew} MB`);
            Qt.callLater(() => Qt.exit(1));
            return;
        }
        console.info(`the collector keeps up: one long call grew memory by ${grew} MB`);
        Qt.callLater(() => Qt.exit(0));
    }
}

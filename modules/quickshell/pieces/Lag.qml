// The desktop or its apps kept waiting, from what lag-watch writes of each
// ten seconds (modules/lag-watch): true once the same thing, memory, the
// disk, the processor or niri, has kept them waiting a minute, since a
// moment's wait Joel feels for himself and a mark for it would only come
// and go. Work waiting on work is never counted. It's a quiet mark, left on
// rather than wanting him, and Now says what waited in lag-watch's
// words; the launcher's "lag" answers with the rest (launcher/Waits.qml).
// Nothing ticks: each window lag-watch writes is heard from the machine
// daemon as it's renamed in (Watchers.qml).
import qs
import QtQuick
import "../Marks.js" as Marks
import "Drawers.js" as Drawers

Piece {
    id: piece

    // A window that ended long ago is lag-watch gone, not a wait still
    // going, and the machine daemon says so once none has ended for 30
    // seconds, since one ends every ten (Watchers.qml).
    readonly property var window: Watchers.lagSilent ? null : Watchers.lag
    // { worst, since } while the same thing keeps them waiting.
    property var held: null
    // How long it has to last, in seconds.
    readonly property real lasts: settings.lasts ?? 60
    readonly property var named: ({
        cpu: "the processor",
        memory: "memory",
        disk: "the disk",
        niri: "niri"
    })
    readonly property string what: held === null ? "" : named[held.worst] ?? held.worst
    readonly property string who: window?.who === "desktop" ? "The desktop has" : "Programs have"
    property bool lasting: false

    onWindowChanged: {
        const at = Date.now() / 1000;
        held = Marks.lasted(window, held, at);
        lasting = held !== null && at - held.since >= lasts;
        // What the test waits on before writing the next window, since two
        // written together would be heard as one.
        Report.say(`heard lag ${held?.worst ?? "nothing"}`);
    }

    present: window !== null
    active: lasting
    kind: "leftOn"
    value: lasting ? `waiting on ${what}` : "nothing waits"
    title: "Lag"
    // What lag-watch said of its last ten seconds, under Now's row
    // (Drawers.js): how long they waited, how each slice stalled, the swap
    // held, and who waited on what in our own sentence, never lag-watch's
    // (VKCF3Z, 2026-11-08).
    drawer: window === null ? null : () => Object.assign({ title: piece.title },
        Drawers.lag(piece.window, piece.lasting ? piece.what : "", {
            named: id => Words.name(id),
            said: Words.of("on:lag").said,
            tells: Words.of("on:lag").tells
        }))
    // Held and lasting change one after the other, so each is asked.
    hint: lasting && held !== null ? `${who} waited on ${what} for ${Math.round((Date.now()
        / 1000 - held.since) / 60)} min` : value
}

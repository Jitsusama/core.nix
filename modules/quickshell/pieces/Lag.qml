// The desktop or its apps kept waiting, from what lag-watch writes of each
// ten seconds (modules/lag-watch): true once the same thing, memory, the
// disk, the processor or niri, has kept them waiting a minute, since a
// moment's wait Joel feels for himself and a mark for it would only come
// and go. Work waiting on work is never counted. It's a quiet mark, left on
// rather than wanting him, and What's on says what waited in lag-watch's
// words; the launcher's "lag" answers with the rest (launcher/Waits.qml).
// Nothing ticks: each window lag-watch writes is heard as it's renamed in.
import qs
import Quickshell
import QtQuick
import "../Marks.js" as Marks

Piece {
    id: piece

    readonly property var window: file.silent ? null : file.said
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
    // Held and lasting change one after the other, so each is asked.
    hint: lasting && held !== null ? `${who} waited on ${what} for ${Math.round((Date.now()
        / 1000 - held.since) / 60)} min` : value
    context: (window?.trouble ?? [])[0] ?? ""

    StatusFile {
        id: file
        path: piece.settings.path ?? `${Quickshell.env("XDG_RUNTIME_DIR")}/lag-watch/status.json`
        // A window ends every ten seconds, and one that ended long ago is
        // lag-watch gone, not a wait still going.
        silence: 30
        parse: text => {
            try {
                const said = JSON.parse(text);
                return typeof said === "object" && said !== null
                    ? Object.assign({}, said, { heartbeat: Number(said.ended ?? 0) }) : null;
            } catch (error) {
                return null;
            }
        }
    }
}

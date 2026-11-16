pragma Singleton
// How each part of the system stands, as the system daemon judges it
// (modules/system): its `part:<name>` records, by name, so a place the
// vocabulary ties to a part (a category's `part`) says from it how it
// stands (launcher/SystemPart.js). Empty while the daemon isn't
// answering, so no old judgement is shown as current, and a machine
// without the daemon has nothing to say.
//
// The shell holds the daemon's socket from the moment it starts, as it
// holds every daemon's (Daemon.qml), and watches parts always. That costs
// the shut shell nothing beyond a change heard: the daemon reads on its
// own and says only what changed, so nothing here polls.
import Quickshell
import QtQuick

Singleton {
    id: root

    // Each part's record as the daemon last told it, by its name.
    property var parts: ({})
    // Whether the daemon is answering, so a part it hasn't said yet reads
    // as unsaid rather than as not read.
    readonly property bool heard: daemon.answeredThisWatch

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/system.sock`
        kind: "part"
        keptThroughGap: false
        onHeard: (changed, gone) => {
            const parts = {};
            for (const key in daemon.held) {
                const record = daemon.held[key];
                if (record?.kind === "part" && typeof record.part === "string")
                    parts[record.part] = record;
            }
            root.parts = parts;
            // What it heard, so a test sees the launcher's half apart from
            // the daemon's.
            Report.say(`system parts ${Object.keys(parts).sort().map(name =>
                `${name} ${parts[name].health}`).join(", ")}`);
        }
    }
}

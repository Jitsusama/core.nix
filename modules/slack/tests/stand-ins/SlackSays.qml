// What the stand-in Slack daemon holds (Daemon.qml beside it), set by
// tests/read-there.qml, by key as the shell's Daemon.qml holds records:
// each record told replaces its key's, each key gone is let go, and the
// change is heard once with both.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    readonly property var held: ({})
    property int revision: 0

    signal heard(var changed, var gone)

    function tell(records, gone) {
        for (const key in records)
            held[key] = records[key];
        for (const key of gone ?? [])
            delete held[key];
        revision += 1;
        heard(Object.keys(records), gone ?? []);
    }
}
